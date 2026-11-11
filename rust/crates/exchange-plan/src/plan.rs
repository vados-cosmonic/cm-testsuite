//! Serializable proposals, validation, and review descriptions.

use anyhow::{Result, ensure};
use cm_catalog::*;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Edit {
    pub path: String,
    pub before: Option<String>,
    pub after: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Guard {
    pub path: String,
    pub sha256: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Kind {
    Import,
    Receive,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Plan {
    pub schema: u32,
    pub kind: Kind,
    pub subject: String,
    pub edits: Vec<Edit>,
    pub guards: Vec<Guard>,
    pub warnings: Vec<String>,
    pub exclusions: BTreeSet<String>,
    pub provenance: BTreeMap<String, Test>,
}

impl Plan {
    /// Reject unsupported schemas, unsafe edits, and inconsistent provenance.
    pub fn validate(&self) -> Result<()> {
        ensure!(self.schema == SCHEMA, "unsupported plan schema");
        ensure!(!self.subject.is_empty(), "empty plan subject");
        let mut paths = BTreeSet::new();
        for e in &self.edits {
            validate_path(&e.path)?;
            ensure!(paths.insert(&e.path), "duplicate edit path");
            ensure!(e.after.is_some() || e.before.is_some(), "empty deletion");
            if let Some(s) = &e.before {
                validate_sha(s)?;
            }
            match self.kind {
                Kind::Import => ensure!(
                    e.path == CATALOG_PATH || e.path.starts_with("corpus/wast/"),
                    "import path outside corpus/catalog"
                ),
                Kind::Receive => ensure!(
                    e.path == RECEIPT_PATH || e.path.ends_with(".wast"),
                    "unexpected receiving path"
                ),
            };
        }
        for g in &self.guards {
            validate_path(&g.path)?;
            if let Some(s) = &g.sha256 {
                validate_sha(s)?;
            }
        }
        for (id, test) in &self.provenance {
            validate_id(id)?;
            ensure!(*id == test.id(), "plan provenance identity mismatch");
            validate_revision(&test.revision)?;
            validate_sha(&test.sha256)?;
            validate_path(&test.source_path)?;
        }
        for id in &self.exclusions {
            validate_id(id)?;
        }
        Ok(())
    }

    /// Compute the canonical proposal identity used for decline suppression.
    pub fn key(&self) -> Result<String> {
        digest(self)
    }

    /// Describe edits, provenance, and exclusion instructions for review.
    pub fn body(&self) -> Result<String> {
        let mut body = format!(
            "cm-testsuite {:?} proposal: {}\n\nPlan SHA-256: `{}`\n\n",
            self.kind,
            self.subject,
            self.key()?
        );
        for e in &self.edits {
            body.push_str(&format!(
                "- {} `{}`\n",
                if e.after.is_none() {
                    "Remove"
                } else if e.before.is_none() {
                    "Add"
                } else {
                    "Update"
                },
                e.path
            ));
        }
        if !self.provenance.is_empty() {
            body.push_str("\nSource provenance:\n\n");
            for (id, test) in &self.provenance {
                body.push_str(&format!(
                    "- `{id}` at `{}`; source `{}`; SHA-256 `{}`. {}\n",
                    test.revision, test.source_path, test.sha256, test.summary
                ));
            }
        }
        if !self.exclusions.is_empty() {
            body.push_str("\nTo permanently decline these tests, merge these identities into the existing `[receive].exclude` array in your manifest (preserve existing entries):\n\n```toml\nexclude = [\n");
            for id in &self.exclusions {
                body.push_str(&format!("  {id:?},\n"));
            }
            body.push_str("]\n```\n");
        }
        for warning in &self.warnings {
            body.push_str(&format!("\n- {warning}\n"));
        }
        Ok(body)
    }
}
