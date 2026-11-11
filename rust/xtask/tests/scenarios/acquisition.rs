//! Immutable local acquisition, manifest addressing, and failed snapshot handling.

use crate::common::*;
use cm_acquisition::{LocalGit, Repository};
use std::fs;

#[test]
fn ignores_dirty_staged_and_untracked_worktree_files() {
    let f = Exchange::new();
    let revision = git(&f.source, &["rev-parse", "HEAD"]).trim().to_owned();
    write(&f.source, "tests/input.wast", "staged invalid WAST");
    git(&f.source, &["add", "tests/input.wast"]);
    write(&f.source, "tests/input.wast", "unstaged invalid WAST");
    write(&f.source, ".cm-testsuite.toml", "invalid manifest");
    write(&f.source, "tests/untracked.wast", "untracked");
    let before = git(&f.source, &["status", "--porcelain"]);
    success(f.acquire("main", ".cm-testsuite.toml"));
    let snapshot = f.snapshot();
    assert_eq!(snapshot.metadata.revision, revision);
    assert_eq!(snapshot.files.values().next().unwrap(), WAST);
    assert_eq!(git(&f.source, &["status", "--porcelain"]), before);
}

#[test]
fn reads_committed_files_even_after_worktree_deletion() {
    let f = Exchange::new();
    fs::remove_file(f.source.join("tests/input.wast")).unwrap();
    fs::remove_file(f.source.join(".cm-testsuite.toml")).unwrap();
    success(f.acquire("main", ".cm-testsuite.toml"));
    assert_eq!(f.snapshot().files.values().next().unwrap(), WAST);
}

#[test]
fn nested_manifest_resolves_relative_and_repository_root_paths() {
    let f = Exchange::new();
    for (url, expected) in [
        ("file://fixtures/test.wast", "config/fixtures/test.wast"),
        ("file:///fixtures/test.wast", "fixtures/test.wast"),
    ] {
        write(&f.source, "config/manifest.toml", manifest(url, ""));
        write(&f.source, expected, WAST);
        commit(&f.source);
        success(f.acquire("main", "config/manifest.toml"));
        assert_eq!(
            f.snapshot().tests.values().next().unwrap().source_path,
            expected
        );
    }
}

#[test]
fn percent_encoded_spaces_and_unicode_preserve_file_bytes() {
    let f = Exchange::new();
    write(&f.source, "tests/café space.wast", WAST);
    write(
        &f.source,
        ".cm-testsuite.toml",
        manifest("file:///tests/caf%C3%A9%20space.wast", ""),
    );
    commit(&f.source);
    success(f.acquire("main", ".cm-testsuite.toml"));
    let snapshot = f.snapshot();
    assert_eq!(
        snapshot.tests.values().next().unwrap().source_path,
        "tests/café space.wast"
    );
    assert_eq!(
        snapshot.files.values().next().unwrap().as_bytes(),
        WAST.as_bytes()
    );
}

#[test]
fn commit_tags_and_per_file_pins_are_immutable() {
    let f = Exchange::new();
    let old = git(&f.source, &["rev-parse", "HEAD"]).trim().to_owned();
    git(&f.source, &["tag", "-a", "fixture-v1", "-m", "fixture"]);
    f.update();
    for reference in [&old, "fixture-v1"] {
        success(f.acquire(reference, ".cm-testsuite.toml"));
        assert_eq!(f.snapshot().metadata.revision, old);
        assert_eq!(f.snapshot().files.values().next().unwrap(), WAST);
    }
    write(
        &f.source,
        ".cm-testsuite.toml",
        manifest(
            "file:///tests/input.wast",
            &format!(
                "revision = '{old}'\nsha256 = '{}'\n",
                cm_catalog::sha256(WAST.as_bytes())
            ),
        ),
    );
    let current = commit(&f.source);
    success(f.acquire("main", ".cm-testsuite.toml"));
    let snapshot = f.snapshot();
    let test = snapshot.tests.values().next().unwrap();
    assert_eq!(test.revision, old);
    assert_eq!(test.manifest_revision, current);
    assert_eq!(snapshot.files.values().next().unwrap(), WAST);
}

#[test]
fn bare_repository_does_not_require_a_working_tree() {
    let f = Exchange::new();
    let bare = f.artifact("bare source.git");
    git(
        f.temp.path(),
        &[
            "clone",
            "--quiet",
            "--bare",
            f.source.to_str().unwrap(),
            bare.to_str().unwrap(),
        ],
    );
    let snapshot = cm_acquisition::acquire(
        &LocalGit { root: bare },
        SOURCE,
        "main",
        ".cm-testsuite.toml",
    )
    .unwrap();
    assert_eq!(snapshot.files.values().next().unwrap(), WAST);
}

#[test]
fn missing_reference_or_manifest_does_not_create_a_snapshot() {
    let f = Exchange::new();
    failure(
        f.acquire("missing-ref", ".cm-testsuite.toml"),
        "Git snapshot read failed",
    );
    failure(
        f.acquire("main", "missing.toml"),
        "non-regular tracked file",
    );
    assert!(!f.artifact("snapshot.toml").exists());
}

