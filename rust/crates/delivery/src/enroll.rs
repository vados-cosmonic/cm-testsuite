//! Turn an "Enroll a repository" issue into a reviewed registry change and its pull request.

mod form;

use crate::gather::default_branch;
use anyhow::{Context, Result, ensure};
use cm_catalog::{Manifest, PARTICIPANTS_PATH, Participant, Participants};
use cm_github::{Api, encode, read_file, repo_path, required, resolve, tree_sha};
use serde_json::json;

/// Where a receiving repository commits the policy that lets the catalog open pull requests.
const TRUST_POLICY_PATH: &str = ".github/chainguard/cm-testsuite.sts.yaml";

/// What every enrollment pull request title starts with.
const TITLE: &str = "feat(registry): enroll";

/// The longest pull request title and commit header the repository's lint accepts.
const MAX_TITLE: usize = 100;

/// One finding from reading the repository that asks to enroll.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Check {
    /// False for a finding the reviewer should resolve or accept before merging.
    pub passed: bool,
    /// The finding, as Markdown.
    pub summary: String,
}

/// A validated enrollment request and what a read-only look at its repository found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Enrollment {
    /// Number of the issue that asked.
    pub issue: u64,
    /// The registry entry to add.
    pub participant: Participant,
    /// Findings for the reviewer; none of them blocks the proposal.
    pub checks: Vec<Check>,
}

impl Enrollment {
    /// The one branch that proposes this issue's enrollment.
    pub fn branch(&self) -> String {
        format!("cm-testsuite/enroll/{}", self.issue)
    }

    /// Title the proposal as a Conventional Commit, since the catalog lints titles.
    pub fn title(&self) -> String {
        let name = self.participant.repository.trim_start_matches("github:");
        let named = format!("{TITLE} {name}");
        if named.len() <= MAX_TITLE {
            named
        } else {
            format!("{TITLE} the repository of #{}", self.issue)
        }
    }

    /// Render the findings as a Markdown task list, with warnings left unticked.
    pub fn checklist(&self) -> String {
        self.checks
            .iter()
            .map(|check| {
                if check.passed {
                    format!("- [x] {}\n", check.summary)
                } else {
                    format!("- [ ] **Warning:** {}\n", check.summary)
                }
            })
            .collect()
    }

    /// Identify the pull request this command owns for the issue.
    fn marker(&self) -> String {
        format!("<!-- cm-testsuite-enroll: {} -->", self.issue)
    }

    /// Describe the proposal; the closing keyword resolves the issue on merge.
    fn body(&self) -> String {
        format!(
            "{}\nEnrolls {} in `{PARTICIPANTS_PATH}`, as requested in #{}.\n\n\
             Read from GitHub when this pull request was last updated:\n\n{}\n\
             Closes #{}\n",
            self.marker(),
            code(self.participant.repository.trim_start_matches("github:")),
            self.issue,
            self.checklist(),
            self.issue
        )
    }

    /// The message of every commit this command writes on the branch.
    fn commit_message(&self) -> String {
        format!("{}\n\nCloses #{}\n", self.title(), self.issue)
    }
}

/// Quote a name as inline code. Names GitHub reports, such as a default
/// branch, are not validated here, so anything that could be markup is replaced.
fn code(text: &str) -> String {
    let safe: String = text
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || "-_./".contains(c) {
                c
            } else {
                '?'
            }
        })
        .collect();
    format!("`{safe}`")
}

/// Read the issue and the repository it names, without writing anything.
///
/// Fails when the issue is not an open, well-formed enrollment request, or
/// when the manifest it names is missing or invalid: gathering from such a
/// repository would fail on every run. Anything else is reported as a check.
pub fn review_enrollment(api: &dyn Api, catalog: &str, issue: u64) -> Result<Enrollment> {
    let path = format!("{}/issues/{issue}", repo_path(catalog)?);
    let found = api
        .request("GET", &path, None)?
        .with_context(|| format!("issue #{issue} not found in {catalog}"))?;
    ensure!(
        found["pull_request"].is_null(),
        "#{issue} is a pull request, not an issue"
    );
    ensure!(found["state"] == "open", "issue #{issue} is not open");
    let request = form::parse(
        found["title"].as_str().unwrap_or_default(),
        found["body"].as_str().unwrap_or_default(),
    )
    .with_context(|| format!("issue #{issue} is not a valid enrollment request"))?;
    let participant = request.participant;
    let source = participant.repository.as_str();

    let default = default_branch(api, source).with_context(|| format!("reading {source}"))?;
    let reference = participant.reference.as_deref().unwrap_or(&default);
    let revision = resolve(api, source, reference)
        .with_context(|| format!("resolving {reference} in {source}"))?;
    let location = format!(
        "{} at {} ({})",
        code(&participant.manifest),
        code(reference),
        &revision[..12]
    );
    let manifest = read_file(api, source, &revision, &participant.manifest)?
        .with_context(|| format!("{source} has no {location}"))?;
    let manifest = Manifest::parse(std::str::from_utf8(&manifest)?, &participant.manifest)
        .with_context(|| format!("{source} has an invalid manifest {location}"))?;

    let finding = |passed, summary| Check { passed, summary };
    let mut checks = vec![finding(true, format!("{location} is a valid manifest."))];
    let (tests, receiving) = (manifest.test.len(), manifest.receive.enabled);
    if tests > 0 {
        let noun = if tests == 1 { "test" } else { "tests" };
        checks.push(finding(
            true,
            format!("The manifest shares {tests} {noun}."),
        ));
    } else if request.submits {
        checks.push(finding(
            false,
            "The issue asks to submit tests, but the manifest declares none.".into(),
        ));
    }
    if receiving {
        // Pull requests with received tests are opened against the default branch,
        // with a token the policy committed there grants.
        let head = resolve(api, source, &default)?;
        let policy = code(TRUST_POLICY_PATH);
        let branch = code(&default);
        checks.push(
            if read_file(api, source, &head, TRUST_POLICY_PATH)?.is_some() {
                finding(
                    true,
                    format!(
                        "The manifest receives tests, and {policy} is committed on {branch}; \
                     its contents were not checked."
                    ),
                )
            } else {
                finding(
                    false,
                    format!(
                        "The manifest receives tests, but {policy} is missing on {branch}; \
                     `distribute` reports this repository as failing until it is committed."
                    ),
                )
            },
        );
    } else if request.receives {
        checks.push(finding(
            false,
            "The issue asks to receive tests, but the manifest does not enable receiving.".into(),
        ));
    }
    if tests == 0 && !receiving {
        checks.push(finding(
            false,
            "The manifest neither shares nor receives tests, so enrollment changes nothing yet."
                .into(),
        ));
    }
    Ok(Enrollment {
        issue,
        participant,
        checks,
    })
}

