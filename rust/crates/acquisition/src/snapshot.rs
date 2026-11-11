//! Validated source snapshots and catalog file loading.

use anyhow::{Context, Result, ensure};
use cm_catalog::*;
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, path::Path};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Snapshot {
    pub source: String,
    pub metadata: SourceSnapshot,
    pub tests: BTreeMap<String, Test>,
    pub files: BTreeMap<String, String>,
}

impl Snapshot {
    /// Check source provenance, exact file membership, digests, and analysis.
    pub fn validate(&self) -> Result<()> {
        let c = Catalog {
            schema: SCHEMA,
            sources: BTreeMap::from([(self.source.clone(), self.metadata.clone())]),
            tests: self.tests.clone(),
            retired: BTreeMap::new(),
        };
        c.validate()?;
        ensure!(
            self.files.len() == self.tests.len(),
            "snapshot file count mismatch"
        );
        for t in self.tests.values() {
            let text = self
                .files
                .get(&t.destination)
                .context("missing snapshot bytes")?;
            ensure!(
                sha256(text.as_bytes()) == t.sha256,
                "snapshot digest mismatch"
            );
            ensure!(
                cm_wast_analysis::analyze(text)? == t.analysis,
                "snapshot analysis does not match its bytes/analyzer"
            );
        }
        Ok(())
    }
}

/// Load every active catalog file and verify its digest and structural analysis.
/// Reject missing, modified, non-UTF-8, or symlinked files before returning any data.
pub fn load_catalog_files(root: &Path, catalog: &Catalog) -> Result<BTreeMap<String, String>> {
    catalog.validate()?;
    let mut files = BTreeMap::new();
    for t in catalog.tests.values() {
        let p = checked_path(root, &t.destination)?;
        let b = std::fs::read(&p)?;
        ensure!(sha256(&b) == t.sha256, "catalog drift: {}", t.destination);
        let text = String::from_utf8(b)?;
        ensure!(
            cm_wast_analysis::analyze(&text)? == t.analysis,
            "catalog analysis does not match its bytes/analyzer"
        );
        files.insert(t.destination.clone(), text);
    }
    Ok(files)
}

/// Load a saved snapshot, validating provenance, file digests, and analysis.
/// Malformed or inconsistent records are rejected before planning can use them.
pub fn load_snapshot(path: &Path) -> Result<Snapshot> {
    let s: Snapshot = records::decode(&std::fs::read(path)?)?;
    s.validate()?;
    Ok(s)
}
