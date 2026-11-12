//! Keep one issue describing the failures of a scheduled exchange run.

use anyhow::{Context, Result, ensure};
use cm_catalog::PARTICIPANTS_PATH;
use cm_github::{Api, repo_path, required};
use serde_json::json;
use std::collections::BTreeSet;

/// One repository a run could not process.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Failure {
    /// The enrolled repository, or a label for a failure that spans them.
    pub repository: String,
    /// Why it failed, as reported to the operator.
    pub error: String,
    /// GitHub logins responsible for the repository.
    pub contacts: Vec<String>,
}

/// What reporting did with the run's issue.
#[derive(Debug, PartialEq, Eq)]
pub enum Report {
    /// Nothing failed and no issue was open.
    Clear,
    /// Failures were filed in a new issue.
    Filed(String),
    /// The open issue now describes the current failures.
    Updated(String),
    /// Nothing failed any more, so the open issue was closed.
    Resolved(String),
}

/// Identify the single issue a task owns among a repository's open issues.
fn marker(task: &str) -> String {
    format!("<!-- cm-testsuite-failures: {task} -->")
}

/// Keep error text inside one Markdown table cell and free of mentions.
fn cell(text: &str) -> String {
    let flat = text.split_whitespace().collect::<Vec<_>>().join(" ");
    let mut clipped: String = flat.chars().take(500).collect();
    if clipped.len() < flat.len() {
        clipped.push('…');
    }
    clipped.replace('|', "\\|").replace('@', "@\u{200b}")
}

fn mentions<'a>(logins: impl IntoIterator<Item = &'a String>) -> String {
    logins
        .into_iter()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .map(|login| format!("@{login}"))
        .collect::<Vec<_>>()
        .join(" ")
}

/// Describe the failures, how to act on them, and who to ask.
fn body(task: &str, failures: &[Failure], maintainers: &[String], run: Option<&str>) -> String {
    let mut body = format!(
        "{}\nThe scheduled `{task}` run could not process {} of the enrolled repositories. \
         Fix the cause, or remove the repository from `{PARTICIPANTS_PATH}` to stop trying. \
         This issue tracks the latest run and closes itself once a run succeeds.\n\n\
         | Repository | Failure | Contacts |\n| --- | --- | --- |\n",
        marker(task),
        failures.len()
    );
    for failure in failures {
        body.push_str(&format!(
            "| `{}` | {} | {} |\n",
            failure.repository,
            cell(&failure.error),
            mentions(&failure.contacts)
        ));
    }
    body.push_str(&format!(
        "\nReproduce locally with `dotslash bins/just {task} --only REPOSITORY`.\n"
    ));
    if let Some(run) = run {
        body.push_str(&format!("Latest failing run: {run}\n"));
    }
    if !maintainers.is_empty() {
        body.push_str(&format!("\ncc {}\n", mentions(maintainers)));
    }
    body
}

/// Reconcile the task's failure issue in `repository` with this run's failures.
///
/// A run with failures opens the issue or rewrites the open one in place, so
/// repeated runs neither pile up issues nor notify again. A clean run closes it.
pub fn report(
    api: &dyn Api,
    repository: &str,
    task: &str,
    failures: &[Failure],
    maintainers: &[String],
    run: Option<&str>,
) -> Result<Report> {
    ensure!(
        !task.is_empty() && task.bytes().all(|b| b.is_ascii_lowercase() || b == b'-'),
        "invalid task name"
    );
    let repo = repo_path(repository)?;
    let marker = marker(task);
    let mut open = None;
    for page in 1..=10 {
        let issues = required(
            api,
            "GET",
            &format!("{repo}/issues?state=open&per_page=100&page={page}"),
            None,
        )?;
        let issues = issues.as_array().context("missing issues")?;
        // The issues API also lists pull requests.
        open = issues
            .iter()
            .find(|issue| {
                issue["pull_request"].is_null()
                    && issue["body"].as_str().unwrap_or_default().contains(&marker)
            })
            .cloned();
        if open.is_some() || issues.len() < 100 {
            break;
        }
    }
    let url = |issue: &serde_json::Value| -> Result<String> {
        Ok(issue["html_url"]
            .as_str()
            .context("missing issue URL")?
            .to_owned())
    };
    let Some(issue) = open else {
        if failures.is_empty() {
            return Ok(Report::Clear);
        }
        let created = required(
            api,
            "POST",
            &format!("{repo}/issues"),
            Some(json!({
                "title": format!("Scheduled {task} run has failures"),
                "body": body(task, failures, maintainers, run),
            })),
        )?;
        return Ok(Report::Filed(url(&created)?));
    };
    let path = format!(
        "{repo}/issues/{}",
        issue["number"].as_u64().context("missing issue number")?
    );
    if failures.is_empty() {
        let closed = required(
            api,
            "PATCH",
            &path,
            Some(json!({"state": "closed", "state_reason": "completed"})),
        )?;
        return Ok(Report::Resolved(url(&closed)?));
    }
    let updated = required(
        api,
        "PATCH",
        &path,
        Some(json!({"body": body(task, failures, maintainers, run)})),
    )?;
    Ok(Report::Updated(url(&updated)?))
}
