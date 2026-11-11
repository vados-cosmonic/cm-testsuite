//! Review-maintained publisher identities used during report admission.

use anyhow::{Result, ensure};
use cm_catalog::{SCHEMA, records};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeSet, fs, path::Path};

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Publishers {
    pub schema: u32,
    pub publishers: BTreeSet<String>,
}

impl Publishers {
    /// Load the approved publisher set from a versioned TOML registry.
    pub fn load(path: &Path) -> Result<BTreeSet<String>> {
        let registry: Self = records::decode(&fs::read(path)?)?;
        ensure!(
            registry.schema == SCHEMA,
            "unsupported publisher registry schema"
        );
        ensure!(
            registry.publishers.iter().all(|id| !id.trim().is_empty()),
            "publisher identifiers must not be empty"
        );
        Ok(registry.publishers)
    }
}
