//! Admit an engine's TOML report against this exact catalog and approved publishers.
use crate::io::{write, write_record};
use anyhow::Result;
use cm_catalog::{Catalog, digest, records};
use std::{
    fs,
    path::{Path, PathBuf},
};

#[derive(clap::Args)]
pub struct Args {
    #[arg(long)]
    pub(crate) report: PathBuf,
    #[arg(long, default_value = "registry/publishers.toml")]
    pub(crate) publishers: PathBuf,
}

/// Admit an engine's TOML report against this exact catalog and approved publishers.
pub fn run(root: &Path, args: Args) -> Result<()> {
    let Args { report, publishers } = args;

    let catalog = Catalog::load(root)?;
    let allowed = cm_results::Publishers::load(&root.join(publishers))?;
    let submitted = fs::read_to_string(report)?;
    let report: cm_results::Report = records::decode(submitted.as_bytes())?;
    report.validate(&catalog, &allowed)?;
    let directory = root.join("reports").join(digest(&catalog)?);

    let output = directory.join(format!("{}.toml", digest(&report)?));
    // Retain the submitted comments on first admission and existing annotations
    // when the same semantic report is admitted again.
    if output.exists() {
        write_record(&output, &report)?;
    } else {
        write(
            &output,
            &records::preserve(&submitted, &records::encode(&report)?)?,
        )?;
    }
    write_record(&directory.join("catalog.toml"), &catalog)?;
    println!("Accepted {}", output.display());

    Ok(())
}
