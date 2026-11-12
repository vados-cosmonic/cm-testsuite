//! Collect updates from every enrolled repository into one import plan.

use anyhow::{Context, Result};
use cm_acquisition::{GitHub, Snapshot, acquire};
use cm_catalog::{Catalog, Participant};
use cm_exchange_plan::{Plan, import};
use cm_github::{Api, repo_path, required};
use std::path::Path;

/// What gathering found in one enrolled repository.
#[derive(Debug)]
pub enum Source {
    /// A complete snapshot of this many tests at the given commit.
    Acquired { tests: usize, revision: String },
    /// The repository shares no tests and the catalog holds none from it.
    NoTests,
    /// Acquisition failed; the catalog keeps what it has from this repository.
    Failed(anyhow::Error),
}

/// The combined import plan and the per-repository findings behind it.
#[derive(Debug)]
pub struct Gathered {
    /// One plan covering every acquired repository; absent when none was acquired.
    pub plan: Option<Plan>,
    /// Findings in registry order, by repository.
    pub sources: Vec<(String, Source)>,
}

/// Name the branch a repository treats as its default.
pub(crate) fn default_branch(api: &dyn Api, repository: &str) -> Result<String> {
    let info = required(api, "GET", &repo_path(repository)?, None)?;
    Ok(info["default_branch"]
        .as_str()
        .context("missing default branch")?
        .to_owned())
}

/// Acquire every enrolled repository and plan one import against the catalog in `root`.
///
/// A repository that fails acquisition is reported and left out, so it can
/// neither block the others nor retire its own tests. Nothing is written.
pub fn gather(api: &dyn Api, root: &Path, participants: &[Participant]) -> Result<Gathered> {
    let catalog = Catalog::load(root)?;
    let mut snapshots: Vec<Snapshot> = vec![];
    let mut sources = vec![];
    for participant in participants {
        let repository = participant.repository.as_str();
        let acquired = participant
            .reference
            .clone()
            .map_or_else(|| default_branch(api, repository), Ok)
            .and_then(|reference| {
                acquire(
                    &GitHub {
                        api,
                        source: repository,
                    },
                    repository,
                    &reference,
                    &participant.manifest,
                )
            });
        let source = match acquired {
            Err(error) => Source::Failed(error),
            // A receive-only participant must not appear in the catalog as an
            // empty source, but a source that dropped every test must retire them.
            Ok(snapshot)
                if snapshot.tests.is_empty() && !catalog.sources.contains_key(repository) =>
            {
                Source::NoTests
            }
            Ok(snapshot) => {
                let source = Source::Acquired {
                    tests: snapshot.tests.len(),
                    revision: snapshot.metadata.revision.clone(),
                };
                snapshots.push(snapshot);
                source
            }
        };
        sources.push((repository.to_owned(), source));
    }
    let plan = if snapshots.is_empty() {
        None
    } else {
        Some(import(root, &snapshots)?)
    };
    Ok(Gathered { plan, sources })
}
