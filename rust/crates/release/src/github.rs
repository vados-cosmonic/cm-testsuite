//! GitHub mutations for explicit tagging and repeatable release notifications.

use crate::{Metadata, RELEASE_COMMIT_PREFIX, history::ReleaseHistory, source};
use anyhow::{Context, Result, bail, ensure};
use cm_catalog::validate_revision;
use cm_github::{Api, HttpStatus, encode, repo_path, required, resolve};
use serde::Serialize;
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};

/// Items requested per page; a shorter page is the last one.
const PAGE_SIZE: usize = 100;

/// Upper bound on pages read from one listing before giving up.
const MAX_PAGES: usize = 100;

/// Plan, and with `apply` create, the release tag at `sha`.
///
/// Returns whether the tag is missing: `true` means it was created (or, without
/// `apply`, would be), and `false` means it already points at `sha`. An existing
/// tag at another commit is an error; tags are never moved.
pub fn create_tag(
    api: &dyn Api,
    repository: &str,
    meta: &Metadata,
    sha: &str,
    apply: bool,
) -> Result<bool> {
    validate_revision(sha)?;
    let source = source(repository);
    let repo = repo_path(&source)?;
    let existing = api.request(
        "GET",
        &format!("{repo}/git/ref/tags/{}", encode(&meta.tag)),
        None,
    )?;
    if existing.is_some() {
        ensure!(
            resolve(api, &source, &meta.tag)? == sha,
            "release tag already points at a different commit"
        );
        return Ok(false);
    }
    if apply {
        required(
            api,
            "POST",
            &format!("{repo}/git/refs"),
            Some(json!({"ref": format!("refs/tags/{}", meta.tag), "sha": sha})),
        )?;
    }
    Ok(true)
}

/// Trailing marker that identifies a comment as this tool's release comment.
pub const COMMENT_MARKER: &str = "<!-- cm-testsuite-release -->";

/// One planned release comment: a new comment, or an edit of the existing one.
#[derive(Debug, Clone, Serialize)]
pub struct CommentEdit {
    /// Issue or PR number that receives the comment.
    pub number: u64,
    /// Existing comment to update; `None` creates a new comment.
    pub comment_id: Option<u64>,
    /// Body of the existing comment when planned. Applying refuses to
    /// overwrite a comment that has changed since.
    pub before: Option<String>,
    /// Complete body to write.
    pub body: String,
}

/// The outcome of read-only notification planning.
#[derive(Debug, Default)]
pub struct NotificationPlan {
    /// Comments to create or update, ordered by issue or PR number.
    pub edits: Vec<CommentEdit>,
    /// Human-readable reasons why expected notifications were left out.
    pub warnings: Vec<String>,
}

/// Read every page of a GitHub listing. `path` must not carry a query string.
fn pages(api: &dyn Api, path: &str) -> Result<Vec<Value>> {
    let mut out = Vec::new();
    for page in 1..=MAX_PAGES {
        let values = required(
            api,
            "GET",
            &format!("{path}?per_page={PAGE_SIZE}&page={page}"),
            None,
        )?;
        let values = values
            .as_array()
            .context("expected paginated GitHub array")?;
        out.extend(values.iter().cloned());
        if values.len() < PAGE_SIZE {
            return Ok(out);
        }
    }
    bail!("pagination limit exceeded for {path}")
}

/// Keywords that make GitHub close the issue referenced by the next word.
const CLOSING_KEYWORDS: [&str; 9] = [
    "fix", "fixes", "fixed", "close", "closes", "closed", "resolve", "resolves", "resolved",
];

/// Punctuation and Markdown emphasis that may surround a keyword or reference.
const SURROUNDING_PUNCTUATION: [char; 10] = ['*', '_', '(', ')', '[', ']', ',', '.', ';', ':'];

