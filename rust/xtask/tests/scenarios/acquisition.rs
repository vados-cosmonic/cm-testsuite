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
    assert_eq!(test.manifest_revision, Some(current));
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
        "missing tracked source manifest",
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

#[test]
fn gathers_committed_direct_children_or_recursively_with_stable_names() {
    let f = Exchange::new();
    write(&f.source, "tests/nested/input.wast", UPDATED);
    write(&f.source, "tests/ignore.wat", "not WAST");
    write(&f.source, "tests-other/outside.wast", "not WAST");
    for recursive in [false, true] {
        write(
            &f.source,
            ".cm-testsuite.toml",
            format!(
                "schema=1\n[[gather]]\npath='file:///tests'\nrecursive={recursive}\nrequires=['component-model']\ncovers=['smoke']\n"
            ),
        );
        let revision = commit(&f.source);
        write(&f.source, "tests/input.wast", "staged invalid WAST");
        git(&f.source, &["add", "tests/input.wast"]);
        write(&f.source, "tests/input.wast", "dirty invalid WAST");
        write(&f.source, "tests/untracked.wast", "untracked invalid WAST");
        success(f.acquire("main", ".cm-testsuite.toml"));
        let snapshot = f.snapshot();
        assert_eq!(snapshot.tests.len(), if recursive { 2 } else { 1 });
        let test = &snapshot.tests[&format!("{SOURCE}/tests/input")];
        assert_eq!(test.source_path, "tests/input.wast");
        assert_eq!(test.revision, revision);
        assert_eq!(snapshot.files[&test.destination], WAST);
        assert!(test.requires.contains("component-model"));
        assert!(test.covers.contains("smoke"));
        let bytes = fs::read(f.artifact("snapshot.toml")).unwrap();
        success(f.acquire("main", ".cm-testsuite.toml"));
        assert_eq!(fs::read(f.artifact("snapshot.toml")).unwrap(), bytes);
        git(&f.source, &["reset", "--hard", "HEAD"]);
        fs::remove_file(f.source.join("tests/untracked.wast")).unwrap();
    }
}

#[test]
fn gather_directories_use_manifest_and_repository_relative_paths() {
    let f = Exchange::new();
    write(&f.source, "config/fixtures/a.wast", WAST);
    for (path, expected) in [
        ("file://fixtures", "config/fixtures/a.wast"),
        ("file:///tests", "tests/input.wast"),
        ("file://", "config/fixtures/a.wast"),
    ] {
        write(
            &f.source,
            "config/manifest.toml",
            format!("schema=1\n[[gather]]\npath='{path}'\nrecursive=true\n"),
        );
        commit(&f.source);
        success(f.acquire("main", "config/manifest.toml"));
        let snapshot = f.snapshot();
        assert_eq!(snapshot.tests.len(), 1);
        assert_eq!(
            snapshot.tests.values().next().unwrap().source_path,
            expected
        );
    }
}

#[test]
fn root_discovery_excludes_received_copies() {
    let f = Exchange::new();
    write(
        &f.source,
        ".cm-testsuite.toml",
        "schema=1\n[[gather]]\npath='file:///'\nrecursive=true\n[[receive.route]]\nname='shared'\ndestination='file:///tests/shared'\n",
    );
    write(
        &f.source,
        "tests/shared/received.wast",
        "invalid received WAST",
    );
    write(
        &f.source,
        "corpus/wast/received.wast",
        "invalid central WAST",
    );
    commit(&f.source);
    success(f.acquire("main", ".cm-testsuite.toml"));
    assert_eq!(f.snapshot().tests.len(), 1);
}

#[test]
fn explicit_tests_override_discovery_with_names_metadata_and_pins() {
    let f = Exchange::new();
    write(&f.source, "tests/café space.wast", WAST);
    let old = commit(&f.source);
    write(&f.source, "tests/café space.wast", "invalid new contents");
    write(
        &f.source,
        ".cm-testsuite.toml",
        manifest(
            "file:///tests/caf%C3%A9%20space.wast",
            &format!(
                "revision='{old}'\nsha256='{}'\n",
                cm_catalog::sha256(WAST.as_bytes())
            ),
        ) + "\n[[gather]]\npath='file:///tests'\nrequires=['discovered']\n",
    );
    commit(&f.source);
    success(f.acquire("main", ".cm-testsuite.toml"));
    let snapshot = f.snapshot();
    assert_eq!(snapshot.tests.len(), 2);
    let test = &snapshot.tests[&format!("{SOURCE}/nested/test")];
    assert_eq!(test.revision, old);
    assert_eq!(test.summary, "Fixture intent");
    assert!(test.requires.contains("component-model"));
    assert!(!test.requires.contains("discovered"));
    assert_eq!(snapshot.files[&test.destination], WAST);
}

