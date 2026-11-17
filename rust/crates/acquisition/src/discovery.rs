//! Expand directory rules before acquisition, preserving explicit per-file overrides.

use crate::Repository;
use anyhow::{Context, Result, ensure};
use cm_catalog::{Manifest, TestSpec, resolve_path, validate_name};
use std::collections::BTreeSet;

pub(crate) fn expand(
    repo: &dyn Repository,
    revision: &str,
    manifest_path: &str,
    manifest: &Manifest,
) -> Result<Vec<(TestSpec, String)>> {
    let mut specs = manifest
        .test
        .iter()
        .map(|spec| Ok((spec.clone(), resolve_path(manifest_path, &spec.path)?)))
        .collect::<Result<Vec<_>>>()?;
    let explicit: BTreeSet<_> = specs.iter().map(|(_, path)| path.clone()).collect();
    let mut names: BTreeSet<_> = specs.iter().map(|(spec, _)| spec.name.clone()).collect();
    let mut discovered = BTreeSet::new();
    for gather in &manifest.gather {
        let directory = gather.directory(manifest_path)?;
        for path in repo.list_wast(revision, &directory, gather.recursive)? {
            ensure!(
                gather.selects(manifest_path, &path)?,
                "repository returned a path outside gather selection: {path}"
            );
            if explicit.contains(&path) || manifest.is_received_path(manifest_path, &path)? {
                continue;
            }
            ensure!(
                discovered.insert(path.clone()),
                "overlapping gather rules select {path}"
            );
            let name = path.strip_suffix(".wast").context("expected WAST path")?;
            validate_name(name).with_context(|| format!("cannot name discovered test {path}; add an explicit [[test]] with a portable name"))?;
            ensure!(names.insert(name.into()), "duplicate test name {name}");
            specs.push((
                TestSpec {
                    name: name.into(),
                    path: format!("file:///{path}"),
                    summary: String::new(),
                    requires: gather.requires.clone(),
                    covers: gather.covers.clone(),
                    sha256: None,
                    revision: None,
                },
                path,
            ));
        }
    }
    Ok(specs)
}
