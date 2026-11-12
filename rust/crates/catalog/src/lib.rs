//! Manifests, immutable identity, provenance, and safe repository-relative paths.
pub mod records;
mod registry;

pub use registry::{PARTICIPANTS_PATH, Participant, Participants};

use anyhow::{Context, Result, bail, ensure};
use percent_encoding::percent_decode_str;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Path, PathBuf},
};

pub const SCHEMA: u32 = 1;
pub const CATALOG_PATH: &str = "metadata/catalog.toml";
pub const RECEIPT_PATH: &str = ".cm-testsuite-lock.toml";
pub fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
/// Internal canonical encoding for semantic digests and GitHub ownership metadata.
pub fn json<T: Serialize>(value: &T) -> Result<String> {
    Ok(serde_json::to_string_pretty(value)? + "\n")
}
/// Hash a record's canonical representation for exchange integrity checks.
///
/// ```
/// use cm_catalog::{digest, validate_sha};
/// let record = vec!["component-model", "arithmetic"];
/// let hash = digest(&record)?;
/// validate_sha(&hash)?;
/// assert_eq!(hash, digest(&record)?);
/// # Ok::<(), anyhow::Error>(())
/// ```
pub fn digest<T: Serialize>(value: &T) -> Result<String> {
    Ok(sha256(json(value)?.as_bytes()))
}
pub fn validate_sha(value: &str) -> Result<()> {
    ensure!(
        value.len() == 64
            && value
                .bytes()
                .all(|x| x.is_ascii_hexdigit() && !x.is_ascii_uppercase()),
        "expected lowercase SHA-256"
    );
    Ok(())
}
pub fn validate_revision(value: &str) -> Result<()> {
    ensure!(
        [40, 64].contains(&value.len())
            && value
                .bytes()
                .all(|x| x.is_ascii_hexdigit() && !x.is_ascii_uppercase()),
        "expected a full lowercase Git commit ID"
    );
    Ok(())
}
/// Deliberately restrictive portable repository path. No OS absolute paths or Git metadata.
pub fn validate_path(value: &str) -> Result<()> {
    ensure!(
        !value.is_empty() && !value.starts_with('/') && !value.contains(['\\', ':', '\0']),
        "invalid repository path: {value}"
    );
    for part in value.split('/') {
        ensure!(
            !part.is_empty()
                && part != "."
                && part != ".."
                && !part.eq_ignore_ascii_case(".git")
                && !part.chars().any(char::is_control),
            "unsafe repository path: {value}"
        );
        ensure!(
            !part.ends_with(['.', ' ']),
            "nonportable repository path: {value}"
        );
    }
    Ok(())
}
pub fn validate_name(value: &str) -> Result<()> {
    validate_path(value)?;
    ensure!(
        value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"-_. /".contains(&b))
            && !value.contains(' '),
        "names use ASCII letters, digits, -, _, ., and /"
    );
    Ok(())
}
pub fn validate_source(source: &str) -> Result<()> {
    let repo = source
        .strip_prefix("github:")
        .context("source must be github:owner/repository")?;
    ensure!(
        repo.split('/').count() == 2,
        "source must identify owner/repository"
    );
    validate_name(repo)
}
pub fn validate_id(id: &str) -> Result<()> {
    let rest = id
        .strip_prefix("github:")
        .context("invalid test identity")?;
    let parts: Vec<_> = rest.splitn(3, '/').collect();
    ensure!(parts.len() == 3, "test identity needs source and name");
    validate_source(&format!("github:{}/{}", parts[0], parts[1]))?;
    validate_name(parts[2])
}
/// Project notation: file:// is relative to the manifest, file:/// to the repository.
pub fn resolve_path(manifest_path: &str, input: &str) -> Result<String> {
    validate_path(manifest_path)?;
    let (root, encoded) = if let Some(p) = input.strip_prefix("file:///") {
        (true, p)
    } else if let Some(p) = input.strip_prefix("file://") {
        (false, p)
    } else {
        bail!("expected file:// or file:/// path")
    };
    // Reject malformed percent escapes rather than accepting ambiguous representations.
    let bytes = encoded.as_bytes();
    for (i, b) in bytes.iter().enumerate() {
        if *b == b'%' {
            ensure!(
                i + 2 < bytes.len()
                    && bytes[i + 1].is_ascii_hexdigit()
                    && bytes[i + 2].is_ascii_hexdigit(),
                "malformed percent escape"
            );
        }
    }
    let decoded = percent_decode_str(encoded).decode_utf8()?.into_owned();
    validate_path(&decoded)?;
    let path = if root {
        decoded
    } else {
        match manifest_path.rsplit_once('/') {
            Some((parent, _)) => format!("{parent}/{decoded}"),
            None => decoded,
        }
    };
    validate_path(&path)?;
    Ok(path)
}
/// Refuse all symlinks, including ancestor symlinks, when accessing managed files.
pub fn checked_path(root: &Path, relative: &str) -> Result<PathBuf> {
    validate_path(relative)?;
    let root = root
        .canonicalize()
        .with_context(|| format!("repository root {}", root.display()))?;
    let mut path = root;
    for part in relative.split('/') {
        path.push(part);
        match std::fs::symlink_metadata(&path) {
            Ok(meta) => ensure!(
                !meta.file_type().is_symlink(),
                "symlink in managed path: {}",
                path.display()
            ),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(e.into()),
        }
    }
    Ok(path)
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    pub schema: u32,
    #[serde(default)]
    pub test: Vec<TestSpec>,
    #[serde(default)]
    pub receive: Receive,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TestSpec {
    pub name: String,
    pub path: String,
    #[serde(default)]
    pub summary: String,
    #[serde(default)]
    pub requires: BTreeSet<String>,
    #[serde(default)]
    pub covers: BTreeSet<String>,
    pub sha256: Option<String>,
    pub revision: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Receive {
    pub enabled: bool,
    pub additions: Policy,
    pub updates: Policy,
    pub deletions: Policy,
    pub exclude: BTreeSet<String>,
    pub route: Vec<Route>,
}
impl Default for Receive {
    fn default() -> Self {
        Self {
            enabled: false,
            additions: Policy::PullRequest,
            updates: Policy::PullRequest,
            deletions: Policy::Manual,
            exclude: BTreeSet::new(),
            route: vec![],
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Policy {
    PullRequest,
    Manual,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Route {
    pub name: String,
    pub destination: String,
    #[serde(default)]
    pub covers_any: BTreeSet<String>,
    #[serde(default)]
    pub supports: BTreeSet<String>,
    #[serde(default)]
    pub exclude: BTreeSet<String>,
    #[serde(default)]
    pub allow_unknown: bool,
}
impl Manifest {
    pub fn parse(text: &str, path: &str) -> Result<Self> {
        let m: Self = toml::from_str(text)?;
        m.validate(path)?;
        Ok(m)
    }
    pub fn validate(&self, path: &str) -> Result<()> {
        ensure!(
            self.schema == SCHEMA,
            "unsupported manifest schema {}",
            self.schema
        );
        validate_path(path)?;
        let mut names = BTreeSet::new();
        for test in &self.test {
            validate_name(&test.name)?;
            ensure!(
                names.insert(&test.name),
                "duplicate test name {}",
                test.name
            );
            let p = resolve_path(path, &test.path)?;
            ensure!(
                p.ends_with(".wast"),
                "only checked-in .wast files are supported"
            );
            if let Some(s) = &test.sha256 {
                validate_sha(s)?;
            }
            if let Some(r) = &test.revision {
                validate_revision(r)?;
            }
        }
        let mut routes = BTreeSet::new();
        for r in &self.receive.route {
            validate_name(&r.name)?;
            ensure!(routes.insert(&r.name), "duplicate receiving route");
            resolve_path(path, &r.destination)?;
            for id in &r.exclude {
                validate_id(id)?;
            }
        }
        for id in &self.receive.exclude {
            validate_id(id)?;
        }
        Ok(())
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Evidence {
    pub feature: String,
    pub byte_offset: usize,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct Analysis {
    pub analyzer: String,
    pub complete: bool,
    pub syntax: Vec<Evidence>,
    pub warnings: Vec<String>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Test {
    pub source: String,
    pub name: String,
    pub revision: String,
    pub source_path: String,
    pub manifest_revision: String,
    pub sha256: String,
    pub destination: String,
    pub summary: String,
    pub requires: BTreeSet<String>,
    pub covers: BTreeSet<String>,
    pub analysis: Analysis,
}
impl Test {
    pub fn id(&self) -> String {
        format!("{}/{}", self.source, self.name)
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceSnapshot {
    pub revision: String,
    pub manifest_path: String,
    pub manifest_sha256: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Catalog {
    pub schema: u32,
    pub sources: BTreeMap<String, SourceSnapshot>,
    pub tests: BTreeMap<String, Test>,
    #[serde(default)]
    pub retired: BTreeMap<String, String>,
}
impl Default for Catalog {
    fn default() -> Self {
        Self {
            schema: SCHEMA,
            sources: BTreeMap::new(),
            tests: BTreeMap::new(),
            retired: BTreeMap::new(),
        }
    }
}
impl Catalog {
    pub fn validate(&self) -> Result<()> {
        ensure!(self.schema == SCHEMA, "unsupported catalog schema");
        let mut paths = BTreeSet::new();
        for (id, sha) in &self.retired {
            validate_id(id)?;
            validate_sha(sha)?;
            ensure!(!self.tests.contains_key(id), "active identity also retired");
        }
        for (id, s) in &self.sources {
            validate_source(id)?;
            validate_revision(&s.revision)?;
            validate_path(&s.manifest_path)?;
            validate_sha(&s.manifest_sha256)?;
        }
        for (id, t) in &self.tests {
            validate_id(id)?;
            ensure!(*id == t.id(), "test identity mismatch");
            let source = self
                .sources
                .get(&t.source)
                .context("test source not in inventory")?;
            ensure!(
                source.revision == t.manifest_revision,
                "manifest revision mismatch"
            );
            validate_revision(&t.revision)?;
            validate_sha(&t.sha256)?;
            validate_path(&t.source_path)?;
            validate_path(&t.destination)?;
            ensure!(
                t.source_path.ends_with(".wast") && t.destination.ends_with(".wast"),
                "catalog contains non-WAST path"
            );
            ensure!(
                t.destination == corpus_path(&t.source, &t.name)?,
                "unexpected corpus destination"
            );
            ensure!(paths.insert(&t.destination), "duplicate corpus destination");
        }
        Ok(())
    }
    pub fn load(root: &Path) -> Result<Self> {
        let p = checked_path(root, CATALOG_PATH)?;
        match std::fs::read(p) {
            Ok(b) => {
                let c: Self = records::decode(&b)?;
                c.validate()?;
                Ok(c)
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Self::default()),
            Err(e) => Err(e.into()),
        }
    }
}
pub fn corpus_path(source: &str, name: &str) -> Result<String> {
    validate_source(source)?;
    validate_name(name)?;
    Ok(format!(
        "corpus/wast/{}/{name}.wast",
        source.strip_prefix("github:").unwrap()
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn paths_have_project_semantics() {
        assert_eq!(
            resolve_path("config/manifest.toml", "file://tests/a.wast").unwrap(),
            "config/tests/a.wast"
        );
        assert_eq!(
            resolve_path("config/manifest.toml", "file:///tests/a.wast").unwrap(),
            "tests/a.wast"
        );
        for bad in [
            "file:///../secret",
            "file://%2e%2e/secret",
            "file:////etc/passwd",
            "file:///a\\b",
            "file:///a/%zz",
            "file://.git/config",
        ] {
            assert!(resolve_path(".cm-testsuite.toml", bad).is_err(), "{bad}");
        }
    }
    #[test]
    fn catches_typos_and_duplicate_ids() {
        let t = "schema=1\n[[test]]\nname='x'\npath='file:///x.wast'\n";
        assert!(
            Manifest::parse(
                &(t.to_owned() + "[[test]]\nname='x'\npath='file:///y.wast'"),
                ".cm-testsuite.toml"
            )
            .is_err()
        );
        assert!(Manifest::parse("schema=1\nrecieve={enabled=true}", ".cm-testsuite.toml").is_err());
    }
    #[cfg(unix)]
    #[test]
    fn no_symlink_escape() {
        let a = tempfile::tempdir().unwrap();
        let b = tempfile::tempdir().unwrap();
        std::os::unix::fs::symlink(b.path(), a.path().join("out")).unwrap();
        assert!(checked_path(a.path(), "out/test.wast").is_err());
    }
}
