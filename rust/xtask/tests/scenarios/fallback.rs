//! External source configuration through the CLI, without modifying upstream.

use crate::common::*;
use cm_catalog::{CATALOG_PATH, Catalog, sha256};
use std::{fs, process::Output};

const FALLBACK: &str = "registry/manifests/source.toml";
const RULES: &str = "schema=1\n[[gather]]\npath='file:///tests'\nrecursive=true\n";

fn acquire(f: &Exchange, manifest: &str, fallback: &str) -> Output {
    cli(&f.hub)
        .args([
            "acquire",
            "--source",
            SOURCE,
            "--reference",
            "main",
            "--manifest",
            manifest,
            "--fallback-manifest",
            fallback,
            "--local",
        ])
        .arg(&f.source)
        .arg("--out")
        .arg(f.artifact("snapshot.toml"))
        .output()
        .unwrap()
}

fn unconfigured() -> Exchange {
    let f = Exchange::new();
    fs::remove_file(f.source.join(".cm-testsuite.toml")).unwrap();
    write(&f.source, "tests/nested/another.wast", UPDATED);
    commit(&f.source);
    write(&f.hub, FALLBACK, RULES);
    f
}

#[test]
fn fallback_import_and_upstream_adoption_keep_identities_and_provenance() {
    let f = unconfigured();
    let source_before = git(&f.source, &["rev-parse", "HEAD"]);
    success(acquire(&f, ".cm-testsuite.toml", FALLBACK));
    let external = f.snapshot();
    assert_eq!(external.source, SOURCE);
    assert_eq!(external.tests.len(), 2);
    assert_eq!(
        external.metadata.fallback_manifest.as_deref(),
        Some(FALLBACK)
    );
    assert_eq!(external.metadata.manifest_sha256, sha256(RULES.as_bytes()));
    assert_eq!(external.metadata.revision, source_before.trim());
    assert!(
        external
            .tests
            .values()
            .all(|t| t.manifest_revision.is_none())
    );
    assert_eq!(
        external
            .files
            .values()
            .find(|s| s.as_str() == WAST)
            .unwrap(),
        WAST
    );
    assert_eq!(git(&f.source, &["rev-parse", "HEAD"]), source_before);
    assert!(git(&f.source, &["status", "--porcelain"]).is_empty());
    success(f.plan_import());
    let plan = f.plan("import.toml");
    assert!(plan.guards.iter().any(|g| g.path == FALLBACK
        && g.sha256.as_deref() == Some(external.metadata.manifest_sha256.as_str())));
    success(f.apply(&f.hub, "import.toml"));
    success(f.plan_import());
    assert!(f.plan("import.toml").edits.is_empty());
    let original = Catalog::load(&f.hub).unwrap();

    // Exactly the same configuration moves upstream. A stale/missing fallback
    // must not block native acquisition, nor cause the handover to be suppressed.
    write(&f.source, ".cm-testsuite.toml", RULES);
    let upstream_commit = commit(&f.source);
    fs::remove_file(f.hub.join(FALLBACK)).unwrap();
    success(acquire(&f, ".cm-testsuite.toml", FALLBACK));
    let native = f.snapshot();
    assert_eq!(native.metadata.fallback_manifest, None);
    assert_eq!(native.files, external.files);
    assert!(
        native
            .tests
            .values()
            .all(|t| t.manifest_revision.as_deref() == Some(upstream_commit.as_str()))
    );
    success(f.plan_import());
    assert!(
        f.plan("import.toml")
            .edits
            .iter()
            .all(|e| e.path == CATALOG_PATH)
    );
    success(f.apply(&f.hub, "import.toml"));
    let current = Catalog::load(&f.hub).unwrap();
    assert_eq!(
        current.tests.keys().collect::<Vec<_>>(),
        original.tests.keys().collect::<Vec<_>>()
    );
    assert!(current.retired.is_empty());
    assert_eq!(current.sources[SOURCE].fallback_manifest, None);
}

#[test]
fn stale_external_configuration_blocks_planning_and_application() {
    let f = unconfigured();
    success(acquire(&f, ".cm-testsuite.toml", FALLBACK));
    success(f.plan_import());
    write(&f.hub, FALLBACK, "schema=1\n");
    failure(
        f.plan_import(),
        "fallback manifest changed since acquisition",
    );
    let applied = f.apply(&f.hub, "import.toml");
    assert!(!applied.status.success());
    assert!(!f.hub.join(CATALOG_PATH).exists());
    assert!(!f.hub.join("corpus").exists());
}

