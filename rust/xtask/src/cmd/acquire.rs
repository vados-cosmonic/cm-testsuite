//! Read an opted-in manifest and its files from immutable Git snapshots.
use crate::io::write_record;
use anyhow::Result;
use std::path::{Path, PathBuf};

#[derive(clap::Args)]
pub struct Args {
    #[arg(long)]
    pub(crate) source: String,
    #[arg(long, default_value = "main")]
    pub(crate) reference: String,
    #[arg(long, default_value = ".cm-testsuite.toml")]
    pub(crate) manifest: String,
    #[arg(long)]
    pub(crate) local: Option<PathBuf>,
    #[arg(long)]
    pub(crate) out: PathBuf,
}

/// Read an opted-in manifest and its files from immutable Git snapshots.
pub fn run(_root: &Path, args: Args) -> Result<()> {
    let Args {
        source,
        reference,
        manifest,
        local,
        out,
    } = args;

    let snapshot = if let Some(path) = local {
        cm_acquisition::acquire(
            &cm_acquisition::LocalGit { root: path },
            &source,
            &reference,
            &manifest,
        )?
    } else {
        let api = cm_github::Client::new(std::env::var("GITHUB_TOKEN").ok())?;
        cm_acquisition::acquire(
            &cm_acquisition::GitHub {
                api: &api,
                source: &source,
            },
            &source,
            &reference,
            &manifest,
        )?
    };
    write_record(&out, &snapshot)?;
    println!(
        "Acquired {} tests at {}",
        snapshot.tests.len(),
        snapshot.metadata.revision
    );

    Ok(())
}
