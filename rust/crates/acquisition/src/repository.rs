//! Read immutable repository objects without running producer code.

use anyhow::{Context, Result, ensure};
use cm_catalog::{validate_path, validate_revision};
use cm_github::Api;
use std::{path::PathBuf, process::Command};

/// Resolve commits and read regular tracked files from a repository.
pub trait Repository {
    /// Resolve a reference to a full immutable commit identifier.
    fn resolve(&self, reference: &str) -> Result<String>;

    /// Read exact bytes from a regular tracked file at the supplied commit.
    fn read(&self, revision: &str, path: &str) -> Result<Vec<u8>>;
}

pub struct GitHub<'a> {
    pub api: &'a dyn Api,
    pub source: &'a str,
}

impl Repository for GitHub<'_> {
    fn resolve(&self, r: &str) -> Result<String> {
        cm_github::resolve(self.api, self.source, r)
    }

    fn read(&self, r: &str, p: &str) -> Result<Vec<u8>> {
        cm_github::read_file(self.api, self.source, r, p)?
            .with_context(|| format!("missing tracked source file {p} at {r}"))
    }
}

/// Local committed Git snapshots are useful for offline development and private sources.
/// Git is used only as the existing repository object reader, not a sync backend.
pub struct LocalGit {
    pub root: PathBuf,
}

impl LocalGit {
    /// Run a read-only Git command with ambient configuration disabled.
    fn git(&self, args: &[&str]) -> Result<Vec<u8>> {
        let o = Command::new("git")
            .arg("--no-pager")
            .arg("-C")
            .arg(&self.root)
            .args(args)
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env(
                "GIT_CONFIG_GLOBAL",
                if cfg!(windows) { "NUL" } else { "/dev/null" },
            )
            .output()?;
        ensure!(
            o.status.success(),
            "Git snapshot read failed: {}",
            String::from_utf8_lossy(&o.stderr)
        );
        Ok(o.stdout)
    }
}

impl Repository for LocalGit {
    fn resolve(&self, r: &str) -> Result<String> {
        ensure!(!r.is_empty(), "empty Git ref");
        let v = String::from_utf8(self.git(&[
            "rev-parse",
            "--verify",
            "--end-of-options",
            &format!("{r}^{{commit}}"),
        ])?)?
        .trim()
        .to_owned();
        validate_revision(&v)?;
        Ok(v)
    }

    fn read(&self, r: &str, p: &str) -> Result<Vec<u8>> {
        validate_revision(r)?;
        validate_path(p)?;
        let entry = self.git(&["ls-tree", "-z", r, "--", p])?;
        ensure!(
            entry.starts_with(b"100644 blob ") || entry.starts_with(b"100755 blob "),
            "missing, symlink, or non-regular tracked file: {p}"
        );
        let b = self.git(&["show", &format!("{r}:{p}")])?;
        ensure!(b.len() <= 8 * 1024 * 1024, "file exceeds 8 MiB limit");
        Ok(b)
    }
}
