//! Verify catalog digests and structurally parse every WAST; never execute it.
use anyhow::Result;
use cm_catalog::{Catalog, digest};
use std::path::Path;

#[derive(clap::Args)]
pub struct Args {}

/// Verify catalog digests and structurally parse every WAST; never execute it.
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

    Ok(())
}
