//! Deterministic, serializable import and receiving plans. This crate never writes files.
use anyhow::{Context, Result, ensure};
use cm_acquisition::Snapshot;
use cm_catalog::*;
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};
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
    pub fn key(&self) -> Result<String> {
        digest(self)
    }
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
pub fn read(root: &Path, path: &str) -> Result<Option<Vec<u8>>> {
    match std::fs::read(checked_path(root, path)?) {
        Ok(b) => Ok(Some(b)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e.into()),
    }
}
pub fn hash_at(root: &Path, path: &str) -> Result<Option<String>> {
    Ok(read(root, path)?.map(|b| sha256(&b)))
}
fn edit(
    root: &Path,
    path: &str,
    expected: Option<&str>,
    after: Option<String>,
) -> Result<Option<Edit>> {
    let current = hash_at(root, path)?;
    ensure!(
        current.as_deref() == expected,
        "managed-file divergence or unowned collision: {path}"
    );
    if current == after.as_ref().map(|s| sha256(s.as_bytes())) {
        return Ok(None);
    }
    Ok(Some(Edit {
        path: path.into(),
        before: current,
        after,
    }))
}
fn metadata_edit(root: &Path, path: &str, after: String) -> Result<Option<Edit>> {
    let current = hash_at(root, path)?;
    if current == Some(sha256(after.as_bytes())) {
        return Ok(None);
    }
    Ok(Some(Edit {
        path: path.into(),
        before: current,
        after: Some(after),
    }))
}
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
    next.validate()?;
    if let Some(e) = metadata_edit(root, CATALOG_PATH, json(&next)?)? {
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
    let old = read(root, RECEIPT_PATH)?
        .map(|b| serde_json::from_slice::<Receipt>(&b))
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
        for spec in &manifest.test {
            ensure!(
                resolve_path(manifest_path, &spec.path)? != destination,
                "delivery would overwrite an authoritative test"
            );
        }
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
    if next.tests != old.tests {
        next.catalog_sha256 = digest(catalog)?;
        if let Some(e) = metadata_edit(root, RECEIPT_PATH, json(&next)?)? {
            edits.insert(RECEIPT_PATH.into(), e);
        }
    }
    next.validate()?;
    plan.edits = edits.into_values().collect();
    plan.validate()?;
    Ok(plan)
}
