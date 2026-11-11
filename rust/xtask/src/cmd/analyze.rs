//! Parse a WAT/WAST file and show conservative syntax evidence and warnings.
use anyhow::Result;
use cm_catalog::records::encode;
use std::{
    fs,
    path::{Path, PathBuf},
};

#[derive(clap::Args)]
pub struct Args {
    pub(crate) file: PathBuf,
}

/// Parse a WAT/WAST file and show conservative syntax evidence and warnings.
pub fn run(_root: &Path, args: Args) -> Result<()> {
    let Args { file } = args;

    println!(
        "{}",
        encode(&cm_wast_analysis::analyze(&fs::read_to_string(file)?)?)?
    );

    Ok(())
}
