//! Build the mdBook with the pinned repository launcher.
use anyhow::{Context, Result, ensure};
use std::{path::Path, process::Command};

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
    let version = Command::new("dotslash")
        .arg(root.join("bins/mdbook"))
        .arg("--version")
        .output()
        .context("DotSlash is required; install it and ensure dotslash is on PATH")?;
    ensure!(
        version.status.success()
            && String::from_utf8_lossy(&version.stdout).trim() == "mdbook v0.5.4",
        "expected pinned mdBook 0.5.4"
    );
    let status = Command::new("dotslash")
        .arg(root.join("bins/mdbook"))
        .arg("build")
        .arg(root.join("docs"))
        .arg("--dest-dir")
        .arg(root.join("site-out/docs"))
        .status()?;
    ensure!(status.success(), "mdBook build failed");
    Ok(())
}
