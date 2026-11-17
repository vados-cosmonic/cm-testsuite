//! Centrally maintained configuration, kept separate from upstream test provenance.

use anyhow::{Context, Result, ensure};
use cm_catalog::checked_path;
use std::{fs, path::Path};

/// Configuration supplied by the catalog checkout instead of the source repository.
pub struct FallbackManifest {
    /// Portable path relative to the catalog checkout root.
    pub path: String,
    pub bytes: Vec<u8>,
}

impl FallbackManifest {
    /// Load a regular file without allowing paths outside the checkout or symlinks.
    pub fn load(root: &Path, path: &str) -> Result<Self> {
        let resolved = checked_path(root, path)?;
        let metadata =
            fs::metadata(&resolved).with_context(|| format!("fallback manifest {path}"))?;
        ensure!(
            metadata.is_file(),
            "fallback manifest is not a regular file: {path}"
        );
        ensure!(
            metadata.len() <= 8 * 1024 * 1024,
            "fallback manifest exceeds 8 MiB limit"
        );
        let bytes = fs::read(&resolved).with_context(|| format!("fallback manifest {path}"))?;
        ensure!(
            bytes.len() <= 8 * 1024 * 1024,
            "fallback manifest exceeds 8 MiB limit"
        );
        Ok(Self {
            path: path.into(),
            bytes,
        })
    }
}
