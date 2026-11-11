//! Immutable repository tree traversal and path validation.

use crate::{Api, required};
use anyhow::{Context, Result, bail, ensure};
use base64::{Engine, engine::general_purpose::STANDARD};
use cm_catalog::{validate_path, validate_revision, validate_source};
use percent_encoding::{NON_ALPHANUMERIC, utf8_percent_encode};

/// Encode one path segment without allowing caller input to change the API route.
pub fn encode(s: &str) -> String {
    utf8_percent_encode(s, NON_ALPHANUMERIC).to_string()
}

/// Validate a source identifier and construct its repository API prefix.
pub fn repo_path(source: &str) -> Result<String> {
    validate_source(source)?;
    Ok(format!(
        "/repos/{}",
        source.strip_prefix("github:").unwrap()
    ))
}

/// Resolve a branch, tag, or revision to a full immutable commit identifier.
pub fn resolve(api: &dyn Api, source: &str, reference: &str) -> Result<String> {
    ensure!(!reference.is_empty(), "empty reference");
    let v = required(
        api,
        "GET",
        &format!("{}/commits/{}", repo_path(source)?, encode(reference)),
        None,
    )?;
    let sha = v["sha"].as_str().context("missing commit SHA")?.to_owned();
    validate_revision(&sha)?;
    Ok(sha)
}

/// Read the root tree identifier belonging to an immutable commit.
pub fn tree_sha(api: &dyn Api, source: &str, revision: &str) -> Result<String> {
    validate_revision(revision)?;
    let v = required(
        api,
        "GET",
        &format!("{}/git/commits/{revision}", repo_path(source)?),
        None,
    )?;
    Ok(v["tree"]["sha"]
        .as_str()
        .context("missing commit tree")?
        .to_owned())
}

/// Traverse trees explicitly: the contents API can follow symlinks, which we reject.
pub fn read_file(
    api: &dyn Api,
    source: &str,
    revision: &str,
    path: &str,
) -> Result<Option<Vec<u8>>> {
    validate_path(path)?;
    validate_revision(revision)?;
    let base = repo_path(source)?;
    let mut tree = tree_sha(api, source, revision)?;
    let parts: Vec<_> = path.split('/').collect();
    for (i, part) in parts.iter().enumerate() {
        let data = required(api, "GET", &format!("{base}/git/trees/{tree}"), None)?;
        ensure!(
            data["truncated"] != true,
            "GitHub tree response truncated; refusing incomplete acquisition"
        );
        let entries = data["tree"].as_array().context("missing tree entries")?;
        let Some(entry) = entries.iter().find(|x| x["path"].as_str() == Some(part)) else {
            return Ok(None);
        };
        let sha = entry["sha"].as_str().context("missing entry SHA")?;
        if i + 1 < parts.len() {
            ensure!(
                entry["type"] == "tree" && entry["mode"] == "040000",
                "non-directory in source path: {path}"
            );
            tree = sha.to_owned();
            continue;
        }
        ensure!(
            entry["type"] == "blob" && (entry["mode"] == "100644" || entry["mode"] == "100755"),
            "source is not a regular file: {path}"
        );
        let blob = required(api, "GET", &format!("{base}/git/blobs/{sha}"), None)?;
        ensure!(
            blob["encoding"] == "base64",
            "unsupported GitHub blob encoding"
        );
        let raw = blob["content"]
            .as_str()
            .context("missing blob content")?
            .chars()
            .filter(|x| !x.is_whitespace())
            .collect::<String>();
        let bytes = STANDARD.decode(raw)?;
        ensure!(bytes.len() <= 8 * 1024 * 1024, "file exceeds 8 MiB limit");
        return Ok(Some(bytes));
    }
    bail!("empty source path")
}
