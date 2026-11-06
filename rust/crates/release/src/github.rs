//! GitHub mutations for explicit tagging and repeatable release notifications.
use crate::{Metadata, history::ReleaseHistory};
use anyhow::{Context, Result, ensure};
use cm_catalog::validate_revision;
use cm_github::{Api, HttpStatus, encode, repo_path, required, resolve};
use serde::Serialize;
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};

pub fn create_tag(
    api: &dyn Api,
    repository: &str,
    meta: &Metadata,
    sha: &str,
    apply: bool,
) -> Result<bool> {
    validate_revision(sha)?;
    let source = format!("github:{repository}");
    let repo = repo_path(&source)?;
    if api
        .request(
            "GET",
            &format!("{repo}/git/ref/tags/{}", encode(&meta.tag)),
            None,
        )?
        .is_some()
    {
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
            Some(json!({"ref":format!("refs/tags/{}",meta.tag),"sha":sha})),
        )?;
    }
    Ok(true)
}

pub const COMMENT_MARKER: &str = "<!-- cm-testsuite-release -->";
#[derive(Debug, Clone, Serialize)]
pub struct CommentEdit {
    pub number: u64,
    pub comment_id: Option<u64>,
    pub before: Option<String>,
    pub body: String,
}

fn pages(api: &dyn Api, path: &str) -> Result<Vec<Value>> {
    let mut out = Vec::new();
    for page in 1..=100 {
        let values = required(
            api,
            "GET",
            &format!("{path}?per_page=100&page={page}"),
            None,
        )?;
        let values = values
            .as_array()
            .context("expected paginated GitHub array")?;
        out.extend(values.iter().cloned());
        if values.len() < 100 {
            return Ok(out);
        }
    }
    anyhow::bail!("pagination limit exceeded for {path}")
}

/// Recognize GitHub closing keywords and same-repository references. Code fences
/// and inline code do not establish an issue's relationship to a release.
pub fn closed_issues(body: &str, repository: &str) -> BTreeSet<u64> {
    let mut result = BTreeSet::new();
    let mut fenced = false;
    for line in body.lines() {
        if line.trim_start().starts_with("```") || line.trim_start().starts_with("~~~") {
            fenced = !fenced;
            continue;
        }
        if fenced {
            continue;
        }
        let mut previous_closes = false;
        let mut inline = false;
        for part in line.split('`') {
            if !inline {
                for word in part.split_whitespace() {
                    let word =
                        word.trim_matches(['*', '_', '(', ')', '[', ']', ',', '.', ';', ':']);
                    if previous_closes {
                        let lower = word.to_ascii_lowercase();
                        let repo = repository.to_ascii_lowercase();
                        let number = lower
                            .strip_prefix('#')
                            .or_else(|| lower.strip_prefix(&format!("{repo}#")))
                            .or_else(|| {
                                lower.strip_prefix(&format!("https://github.com/{repo}/issues/"))
                            });
                        if let Some(number) = number
                            && !number.is_empty()
                            && number.bytes().all(|b| b.is_ascii_digit())
                            && let Ok(number) = number.parse::<u64>()
                            && number > 0
                        {
                            result.insert(number);
                        }
                    }
                    previous_closes = matches!(
                        word.to_ascii_lowercase().as_str(),
                        "fix"
                            | "fixes"
                            | "fixed"
                            | "close"
                            | "closes"
                            | "closed"
                            | "resolve"
                            | "resolves"
                            | "resolved"
                    );
                }
            } else {
                previous_closes = false;
            }
            inline = !inline;
        }
    }
    result
}

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

/// Read-only planning. PRs without linked closing references get a notification;
/// otherwise their linked issues do, matching jco's workflow. Releases accumulate
/// in the comment so rerunning an older release cannot overwrite a newer one.
pub fn notifications(
    api: &dyn Api,
    repository: &str,
    tag: &str,
    history: &ReleaseHistory,
    author: &str,
) -> Result<Vec<CommentEdit>> {
    ensure!(!author.trim().is_empty(), "comment author is required");
    let meta = Metadata::from_tag(tag)?;
    let repo = repo_path(&format!("github:{repository}"))?;
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
            if pr["merged_at"].is_null()
                || pr["base"]["ref"] != "main"
                || pr["base"]["repo"]["full_name"] != repository
                || !pr["merge_commit_sha"]
                    .as_str()
                    .is_some_and(|sha| history.ancestors.contains(sha))
            {
                continue;
            }
            let issues = closed_issues(pr["body"].as_str().unwrap_or(""), repository);
            if issues.is_empty() {
                targets.insert(number, false);
            }
            for issue in issues {
                targets.insert(issue, true);
            }
        }
    }
    let release_line = format!(
        "- [{}](https://github.com/{repository}/releases/tag/{})",
        meta.tag,
        encode(tag)
    );
    let mut edits = Vec::new();
    for (number, linked) in targets {
        // A typo, transferred, or deleted reference must not block every other
        // notification, including on reruns.
        let target = match api.request("GET", &format!("{repo}/issues/{number}"), None) {
            Ok(Some(target)) => target,
            Ok(None) => {
                eprintln!("warning: skipping #{number}: not found");
                continue;
            }
            Err(error) => match error.downcast_ref::<HttpStatus>() {
                Some(HttpStatus { status, .. }) if matches!(*status, 301 | 410) => {
                    eprintln!("warning: skipping #{number}: moved or deleted (HTTP {status})");
                    continue;
                }
                _ => return Err(error),
            },
        };
        let is_pr = target.get("pull_request").is_some_and(|v| !v.is_null());
        let comments = pages(api, &format!("{repo}/issues/{number}/comments"))?;
        let existing = owned_comment(&comments, author)?;
        let (comment_id, before, body) = if let Some(comment) = existing {
            let body = comment["body"].as_str().context("missing comment body")?;
            if body.lines().any(|line| line == release_line) {
                continue;
            }
            let updated = format!(
                "{}\n{release_line}\n\n{COMMENT_MARKER}\n",
                body.trim_end()
                    .strip_suffix(COMMENT_MARKER)
                    .unwrap()
                    .trim_end()
            );
            (
                Some(comment["id"].as_u64().context("missing comment ID")?),
                Some(body.into()),
                updated,
            )
        } else {
            let subject = match (linked, is_pr) {
                (false, _) => "The changes in this PR",
                (true, false) => "The changes linked to this issue",
                (true, true) => "The changes linked to this PR",
            };
            (
                None,
                None,
                format!("{subject} are available in:\n\n{release_line}\n\n{COMMENT_MARKER}\n"),
            )
        };
        edits.push(CommentEdit {
            number,
            comment_id,
            before,
            body,
        });
    }
    Ok(edits)
}

pub fn apply_notifications(
    api: &dyn Api,
    repository: &str,
    author: &str,
    edits: &[CommentEdit],
) -> Result<()> {
    let repo = repo_path(&format!("github:{repository}"))?;
    for edit in edits {
        if let Some(id) = edit.comment_id {
            let path = format!("{repo}/issues/comments/{id}");
            let existing = required(api, "GET", &path, None)?;
            ensure!(
                existing["user"]["login"] == author
                    && existing["body"].as_str() == edit.before.as_deref(),
                "release comment changed since planning; retry"
            );
            required(api, "PATCH", &path, Some(json!({"body":edit.body})))?;
        } else {
            let path = format!("{repo}/issues/{}/comments", edit.number);
            ensure!(
                owned_comment(&pages(api, &path)?, author)?.is_none(),
                "release comment appeared since planning; retry"
            );
            required(api, "POST", &path, Some(json!({"body":edit.body})))?;
        }
    }
    Ok(())
}
