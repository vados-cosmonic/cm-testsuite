//! Release history, tag targets, and merge checks against throwaway Git repositories.

use cm_release::{Metadata, history};
use std::{fs, path::Path, process::Command};

/// Run Git in a fixture repository without the developer's identity or
/// configuration, returning trimmed standard output.
fn git(root: &Path, args: &[&str]) -> String {
    let out = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(args)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env(
            "GIT_CONFIG_GLOBAL",
            if cfg!(windows) { "NUL" } else { "/dev/null" },
        )
        .env("GIT_AUTHOR_NAME", "Release test")
        .env("GIT_AUTHOR_EMAIL", "release@example.invalid")
        .env("GIT_COMMITTER_NAME", "Release test")
        .env("GIT_COMMITTER_EMAIL", "release@example.invalid")
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout).unwrap().trim().into()
}

/// Commit the whole working tree (or nothing) and return the new commit.
fn commit(root: &Path, message: &str) -> String {
    git(root, &["add", "."]);
    git(root, &["commit", "--allow-empty", "-m", message]);
    git(root, &["rev-parse", "HEAD"])
}

/// Write release files for `version` with the given changelog section body,
/// then commit and tag them as that release.
fn release_with_notes(root: &Path, version: &str, notes: &str) -> String {
    fs::create_dir_all(root.join("rust")).unwrap();
    fs::write(
        root.join("rust/Cargo.toml"),
        format!("[workspace.package]\nversion='{version}'\n"),
    )
    .unwrap();
    fs::write(
        root.join("rust/Cargo.lock"),
        format!("version=4\n[[package]]\nname='xtask'\nversion='{version}'\n"),
    )
    .unwrap();
    fs::write(
        root.join("CHANGELOG.md"),
        format!("# Changelog\n\n## [{version}] - 2026-11-06\n{notes}"),
    )
    .unwrap();
    let sha = commit(root, &format!("release: cm-testsuite v{version}"));
    git(root, &["tag", &format!("cm-testsuite-v{version}")]);
    sha
}

/// Commit and tag a well-formed release of `version`.
fn release(root: &Path, version: &str) -> String {
    release_with_notes(root, version, "\n- changes\n")
}

#[test]
fn stable_history_includes_rc_changes_but_excludes_later_commits() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    git(root, &["init", "-b", "main"]);
    release(root, "0.1.0");
    let alpha = commit(root, "feat(catalog): add alpha");
    release(root, "0.2.0-rc.1");
    let beta = commit(root, "fix(catalog): fix beta");
    let stable = release(root, "0.2.0");
    let later = commit(root, "feat(catalog): later change");
    git(root, &["update-ref", "refs/remotes/origin/main", &later]);

    let h = history::release_history(root, "cm-testsuite-v0.2.0").unwrap();
    assert_eq!(h.previous.as_deref(), Some("cm-testsuite-v0.1.0"));
    assert!(h.commits.contains(&alpha) && h.commits.contains(&beta));
    assert!(!h.commits.contains(&later));
    assert!(!h.commits.contains(&stable));

    let rc = history::release_history(root, "cm-testsuite-v0.2.0-rc.1").unwrap();
    assert_eq!(rc.previous.as_deref(), Some("cm-testsuite-v0.1.0"));
    assert_eq!(rc.commits, vec![alpha]);

    assert_eq!(
        history::tag_target(root, &Metadata::new("0.2.0").unwrap(), &stable).unwrap(),
        stable
    );
    assert!(history::tag_target(root, &Metadata::new("0.3.0").unwrap(), &stable).is_err());

    git(root, &["checkout", "--detach", "cm-testsuite-v0.2.0"]);
    history::validate_checkout(root, "cm-testsuite-v0.2.0").unwrap();
    assert!(history::validate_checkout(root, "cm-testsuite-v0.2.0-rc.1").is_err());
}

#[test]
fn first_release_and_detached_non_main_target() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    git(root, &["init", "-b", "main"]);
    let first = commit(root, "feat: initial feature");
    release(root, "0.1.0");

    let h = history::release_history(root, "cm-testsuite-v0.1.0").unwrap();
    assert_eq!(h.commits, vec![first.clone()]);
    assert!(h.previous.is_none());

    git(root, &["update-ref", "refs/remotes/origin/main", &first]);
    assert!(history::tag_target(root, &Metadata::new("0.1.0").unwrap(), "HEAD").is_err());
    // A maintainer-pushed tag outside main must not be released either.
    git(root, &["checkout", "--detach", "cm-testsuite-v0.1.0"]);
    assert!(history::validate_checkout(root, "cm-testsuite-v0.1.0").is_err());
}

#[test]
fn releases_without_notes_are_neither_tagged_nor_published() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    git(root, &["init", "-b", "main"]);
    let sha = release_with_notes(root, "0.1.0", "");
    git(root, &["update-ref", "refs/remotes/origin/main", &sha]);

    let meta = Metadata::new("0.1.0").unwrap();
    let error = history::tag_target(root, &meta, &sha).unwrap_err();
    assert!(error.to_string().contains("has no notes"), "{error}");
    git(root, &["checkout", "--detach", "cm-testsuite-v0.1.0"]);
    let error = history::validate_checkout(root, "cm-testsuite-v0.1.0").unwrap_err();
    assert!(error.to_string().contains("has no notes"), "{error}");
}

#[test]
fn tags_only_merges_that_land_directly_on_the_prepared_base() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    git(root, &["init", "-b", "main"]);
    let base = commit(root, "feat: before the release");
    let at = |sha: &str| git(root, &["checkout", "-q", "--detach", sha]);
    let ok =
        |merge: &str, commits| history::ensure_merged_onto(root, merge, &base, commits).is_ok();
    let merge_pr = |head: &str| {
        let message = "Merge pull request #5";
        git(root, &["merge", "-q", "--no-ff", "-m", message, head]);
        git(root, &["rev-parse", "HEAD"])
    };

    // Squash merges.
    let squash = commit(root, "release: cm-testsuite v0.2.0 (#5)");
    assert!(ok(&squash, 1));
    at(&base);
    commit(root, "feat: landed after preparation");
    let late_squash = commit(root, "release: cm-testsuite v0.2.0 (#5)");
    assert!(!ok(&late_squash, 1));
    assert!(!ok(&late_squash, 2));

    // Merge commits.
    at(&base);
    let release = commit(root, "release: cm-testsuite v0.2.0");
    at(&base);
    assert!(ok(&merge_pr(&release), 1));
    at(&base);
    commit(root, "feat: landed after preparation");
    assert!(!ok(&merge_pr(&release), 1));

    // Rebase merges of two release commits.
    at(&base);
    commit(root, "release: cm-testsuite v0.2.0");
    let rebased = commit(root, "release: cm-testsuite v0.2.0");
    assert!(ok(&rebased, 2));
    at(&base);
    commit(root, "feat: landed after preparation");
    commit(root, "release: cm-testsuite v0.2.0");
    let late_rebased = commit(root, "release: cm-testsuite v0.2.0");
    assert!(!ok(&late_rebased, 2));
}
