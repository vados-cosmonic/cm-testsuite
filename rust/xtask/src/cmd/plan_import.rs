//! Prepare a reviewed catalog/corpus change; does not mutate the repository.
use crate::io::write_record;
use anyhow::Result;
use std::path::{Path, PathBuf};

#[derive(clap::Args)]
pub struct Args {
    #[arg(long, required = true)]
    pub(crate) snapshot: Vec<PathBuf>,
    #[arg(long)]
    pub(crate) out: PathBuf,
}

/// Prepare a reviewed catalog/corpus change; does not mutate the repository.
pub fn run(root: &Path, args: Args) -> Result<()> {
    let Args { snapshot, out } = args;

    let snapshots = snapshot
        .iter()
        .map(|p| cm_acquisition::load_snapshot(p))
        .collect::<Result<Vec<_>>>()?;
    let plan = cm_exchange_plan::import(root, &snapshots)?;
    write_record(&out, &plan)?;
    println!("Planned {} file changes", plan.edits.len());

    Ok(())
}
