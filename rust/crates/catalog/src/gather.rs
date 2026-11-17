//! Directory selection shared by acquisition and protection of producer files.

use crate::{Manifest, resolve_path, validate_path};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GatherSpec {
    pub path: String,
    #[serde(default)]
    pub recursive: bool,
    #[serde(default)]
    pub requires: BTreeSet<String>,
    #[serde(default)]
    pub covers: BTreeSet<String>,
}

impl GatherSpec {
    /// Resolve a directory, using an empty string for the repository root.
    pub fn directory(&self, manifest_path: &str) -> Result<String> {
        validate_path(manifest_path)?;
        match self.path.as_str() {
            "file:///" => Ok(String::new()),
            "file://" => Ok(manifest_path.rsplit_once('/').map_or("", |(p, _)| p).into()),
            _ => resolve_path(manifest_path, &self.path),
        }
    }

    /// Whether a repository-relative file is selected by this directory rule.
    pub fn selects(&self, manifest_path: &str, path: &str) -> Result<bool> {
        let directory = self.directory(manifest_path)?;
        let relative = if directory.is_empty() {
            Some(path)
        } else {
            path.strip_prefix(&(directory + "/"))
        };
        Ok(relative.is_some_and(|p| p.ends_with(".wast") && (self.recursive || !p.contains('/'))))
    }
}

impl Manifest {
    /// Exclude received copies from automatic discovery, including broad root scans.
    pub fn is_received_path(&self, manifest_path: &str, path: &str) -> Result<bool> {
        if path.starts_with("corpus/wast/") {
            return Ok(true);
        }
        for route in &self.receive.route {
            let dest = resolve_path(manifest_path, &route.destination)?;
            if path == dest || path.starts_with(&(dest + "/")) {
                return Ok(true);
            }
        }
        Ok(false)
    }

    /// Protect explicitly declared and automatically selected producer files.
    pub fn is_authoritative_path(&self, manifest_path: &str, path: &str) -> Result<bool> {
        for test in &self.test {
            if resolve_path(manifest_path, &test.path)? == path {
                return Ok(true);
            }
        }
        if !self.is_received_path(manifest_path, path)? {
            for gather in &self.gather {
                if gather.selects(manifest_path, path)? {
                    return Ok(true);
                }
            }
        }
        Ok(false)
    }
}
