//! Read-only file baselines and guarded edit construction.

use crate::Edit;
use anyhow::{Result, ensure};
use cm_catalog::*;
use std::path::Path;

/// Read a managed file while rejecting symlink traversal; absence is distinct.
pub fn read(root: &Path, path: &str) -> Result<Option<Vec<u8>>> {
    match std::fs::read(checked_path(root, path)?) {
        Ok(b) => Ok(Some(b)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e.into()),
    }
}

/// Hash the exact bytes currently present at a managed path.
pub fn hash_at(root: &Path, path: &str) -> Result<Option<String>> {
    Ok(read(root, path)?.map(|b| sha256(&b)))
}

/// Plan an edit only when the current contents match their ownership baseline.
pub(crate) fn edit(
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

/// Compare serialized metadata with its current bytes before proposing a write.
pub(crate) fn metadata_edit(root: &Path, path: &str, after: String) -> Result<Option<Edit>> {
    let after = match read(root, path)? {
        Some(bytes) => records::preserve(std::str::from_utf8(&bytes)?, &after)?,
        None => after,
    };
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
