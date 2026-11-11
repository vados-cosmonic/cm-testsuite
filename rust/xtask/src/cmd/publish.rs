//! Explicitly create/update a draft PR. Requires caller-supplied GITHUB_TOKEN.
use crate::io::load_plan;
use anyhow::{Context, Result, ensure};
use std::path::{Path, PathBuf};

#[derive(clap::Args)]
pub struct Args {
    #[arg(long)]
    pub(crate) repository: String,
    #[arg(long)]
    pub(crate) plan: PathBuf,
}

/// Explicitly create/update a draft PR. Requires caller-supplied GITHUB_TOKEN.
pub fn run(_root: &Path, args: Args) -> Result<()> {
    let Args { repository, plan } = args;

    let token = std::env::var("GITHUB_TOKEN")
        .context("publish requires GITHUB_TOKEN supplied by the caller")?;
    ensure!(!token.is_empty(), "GITHUB_TOKEN is empty");
    let api = cm_github::Client::new(Some(token))?;
    println!(
        "{:?}",
        cm_delivery::publish(&api, &repository, &load_plan(&plan)?)?
    );

    Ok(())
}
