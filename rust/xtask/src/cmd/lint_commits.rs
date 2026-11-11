//! Validate an exact Git range with the repository's commitlint configuration.

use anyhow::{Context, Result, ensure};
use cm_release::history;
use std::{path::Path, process::Command};

#[derive(clap::Args)]
pub struct Args {
    #[arg(long)]
    from: String,
    #[arg(long)]
    to: String,
}

/// Validate every commit in the requested range with the repository configuration.
pub fn run(root: &Path, args: Args) -> Result<()> {
    let Args { from, to } = args;
    let from = history::resolve(root, &from)?;
    let to = history::resolve(root, &to)?;
    // Rust only infers .exe on Windows; npm is installed as a .cmd shim.
    let npm = if cfg!(windows) { "npm.cmd" } else { "npm" };
    let status = Command::new(npm)
        .current_dir(root)
        .args([
            "exec",
            "--no",
            "--",
            "commitlint",
            "--from",
            &from,
            "--to",
            &to,
            "--verbose",
        ])
        .status()
        .context("install Node.js and run npm ci before linting commits")?;
    ensure!(status.success(), "commitlint rejected one or more commits");
    Ok(())
}
