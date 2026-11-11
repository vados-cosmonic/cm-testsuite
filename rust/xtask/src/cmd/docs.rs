//! Build the mdBook with the pinned repository launcher.
use crate::tools;
use anyhow::{Result, ensure};
use std::path::Path;

/// Exact `--version` output of the mdBook release pinned by `bins/mdbook`.
const MDBOOK_VERSION: &str = "mdbook v0.5.4";

#[derive(clap::Args)]
pub struct Args {}

/// Build the mdBook with the pinned repository launcher.
pub fn run(root: &Path, args: Args) -> Result<()> {
    let _ = args;

    docs(root)?;

    Ok(())
}

/// Verify the pinned mdBook and render documentation into the site directory.
fn docs(root: &Path) -> Result<()> {
    let status = tools::pinned(root, "mdbook", MDBOOK_VERSION)?
        .arg("build")
        .arg(root.join("docs"))
        .arg("--dest-dir")
        .arg(root.join("site-out/docs"))
        .status()?;
    ensure!(status.success(), "mdBook build failed");
    Ok(())
}
