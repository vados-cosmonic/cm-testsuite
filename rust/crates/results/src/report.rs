//! Publisher-supplied report schema and admission rules.

use anyhow::{Result, ensure};
use cm_catalog::*;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Report {
    pub schema: u32,
    pub publisher: String,
    pub engine: String,
    pub engine_version: String,
    pub run_id: String,
    pub catalog_sha256: String,
    pub reported_at: String,
    pub platform: String,
    pub configuration: String,
    pub results: Vec<Observation>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Observation {
    pub id: String,
    pub sha256: String,
    pub status: Status,
    #[serde(default)]
    pub diagnostic: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Status {
    Pass,
    Fail,
    Skip,
    Unsupported,
    Timeout,
    InfrastructureError,
}

impl Status {
    /// Return the stable status spelling shared by reports and the results page.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pass => "pass",
            Self::Fail => "fail",
            Self::Skip => "skip",
            Self::Unsupported => "unsupported",
            Self::Timeout => "timeout",
            Self::InfrastructureError => "infrastructure-error",
        }
    }
}

impl Report {
    /// Check publisher approval, catalog identity, observation uniqueness, and file digests.
    pub fn validate(&self, catalog: &Catalog, publishers: &BTreeSet<String>) -> Result<()> {
        catalog.validate()?;
        ensure!(self.schema == SCHEMA, "unsupported report schema");
        ensure!(
            publishers.contains(&self.publisher),
            "publisher is not approved"
        );
        ensure!(
            self.catalog_sha256 == digest(catalog)?,
            "report refers to a different catalog snapshot"
        );
        for field in [
            &self.engine,
            &self.engine_version,
            &self.run_id,
            &self.reported_at,
            &self.platform,
            &self.configuration,
        ] {
            ensure!(
                !field.trim().is_empty(),
                "report metadata must not be empty"
            );
        }

        // A publisher may omit observations, but may not duplicate or invent tests.
        let mut seen = BTreeSet::new();
        for result in &self.results {
            ensure!(seen.insert(&result.id), "duplicate result identity");
            let Some(test) = catalog.tests.get(&result.id) else {
                anyhow::bail!("unknown test identity {}", result.id)
            };
            ensure!(
                result.sha256 == test.sha256,
                "result content digest mismatch"
            );
        }
        Ok(())
    }
}
