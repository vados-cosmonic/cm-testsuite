//! Explicitly create/update a draft PR with caller-supplied or Octo STS credentials.
use crate::exchange::{Access, Credentials};
use crate::io::load_plan;
use anyhow::Result;
use cm_delivery::{Offer, Review, branch_for};
use std::path::{Path, PathBuf};

#[derive(clap::Args)]
pub struct Args {
    #[arg(long)]
    pub(crate) repository: String,
    #[arg(long)]
    pub(crate) plan: PathBuf,
    /// Open the pull request again when an identical one was closed without merging.
    #[arg(long)]
    pub(crate) recreate: bool,
    #[command(flatten)]
    pub(crate) credentials: Credentials,
}

/// Explicitly create/update a draft PR with caller-supplied or Octo STS credentials.
pub fn run(_root: &Path, args: Args) -> Result<()> {
    let Args {
        repository,
        plan,
        recreate,
        credentials,
    } = args;

    let access = Access::new(&credentials, true)?;
    let plan = load_plan(&plan)?;
    let offer = Offer {
        review: Review::Draft,
        recreate,
    };
    println!(
        "{:?}",
        cm_delivery::publish_on(&access.api(), &repository, &plan, &branch_for(&plan), offer)?
    );

    Ok(())
}