/// Parse `#N`, `owner/repo#N`, or an issue URL that refers to `repository`,
/// which must already be lowercase.
fn issue_number(word: &str, repository: &str) -> Option<u64> {
    let word = word.to_ascii_lowercase();
    let short = || word.strip_prefix('#');
    let qualified = || word.strip_prefix(repository)?.strip_prefix('#');
    let url = || {
        word.strip_prefix("https://github.com/")?
            .strip_prefix(repository)?
            .strip_prefix("/issues/")
    };
    let digits = short().or_else(qualified).or_else(url)?;
    // `parse` alone would also accept a leading `+`.
    if !digits.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    digits.parse().ok().filter(|&number| number > 0)
}

/// Issues named directly after a closing keyword in one span of prose.
fn closing_references<'a>(prose: &'a str, repository: &'a str) -> impl Iterator<Item = u64> + 'a {
    let words = prose
        .split_whitespace()
        .map(|word| word.trim_matches(SURROUNDING_PUNCTUATION));
    // Pair every word with the one that follows it.
    words
        .clone()
        .zip(words.skip(1))
        .filter(|(keyword, _)| {
            CLOSING_KEYWORDS
                .iter()
                .any(|closing| keyword.eq_ignore_ascii_case(closing))
        })
        .filter_map(move |(_, reference)| issue_number(reference, repository))
}

/// Recognize GitHub closing keywords and same-repository references. Code fences
/// and inline code do not establish an issue's relationship to a release.
pub fn closed_issues(body: &str, repository: &str) -> BTreeSet<u64> {
    let repository = repository.to_ascii_lowercase();
    let mut issues = BTreeSet::new();
    let mut fenced = false;
    for line in body.lines() {
        let start = line.trim_start();
        if start.starts_with("```") || start.starts_with("~~~") {
            fenced = !fenced;
            continue;
        }
        if fenced {
            continue;
        }
        // Splitting on backticks alternates prose (even spans) with inline
        // code (odd spans); a keyword and its reference must share a prose span.
        for prose in line.split('`').step_by(2) {
            issues.extend(closing_references(prose, &repository));
        }
    }
    issues
}

/// Find the release comment that `author` owns among an issue's comments.
/// A match always ends with [`COMMENT_MARKER`], ignoring trailing whitespace.
fn owned_comment<'a>(comments: &'a [Value], author: &str) -> Result<Option<&'a Value>> {
    let owned: Vec<_> = comments
        .iter()
        .filter(|c| {
            c["user"]["login"].as_str() == Some(author)
                && c["body"]
                    .as_str()
                    .is_some_and(|b| b.trim_end().ends_with(COMMENT_MARKER))
        })
        .collect();
    ensure!(
        owned.len() <= 1,
        "multiple owned release comments; reconcile them before retrying"
    );
    Ok(owned.first().copied())
}

/// Why an issue or PR is told about a release.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Relation {
    /// A PR whose own changes shipped in the release.
    Shipped,
    /// An issue or PR that a shipped PR referenced with a closing keyword.
    Linked,
}

/// Require the release for `tag` to be published with matching metadata.
fn ensure_published(api: &dyn Api, repo: &str, tag: &str, meta: &Metadata) -> Result<()> {
    let release = required(
        api,
        "GET",
        &format!("{repo}/releases/tags/{}", encode(tag)),
        None,
    )?;
    ensure!(
        release["draft"] == false
            && release["tag_name"] == tag
            && release["prerelease"] == meta.prerelease,
        "release must already be published with matching version metadata"
    );
    Ok(())
}

