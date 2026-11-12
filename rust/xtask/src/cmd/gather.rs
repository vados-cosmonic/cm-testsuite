//! Gather updates from enrolled repositories into one catalog import.
use crate::exchange::{
    Access, Credentials, Selection, github_name, participants, planned, published, settle,
};
use anyhow::{Context, Result};
use cm_catalog::PARTICIPANTS_PATH;
use cm_delivery::{Failure, GATHER_BRANCH, Gathered, Offer, Review, Source, branch_for};
use cm_github::Api;
use std::path::{Path, PathBuf};

#[derive(clap::Args)]
pub struct Args {
    /// Registry of enrolled repositories, relative to --root.
    #[arg(long, default_value = PARTICIPANTS_PATH)]
    pub(crate) registry: PathBuf,
    #[command(flatten)]
    pub(crate) selection: Selection,
    /// Write the import into the catalog checkout at --root.
    #[arg(long, conflicts_with = "publish")]
    pub(crate) apply: bool,
    /// Open or update the catalog repository's import pull request: the shared one
    /// for a whole-registry run, a separate one for a run narrowed by --only or --except.
    /// A new one opens ready for review; an open one keeps its draft state.
    #[arg(long)]
    pub(crate) publish: bool,
    /// Open the import pull request again when an identical one was closed without merging.
    #[arg(long, requires = "publish")]
    pub(crate) recreate: bool,
    /// Catalog repository to publish to; defaults to GITHUB_REPOSITORY.
    #[arg(long)]
    pub(crate) repository: Option<String>,
    #[command(flatten)]
    pub(crate) credentials: Credentials,
    /// File failures as one issue in this repository instead of failing the run.
    #[arg(long, conflicts_with_all = ["only", "except"])]
    pub(crate) report_to: Option<String>,
}

/// What to do with the combined import once it is planned.
enum Mode {
    DryRun,
    Apply,
    /// Publish to `repository`, on the shared branch when the run covers the whole registry.
    Publish {
        repository: Option<String>,
        shared: bool,
        recreate: bool,
    },
}

/// Act on the combined import and say in one phrase what happened.
fn conclude(api: &dyn Api, root: &Path, gathered: &Gathered, mode: &Mode) -> Result<String> {
    let Some(plan) = &gathered.plan else {
        return Ok("nothing to gather".into());
    };
    match mode {
        Mode::DryRun => Ok(format!(
            "{} (dry run; pass --apply or --publish)",
            planned(plan)
        )),
        Mode::Apply => {
            cm_delivery::apply(root, plan)?;
            Ok(format!("applied {}", planned(plan)))
        }
        Mode::Publish {
            repository,
            shared,
            recreate,
        } => {
            let target = repository
                .clone()
                .or_else(|| std::env::var("GITHUB_REPOSITORY").ok())
                .map(|name| github_name(&name))
                .context("--publish needs --repository or GITHUB_REPOSITORY")?;
            let unread = gathered
                .sources
                .iter()
                .any(|(_, source)| matches!(source, Source::Failed(_)));
            // An empty plan withdraws the open proposal. That is wrong while a
            // repository could not be read: its pending changes are still wanted.
            if plan.edits.is_empty() && unread {
                return Ok("nothing new from the repositories that could be read; \
                           any open import pull request is left as it is"
                    .into());
            }
            // Only a run over the whole registry speaks for the shared proposal.
            let branch = if *shared {
                GATHER_BRANCH.to_owned()
            } else {
                branch_for(plan)
            };
            // The catalog runs gather for itself, so either proposal opens ready for review.
            let offer = Offer {
                review: Review::Ready,
                recreate: *recreate,
            };
            let publication = cm_delivery::publish_on(api, &target, plan, &branch, offer)
                .with_context(|| format!("publishing the import to {target}"))?;
            Ok(published(plan, &publication))
        }
    }
}

