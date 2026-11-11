//! Validate an exact Git range with the repository's commitlint configuration.

use anyhow::{Context, Result, ensure};
use cm_acquisition::{LocalGit, Repository};
use std::{path::Path, process::Command};

#[derive(clap::Args)]
pub struct Args {
    /// Commit or ref just before the first commit to lint (exclusive).
    #[arg(long)]
    from: String,

    /// Commit or ref of the last commit to lint (inclusive).
    #[arg(long)]
    to: String,
}

/// Validate every commit in the requested range with the repository configuration.
pub fn run(root: &Path, args: Args) -> Result<()> {
    let Args { from, to } = args;

    // Resolve both ends first so commitlint receives exact commits, never
    // option-like or ambiguous text.
    let repository = LocalGit {
        root: root.to_owned(),
    };
    let from = repository.resolve(&from)?;
    let to = repository.resolve(&to)?;

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
