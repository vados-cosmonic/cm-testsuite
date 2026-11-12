//! Read-only file baselines and guarded edit construction.

use crate::Edit;
use anyhow::{Result, ensure};
use cm_catalog::*;
use std::path::Path;

/// Read access to the files of one repository state.
///
/// Planning inspects a repository only through this boundary, so the same
/// logic serves a local checkout and a remote revision.
pub trait Tree {
    /// Read a regular file; absence is distinct from an error.
    fn read(&self, path: &str) -> Result<Option<Vec<u8>>>;
}

/// The working files of a local checkout, rejecting symlink traversal.
pub struct Checkout<'a>(pub &'a Path);

impl Tree for Checkout<'_> {
    fn read(&self, path: &str) -> Result<Option<Vec<u8>>> {
        match std::fs::read(checked_path(self.0, path)?) {
            Ok(b) => Ok(Some(b)),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(e.into()),
        }
    }
}

/// Read a managed file while rejecting symlink traversal; absence is distinct.
pub fn read(root: &Path, path: &str) -> Result<Option<Vec<u8>>> {
    Checkout(root).read(path)
}

/// Hash the exact bytes currently present at a managed path.
pub fn hash_at(root: &Path, path: &str) -> Result<Option<String>> {
    hash_in(&Checkout(root), path)
}

/// Hash the exact bytes a tree holds at a managed path.
pub(crate) fn hash_in(tree: &dyn Tree, path: &str) -> Result<Option<String>> {
    Ok(tree.read(path)?.map(|b| sha256(&b)))
}

/// Plan an edit only when the current contents match their ownership baseline.
pub(crate) fn edit(
    tree: &dyn Tree,
    path: &str,
    expected: Option<&str>,
    after: Option<String>,
) -> Result<Option<Edit>> {
    let current = hash_in(tree, path)?;
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

/// Compare serialized metadata with its current bytes before proposing a write.
pub(crate) fn metadata_edit(tree: &dyn Tree, path: &str, after: String) -> Result<Option<Edit>> {
    moved_edit(tree, path, None, after)
}

/// Plan a metadata write like [`metadata_edit`]. A record that does not exist
/// yet takes its comments from `origin`, the text its fields are moving out of.
pub(crate) fn moved_edit(
    tree: &dyn Tree,
    path: &str,
    origin: Option<&[u8]>,
    after: String,
) -> Result<Option<Edit>> {
    let existing = tree.read(path)?;
    let after = match existing.as_deref().or(origin) {
        Some(bytes) => records::preserve(std::str::from_utf8(bytes)?, &after)?,
        None => after,
    };
    let current = existing.map(|bytes| sha256(&bytes));
    if current == Some(sha256(after.as_bytes())) {
        return Ok(None);
    }
    Ok(Some(Edit {
        path: path.into(),
        before: current,
        after: Some(after),
    }))
}
