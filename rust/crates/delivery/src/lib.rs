//! Apply reviewed plans locally, or reconcile an explicitly requested GitHub proposal.
use anyhow::{Context, Result, ensure};
use cm_catalog::*;
use cm_exchange_plan::{Kind, Plan, hash_at, read};
use cm_github::{Api, encode, read_file, repo_path, required, resolve, tree_sha};
use serde_json::{Value, json};
use std::{collections::BTreeSet, fs, path::Path};
/// Validate every precondition before writes; roll back completed writes on an I/O error.
/// The repository lock coordinates this tool, not arbitrary external editors.
pub fn apply(root: &Path, plan: &Plan) -> Result<usize> {
    plan.validate()?;
    let lock_path = checked_path(root, ".cm-testsuite-write-lock")?;
    let lock=fs::OpenOptions::new().write(true).create_new(true).open(&lock_path).context("another apply may be active; inspect .cm-testsuite-write-lock before removing a stale lock")?;
    struct Lock(std::path::PathBuf, fs::File);
    impl Drop for Lock {
        fn drop(&mut self) {
            let _ = self.1.sync_all();
            let _ = fs::remove_file(&self.0);
        }
    }
    let _lock = Lock(lock_path, lock);
    let desired = |e: &cm_exchange_plan::Edit| e.after.as_ref().map(|s| sha256(s.as_bytes()));
    let already = plan
        .edits
        .iter()
        .map(|e| Ok(hash_at(root, &e.path)? == desired(e)))
        .collect::<Result<Vec<_>>>()?;
    if already.iter().all(|x| *x) {
        return Ok(0);
    }
    for guard in &plan.guards {
        ensure!(
            hash_at(root, &guard.path)? == guard.sha256,
            "stale plan: {} changed",
            guard.path
        );
    }
    let scratch = tempfile::Builder::new()
        .prefix(".cm-testsuite-stage-")
        .tempdir_in(root)?;
    let mut originals = vec![];
    for (i, e) in plan.edits.iter().enumerate() {
        ensure!(
            hash_at(root, &e.path)? == e.before,
            "stale plan or local edits: {}",
            e.path
        );
        originals.push(read(root, &e.path)?);
        if let Some(text) = &e.after {
            fs::write(scratch.path().join(i.to_string()), text)?;
        }
    }
    let mut completed = 0;
    let result = (|| -> Result<()> {
        for (i, e) in plan.edits.iter().enumerate() {
            let path = checked_path(root, &e.path)?;
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)?;
            }
            ensure!(
                hash_at(root, &e.path)? == e.before,
                "file changed during apply: {}",
                e.path
            );
            if e.after.is_some() {
                fs::rename(scratch.path().join(i.to_string()), path)?;
            } else {
                fs::remove_file(path)?;
            }
            completed = i + 1;
        }
        Ok(())
    })();
    if let Err(error) = result {
        let mut failures = vec![];
        for (i, original) in originals.iter().enumerate().take(completed).rev() {
            let e = &plan.edits[i];
            let rollback = (|| -> Result<()> {
                ensure!(
                    hash_at(root, &e.path)? == desired(e),
                    "external edit during rollback"
                );
                let path = checked_path(root, &e.path)?;
                match original {
                    Some(bytes) => fs::write(path, bytes)?,
                    None => fs::remove_file(path)?,
                };
                Ok(())
            })();
            if let Err(e) = rollback {
                failures.push(e.to_string());
            }
        }
        ensure!(
            failures.is_empty(),
            "apply failed ({error}); rollback needs manual recovery: {failures:?}"
        );
        return Err(error);
    }
    Ok(plan.edits.len())
}
#[derive(Debug, PartialEq, Eq)]
pub enum Publication {
    NoChanges,
    Declined,
    Closed(String),
    PullRequest(String),
}
fn remote_hash(api: &dyn Api, source: &str, rev: &str, path: &str) -> Result<Option<String>> {
    Ok(read_file(api, source, rev, path)?.map(|b| sha256(&b)))
}
/// A stable branch per workflow subject. Publication never auto-merges.
pub fn branch_for(plan: &Plan) -> String {
    format!(
        "cm-testsuite/{}/{}",
        if plan.kind == Kind::Import {
            "import"
        } else {
            "receive"
        },
        &sha256(plan.subject.as_bytes())[..20]
    )
}
const MARKER: &str = "\n\nCM-TESTSUITE-PLAN\n";
fn verify_owned_branch(api: &dyn Api, source: &str, head: &str, branch: &str) -> Result<()> {
    let repo = repo_path(source)?;
    let commit = required(api, "GET", &format!("{repo}/git/commits/{head}"), None)?;
    let message = commit["message"]
        .as_str()
        .context("missing branch commit message")?;
    let record: Value = serde_json::from_str(
        message
            .split_once(MARKER)
            .context("refusing to overwrite a branch not owned by cm-testsuite")?
            .1,
    )?;
    ensure!(
        record["schema"] == 1 && record["branch"] == branch,
        "branch ownership mismatch"
    );
    validate_sha(
        record["plan_sha256"]
            .as_str()
            .context("missing plan digest")?,
    )?;
    let files: std::collections::BTreeMap<String, Option<String>> =
        serde_json::from_value(record["files"].clone())?;
    ensure!(files.len() <= 200, "previous proposal exceeds draft limit");
    for (path, hash) in &files {
        validate_path(path)?;
        if let Some(hash) = hash {
            validate_sha(hash)?;
        }
    }
    let base = record["base"].as_str().context("missing previous base")?;
    validate_revision(base)?;
    let diff = required(api, "GET", &format!("{repo}/compare/{base}...{head}"), None)?;
    let allowed: BTreeSet<_> = files.keys().map(String::as_str).collect();
    let changed = diff["files"].as_array().context("missing branch diff")?;
    ensure!(
        changed.len() < 300,
        "branch diff too large to verify safely"
    );
    for file in changed {
        ensure!(
            allowed.contains(file["filename"].as_str().context("missing changed path")?),
            "branch contains unrelated changes"
        );
        if let Some(previous) = file["previous_filename"].as_str() {
            ensure!(
                allowed.contains(previous),
                "branch contains unrelated rename"
            );
        }
    }
    for (path, hash) in files {
        ensure!(
            remote_hash(api, source, head, &path)? == hash,
            "proposal branch has local edits: {path}"
        );
    }
    Ok(())
}
/// Withdraw a now-unnecessary owned proposal, retaining its branch for inspection.
fn withdraw(api: &dyn Api, repository: &str, plan: &Plan) -> Result<Publication> {
    let repo = repo_path(repository)?;
    let info = required(api, "GET", &repo, None)?;
    let base_name = info["default_branch"]
        .as_str()
        .context("missing default branch")?;
    let branch = branch_for(plan);
    let owner = repository
        .strip_prefix("github:")
        .unwrap()
        .split('/')
        .next()
        .unwrap();
    let data = required(
        api,
        "GET",
        &format!(
            "{repo}/pulls?state=open&head={}&base={}&per_page=100",
            encode(&format!("{owner}:{branch}")),
            encode(base_name)
        ),
        None,
    )?;
    let prs = data.as_array().context("missing pull requests")?;
    ensure!(prs.len() <= 1, "multiple open proposals for one branch");
    let Some(pr) = prs.first() else {
        return Ok(Publication::NoChanges);
    };
    ensure!(
        pr["body"]
            .as_str()
            .unwrap_or_default()
            .contains("<!-- cm-testsuite -->"),
        "existing PR is not owned by cm-testsuite"
    );
    let reference = required(api, "GET", &format!("{repo}/git/ref/heads/{branch}"), None)?;
    let head = reference["object"]["sha"]
        .as_str()
        .context("missing proposal head")?;
    verify_owned_branch(api, repository, head, &branch)?;
    let closed = required(
        api,
        "PATCH",
        &format!(
            "{repo}/pulls/{}",
            pr["number"].as_u64().context("missing PR number")?
        ),
        Some(json!({"state":"closed"})),
    )?;
    Ok(Publication::Closed(
        closed["html_url"]
            .as_str()
            .context("missing PR URL")?
            .into(),
    ))
}
/// Explicit mutation API. Caller must deliberately invoke it with a reviewed plan and token.
/// First draft supports same-repository proposal branches, not fork delivery.
pub fn publish(api: &dyn Api, repository: &str, plan: &Plan) -> Result<Publication> {
    plan.validate()?;
    validate_source(repository)?;
    ensure!(
        plan.edits.len() <= 200,
        "draft publication limit is 200 file edits per proposal"
    );
    if plan.kind == Kind::Receive {
        ensure!(
            repository == plan.subject,
            "receiving plan targets a different repository"
        );
    }
    if plan.edits.is_empty() {
        return withdraw(api, repository, plan);
    }
    let body = format!("<!-- cm-testsuite -->\n{}", plan.body()?);
    let title = format!("cm-testsuite: {}", plan.subject);
    ensure!(
        body.len() <= 60000 && title.len() <= 240,
        "proposal description exceeds draft size limit; use a smaller batch"
    );
    let repo = repo_path(repository)?;
    let info = required(api, "GET", &repo, None)?;
    let base_name = info["default_branch"]
        .as_str()
        .context("missing default branch")?;
    let base = resolve(api, repository, base_name)?;
    let branch = branch_for(plan);
    let key = plan.key()?;
    let mut all_match = true;
    for e in &plan.edits {
        all_match &= remote_hash(api, repository, &base, &e.path)?
            == e.after.as_ref().map(|s| sha256(s.as_bytes()));
    }
    if all_match {
        return withdraw(api, repository, plan);
    }
    for guard in &plan.guards {
        ensure!(
            remote_hash(api, repository, &base, &guard.path)? == guard.sha256,
            "remote plan baseline changed: {}",
            guard.path
        );
    }
    for e in &plan.edits {
        ensure!(
            remote_hash(api, repository, &base, &e.path)? == e.before,
            "remote managed-file divergence: {}",
            e.path
        );
    }
    let owner = repository
        .strip_prefix("github:")
        .unwrap()
        .split('/')
        .next()
        .unwrap();
    let mut open = None;
    let mut finished = false;
    for page in 1..=10 {
        let prs = required(
            api,
            "GET",
            &format!(
                "{repo}/pulls?state=all&head={}&base={}&per_page=100&page={page}",
                encode(&format!("{owner}:{branch}")),
                encode(base_name)
            ),
            None,
        )?;
        let list = prs.as_array().context("missing pull requests")?;
        for pr in list {
            let body = pr["body"].as_str().unwrap_or_default();
            if pr["state"] == "open" {
                ensure!(
                    body.contains("<!-- cm-testsuite -->"),
                    "existing PR is not owned by cm-testsuite"
                );
                ensure!(open.is_none(), "multiple open proposals for one branch");
                open = Some(pr.clone());
            } else if pr["merged_at"].is_null() && body.contains(&format!("Plan SHA-256: `{key}`"))
            {
                return Ok(Publication::Declined);
            }
        }
        if list.len() < 100 {
            finished = true;
            break;
        }
    }
    ensure!(
        finished,
        "too many historical PRs to check decline state safely"
    );
    let reference_path = format!("{repo}/git/ref/heads/{branch}");
    let reference = api.request("GET", &reference_path, None)?;
    let head = reference
        .as_ref()
        .and_then(|v| v["object"]["sha"].as_str())
        .map(str::to_owned);
    if let Some(head) = &head {
        verify_owned_branch(api, repository, head, &branch)?;
    }
    let base_tree = tree_sha(api, repository, &base)?;
    let entries: Vec<_> = plan
        .edits
        .iter()
        .map(|e| match &e.after {
            Some(s) => json!({"path":e.path,"mode":"100644","type":"blob","content":s}),
            None => json!({"path":e.path,"mode":"100644","type":"blob","sha":null}),
        })
        .collect();
    let tree = required(
        api,
        "POST",
        &format!("{repo}/git/trees"),
        Some(json!({"base_tree":base_tree,"tree":entries})),
    )?;
    let new_tree = tree["sha"].as_str().context("missing new tree")?;
    let same_tree = if let Some(head) = &head {
        tree_sha(api, repository, head)? == new_tree
    } else {
        false
    };
    if !same_tree {
        let mut parents = vec![head.as_deref().unwrap_or(&base)];
        if head.is_some() && head.as_deref() != Some(&base) {
            parents.push(&base);
        }
        let commit = required(
            api,
            "POST",
            &format!("{repo}/git/commits"),
            Some(
                json!({"message":format!("cm-testsuite: {}{MARKER}{}",plan.subject,serde_json::to_string(&json!({"schema":1,"base":base,"branch":branch,"plan_sha256":key,"files":plan.edits.iter().map(|e|(e.path.clone(),e.after.as_ref().map(|s|sha256(s.as_bytes())))).collect::<std::collections::BTreeMap<_,_>>()}))?),"tree":new_tree,"parents":parents}),
            ),
        )?;
        let sha = commit["sha"].as_str().context("missing new commit")?;
        ensure!(
            resolve(api, repository, base_name)? == base,
            "default branch moved during publication; retry with a fresh plan"
        );
        if head.is_some() {
            required(
                api,
                "PATCH",
                &format!("{repo}/git/refs/heads/{branch}"),
                Some(json!({"sha":sha,"force":false})),
            )?;
        } else {
            required(
                api,
                "POST",
                &format!("{repo}/git/refs"),
                Some(json!({"ref":format!("refs/heads/{branch}"),"sha":sha})),
            )?;
        }
    }

    let pr = if let Some(pr) = open {
        required(
            api,
            "PATCH",
            &format!(
                "{repo}/pulls/{}",
                pr["number"].as_u64().context("missing PR number")?
            ),
            Some(json!({"title":title,"body":body})),
        )?
    } else {
        required(
            api,
            "POST",
            &format!("{repo}/pulls"),
            Some(json!({"title":title,"body":body,"head":branch,"base":base_name,"draft":true})),
        )?
    };
    Ok(Publication::PullRequest(
        pr["html_url"].as_str().context("missing PR URL")?.into(),
    ))
}
#[cfg(test)]
mod tests {
    use super::*;
    use cm_exchange_plan::{Edit, Guard};
    fn plan() -> Plan {
        Plan {
            schema: 1,
            kind: Kind::Import,
            subject: "github:example/source".into(),
            edits: vec![Edit {
                path: "corpus/wast/a.wast".into(),
                before: None,
                after: Some("(module)".into()),
            }],
            guards: vec![],
            warnings: vec![],
            exclusions: BTreeSet::new(),
            provenance: std::collections::BTreeMap::new(),
        }
    }
    #[test]
    fn apply_is_idempotent_and_preserves_unmanaged_files() {
        let d = tempfile::tempdir().unwrap();
        fs::write(d.path().join("README"), "local").unwrap();
        let p = plan();
        assert_eq!(apply(d.path(), &p).unwrap(), 1);
        assert_eq!(apply(d.path(), &p).unwrap(), 0);
        assert_eq!(
            fs::read_to_string(d.path().join("README")).unwrap(),
            "local"
        );
    }
    #[test]
    fn preflights_entire_plan_before_any_write() {
        let d = tempfile::tempdir().unwrap();
        let mut p = plan();
        p.edits.push(Edit {
            path: "corpus/wast/missing.wast".into(),
            before: Some("a".repeat(64)),
            after: None,
        });
        assert!(apply(d.path(), &p).is_err());
        assert!(!d.path().join("corpus/wast/a.wast").exists());
        assert!(!d.path().join(".cm-testsuite-write-lock").exists());
    }
    #[test]
    fn stale_policy_blocks_application() {
        let d = tempfile::tempdir().unwrap();
        fs::write(d.path().join("policy.toml"), "new").unwrap();
        let mut p = plan();
        p.guards.push(Guard {
            path: "policy.toml".into(),
            sha256: Some(sha256(b"old")),
        });
        assert!(apply(d.path(), &p).is_err());
    }
    #[test]
    fn collisions_are_not_overwritten() {
        let d = tempfile::tempdir().unwrap();
        fs::create_dir_all(d.path().join("corpus/wast")).unwrap();
        fs::write(d.path().join("corpus/wast/a.wast"), "my edit").unwrap();
        assert!(apply(d.path(), &plan()).is_err());
    }
}