#[test]
fn fallback_paths_resolve_at_the_upstream_manifest_address() {
    let f = unconfigured();
    write(&f.source, "config/fixtures/pinned.wast", WAST);
    let revision = commit(&f.source);
    write(
        &f.hub,
        FALLBACK,
        manifest(
            "file://fixtures/pinned.wast",
            &format!(
                "revision='{revision}'\nsha256='{}'",
                sha256(WAST.as_bytes())
            ),
        ),
    );
    success(acquire(&f, "config/tests.toml", FALLBACK));
    let snapshot = f.snapshot();
    let test = snapshot.tests.values().next().unwrap();
    assert_eq!(test.source_path, "config/fixtures/pinned.wast");
    assert_eq!(test.revision, revision);
    assert_eq!(test.manifest_revision, None);
    assert_eq!(snapshot.metadata.manifest_path, "config/tests.toml");
}

#[test]
fn invalid_upstream_is_never_replaced_and_failures_preserve_output() {
    let f = unconfigured();
    success(acquire(&f, ".cm-testsuite.toml", FALLBACK));
    let before = fs::read(f.artifact("snapshot.toml")).unwrap();
    for contents in [
        "not valid TOML",
        "schema=99",
        "schema=1\n[[test]]\nname='missing'\npath='file:///missing.wast'\n",
    ] {
        write(&f.source, ".cm-testsuite.toml", contents);
        commit(&f.source);
        assert!(!acquire(&f, ".cm-testsuite.toml", FALLBACK).status.success());
        assert_eq!(fs::read(f.artifact("snapshot.toml")).unwrap(), before);
    }
    // A present, valid manifest with no tests is authoritative too.
    write(&f.source, ".cm-testsuite.toml", "schema=1\n");
    commit(&f.source);
    success(acquire(&f, ".cm-testsuite.toml", FALLBACK));
    assert!(f.snapshot().tests.is_empty());
    assert_eq!(f.snapshot().metadata.fallback_manifest, None);
}

#[test]
fn missing_invalid_or_unsafe_fallback_cannot_create_a_snapshot() {
    let f = unconfigured();
    for path in [
        "missing.toml",
        "../outside.toml",
        "/tmp/outside.toml",
        ".git/config",
    ] {
        assert!(!acquire(&f, ".cm-testsuite.toml", path).status.success());
        assert!(!f.artifact("snapshot.toml").exists());
    }
    write(&f.hub, FALLBACK, "invalid manifest");
    assert!(!acquire(&f, ".cm-testsuite.toml", FALLBACK).status.success());
    assert!(!f.artifact("snapshot.toml").exists());
}

#[test]
fn nonregular_upstream_manifest_or_ancestor_does_not_enable_fallback() {
    for (mode, path, manifest) in [
        ("120000", ".cm-testsuite.toml", ".cm-testsuite.toml"),
        ("160000", ".cm-testsuite.toml", ".cm-testsuite.toml"),
        ("120000", "config", "config/tests.toml"),
        ("160000", "config", "config/tests.toml"),
    ] {
        let f = unconfigured();
        let object = if mode == "120000" {
            "HEAD:tests/input.wast"
        } else {
            "HEAD"
        };
        let sha = git(&f.source, &["rev-parse", object]);
        git(
            &f.source,
            &[
                "update-index",
                "--add",
                "--cacheinfo",
                &format!("{mode},{},{path}", sha.trim()),
            ],
        );
        git(
            &f.source,
            &["commit", "--quiet", "-m", "test: nonregular manifest"],
        );
        let output = acquire(&f, manifest, FALLBACK);
        failure(
            output,
            if path == "config" {
                "non-directory"
            } else {
                "non-regular"
            },
        );
        assert!(!f.artifact("snapshot.toml").exists());
    }
}

#[cfg(unix)]
#[test]
fn fallback_symlinks_are_rejected() {
    let f = unconfigured();
    fs::remove_file(f.hub.join(FALLBACK)).unwrap();
    let outside = f.artifact("external.toml");
    fs::write(&outside, RULES).unwrap();
    std::os::unix::fs::symlink(outside, f.hub.join(FALLBACK)).unwrap();
    failure(acquire(&f, ".cm-testsuite.toml", FALLBACK), "symlink");
    assert!(!f.artifact("snapshot.toml").exists());
}
