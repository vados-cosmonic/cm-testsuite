//! Verify catalog digests, parse every WAST without executing it, and check the registries.
use anyhow::{Context, Result};
use cm_catalog::{Catalog, PARTICIPANTS_PATH, Participants, digest};
use cm_results::{PUBLISHERS_PATH, Publishers};
use std::path::Path;

#[derive(clap::Args)]
pub struct Args {}

/// Verify the catalog, parse every WAST without executing it, and check the registries.
pub fn run(root: &Path, args: Args) -> Result<()> {
    let _ = args;

    let catalog = Catalog::load(root)?;
    let files = cm_acquisition::load_catalog_files(root, &catalog)?;
    for text in files.values() {
        cm_wast_analysis::analyze(text)?;
    }
    println!(
        "Validated {} WAST files; catalog {}",
        catalog.tests.len(),
        digest(&catalog)?
    );
    registries(root)
}

/// Load both registries through the loaders the exchange itself uses.
///
/// A catalog checkout without a `registry/` directory, such as a demo or
/// fixture hub, has nothing to check. Once the directory exists, both
/// registries must load.
fn registries(root: &Path) -> Result<()> {
    if !root.join("registry").is_dir() {
        return Ok(());
    }
    let participants = Participants::load(&root.join(PARTICIPANTS_PATH))
        .with_context(|| format!("invalid participant registry {PARTICIPANTS_PATH}"))?;
    let publishers = Publishers::load(&root.join(PUBLISHERS_PATH))
        .with_context(|| format!("invalid publisher registry {PUBLISHERS_PATH}"))?;
    println!(
        "Validated registries; {} participants, {} publishers",
        participants.participant.len(),
        publishers.len()
    );

    Ok(())
}
