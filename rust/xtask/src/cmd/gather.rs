//! Gather updates from enrolled repositories into one catalog import each.
use crate::exchange::{
    Access, Credentials, Selection, github_name, participants, planned, published, settle,
};
use anyhow::{Context, Result};
use cm_catalog::PARTICIPANTS_PATH;
use cm_delivery::{Failure, Source};
use cm_exchange_plan::Plan;
use cm_github::Api;
use std::path::{Path, PathBuf};

#[derive(clap::Args)]
pub struct Args {
    /// Registry of enrolled repositories, relative to --root.
    #[arg(long, default_value = PARTICIPANTS_PATH)]
    pub(crate) registry: PathBuf,
    #[command(flatten)]
    pub(crate) selection: Selection,
    /// Write the imports into the catalog checkout at --root.
    #[arg(long, conflicts_with = "publish")]
    pub(crate) apply: bool,
    /// Open, update, or withdraw one draft import pull request per source
    /// repository in the catalog repository.
    #[arg(long)]
    pub(crate) publish: bool,
    /// Catalog repository to publish to; defaults to GITHUB_REPOSITORY.
    #[arg(long)]
    pub(crate) repository: Option<String>,
    #[command(flatten)]
    pub(crate) credentials: Credentials,
    /// File failures as one issue in this repository instead of failing the run.
    #[arg(long, conflicts_with_all = ["only", "except"])]
    pub(crate) report_to: Option<String>,
}

/// What to do with each repository's import once it is planned.
enum Mode {
    DryRun,
    Apply,
    /// Publish to this catalog repository.
    Publish(String),
}

/// Act on one repository's import and say in one phrase what happened.
fn conclude(api: &dyn Api, root: &Path, plan: &Plan, mode: &Mode) -> Result<String> {
    match mode {
        Mode::Apply if !plan.edits.is_empty() => {
            cm_delivery::apply(root, plan)?;
            Ok(format!("applied {}", planned(plan)))
        }
        Mode::DryRun | Mode::Apply => Ok(planned(plan)),
        Mode::Publish(target) => {
            // Every import proposal is opened here, on its source's own branch.
            let publication = cm_delivery::publish(api, target, plan)
                .with_context(|| format!("publishing the import to {target}"))?;
            Ok(published(plan, &publication))
        }
    }
}

/// Gather one repository and say in one phrase what was found and done.
///
/// Each repository is planned after the ones before it were concluded, so
/// with --apply a plan sees the imports already written.
fn gather(
    api: &dyn Api,
    root: &Path,
    participant: &cm_catalog::Participant,
    mode: &Mode,
) -> Result<String> {
    Ok(match cm_delivery::gather(api, root, participant)? {
        Source::Acquired {
            tests,
            revision,
            plan,
        } => format!(
            "{tests} tests at {}; {}",
            &revision[..12],
            conclude(api, root, &plan, mode)?
        ),
        // Nothing to import; publishing only withdraws a proposal left open.
        Source::NoTests(plan) => match mode {
            Mode::Publish(_) => format!("shares no tests; {}", conclude(api, root, &plan, mode)?),
            Mode::DryRun | Mode::Apply => "shares no tests".into(),
        },
    })
}

