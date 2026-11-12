//! Inputs and reporting shared by the gather and distribute commands.
use anyhow::{Context, Result, ensure};
use cm_catalog::Participants;
use cm_delivery::{Failure, Publication, Report};
use cm_exchange_plan::Plan;
use cm_github::{OctoSts, Scoped};
use std::path::Path;

/// Limit a run to part of the registry.
#[derive(clap::Args, Default)]
pub struct Selection {
    /// Process only these enrolled repositories; separate names with commas or repeat the flag.
    #[arg(long, value_delimiter = ',', conflicts_with = "except")]
    pub(crate) only: Vec<String>,
    /// Skip these enrolled repositories; separate names with commas or repeat the flag.
    #[arg(long, value_delimiter = ',')]
    pub(crate) except: Vec<String>,
}

impl Selection {
    /// Whether the run was narrowed to part of the registry.
    pub fn is_partial(&self) -> bool {
        !self.only.is_empty() || !self.except.is_empty()
    }

    /// Keep the selected participants, rejecting names that are not enrolled
    /// so a typo cannot silently widen or empty a run.
    fn apply(&self, registry: &mut Participants) -> Result<()> {
        // Lists typed by hand come as "a/b, c/d" or with a trailing comma.
        let names = |names: &[String]| -> Vec<String> {
            names
                .iter()
                .map(|name| name.trim())
                .filter(|name| !name.is_empty())
                .map(github_name)
                .collect()
        };
        let (only, except) = (names(&self.only), names(&self.except));
        // A list of nothing but commas or blanks must not fall back to every repository.
        ensure!(
            only.is_empty() == self.only.is_empty() && except.is_empty() == self.except.is_empty(),
            "--only and --except need at least one repository name"
        );
        for repository in only.iter().chain(&except) {
            ensure!(
                registry
                    .participant
                    .iter()
                    .any(|p| &p.repository == repository),
                "{repository} is not enrolled"
            );
        }
        registry.participant.retain(|p| {
            (only.is_empty() || only.contains(&p.repository)) && !except.contains(&p.repository)
        });
        Ok(())
    }
}

/// Load the registry, narrowed to the selected repositories.
pub fn participants(root: &Path, registry: &Path, selection: &Selection) -> Result<Participants> {
    let mut participants = Participants::load(&root.join(registry))?;
    selection
        .apply(&mut participants)
        .with_context(|| format!("selecting from {}", registry.display()))?;
    Ok(participants)
}

/// Read a caller-supplied token; reads of public repositories work without one.
fn token_from(variable: &str) -> Option<String> {
    std::env::var(variable)
        .ok()
        .filter(|token| !token.trim().is_empty())
}

/// The token from the environment, used to read repositories and,
/// without Octo STS, to publish.
fn token() -> Option<String> {
    token_from("GITHUB_TOKEN").or_else(|| token_from("GH_TOKEN"))
}

/// The token of the GitHub CLI's logged-in account, when `gh` is installed and signed in.
fn ambient_token() -> Option<String> {
    let output = std::process::Command::new("gh")
        .args(["auth", "token"])
        .stderr(std::process::Stdio::null())
        .output()
        .ok()
        .filter(|output| output.status.success())?;
    let token = String::from_utf8(output.stdout).ok()?;
    Some(token.trim().to_owned()).filter(|token| !token.is_empty())
}

/// Whether this process is a GitHub Actions job, where Octo STS can be used.
fn in_actions() -> bool {
    std::env::var("GITHUB_ACTIONS").is_ok_and(|value| value == "true")
}

/// How a run authenticates to GitHub.
#[derive(clap::Args, Default)]
pub struct Credentials {
    /// Inside GitHub Actions, publish with short-lived Octo STS tokens for this
    /// trust-policy name, minted per destination repository. Ignored elsewhere,
    /// where GITHUB_TOKEN, GH_TOKEN, or the GitHub CLI's login is used.
    #[arg(long, value_name = "NAME")]
    pub(crate) sts_identity: Option<String>,
}