#[test]
fn invalid_or_ambiguous_discovery_preserves_previous_snapshot() {
    let f = Exchange::new();
    success(f.acquire("main", ".cm-testsuite.toml"));
    let before = fs::read(f.artifact("snapshot.toml")).unwrap();
    for (rules, diagnostic) in [
        (
            "[[gather]]\npath='file:///absent'",
            "Git snapshot read failed",
        ),
        (
            "[[gather]]\npath='file:///tests/input.wast'",
            "not a tracked directory",
        ),
        (
            "[[gather]]\npath='file:///../tests'",
            "unsafe repository path",
        ),
        (
            "[[gather]]\npath='file:///tests'\nrecusive=true",
            "unknown field",
        ),
        (
            "[[gather]]\npath='file:///tests'\n[[gather]]\npath='file:///tests'",
            "overlapping gather",
        ),
        (
            "[[test]]\nname='tests/input'\npath='file:///other.wast'\n[[gather]]\npath='file:///tests'",
            "duplicate test name",
        ),
    ] {
        write(
            &f.source,
            ".cm-testsuite.toml",
            format!("schema=1\n{rules}\n"),
        );
        commit(&f.source);
        failure(f.acquire("main", ".cm-testsuite.toml"), diagnostic);
        assert_eq!(fs::read(f.artifact("snapshot.toml")).unwrap(), before);
    }
}

#[test]
fn discovery_rejects_selected_symlinks_and_submodules() {
    for mode in ["120000", "160000"] {
        let f = Exchange::new();
        write(
            &f.source,
            ".cm-testsuite.toml",
            "schema=1\n[[gather]]\npath='file:///tests'\nrecursive=true\n",
        );
        commit(&f.source);
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
                "--cacheinfo",
                &format!("{mode},{},tests/input.wast", sha.trim()),
            ],
        );
        git(
            &f.source,
            &["commit", "--quiet", "-m", "test: nonregular entry"],
        );
        failure(
            f.acquire("main", ".cm-testsuite.toml"),
            "non-regular tracked WAST",
        );
        assert!(!f.artifact("snapshot.toml").exists());
    }
}

#[test]
fn existing_directory_without_wast_is_a_valid_empty_selection() {
    let f = Exchange::new();
    write(&f.source, "empty/README", "No tests yet");
    write(
        &f.source,
        ".cm-testsuite.toml",
        "schema=1\n[[gather]]\npath='file:///empty'\n",
    );
    commit(&f.source);
    success(f.acquire("main", ".cm-testsuite.toml"));
    assert!(f.snapshot().tests.is_empty());
}

#[test]
fn github_discovery_matches_local_committed_selection() {
    use cm_acquisition::{GitHub, acquire};
    use cm_test_support::github::FakeGitHubs;

    let f = Exchange::new();
    let api = FakeGitHubs::new(&["example/source"]);
    for recursive in [false, true] {
        let manifest =
            format!("schema=1\n[[gather]]\npath='file:///tests'\nrecursive={recursive}\n");
        let files = [
            (".cm-testsuite.toml", manifest.as_str()),
            ("tests/input.wast", WAST),
            ("tests/nested/input.wast", UPDATED),
            ("tests/ignore.wat", "ignored"),
        ];
        for (path, contents) in files {
            write(&f.source, path, contents);
        }
        commit(&f.source);
        api.push("example/source", &files);
        let remote = acquire(
            &GitHub {
                api: &api,
                source: SOURCE,
            },
            SOURCE,
            "main",
            ".cm-testsuite.toml",
        )
        .unwrap();
        success(f.acquire("main", ".cm-testsuite.toml"));
        let local = f.snapshot();
        assert_eq!(remote.files, local.files);
        assert_eq!(
            remote.tests.keys().collect::<Vec<_>>(),
            local.tests.keys().collect::<Vec<_>>()
        );
        for (id, test) in &remote.tests {
            assert_eq!(test.source_path, local.tests[id].source_path);
            assert_eq!(test.sha256, local.tests[id].sha256);
        }
    }
    assert_eq!(api.writes(), 0);
}

#[test]
fn github_discovery_rejects_incomplete_trees_and_nonregular_wast() {
    use cm_acquisition::{GitHub, acquire};
    use cm_test_support::github::FakeGitHubs;
    use serde_json::json;

    for (mode, diagnostic) in [
        ("truncated", "tree response truncated"),
        ("missing", "missing tree"),
        ("120000", "non-regular tracked WAST"),
        ("160000", "non-regular tracked WAST"),
    ] {
        let api = FakeGitHubs::new(&["example/source"]);
        api.push(
            "example/source",
            &[
                (
                    ".cm-testsuite.toml",
                    "schema=1\n[[gather]]\npath='file:///tests'\nrecursive=true\n",
                ),
                ("tests/nested/input.wast", WAST),
            ],
        );
        {
            let mut state = api.repository("example/source").state_mut();
            let tree = state
                .trees
                .iter()
                .find_map(|(sha, tree)| {
                    tree["tree"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .any(|e| e["path"] == "input.wast")
                        .then(|| sha.clone())
                })
                .unwrap();
            if mode == "missing" {
                state.trees.remove(&tree);
            } else if mode == "truncated" {
                state.trees.get_mut(&tree).unwrap()["truncated"] = json!(true);
            } else {
                let entry = &mut state.trees.get_mut(&tree).unwrap()["tree"][0];
                entry["mode"] = json!(mode);
                if mode == "160000" {
                    entry["type"] = json!("commit");
                }
            }
        }
        let error = acquire(
            &GitHub {
                api: &api,
                source: SOURCE,
            },
            SOURCE,
            "main",
            ".cm-testsuite.toml",
        )
        .unwrap_err();
        assert!(error.to_string().contains(diagnostic), "{error:#}");
    }
}
