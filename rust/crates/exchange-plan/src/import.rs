//! Reconcile complete source snapshots with the curated catalog.

use crate::files::{edit, metadata_edit};
use crate::{Guard, Kind, Plan, hash_at};
use anyhow::{Context, Result, ensure};
use cm_acquisition::Snapshot;
use cm_catalog::*;
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};

/// Plan a complete import without modifying files; missing source entries become retirements.
pub fn import(root: &Path, snapshots: &[Snapshot]) -> Result<Plan> {
    let old = Catalog::load(root)?;
    let mut next = old.clone();
    let mut plan = Plan {
        schema: SCHEMA,
        kind: Kind::Import,
        subject: snapshots
            .iter()
            .map(|x| x.source.as_str())
            .collect::<Vec<_>>()
            .join(", "),
        edits: vec![],
        guards: vec![Guard {
            path: CATALOG_PATH.into(),
            sha256: hash_at(root, CATALOG_PATH)?,
        }],
        warnings: vec![],
        exclusions: BTreeSet::new(),
        provenance: BTreeMap::new(),
    };
    ensure!(!snapshots.is_empty(), "no snapshots supplied");

    // A complete snapshot retires only identities owned by that source.
    let mut sources = BTreeSet::new();
    for snapshot in snapshots {
        snapshot.validate()?;
        ensure!(
            sources.insert(&snapshot.source),
            "duplicate source snapshot"
        );
        for (id, test) in old
            .tests
            .iter()
            .filter(|(_, t)| t.source == snapshot.source)
        {
            if !snapshot.tests.contains_key(id) {
                if let Some(e) = edit(root, &test.destination, Some(&test.sha256), None)? {
                    plan.edits.push(e);
                }
                plan.provenance.insert(id.clone(), test.clone());
                next.tests.remove(id);
                next.retired.insert(id.clone(), test.sha256.clone());
            }
        }

        // Reconcile exact file bytes before advancing their catalog provenance.
        for (id, test) in &snapshot.tests {
            let before = old.tests.get(id).map(|x| x.sha256.as_str());
            let content = snapshot
                .files
                .get(&test.destination)
                .context("missing snapshot file")?
                .clone();
            if let Some(e) = edit(root, &test.destination, before, Some(content))? {
                plan.edits.push(e);
            }
            for warning in &test.analysis.warnings {
                plan.warnings.push(format!("{id}: {warning}"));
            }
            next.retired.remove(id);
            if old.tests.get(id) != Some(test) {
                plan.provenance.insert(id.clone(), test.clone());
            }
            next.tests.insert(id.clone(), test.clone());
        }
        next.sources
            .insert(snapshot.source.clone(), snapshot.metadata.clone());
    }

    // Keep metadata comments and deterministic ordering in the final review plan.
    next.validate()?;
    if let Some(e) = metadata_edit(root, CATALOG_PATH, records::encode(&next)?)? {
        plan.edits.push(e);
    }
    plan.edits.sort_by(|a, b| a.path.cmp(&b.path));
    plan.warnings.sort();
    let mut names = snapshots
        .iter()
        .map(|s| s.source.as_str())
        .collect::<Vec<_>>();
    names.sort();
    plan.subject = names.join(", ");
    plan.validate()?;
    Ok(plan)
}
