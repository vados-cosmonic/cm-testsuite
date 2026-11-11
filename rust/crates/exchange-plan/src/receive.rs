//! Select and reconcile deliveries using receiving policy and prior receipts.

use crate::files::{edit, metadata_edit};
use crate::{Guard, Kind, Plan, hash_at, read};
use anyhow::{Context, Result, ensure};
use cm_catalog::*;
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Delivered {
    pub path: String,
    pub sha256: String,
    pub revision: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub route: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Receipt {
    pub schema: u32,
    pub catalog_sha256: String,
    pub tests: BTreeMap<String, Delivered>,
}

impl Receipt {
    /// Verify receipt identities, managed paths, digests, and route names.
    pub fn validate(&self) -> Result<()> {
        ensure!(self.schema == SCHEMA, "unsupported receipt schema");
        validate_sha(&self.catalog_sha256)?;
        let mut paths = BTreeSet::new();
        for (id, t) in &self.tests {
            validate_id(id)?;
            validate_path(&t.path)?;
            ensure!(t.path.ends_with(".wast"), "receipt path must be WAST");
            validate_sha(&t.sha256)?;
            validate_revision(&t.revision)?;
            if let Some(route) = &t.route {
                validate_name(route)?;
            }
            ensure!(paths.insert(&t.path), "duplicate receipt path");
        }
        Ok(())
    }
}

/// Preserve excluded files, including receipts created before route names were stored.
fn retirement_excluded(
    id: &str,
    prior: &Delivered,
    manifest: &Manifest,
    manifest_path: &str,
) -> Result<bool> {
    let routes = &manifest.receive.route;
    if let Some(name) = &prior.route
        && let Some(route) = routes.iter().find(|r| &r.name == name)
    {
        return Ok(route.exclude.contains(id));
    }
    // Older receipts did not record route names. Infer the route from the
    // delivered path, retaining the file conservatively if a move makes that
    // ambiguous and any route explicitly excludes this identity.
    let mut matched = false;
    let mut excluded = false;
    for route in routes {
        let destination = format!(
            "{}/{}.wast",
            resolve_path(manifest_path, &route.destination)?,
            id.strip_prefix("github:")
                .context("invalid receipt identity")?
        );
        if prior.path == destination {
            matched = true;
            excluded |= route.exclude.contains(id);
        }
    }
    Ok(if matched {
        excluded
    } else {
        routes.iter().any(|r| r.exclude.contains(id))
    })
}

/// Plan opted-in delivery against recipient policy and exact managed-file baselines.
/// Reject ambiguous routes, source overwrites, collisions, and local divergence.
pub fn receive(
    root: &Path,
    recipient: &str,
    manifest_path: &str,
    catalog: &Catalog,
    files: &BTreeMap<String, String>,
) -> Result<Plan> {
    validate_source(recipient)?;
    catalog.validate()?;
    let raw = read(root, manifest_path)?.context("recipient manifest is required")?;
    let manifest = Manifest::parse(std::str::from_utf8(&raw)?, manifest_path)?;
    ensure!(
        manifest.receive.enabled,
        "recipient has not opted into receiving"
    );
    let authoritative = manifest
        .test
        .iter()
        .map(|test| resolve_path(manifest_path, &test.path))
        .collect::<Result<BTreeSet<_>>>()?;
    let old = read(root, RECEIPT_PATH)?
        .map(|b| records::decode::<Receipt>(&b))
        .transpose()?
        .unwrap_or(Receipt {
            schema: SCHEMA,
            catalog_sha256: digest(catalog)?,
            tests: BTreeMap::new(),
        });
    old.validate()?;
    let mut next = old.clone();
    let mut plan = Plan {
        schema: SCHEMA,
        kind: Kind::Receive,
        subject: recipient.into(),
        edits: vec![],
        guards: vec![
            Guard {
                path: manifest_path.into(),
                sha256: Some(sha256(&raw)),
            },
            Guard {
                path: RECEIPT_PATH.into(),
                sha256: hash_at(root, RECEIPT_PATH)?,
            },
        ],
        warnings: vec![],
        exclusions: BTreeSet::new(),
        provenance: BTreeMap::new(),
    };
    // Select routes first, then protect both destination ownership and source files
    // before proposing additions, updates, or moves.
    let mut destinations = BTreeSet::new();
    let mut edits = BTreeMap::new();
    for (id, test) in &catalog.tests {
        if test.source == recipient || manifest.receive.exclude.contains(id) {
            continue;
        }
        let mut routes = vec![];
        for route in &manifest.receive.route {
            if route.exclude.contains(id)
                || (!route.covers_any.is_empty() && route.covers_any.is_disjoint(&test.covers))
                || !test.requires.is_subset(&route.supports)
            {
                continue;
            }
            if !test.analysis.complete && !route.allow_unknown {
                plan.warnings.push(format!(
                    "{id}: route {} needs explicit allow_unknown for incomplete feature inference",
                    route.name
                ));
                continue;
            }
            routes.push(route);
        }
        ensure!(routes.len() <= 1, "ambiguous receiving routes for {id}");
        let Some(route) = routes.first() else {
            continue;
        };
        let previous = old.tests.get(id);
        if (previous.is_none() && manifest.receive.additions == Policy::Manual)
            || (previous.is_some() && manifest.receive.updates == Policy::Manual)
        {
            continue;
        }
        let destination = format!(
            "{}/{}/{}.wast",
            resolve_path(manifest_path, &route.destination)?,
            test.source.strip_prefix("github:").unwrap(),
            test.name
        );
        validate_path(&destination)?;
        ensure!(
            destinations.insert(destination.clone()),
            "receiving destination collision"
        );
        ensure!(
            !authoritative.contains(&destination),
            "delivery would overwrite an authoritative test"
        );
        ensure!(
            !old.tests
                .iter()
                .any(|(other, t)| other != id && t.path == destination),
            "destination owned by a different test"
        );
        let content = files
            .get(&test.destination)
            .context("missing catalog WAST bytes")?
            .clone();
        ensure!(
            sha256(content.as_bytes()) == test.sha256,
            "catalog content mismatch"
        );
        let baseline = previous
            .filter(|t| t.path == destination)
            .map(|t| t.sha256.as_str());
        if let Some(e) = edit(root, &destination, baseline, Some(content))? {
            edits.insert(destination.clone(), e);
            plan.exclusions.insert(id.clone());
        }
        if let Some(prior) = previous.filter(|t| t.path != destination)
            && let Some(e) = edit(root, &prior.path, Some(&prior.sha256), None)?
        {
            ensure!(!edits.contains_key(&prior.path), "managed move collision");
            edits.insert(prior.path.clone(), e);
        }
        let delivered = Delivered {
            path: destination,
            sha256: test.sha256.clone(),
            revision: test.revision.clone(),
            route: Some(route.name.clone()),
        };
        if previous != Some(&delivered) {
            plan.provenance.insert(id.clone(), test.clone());
            plan.exclusions.insert(id.clone());
        }
        next.tests.insert(id.clone(), delivered);
        if !test.analysis.complete {
            plan.warnings.push(format!(
                "{id}: accepted incomplete feature inference via route {}",
                route.name
            ));
        }
    }
    for (id, prior) in &old.tests {
        // Absence alone is not deletion; require an explicit central retirement record.
        if catalog.retired.contains_key(id)
            && !catalog.tests.contains_key(id)
            && !manifest.receive.exclude.contains(id)
            && manifest.receive.deletions == Policy::PullRequest
            && !retirement_excluded(id, prior, &manifest, manifest_path)?
        {
            if let Some(e) = edit(root, &prior.path, Some(&prior.sha256), None)? {
                ensure!(
                    !edits.contains_key(&prior.path),
                    "deletion collides with receiving edit"
                );
                edits.insert(prior.path.clone(), e);
            }
            next.tests.remove(id);
        }
    }
    // Update the receipt only when managed deliveries change, avoiding unrelated
    // catalog churn in receiving repositories.
    if next.tests != old.tests {
        next.catalog_sha256 = digest(catalog)?;
        if let Some(e) = metadata_edit(root, RECEIPT_PATH, records::encode(&next)?)? {
            edits.insert(RECEIPT_PATH.into(), e);
        }
    }
    next.validate()?;
    plan.edits = edits.into_values().collect();
    for edit in &plan.edits {
        ensure!(
            !authoritative.contains(&edit.path),
            "delivery would modify or remove an authoritative test: {}",
            edit.path
        );
    }
    plan.validate()?;
    Ok(plan)
}
