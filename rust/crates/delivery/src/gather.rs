//! Plan the import of one enrolled repository's tests into the catalog.

use anyhow::{Context, Result};
use cm_acquisition::{GitHub, Snapshot, acquire};
use cm_catalog::{Catalog, Participant};
use cm_exchange_plan::{Plan, import};
use cm_github::{Api, repo_path, required};
use std::path::Path;

/// What gathering found in one enrolled repository.
#[derive(Debug)]
pub enum Source {
    /// A complete snapshot of this many tests at the given commit, and the
    /// import it calls for. The plan is empty when no shared test changed.
    Acquired {
        tests: usize,
        revision: String,
        plan: Plan,
    },
    /// The repository shares no tests and the catalog holds none from it.
    /// The plan is empty; publishing it withdraws a proposal left open from
    /// when the repository did share tests.
    NoTests(Plan),
}

impl Source {
    /// The import of this repository alone.
    pub fn plan(&self) -> &Plan {
        match self {
            Self::Acquired { plan, .. } | Self::NoTests(plan) => plan,
        }
    }
}

/// Name the branch a repository treats as its default.
pub(crate) fn default_branch(api: &dyn Api, repository: &str) -> Result<String> {
    let info = required(api, "GET", &repo_path(repository)?, None)?;
    Ok(info["default_branch"]
        .as_str()
        .context("missing default branch")?
        .to_owned())
}

/// Acquire one enrolled repository and plan its import against the catalog in `root`.
///
/// Each repository has its own plan, so one that cannot be read or planned
/// fails alone: it can neither block the others nor retire its own tests.
/// Nothing is written.
pub fn gather(api: &dyn Api, root: &Path, participant: &Participant) -> Result<Source> {
    let repository = participant.repository.as_str();
    let reference = match &participant.reference {
        Some(reference) => reference.clone(),
        None => default_branch(api, repository)?,
    };
    let source = GitHub {
        api,
        source: repository,
    };
    let snapshot = acquire(&source, repository, &reference, &participant.manifest)?;
    let catalog = Catalog::load(root)?;
    let mut plan = import(root, std::slice::from_ref(&snapshot))?;
    // A receive-only participant must not appear in the catalog as an empty
    // source, but a source that dropped every test must retire them.
    let absent = snapshot.tests.is_empty() && !catalog.sources.contains_key(repository);
    // Commits that touch no shared test would otherwise advance the recorded
    // revision, proposing an import on every run and a receipt update in
    // every receiving repository after it.
    if absent || current(&catalog, &snapshot) {
        plan.edits.clear();
        plan.provenance.clear();
    }
    Ok(if absent {
        Source::NoTests(plan)
    } else {
        Source::Acquired {
            tests: snapshot.tests.len(),
            revision: snapshot.metadata.revision,
            plan,
        }
    })
}

/// Whether the catalog already holds everything in a snapshot, apart from
/// the commit the snapshot was read at.
fn current(catalog: &Catalog, snapshot: &Snapshot) -> bool {
    let Some(recorded) = catalog.sources.get(&snapshot.source) else {
        return false;
    };
    let held = || {
        catalog
            .tests
            .iter()
            .filter(|(_, test)| test.source == snapshot.source)
    };
    recorded.manifest_path == snapshot.metadata.manifest_path
        && recorded.manifest_sha256 == snapshot.metadata.manifest_sha256
        && held().count() == snapshot.tests.len()
        && held().all(|(id, old)| {
            snapshot.tests.get(id).is_some_and(|new| {
                let mut new = new.clone();
                new.revision.clone_from(&old.revision);
                new.manifest_revision.clone_from(&old.manifest_revision);
                &new == old
            })
        })
}
