//! Plan delivery using the recipient manifest in --root and a curated catalog.
use crate::io::write_record;
use anyhow::Result;
use cm_catalog::Catalog;
use std::path::{Path, PathBuf};

#[derive(clap::Args)]
pub struct Args {
    #[arg(long)]
    pub(crate) catalog_root: PathBuf,
    #[arg(long)]
    pub(crate) recipient: String,
    #[arg(long, default_value = ".cm-testsuite.toml")]
    pub(crate) manifest: String,
    #[arg(long)]
    pub(crate) out: PathBuf,
}

/// Plan delivery using the recipient manifest in --root and a curated catalog.
pub fn run(root: &Path, args: Args) -> Result<()> {
    let Args {
        catalog_root,
        recipient,
        manifest,
        out,
    } = args;

    let catalog = Catalog::load(&catalog_root)?;
    let files = cm_acquisition::load_catalog_files(&catalog_root, &catalog)?;
    let plan = cm_exchange_plan::receive(root, &recipient, &manifest, &catalog, &files)?;
    write_record(&out, &plan)?;
    println!("Planned {} receiving changes", plan.edits.len());
    for line in plan.warning_summary() {
        eprintln!("{line}");
    }

    Ok(())
}