#[test]
fn failed_acquisition_preserves_existing_output() {
    let f = Exchange::new();
    success(f.acquire("main", ".cm-testsuite.toml"));
    let before = fs::read(f.artifact("snapshot.toml")).unwrap();
    write(
        &f.source,
        ".cm-testsuite.toml",
        manifest(
            "file:///tests/input.wast",
            &format!("sha256 = '{}'", "0".repeat(64)),
        ),
    );
    commit(&f.source);
    failure(f.acquire("main", ".cm-testsuite.toml"), "SHA-256 mismatch");
    assert_eq!(fs::read(f.artifact("snapshot.toml")).unwrap(), before);
}

#[test]
fn one_missing_file_rejects_the_entire_snapshot() {
    let f = Exchange::new();
    write(
        &f.source,
        ".cm-testsuite.toml",
        manifest("file:///tests/input.wast", "")
            + "\n[[test]]\nname = 'missing'\npath = 'file:///missing.wast'\n",
    );
    commit(&f.source);
    failure(
        f.acquire("main", ".cm-testsuite.toml"),
        "non-regular tracked file",
    );
    assert!(!f.artifact("snapshot.toml").exists());
}

#[test]
fn invalid_utf8_and_invalid_wast_are_rejected() {
    let f = Exchange::new();
    for (bytes, message) in [
        (vec![0xff, 0xfe], "WAST must be UTF-8"),
        (b"(module".to_vec(), "invalid WAST"),
    ] {
        write(&f.source, "tests/input.wast", bytes);
        commit(&f.source);
        failure(f.acquire("main", ".cm-testsuite.toml"), message);
        assert!(!f.artifact("snapshot.toml").exists());
    }
}

#[test]
fn traversal_and_git_metadata_paths_are_rejected() {
    let f = Exchange::new();
    for url in [
        "file://../outside.wast",
        "file://%2e%2e/outside.wast",
        "file:///.git/config.wast",
        "file:////absolute.wast",
        "file:///tests/%00.wast",
        "file:///tests/%zz.wast",
    ] {
        write(&f.source, ".cm-testsuite.toml", manifest(url, ""));
        commit(&f.source);
        let output = f.acquire("main", ".cm-testsuite.toml");
        assert!(!output.status.success(), "accepted {url}");
        assert!(!f.artifact("snapshot.toml").exists());
    }
}

#[test]
fn git_tree_and_blob_ids_are_not_commit_pins() {
    let f = Exchange::new();
    let repo = LocalGit {
        root: f.source.clone(),
    };
    for object in ["HEAD^{tree}", "HEAD:tests/input.wast"] {
        let hash = git(&f.source, &["rev-parse", object]);
        assert!(repo.resolve(hash.trim()).is_err());
    }
    assert!(repo.resolve("--help").is_err());
}

#[test]
fn git_symlink_entries_are_rejected_without_following_them() {
    let f = Exchange::new();
    // Set the Git entry mode directly so this works without OS symlink privileges.
    let blob = git(&f.source, &["rev-parse", "HEAD:tests/input.wast"]);
    git(
        &f.source,
        &[
            "update-index",
            "--cacheinfo",
            &format!("120000,{},tests/input.wast", blob.trim()),
        ],
    );
    git(
        &f.source,
        &["commit", "--quiet", "-m", "test: symlink entry"],
    );
    failure(f.acquire("main", ".cm-testsuite.toml"), "symlink");
}

#[test]
fn tracked_directory_and_submodule_are_not_wast_files() {
    let f = Exchange::new();
    write(&f.source, "directory.wast/child", "not a regular file");
    write(
        &f.source,
        ".cm-testsuite.toml",
        manifest("file:///directory.wast", ""),
    );
    let revision = commit(&f.source);
    failure(
        f.acquire("main", ".cm-testsuite.toml"),
        "non-regular tracked file",
    );
    write(
        &f.source,
        ".cm-testsuite.toml",
        manifest("file:///submodule.wast", ""),
    );
    git(&f.source, &["add", ".cm-testsuite.toml"]);
    git(
        &f.source,
        &[
            "update-index",
            "--add",
            "--cacheinfo",
            &format!("160000,{revision},submodule.wast"),
        ],
    );
    git(
        &f.source,
        &["commit", "--quiet", "-m", "test: gitlink entry"],
    );
    failure(
        f.acquire("main", ".cm-testsuite.toml"),
        "non-regular tracked file",
    );
}

#[test]
fn executable_git_files_remain_valid_sources() {
    let f = Exchange::new();
    git(
        &f.source,
        &["update-index", "--chmod=+x", "tests/input.wast"],
    );
    git(
        &f.source,
        &["commit", "--quiet", "-m", "test: executable source"],
    );
    success(f.acquire("main", ".cm-testsuite.toml"));
    assert_eq!(f.snapshot().files.values().next().unwrap(), WAST);
}

#[test]
fn oversize_source_is_rejected_without_partial_output() {
    let f = Exchange::new();
    write(
        &f.source,
        "tests/input.wast",
        " ".repeat(8 * 1024 * 1024 + 1),
    );
    commit(&f.source);
    failure(
        f.acquire("main", ".cm-testsuite.toml"),
        "exceeds 8 MiB limit",
    );
    assert!(!f.artifact("snapshot.toml").exists());
}
