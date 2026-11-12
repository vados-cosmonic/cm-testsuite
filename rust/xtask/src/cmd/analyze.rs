//! Parse a WAT/WAST file and show conservative syntax evidence and warnings.
use anyhow::Result;
use cm_catalog::{records::encode, warnings::explain};
use std::{
    fs,
    path::{Path, PathBuf},
};

#[derive(clap::Args)]
pub struct Args {
    pub(crate) file: PathBuf,
}

/// Parse a WAT/WAST file and show conservative syntax evidence and warnings.
///
/// The analysis record goes to standard output as it is stored;
/// what each warning means and who acts on it goes to standard error.
pub fn run(_root: &Path, args: Args) -> Result<()> {
    let Args { file } = args;

    let analysis = cm_wast_analysis::analyze(&fs::read_to_string(file)?)?;
    println!("{}", encode(&analysis)?);
    for warning in &analysis.warnings {
        eprintln!("{}", explain(warning));
    }

    Ok(())
}