/// Gather updates from enrolled repositories into one catalog import.
///
/// Without --apply or --publish this only reports what would change.
pub fn run(root: &Path, args: Args) -> Result<()> {
    let Args {
        registry,
        selection,
        apply,
        publish,
        recreate,
        repository,
        credentials,
        report_to,
    } = args;

    let registry = participants(root, &registry, &selection)?;
    let access = Access::new(&credentials, publish)?;
    let mode = match (apply, publish) {
        (true, _) => Mode::Apply,
        (_, true) => Mode::Publish {
            repository,
            shared: !selection.is_partial(),
            recreate,
        },
        _ => Mode::DryRun,
    };
    let api = access.api();

    // A repository that cannot be read is reported and left out. A failure of
    // the combined import is reported too, against the catalog as a whole.
    let mut failures = vec![];
    let catalog = cm_delivery::gather(&api, root, &registry.participant).and_then(|gathered| {
        for (participant, (repository, source)) in
            registry.participant.iter().zip(&gathered.sources)
        {
            match source {
                Source::Acquired { tests, revision } => {
                    println!("{repository}: {tests} tests at {}", &revision[..12]);
                }
                Source::NoTests => println!("{repository}: shares no tests"),
                Source::Failed(error) => {
                    println!("{repository}: failed: {error:#}");
                    failures.push(Failure {
                        repository: repository.clone(),
                        error: format!("{error:#}"),
                        contacts: participant.maintainers.clone(),
                    });
                }
            }
        }
        conclude(&api, root, &gathered, &mode)
    });
    // Publishing is over: revoke the import token before reporting.
    drop(api);
    match catalog {
        Ok(outcome) => println!("catalog: {outcome}"),
        Err(error) => {
            println!("catalog: failed: {error:#}");
            failures.push(Failure {
                repository: "catalog import".into(),
                error: format!("{error:#}"),
                contacts: vec![],
            });
        }
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
    use cm_catalog::Participants;
    use cm_test_support::github::FakeGitHubs;

    #[test]
    fn publishing_opens_ready_import_pull_requests_and_recreates_closed_ones() {
        let hub = FakeGitHubs::new(&["example/a", "example/catalog"]);
        hub.push(
            "example/a",
            &[
                (
                    ".cm-testsuite.toml",
                    "schema=1\n[[test]]\nname='one'\npath='file:///tests/one.wast'\n\
                     summary='Check one'\nrequires=['component-model']\ncovers=['smoke']\n",
                ),
                ("tests/one.wast", ";; intent\n(module)\n"),
            ],
        );
        let registry =
            Participants::parse(b"schema=1\n[[participant]]\nrepository='github:example/a'\n")
                .unwrap();
        let root = tempfile::tempdir().unwrap();
        let gathered = cm_delivery::gather(&hub, root.path(), &registry.participant).unwrap();
        let plan = gathered.plan.as_ref().unwrap();
        let publish = |shared, recreate| {
            let mode = Mode::Publish {
                repository: Some("example/catalog".into()),
                shared,
                recreate,
            };
            conclude(&hub, root.path(), &gathered, &mode).unwrap()
        };
        let pulls = || hub.repository("example/catalog").state().prs.clone();

        // A dry run writes nothing.
        conclude(&hub, root.path(), &gathered, &Mode::DryRun).unwrap();
        assert_eq!(hub.writes(), 0);

        // The whole-registry proposal and a narrowed one each open ready for review.
        for (shared, branch) in [(true, GATHER_BRANCH.to_owned()), (false, branch_for(plan))] {
            publish(shared, false);
            let pr = pulls().pop().unwrap();
            assert_eq!(pr["head"], branch);
            assert_eq!(pr["draft"], false);
            assert_eq!(pr["state"], "open");
        }
        assert_eq!(pulls().len(), 2);

        // A closed import stays closed until a run asks for it again, each time it is closed.
        for round in 0..2 {
            let last = pulls().len() - 1;
            hub.repository("example/catalog").state_mut().prs[last]["state"] = "closed".into();
            let declined = publish(false, false);
            assert!(declined.contains("--recreate"), "{declined}");
            assert_eq!(pulls().len(), 2 + round);
            publish(false, true);
            let pr = pulls().pop().unwrap();
            assert_eq!(pulls().len(), 3 + round);
            assert_eq!(pr["head"], branch_for(plan));
            assert_eq!(pr["state"], "open");
            assert_eq!(pr["draft"], false);
        }
    }
}
