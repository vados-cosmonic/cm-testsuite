//! Acquire immutable source files using upstream or centrally maintained configuration.
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
    /// Manifest relative to --root, used only when the source manifest is absent.
    #[arg(long)]
    pub(crate) fallback_manifest: Option<String>,
    #[arg(long)]
    pub(crate) local: Option<PathBuf>,
    #[arg(long)]
    pub(crate) out: PathBuf,
}

/// Read source configuration and acquire files from immutable Git snapshots.
pub fn run(root: &Path, args: Args) -> Result<()> {
    let Args {
        source,
        reference,
        manifest,
        fallback_manifest,
        local,
        out,
    } = args;

    let acquire = |repo: &dyn cm_acquisition::Repository| match &fallback_manifest {
        Some(path) => {
            cm_acquisition::acquire_with_fallback(repo, &source, &reference, &manifest, || {
                cm_acquisition::FallbackManifest::load(root, path)
            })
        }
        None => cm_acquisition::acquire(repo, &source, &reference, &manifest),
    };
    let snapshot = if let Some(path) = local {
        acquire(&cm_acquisition::LocalGit { root: path })?
    } else {
        let api = cm_github::Client::new(std::env::var("GITHUB_TOKEN").ok())?;
        acquire(&cm_acquisition::GitHub {
            api: &api,
            source: &source,
        })?
    };
    write_record(&out, &snapshot)?;
    if let Some(path) = &snapshot.metadata.fallback_manifest {
        println!("Using centrally maintained fallback manifest {path}");
    }
    println!(
        "Acquired {} tests at {}",
        snapshot.tests.len(),
        snapshot.metadata.revision
    );

    Ok(())
}
