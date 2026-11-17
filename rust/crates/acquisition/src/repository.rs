//! Read immutable repository objects without running producer code.

use anyhow::{Context, Result, bail, ensure};
use cm_catalog::{validate_path, validate_revision};
use cm_github::Api;
use std::{path::PathBuf, process::Command};

/// Resolve commits and read regular tracked files from a repository.
pub trait Repository {
    /// Resolve a reference to a full immutable commit identifier.
    fn resolve(&self, reference: &str) -> Result<String>;

    /// Read exact bytes from a regular tracked file at the supplied commit.
    fn read(&self, revision: &str, path: &str) -> Result<Vec<u8>>;

    /// Return None only for a path proven absent from an immutable tree.
    /// Implementations without absence detection conservatively propagate read errors.
    fn read_optional(&self, revision: &str, path: &str) -> Result<Option<Vec<u8>>> {
        self.read(revision, path).map(Some)
    }

    /// List committed WAST paths in a directory (empty for root), without following links.
    fn list_wast(
        &self,
        _revision: &str,
        _directory: &str,
        _recursive: bool,
    ) -> Result<Vec<String>> {
        bail!("repository does not support directory discovery")
    }
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

    fn read_optional(&self, r: &str, p: &str) -> Result<Option<Vec<u8>>> {
        cm_github::read_file(self.api, self.source, r, p)
    }

    fn list_wast(&self, r: &str, directory: &str, recursive: bool) -> Result<Vec<String>> {
        cm_github::list_wast(self.api, self.source, r, directory, recursive)
    }
}

/// Local committed Git snapshots are useful for offline development and private sources.
/// Git is used only as the existing repository object reader, not a sync backend.
pub struct LocalGit {
    pub root: PathBuf,
}

impl LocalGit {
    /// Run a read-only Git command with ambient configuration disabled.
    pub fn git(&self, args: &[&str]) -> Result<Vec<u8>> {
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
            "Git snapshot read failed (git {}): {}",
            args.join(" "),
            String::from_utf8_lossy(&o.stderr)
        );
        Ok(o.stdout)
    }
}

impl Repository for LocalGit {
    fn list_wast(&self, r: &str, directory: &str, recursive: bool) -> Result<Vec<String>> {
        validate_revision(r)?;
        if !directory.is_empty() {
            validate_path(directory)?;
        }
        let tree = format!("{r}:{directory}");
        ensure!(
            self.git(&["cat-file", "-t", &tree])? == b"tree\n",
            "gather path is not a tracked directory: {directory}"
        );
        let mut args = vec!["ls-tree", "-z"];
        if recursive {
            args.push("-r");
        }
        args.push(&tree);
        let entries = self.git(&args)?;
        let mut paths = Vec::new();
        for entry in entries.split(|b| *b == 0).filter(|e| !e.is_empty()) {
            let entry = std::str::from_utf8(entry)?;
            let (kind, relative) = entry.split_once('\t').context("invalid Git tree entry")?;
            if !relative.ends_with(".wast") {
                continue;
            }
            // Directories named *.wast are containers, not candidate test files.
            if kind.starts_with("040000 tree ") {
                continue;
            }
            ensure!(
                kind.starts_with("100644 blob ") || kind.starts_with("100755 blob "),
                "symlink or non-regular tracked WAST file: {relative}"
            );
            let path = if directory.is_empty() {
                relative.to_owned()
            } else {
                format!("{directory}/{relative}")
            };
            validate_path(&path)?;
            paths.push(path);
        }
        paths.sort();
        Ok(paths)
    }

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
        self.read_optional(r, p)?
            .with_context(|| format!("missing, symlink, or non-regular tracked file: {p}"))
    }

    fn read_optional(&self, r: &str, p: &str) -> Result<Option<Vec<u8>>> {
        validate_revision(r)?;
        validate_path(p)?;
        // Inspect every ancestor: a symlink or submodule must not masquerade as
        // an absent nested manifest and enable the fallback.
        let mut path = String::new();
        let parts: Vec<_> = p.split('/').collect();
        for (i, part) in parts.iter().enumerate() {
            if !path.is_empty() {
                path.push('/');
            }
            path.push_str(part);
            let entry = self.git(&["ls-tree", "-z", r, "--", &path])?;
            if entry.is_empty() {
                return Ok(None);
            }
            if i + 1 < parts.len() {
                ensure!(
                    entry.starts_with(b"040000 tree "),
                    "non-directory in source path: {p}"
                );
            } else {
                ensure!(
                    entry.starts_with(b"100644 blob ") || entry.starts_with(b"100755 blob "),
                    "symlink or non-regular tracked file: {p}"
                );
            }
        }
        let b = self.git(&["show", &format!("{r}:{p}")])?;
        ensure!(b.len() <= 8 * 1024 * 1024, "file exceeds 8 MiB limit");
        Ok(Some(b))
    }
}
