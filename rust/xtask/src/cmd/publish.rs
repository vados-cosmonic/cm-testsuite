//! Explicitly create/update a draft PR with caller-supplied or Octo STS credentials.
use crate::exchange::{Access, Credentials};
use crate::io::load_plan;
use anyhow::Result;
use std::path::{Path, PathBuf};

#[derive(clap::Args)]
pub struct Args {
    #[arg(long)]
    pub(crate) repository: String,
    #[arg(long)]
    pub(crate) plan: PathBuf,
    #[command(flatten)]
    pub(crate) credentials: Credentials,
}

/// Explicitly create/update a draft PR with caller-supplied or Octo STS credentials.
pub fn run(_root: &Path, args: Args) -> Result<()> {
    let Args {
        repository,
        plan,
        credentials,
    } = args;

    let access = Access::new(&credentials, true)?;
    println!(
        "{:?}",
        cm_delivery::publish(&access.api(), &repository, &load_plan(&plan)?)?
    );

    Ok(())
}
