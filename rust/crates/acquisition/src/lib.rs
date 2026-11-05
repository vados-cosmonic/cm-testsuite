//! Resolve opted-in manifests at immutable revisions and fetch an entire source before mutation.
use anyhow::{Context, Result, ensure};
use cm_catalog::*;
use cm_github::Api;
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    process::Command,
};
pub trait Repository {
    fn resolve(&self, reference: &str) -> Result<String>;
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
    fn git(&self, args: &[&str]) -> Result<Vec<u8>> {
        let o = Command::new("git")
            .arg("--no-pager")
            .arg("-C")
            .arg(&self.root)
            .args(args)
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
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
        ensure!(b.len() <= 8 * 1024 * 1024, "file exceeds 8 MiB draft limit");
        Ok(b)
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Snapshot {
    pub source: String,
    pub metadata: SourceSnapshot,
    pub tests: BTreeMap<String, Test>,
    pub files: BTreeMap<String, String>,
}
impl Snapshot {
    pub fn validate(&self) -> Result<()> {
        let c = Catalog {
            schema: SCHEMA,
            sources: BTreeMap::from([(self.source.clone(), self.metadata.clone())]),
            tests: self.tests.clone(),
            retired: BTreeMap::new(),
        };
        c.validate()?;
        ensure!(
            self.files.len() == self.tests.len(),
            "snapshot file count mismatch"
        );
        for t in self.tests.values() {
            let text = self
                .files
                .get(&t.destination)
                .context("missing snapshot bytes")?;
            ensure!(
                sha256(text.as_bytes()) == t.sha256,
                "snapshot digest mismatch"
            );
            ensure!(
                cm_wast_analysis::analyze(text)? == t.analysis,
                "snapshot analysis does not match its bytes/analyzer"
            );
        }
        Ok(())
    }
}
pub fn acquire(
    repo: &dyn Repository,
    source: &str,
    reference: &str,
    manifest_path: &str,
) -> Result<Snapshot> {
    validate_source(source)?;
    validate_path(manifest_path)?;
    let revision = repo.resolve(reference)?;
    validate_revision(&revision)?;
    let raw = repo.read(&revision, manifest_path)?;
    let manifest = Manifest::parse(std::str::from_utf8(&raw)?, manifest_path)?;
    let metadata = SourceSnapshot {
        revision: revision.clone(),
        manifest_path: manifest_path.into(),
        manifest_sha256: sha256(&raw),
    };
    let mut tests = BTreeMap::new();
    let mut files = BTreeMap::new();
    for spec in manifest.test {
        let pinned = spec.revision.unwrap_or_else(|| revision.clone());
        validate_revision(&pinned)?;
        // Resolve the override as a commit too; reject tags, blobs, or missing object IDs.
        ensure!(
            repo.resolve(&pinned)? == pinned,
            "per-test revision did not resolve to its exact commit"
        );
        let path = resolve_path(manifest_path, &spec.path)?;
        ensure!(
            !path.starts_with("corpus/wast/"),
            "received central copies cannot be exported as authoritative tests"
        );
        for route in &manifest.receive.route {
            let dest = resolve_path(manifest_path, &route.destination)?;
            ensure!(
                path != dest && !path.starts_with(&(dest + "/")),
                "test points inside a receiving destination"
            );
        }
        let bytes = repo.read(&pinned, &path)?;
        let sha = sha256(&bytes);
        if let Some(expected) = spec.sha256 {
            ensure!(sha == expected, "SHA-256 mismatch for {}", spec.name);
        }
        let text = String::from_utf8(bytes).context("WAST must be UTF-8")?;
        let analysis = cm_wast_analysis::analyze(&text)
            .with_context(|| format!("invalid WAST {}", spec.name))?;
        let destination = corpus_path(source, &spec.name)?;
        let test = Test {
            source: source.into(),
            name: spec.name,
            revision: pinned,
            manifest_revision: revision.clone(),
            source_path: path,
            sha256: sha,
            destination: destination.clone(),
            summary: spec.summary,
            requires: spec.requires,
            covers: spec.covers,
            analysis,
        };
        tests.insert(test.id(), test);
        files.insert(destination, text);
    }
    let snapshot = Snapshot {
        source: source.into(),
        metadata,
        tests,
        files,
    };
    snapshot.validate()?;
    Ok(snapshot)
}
pub fn load_catalog_files(root: &Path, catalog: &Catalog) -> Result<BTreeMap<String, String>> {
    catalog.validate()?;
    let mut files = BTreeMap::new();
    for t in catalog.tests.values() {
        let p = checked_path(root, &t.destination)?;
        let b = std::fs::read(&p)?;
        ensure!(sha256(&b) == t.sha256, "catalog drift: {}", t.destination);
        let text = String::from_utf8(b)?;
        ensure!(
            cm_wast_analysis::analyze(&text)? == t.analysis,
            "catalog analysis does not match its bytes/analyzer"
        );
        files.insert(t.destination.clone(), text);
    }
    Ok(files)
}
pub fn load_snapshot(path: &Path) -> Result<Snapshot> {
    let s: Snapshot = serde_json::from_slice(&std::fs::read(path)?)?;
    s.validate()?;
    Ok(s)
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;
    struct Fake {
        fail: bool,
        reads: Cell<usize>,
    }
    impl Repository for Fake {
        fn resolve(&self, _: &str) -> Result<String> {
            Ok("a".repeat(40))
        }
        fn read(&self, _: &str, p: &str) -> Result<Vec<u8>> {
            self.reads.set(self.reads.get() + 1);
            match p{".cm-testsuite.toml"=>Ok(b"schema=1\n[[test]]\nname='one'\npath='file:///one.wast'\n[[test]]\nname='two'\npath='file:///two.wast'\n".to_vec()),"two.wast" if self.fail=>anyhow::bail!("unavailable"),_=>Ok(b";; Explain the test intent\n(module)\r\n".to_vec())}
        }
    }
    #[test]
    fn fetches_exact_bytes_and_fails_as_a_unit() {
        let good = Fake {
            fail: false,
            reads: Cell::new(0),
        };
        let s = acquire(&good, "github:example/source", "main", ".cm-testsuite.toml").unwrap();
        assert_eq!(s.tests.len(), 2);
        assert!(s.files.values().all(|x| x.ends_with("\r\n")));
        assert!(
            acquire(
                &Fake {
                    fail: true,
                    reads: Cell::new(0)
                },
                "github:example/source",
                "main",
                ".cm-testsuite.toml"
            )
            .is_err()
        );
    }
}
