//! Reconcile explicitly requested GitHub proposals without overwriting human edits.

use anyhow::{Context, Result, ensure};
use cm_catalog::*;
use cm_exchange_plan::{Kind, Plan};
use cm_github::{Api, encode, read_file, repo_path, required, resolve, tree_sha};
use serde_json::{Value, json};
use std::collections::BTreeSet;

#[derive(Debug, PartialEq, Eq)]
pub enum Publication {
    NoChanges,
    Declined,
    Closed(String),
    PullRequest(String),
}

/// Hash a regular remote file, retaining absence as a distinct baseline.
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

/// Read the exact plan digest recorded in a managed pull request body.
fn published_plan_key(body: &str) -> Option<&str> {
    body.lines()
        .find_map(|line| line.strip_prefix("Plan SHA-256: `")?.strip_suffix('`'))
        .filter(|key| validate_sha(key).is_ok())
}

/// Distinguish tool withdrawals from maintainer declines when offering a plan again.
fn withdrawal_marker(key: &str) -> String {
    format!("<!-- cm-testsuite-withdrawn: {key} -->")
}

/// Refuse edits or closure unless the branch still matches its recorded proposal.
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
    let base = resolve(api, repository, base_name)?;
    // An empty plan supersedes the current proposal only while its policy and
    // catalog baselines remain current. Applied plans have different baselines;
    // those may withdraw only the exact proposal they originally published.
    if plan.edits.is_empty() {
        for guard in &plan.guards {
            ensure!(
                remote_hash(api, repository, &base, &guard.path)? == guard.sha256,
                "remote plan baseline changed: {}",
                guard.path
            );
        }
    } else {
        for edit in &plan.edits {
            ensure!(
                remote_hash(api, repository, &base, &edit.path)?
                    == edit.after.as_ref().map(|s| sha256(s.as_bytes())),
                "applied plan contents changed before withdrawal: {}",
                edit.path
            );
        }
    }
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
    let pr_body = pr["body"].as_str().unwrap_or_default();
    ensure!(
        pr_body.contains("<!-- cm-testsuite -->"),
        "existing PR is not owned by cm-testsuite"
    );
    let previous_key = published_plan_key(pr_body).context("missing proposal plan digest")?;
    if !plan.edits.is_empty() && previous_key != plan.key()? {
        return Ok(Publication::NoChanges);
    }
    let reference = required(api, "GET", &format!("{repo}/git/ref/heads/{branch}"), None)?;
    let head = reference["object"]["sha"]
        .as_str()
        .context("missing proposal head")?;
    verify_owned_branch(api, repository, head, &branch)?;
    ensure!(
        resolve(api, repository, base_name)? == base,
        "default branch moved during withdrawal; retry with a fresh plan"
    );
    let closed = required(
        api,
        "PATCH",
        &format!(
            "{repo}/pulls/{}",
            pr["number"].as_u64().context("missing PR number")?
        ),
        Some(json!({
            "state":"closed",
            "body":format!("{pr_body}\n{}", withdrawal_marker(previous_key)),
        })),
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
    // Inspect history before mutating refs so an identical declined proposal
    // stays closed and an existing managed PR can be updated in place.
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
            } else if pr["merged_at"].is_null()
                && published_plan_key(body) == Some(key.as_str())
                && !body.trim_end().ends_with(&withdrawal_marker(&key))
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
    // Build from the current default tree, preserving unrelated repository files.
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
        // Record the exact edited paths and baseline so later updates can detect
        // human changes before reusing this branch.
        let files = plan
            .edits
            .iter()
            .map(|edit| {
                (
                    edit.path.clone(),
                    edit.after.as_ref().map(|text| sha256(text.as_bytes())),
                )
            })
            .collect::<std::collections::BTreeMap<_, _>>();
        let record = json!({
            "schema": 1,
            "base": base,
            "branch": branch,
            "plan_sha256": key,
            "files": files,
        });
        let message = format!(
            "cm-testsuite: {}{MARKER}{}",
            plan.subject,
            serde_json::to_string(&record)?,
        );
        let commit = required(
            api,
            "POST",
            &format!("{repo}/git/commits"),
            Some(json!({"message": message, "tree": new_tree, "parents": parents})),
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

    // Publish only after the branch update succeeds; creation always uses draft mode.
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
