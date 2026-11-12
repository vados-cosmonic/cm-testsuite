//! Plan and propose catalog deliveries to one enrolled repository.

use crate::gather::default_branch;
use crate::{Publication, publish};
use anyhow::{Context, Result};
use cm_catalog::{Catalog, Manifest, Participant};
use cm_exchange_plan::{Plan, Tree, receive_from};
use cm_github::{Api, read_file, resolve};
use std::{cell::RefCell, collections::BTreeMap};

/// The files of a repository at one immutable commit, read through the API.
struct Revision<'a> {
    api: &'a dyn Api,
    repository: &'a str,
    revision: String,
    // Planning reads the manifest and receipt several times.
    cache: RefCell<BTreeMap<String, Option<Vec<u8>>>>,
}

impl Tree for Revision<'_> {
    fn read(&self, path: &str) -> Result<Option<Vec<u8>>> {
        if let Some(known) = self.cache.borrow().get(path) {
            return Ok(known.clone());
        }
        let bytes = read_file(self.api, self.repository, &self.revision, path)?;
        self.cache.borrow_mut().insert(path.into(), bytes.clone());
        Ok(bytes)
    }
}

/// What distribution did for one enrolled repository.
#[derive(Debug)]
pub enum Delivery {
    /// The repository's manifest does not enable receiving.
    NotReceiving,
    /// The planned changes; nothing was published.
    Planned(Plan),
    /// The planned changes and the state of the repository's proposal.
    Published(Plan, Publication),
}

/// Plan delivery against a repository's default branch and optionally publish it.
///
/// Publishing reconciles the repository's single draft pull request: it opens
/// or updates it when the plan has changes and withdraws it when none remain.
pub fn distribute(
    api: &dyn Api,
    participant: &Participant,
    catalog: &Catalog,
    files: &BTreeMap<String, String>,
    publish_proposal: bool,
) -> Result<Delivery> {
    let repository = participant.repository.as_str();
    let tree = Revision {
        api,
        repository,
        revision: resolve(api, repository, &default_branch(api, repository)?)?,
        cache: RefCell::default(),
    };
    let raw = tree
        .read(&participant.manifest)?
        .with_context(|| format!("missing manifest {}", participant.manifest))?;
    let manifest = Manifest::parse(std::str::from_utf8(&raw)?, &participant.manifest)?;
    if !manifest.receive.enabled {
        return Ok(Delivery::NotReceiving);
    }
    let plan = receive_from(&tree, repository, &participant.manifest, catalog, files)?;
    if !publish_proposal {
        return Ok(Delivery::Planned(plan));
    }
    // Publication re-reads the default branch and rejects a plan it has outrun.
    let publication = publish(api, repository, &plan)?;
    Ok(Delivery::Published(plan, publication))
}