/// The credentials of one run: an environment token for reading, and for
/// publishing either a token Octo STS mints for each destination, that same
/// environment token, or the GitHub CLI's login.
pub struct Access {
    token: Option<String>,
    reader: cm_github::Client,
    sts: Option<OctoSts>,
}

impl Access {
    /// Check up front that a publishing run has a way to obtain write access.
    pub fn new(credentials: &Credentials, publishing: bool) -> Result<Self> {
        let mut token = token();
        // Octo STS trusts GitHub Actions workflows only, so the same command
        // line run anywhere else publishes with the caller's own credentials.
        let sts = credentials
            .sts_identity
            .as_deref()
            .filter(|_| in_actions())
            .map(OctoSts::from_actions)
            .transpose()?;
        if publishing && sts.is_none() {
            // Only a publishing run reaches for the GitHub CLI's login, so
            // reporting runs never depend on who happens to be signed in.
            token = token.or_else(ambient_token);
            ensure!(
                token.is_some(),
                "--publish requires GITHUB_TOKEN or GH_TOKEN, a GitHub CLI login (gh auth login), \
                 or --sts-identity inside GitHub Actions"
            );
        }
        Ok(Self {
            reader: cm_github::Client::new(token.clone())?,
            token,
            sts,
        })
    }

    /// The API for this run. Write access is obtained per repository, on its first write.
    ///
    /// An Octo STS token is revoked when its repository is released or the API
    /// is dropped. The caller's own token is never revoked.
    pub fn api(&self) -> Scoped<'_> {
        Scoped::new(
            &self.reader,
            Box::new(|repository| match &self.sts {
                Some(sts) => Ok(Box::new(sts.connect(repository)?)),
                None => Ok(Box::new(cm_github::Client::new(self.token.clone())?)),
            }),
        )
    }
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
        Publication::Declined => {
            "identical proposal was closed earlier; not opened again (pass --recreate to do so)"
                .into()
        }
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
/// REPORT_TOKEN when set, then the environment or GitHub CLI token; never an Octo STS token.
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
        .or_else(ambient_token)
        .context(
            "--report-to requires REPORT_TOKEN, GITHUB_TOKEN, GH_TOKEN, or a GitHub CLI login",
        )?;
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

#[cfg(test)]
mod tests {
    use super::*;

    fn selected(only: &[&str], except: &[&str]) -> Result<Vec<String>> {
        let mut registry = Participants::parse(
            b"schema = 1\n[[participant]]\nrepository = 'github:example/a'\n\
              [[participant]]\nrepository = 'github:example/b'\n\
              [[participant]]\nrepository = 'github:example/c'\n",
        )?;
        let names = |names: &[&str]| names.iter().map(|name| (*name).to_owned()).collect();
        let selection = Selection {
            only: names(only),
            except: names(except),
        };
        selection.apply(&mut registry)?;
        Ok(registry
            .participant
            .into_iter()
            .map(|p| p.repository)
            .collect())
    }

    #[test]
    fn selects_by_inclusion_or_exclusion_in_registry_order() {
        let all = ["github:example/a", "github:example/b", "github:example/c"];
        assert_eq!(selected(&[], &[]).unwrap(), all);
        // Either spelling of a repository name is accepted.
        assert_eq!(
            selected(&["example/c", "github:example/a"], &[]).unwrap(),
            [all[0], all[2]]
        );
        assert_eq!(selected(&[], &["example/b"]).unwrap(), [all[0], all[2]]);
        // Padding and empty entries left by a hand-typed list are ignored.
        assert_eq!(selected(&[" example/b", ""], &[]).unwrap(), [all[1]]);
        // A list that names nothing is an error, not the whole registry.
        assert!(selected(&[" ", ""], &[]).is_err());
        assert!(selected(&[], &[""]).is_err());
        assert!(selected(&["example/missing"], &[]).is_err());
        assert!(selected(&[], &["example/missing"]).is_err());
    }
}