/// Gather updates from enrolled repositories into one catalog import each.
///
/// Without --apply or --publish this only reports what would change.
pub fn run(root: &Path, args: Args) -> Result<()> {
    let Args {
        registry: path,
        selection,
        apply,
        publish,
        repository,
        credentials,
        report_to,
    } = args;

    let registry = participants(root, &path, &selection)?;
    let access = Access::new(&credentials, publish)?;
    let mode = match (apply, publish) {
        (true, _) => Mode::Apply,
        (_, true) => Mode::Publish(
            repository
                .or_else(|| std::env::var("GITHUB_REPOSITORY").ok())
                .map(|name| github_name(&name))
                .context("--publish needs --repository or GITHUB_REPOSITORY")?,
        ),
        _ => Mode::DryRun,
    };
    let api = access.api();

    // A repository that cannot be read, planned, or published is reported and
    // left as it is, with its open pull request; the others carry on.
    let mut failures = vec![];
    for participant in &registry.participant {
        let repository = &participant.repository;
        match gather(&api, root, participant, &mode) {
            Ok(outcome) => println!("{repository}: {outcome}"),
            Err(error) => {
                println!("{repository}: failed: {error:#}");
                failures.push(Failure {
                    repository: repository.clone(),
                    error: format!("{error:#}"),
                    contacts: participant.maintainers.clone(),
                });
            }
        }
    }
    // Publishing is over: revoke the import token before reporting.
    drop(api);
    if registry.participant.is_empty() {
        println!("no repositories are enrolled in {}", path.display());
    } else if matches!(mode, Mode::DryRun) {
        println!("dry run; pass --apply or --publish");
    }

    settle(
        "gather",
        report_to.as_deref(),
        &failures,
        &registry.maintainers,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use cm_catalog::{Catalog, Participants};
    use cm_test_support::github::FakeGitHubs;

    const CATALOG: &str = "github:example/catalog";

    fn share(hub: &FakeGitHubs, repository: &str, name: &str, text: &str) {
        let manifest = format!(
            "schema=1\n[[test]]\nname='{name}'\npath='file:///tests/{name}.wast'\n\
             summary='Check {name}'\nrequires=['component-model']\ncovers=['smoke']\n"
        );
        let path = format!("tests/{name}.wast");
        hub.push(
            repository,
            &[(".cm-testsuite.toml", &manifest), (&path, text)],
        );
    }

    fn enrolled(names: &[&str]) -> Vec<cm_catalog::Participant> {
        let entries: String = names
            .iter()
            .map(|name| format!("[[participant]]\nrepository='github:{name}'\n"))
            .collect();
        Participants::parse(format!("schema=1\n{entries}").as_bytes())
            .unwrap()
            .participant
    }

    #[test]
    fn each_repository_is_concluded_alone_and_says_what_happened() {
        let hub = FakeGitHubs::new(&["example/a", "example/b", "example/r", "example/catalog"]);
        share(&hub, "example/a", "one", ";; intent\n(module)\n");
        share(&hub, "example/b", "two", ";; intent\n(component)\n");
        hub.push("example/r", &[(".cm-testsuite.toml", "schema=1\n")]);
        let root = tempfile::tempdir().unwrap();
        let participants = enrolled(&["example/a", "example/missing", "example/b", "example/r"]);
        let run = |mode: &Mode| -> Vec<Result<String>> {
            let each = participants.iter();
            each.map(|participant| gather(&hub, root.path(), participant, mode))
                .collect()
        };

        // A dry run writes nothing, and an unreadable repository fails alone.
        let found = run(&Mode::DryRun);
        assert_eq!(hub.writes(), 0);
        assert!(root.path().read_dir().unwrap().next().is_none());
        let said = |outcome: &Result<String>| outcome.as_ref().unwrap().clone();
        assert!(
            said(&found[0]).ends_with("; 1 added"),
            "{}",
            said(&found[0])
        );
        assert!(found[1].is_err());
        assert!(said(&found[2]).starts_with("1 tests at "));
        assert_eq!(said(&found[3]), "shares no tests");

        // Publishing names each pull request beside its repository.
        let published = run(&Mode::Publish(CATALOG.into()));
        assert!(said(&published[0]).ends_with("; 1 added; https://example.invalid/pull/1"));
        assert!(published[1].is_err());
        assert!(said(&published[2]).ends_with("; 1 added; https://example.invalid/pull/2"));
        assert_eq!(said(&published[3]), "shares no tests; up to date");
        {
            let state = hub.repository("example/catalog").state();
            let heads: Vec<_> = state.prs.iter().map(|pr| pr["head"].clone()).collect();
            assert_eq!(
                heads,
                [
                    "cm-testsuite/sources/example/a",
                    "cm-testsuite/sources/example/b"
                ]
            );
            assert!(state.prs.iter().all(|pr| pr["draft"] == true));
        }

        // Applying writes each import before the next repository is planned.
        let applied = run(&Mode::Apply);
        assert!(said(&applied[0]).ends_with("; applied 1 added"));
        assert!(said(&applied[2]).ends_with("; applied 1 added"));
        assert_eq!(Catalog::load(root.path()).unwrap().tests.len(), 2);
        let repeated = run(&Mode::Apply);
        assert!(said(&repeated[0]).ends_with("; up to date"));
        assert!(said(&repeated[2]).ends_with("; up to date"));
    }
}
