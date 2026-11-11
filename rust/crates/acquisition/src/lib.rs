//! Acquire validated WAST snapshots from immutable repository revisions.

mod repository;
mod snapshot;

pub use repository::{GitHub, LocalGit, Repository};
pub use snapshot::{Snapshot, load_catalog_files, load_snapshot};

use anyhow::{Context, Result, ensure};
use cm_catalog::*;
use std::collections::BTreeMap;

/// Acquire a complete source snapshot from a committed opt-in manifest.
///
/// Resolve the requested reference once, honor per-test commit/hash pins, and
/// retain exact UTF-8 WAST bytes with their provenance and structural analysis.
/// Return no snapshot if any manifest, file, or integrity check fails; never
/// write to the destination repository or execute the producer's tests.
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

    // Resolve provenance before collecting files; partial acquisition is never returned.
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

        // Preserve producer bytes exactly, checking pins before structural analysis.
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
            match p {
                ".cm-testsuite.toml" => Ok(concat!(
                    "schema=1\n[[test]]\nname='one'\npath='file:///one.wast'\n",
                    "[[test]]\nname='two'\npath='file:///two.wast'\n",
                )
                .as_bytes()
                .to_vec()),
                "two.wast" if self.fail => anyhow::bail!("unavailable"),
                _ => Ok(b";; Explain the test intent\n(module)\r\n".to_vec()),
            }
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
