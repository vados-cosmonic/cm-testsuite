//! Apply a reviewed plan after checking all file and policy baselines.
use crate::io::load_plan;
use anyhow::Result;
use std::path::{Path, PathBuf};

#[derive(clap::Args)]
pub struct Args {
    #[arg(long)]
    pub(crate) plan: PathBuf,
}

/// Apply a reviewed plan after checking all file and policy baselines.
pub fn run(root: &Path, args: Args) -> Result<()> {
    let Args { plan } = args;

    let count = cm_delivery::apply(root, &load_plan(&plan)?)?;
    println!("Applied {count} file changes");

    Ok(())
}
