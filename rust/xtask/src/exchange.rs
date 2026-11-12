//! Inputs and reporting shared by the gather and distribute commands.
use anyhow::{Context, Result, ensure};
use cm_catalog::Participants;
use cm_delivery::{Failure, Publication, Report};
use cm_exchange_plan::Plan;
use std::path::Path;

/// Load the registry, optionally narrowed to the named repositories.
pub fn participants(root: &Path, registry: &Path, only: &[String]) -> Result<Participants> {
    let mut participants = Participants::load(&root.join(registry))?;
    for repository in only {
        ensure!(
            participants
                .participant
                .iter()
                .any(|p| &p.repository == repository),
            "{repository} is not enrolled in {}",
            registry.display()
        );
    }
    if !only.is_empty() {
        participants
            .participant
            .retain(|p| only.contains(&p.repository));
    }
    Ok(participants)
}

/// Read a caller-supplied token; reads of public repositories work without one.
fn token_from(variable: &str) -> Option<String> {
    std::env::var(variable)
        .ok()
        .filter(|token| !token.trim().is_empty())
}

/// The token used to read repositories and publish pull requests.
pub fn token() -> Option<String> {
    token_from("GITHUB_TOKEN")
}

/// Require a token before any step that opens or changes a pull request.
pub fn require_token(token: &Option<String>) -> Result<()> {
    token
        .as_ref()
        .map(|_| ())
        .context("--publish requires GITHUB_TOKEN supplied by the caller")
}

/// Accept `owner/repository` as GitHub Actions spells it, or the tool's `github:` form.
pub fn github_name(name: &str) -> String {
    if name.starts_with("github:") {
        name.to_owned()
    } else {
        format!("github:{name}")
    }
}

/// Say in one phrase what publication did with a plan.
pub fn published(plan: &Plan, publication: &Publication) -> String {
    match publication {
        Publication::NoChanges => "up to date".into(),
        Publication::Declined => "identical proposal was closed earlier; not reopened".into(),
        Publication::Closed(url) => format!("nothing left to propose; closed {url}"),
        Publication::PullRequest(url) => format!("{}; {url}", plan.changes()),
    }
}

/// Say in one phrase what a plan would change.
pub fn planned(plan: &Plan) -> String {
    if plan.edits.is_empty() {
        "up to date".into()
    } else {
        plan.changes().to_string()
    }
}

/// Link the GitHub Actions run this process belongs to, when there is one.
fn run_url() -> Option<String> {
    let variable = |name| std::env::var(name).ok().filter(|value| !value.is_empty());
    Some(format!(
        "{}/{}/actions/runs/{}",
        variable("GITHUB_SERVER_URL")?,
        variable("GITHUB_REPOSITORY")?,
        variable("GITHUB_RUN_ID")?
    ))
}

/// Finish a run: fail on failures, or file them when a report target is named.
///
/// With a target, per-repository failures leave the exit status clean. The
/// issue there is opened, rewritten, or closed to match this run, using
/// REPORT_TOKEN when set and GITHUB_TOKEN otherwise.
pub fn settle(
    task: &str,
    report_to: Option<&str>,
    failures: &[Failure],
    maintainers: &[String],
) -> Result<()> {
    let Some(target) = report_to else {
        ensure!(
            failures.is_empty(),
            "{} repositories failed; see above",
            failures.len()
        );
        return Ok(());
    };
    let token = token_from("REPORT_TOKEN")
        .or_else(token)
        .context("--report-to requires REPORT_TOKEN or GITHUB_TOKEN")?;
    let api = cm_github::Client::new(Some(token))?;
    let report = cm_delivery::report(
        &api,
        &github_name(target),
        task,
        failures,
        maintainers,
        run_url().as_deref(),
    )
    .context("reporting failures")?;
    match report {
        Report::Clear => {}
        Report::Filed(url) => println!("failures: filed {url}"),
        Report::Updated(url) => println!("failures: updated {url}"),
        Report::Resolved(url) => println!("failures: none left; closed {url}"),
    }
    Ok(())
}
