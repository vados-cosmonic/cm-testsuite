//! Discover WAST files using complete, non-recursive Git tree responses.

use crate::{Api, repo_path, required, tree_sha};
use anyhow::{Context, Result, ensure};
use cm_catalog::validate_path;
use serde_json::Value;

fn entries(api: &dyn Api, base: &str, tree: &str) -> Result<Vec<Value>> {
    let data = required(api, "GET", &format!("{base}/git/trees/{tree}"), None)?;
    ensure!(
        data["truncated"] != true,
        "GitHub tree response truncated; refusing incomplete acquisition"
    );
    Ok(data["tree"]
        .as_array()
        .context("missing tree entries")?
        .clone())
}

/// List committed WAST paths below a directory, using an empty string for root.
/// Missing directories and incomplete listings fail rather than retiring tests.
pub fn list_wast(
    api: &dyn Api,
    source: &str,
    revision: &str,
    directory: &str,
    recursive: bool,
) -> Result<Vec<String>> {
    if !directory.is_empty() {
        validate_path(directory)?;
    }
    let base = repo_path(source)?;
    let mut tree = tree_sha(api, source, revision)?;
    for part in directory.split('/').filter(|p| !p.is_empty()) {
        let entries = entries(api, &base, &tree)?;
        let entry = entries
            .iter()
            .find(|e| e["path"] == part)
            .with_context(|| format!("missing tracked directory: {directory}"))?;
        ensure!(
            entry["type"] == "tree" && entry["mode"] == "040000",
            "gather path is not a tracked directory: {directory}"
        );
        tree = entry["sha"].as_str().context("missing entry SHA")?.into();
    }

    let mut pending = vec![(directory.to_owned(), tree)];
    let mut paths = Vec::new();
    while let Some((parent, tree)) = pending.pop() {
        for entry in entries(api, &base, &tree)? {
            let name = entry["path"].as_str().context("missing entry path")?;
            ensure!(!name.contains('/'), "expected a single tree path segment");
            let path = if parent.is_empty() {
                name.into()
            } else {
                format!("{parent}/{name}")
            };
            if entry["type"] == "tree" && entry["mode"] == "040000" {
                if recursive {
                    validate_path(&path)?;
                    pending.push((
                        path,
                        entry["sha"].as_str().context("missing entry SHA")?.into(),
                    ));
                }
            } else if name.ends_with(".wast") {
                validate_path(&path)?;
                ensure!(
                    entry["type"] == "blob"
                        && (entry["mode"] == "100644" || entry["mode"] == "100755"),
                    "symlink or non-regular tracked WAST file: {path}"
                );
                paths.push(path);
            }
        }
    }
    paths.sort();
    Ok(paths)
}