/// Open or update the draft pull request that enrolls the repository in `catalog`.
///
/// Each issue has one branch and one open pull request, which a repeated run
/// updates in place. The registry is edited as it stands on the default
/// branch, and a branch that anyone else has committed to is left alone.
/// Returns the pull request's URL.
pub fn publish_enrollment(api: &dyn Api, catalog: &str, enrollment: &Enrollment) -> Result<String> {
    let repo = repo_path(catalog)?;
    let base_name = default_branch(api, catalog)?;
    let base = resolve(api, catalog, &base_name)?;
    let current = read_file(api, catalog, &base, PARTICIPANTS_PATH)?
        .with_context(|| format!("{catalog} has no {PARTICIPANTS_PATH}"))?;
    let registry = Participants::enroll(std::str::from_utf8(&current)?, &enrollment.participant)?;
    let (branch, title, body) = (enrollment.branch(), enrollment.title(), enrollment.body());

    // Inspect the pull request and the branch before mutating anything.
    let owner = catalog.trim_start_matches("github:").split('/').next();
    let prs = required(
        api,
        "GET",
        &format!(
            "{repo}/pulls?state=open&head={}&base={}&per_page=100",
            encode(&format!("{}:{branch}", owner.unwrap_or_default())),
            encode(&base_name)
        ),
        None,
    )?;
    let prs = prs.as_array().context("missing pull requests")?;
    ensure!(prs.len() <= 1, "multiple open pull requests for {branch}");
    let open = prs.first();
    if let Some(pr) = open {
        ensure!(
            pr["body"]
                .as_str()
                .unwrap_or_default()
                .contains(&enrollment.marker()),
            "the open pull request for {branch} was not opened by this command"
        );
    }
    let reference = api.request("GET", &format!("{repo}/git/ref/heads/{branch}"), None)?;
    let head = reference
        .as_ref()
        .and_then(|found| found["object"]["sha"].as_str());
    if let Some(head) = head {
        // Every commit this command writes carries the same message, so any
        // other head means someone else has committed to the branch.
        let commit = required(api, "GET", &format!("{repo}/git/commits/{head}"), None)?;
        let message = commit["message"].as_str().unwrap_or_default();
        ensure!(
            message.starts_with(TITLE)
                && message
                    .trim_end()
                    .ends_with(&format!("\n\nCloses #{}", enrollment.issue)),
            "{branch} has commits this command did not write; \
             finish that pull request by hand, or delete the branch and run again"
        );
    }

    // Build from the current default tree, so the proposal changes the registry alone.
    let tree = required(
        api,
        "POST",
        &format!("{repo}/git/trees"),
        Some(json!({
            "base_tree": tree_sha(api, catalog, &base)?,
            "tree": [{
                "path": PARTICIPANTS_PATH, "mode": "100644", "type": "blob", "content": registry,
            }],
        })),
    )?;
    let tree = tree["sha"].as_str().context("missing new tree")?;
    let unchanged = match head {
        Some(head) => tree_sha(api, catalog, head)? == tree,
        None => false,
    };
    if !unchanged {
        let mut parents = vec![head.unwrap_or(&base)];
        if head.is_some_and(|head| head != base) {
            parents.push(&base);
        }
        let commit = required(
            api,
            "POST",
            &format!("{repo}/git/commits"),
            Some(json!({
                "message": enrollment.commit_message(), "tree": tree, "parents": parents,
            })),
        )?;
        let sha = commit["sha"].as_str().context("missing new commit")?;
        ensure!(
            resolve(api, catalog, &base_name)? == base,
            "default branch moved during publication; run again"
        );
        if head.is_some() {
            required(
                api,
                "PATCH",
                &format!("{repo}/git/refs/heads/{branch}"),
                Some(json!({"sha": sha, "force": false})),
            )?;
        } else {
            required(
                api,
                "POST",
                &format!("{repo}/git/refs"),
                Some(json!({"ref": format!("refs/heads/{branch}"), "sha": sha})),
            )?;
        }
    }

    // The pull request follows the branch; it is always created as a draft.
    let pr = match open {
        Some(pr) => required(
            api,
            "PATCH",
            &format!(
                "{repo}/pulls/{}",
                pr["number"].as_u64().context("missing PR number")?
            ),
            Some(json!({"title": title, "body": body})),
        )?,
        None => required(
            api,
            "POST",
            &format!("{repo}/pulls"),
            Some(json!({
                "title": title, "body": body, "head": branch, "base": base_name, "draft": true,
            })),
        )?,
    };
    Ok(pr["html_url"].as_str().context("missing PR URL")?.into())
}