/// Find the issues and PRs to notify: for every PR merged into main whose merge
/// commit shipped in this release, the issues it closes or, when it names
/// none, the PR itself. Release preparation PRs are not notified.
fn notification_targets(
    api: &dyn Api,
    repo: &str,
    repository: &str,
    history: &ReleaseHistory,
) -> Result<BTreeMap<u64, Relation>> {
    let mut seen = BTreeSet::new();
    let mut targets = BTreeMap::new();
    for sha in &history.commits {
        validate_revision(sha)?;
        for associated in pages(api, &format!("{repo}/commits/{sha}/pulls"))? {
            let number = associated["number"].as_u64().context("missing PR number")?;
            if !seen.insert(number) {
                continue;
            }
            let pr = required(api, "GET", &format!("{repo}/pulls/{number}"), None)?;
            let shipped = !pr["merged_at"].is_null()
                && pr["base"]["ref"] == "main"
                && pr["base"]["repo"]["full_name"] == repository
                && pr["merge_commit_sha"]
                    .as_str()
                    .is_some_and(|sha| history.ancestors.contains(sha));
            // A merge commit of the release PR does not carry the release
            // commit subject, so the PR is recognized by its title instead.
            let is_release_pr = pr["title"]
                .as_str()
                .is_some_and(|title| title.starts_with(RELEASE_COMMIT_PREFIX));
            if !shipped || is_release_pr {
                continue;
            }
            let issues = closed_issues(pr["body"].as_str().unwrap_or(""), repository);
            if issues.is_empty() {
                targets.insert(number, Relation::Shipped);
            }
            for issue in issues {
                targets.insert(issue, Relation::Linked);
            }
        }
    }
    Ok(targets)
}

/// Fetch an issue or PR, or record a warning and return `None` when it is
/// missing, moved, or deleted. A typo, transferred, or deleted reference must
/// not block every other notification, including on reruns.
fn notifiable_target(
    api: &dyn Api,
    repo: &str,
    number: u64,
    warnings: &mut Vec<String>,
) -> Result<Option<Value>> {
    match api.request("GET", &format!("{repo}/issues/{number}"), None) {
        Ok(Some(target)) => Ok(Some(target)),
        Ok(None) => {
            warnings.push(format!("skipping #{number}: not found"));
            Ok(None)
        }
        Err(error) => match error.downcast_ref::<HttpStatus>() {
            // GitHub answers 301 for a transferred issue and 410 for a deleted one.
            Some(HttpStatus { status, .. }) if matches!(*status, 301 | 410) => {
                warnings.push(format!(
                    "skipping #{number}: moved or deleted (HTTP {status})"
                ));
                Ok(None)
            }
            _ => Err(error),
        },
    }
}

/// Plan the comment that adds `release_line` to issue or PR `number`, or `None`
/// when its release comment already lists the release. An existing comment
/// keeps its earlier release links; `subject` opens a new comment.
fn planned_comment(
    number: u64,
    existing: Option<&Value>,
    release_line: &str,
    subject: &str,
) -> Result<Option<CommentEdit>> {
    let Some(comment) = existing else {
        return Ok(Some(CommentEdit {
            number,
            comment_id: None,
            before: None,
            body: format!("{subject} are available in:\n\n{release_line}\n\n{COMMENT_MARKER}\n"),
        }));
    };
    let body = comment["body"].as_str().context("missing comment body")?;
    if body.lines().any(|line| line == release_line) {
        return Ok(None);
    }
    let listed = body
        .trim_end()
        .strip_suffix(COMMENT_MARKER)
        .context("release comment does not end with its marker")?
        .trim_end();
    Ok(Some(CommentEdit {
        number,
        comment_id: Some(comment["id"].as_u64().context("missing comment ID")?),
        before: Some(body.into()),
        body: format!("{listed}\n{release_line}\n\n{COMMENT_MARKER}\n"),
    }))
}

/// Read-only planning. PRs without linked closing references get a notification;
/// otherwise their linked issues do, matching jco's workflow. Releases accumulate
/// in the comment so rerunning an older release cannot overwrite a newer one.
///
/// `author` is the login whose marker comments may be updated.
pub fn notifications(
    api: &dyn Api,
    repository: &str,
    tag: &str,
    history: &ReleaseHistory,
    author: &str,
) -> Result<NotificationPlan> {
    ensure!(!author.trim().is_empty(), "comment author is required");
    let meta = Metadata::from_tag(tag)?;
    let repo = repo_path(&source(repository))?;
    ensure_published(api, &repo, tag, &meta)?;
    let mut plan = NotificationPlan::default();
    if history.previous.is_none() {
        // The first release's range is the entire history; announcing it on
        // every past PR and issue would be noise and exhaust the API budget.
        plan.warnings.push(format!(
            "{tag} is the first release; skipping notifications"
        ));
        return Ok(plan);
    }
    let release_line = format!(
        "- [{}](https://github.com/{repository}/releases/tag/{})",
        meta.tag,
        encode(tag)
    );
    for (number, relation) in notification_targets(api, &repo, repository, history)? {
        let Some(target) = notifiable_target(api, &repo, number, &mut plan.warnings)? else {
            continue;
        };
        // The issues API also serves PRs, marking them with this field.
        let is_pr = target.get("pull_request").is_some_and(|v| !v.is_null());
        let subject = match (relation, is_pr) {
            (Relation::Shipped, _) => "The changes in this PR",
            (Relation::Linked, false) => "The changes linked to this issue",
            (Relation::Linked, true) => "The changes linked to this PR",
        };
        let comments = pages(api, &format!("{repo}/issues/{number}/comments"))?;
        let existing = owned_comment(&comments, author)?;
        plan.edits
            .extend(planned_comment(number, existing, &release_line, subject)?);
    }
    Ok(plan)
}

/// Write planned comments, rechecking each target first: a comment that changed
/// or appeared since planning is an error, and the command can be rerun.
pub fn apply_notifications(
    api: &dyn Api,
    repository: &str,
    author: &str,
    edits: &[CommentEdit],
) -> Result<()> {
    let repo = repo_path(&source(repository))?;
    for edit in edits {
        if let Some(id) = edit.comment_id {
            let path = format!("{repo}/issues/comments/{id}");
            let existing = required(api, "GET", &path, None)?;
            ensure!(
                existing["user"]["login"] == author
                    && existing["body"].as_str() == edit.before.as_deref(),
                "release comment changed since planning; retry"
            );
            required(api, "PATCH", &path, Some(json!({"body": edit.body})))?;
        } else {
            let path = format!("{repo}/issues/{}/comments", edit.number);
            ensure!(
                owned_comment(&pages(api, &path)?, author)?.is_none(),
                "release comment appeared since planning; retry"
            );
            required(api, "POST", &path, Some(json!({"body": edit.body})))?;
        }
    }
    Ok(())
}

/// Where a release PR's commits start and how many there are.
#[derive(Debug, PartialEq, Eq)]
pub struct ReleasePrBase {
    /// The main commit the release PR was prepared on.
    pub sha: String,
    /// Number of commits in the release PR.
    pub commits: usize,
}

/// Find the base of release PR `number`.
///
/// The PR must be one unbroken chain of release commits, so maintainer fixes
/// or "Update branch" merges are refused rather than tagged.
pub fn release_pr_base(api: &dyn Api, repository: &str, number: u64) -> Result<ReleasePrBase> {
    let repo = repo_path(&source(repository))?;
    let commits = pages(api, &format!("{repo}/pulls/{number}/commits"))?;
    // GitHub lists PR commits oldest first: each must sit on the one before
    // it, and the first one's parent is the base.
    let mut base = None;
    let mut previous = None;
    for commit in &commits {
        let sha = commit["sha"].as_str().context("missing commit SHA")?;
        validate_revision(sha)?;
        ensure!(
            commit["commit"]["message"]
                .as_str()
                .is_some_and(|m| m.starts_with(RELEASE_COMMIT_PREFIX)),
            "release PR contains a non-release commit {sha}; rerun create-release-pr"
        );
        let parents = commit["parents"]
            .as_array()
            .context("missing commit parents")?
            .iter()
            .map(|p| p["sha"].as_str().context("missing parent SHA"))
            .collect::<Result<Vec<_>>>()?;
        let [parent] = parents.as_slice() else {
            bail!("release PR contains merge commit {sha}; rerun create-release-pr");
        };
        match previous {
            Some(previous) => ensure!(
                *parent == previous,
                "release PR commits are not a single chain"
            ),
            None => base = Some(*parent),
        }
        previous = Some(sha);
    }
    let sha = base.context("release PR has no commits")?.to_owned();
    validate_revision(&sha)?;
    Ok(ReleasePrBase {
        sha,
        commits: commits.len(),
    })
}
