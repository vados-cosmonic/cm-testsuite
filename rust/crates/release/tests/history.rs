use cm_release::{Metadata, history};
use std::{fs, path::Path, process::Command};
fn git(root: &Path, args: &[&str]) -> String {
    let out = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(args)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
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
fn commit(root: &Path, message: &str) -> String {
    git(root, &["add", "."]);
    git(root, &["commit", "--allow-empty", "-m", message]);
    git(root, &["rev-parse", "HEAD"])
}
fn release(root: &Path, v: &str) -> String {
    fs::create_dir_all(root.join("rust")).unwrap();
    fs::write(
        root.join("rust/Cargo.toml"),
        format!("[workspace.package]\nversion='{v}'\n"),
    )
    .unwrap();
    fs::write(
        root.join("rust/Cargo.lock"),
        format!("version=4\n[[package]]\nname='xtask'\nversion='{v}'\n"),
    )
    .unwrap();
    fs::write(
        root.join("CHANGELOG.md"),
        format!("# Changelog\n\n## [{v}] - 2026-11-06\n\n- changes\n"),
    )
    .unwrap();
    let sha = commit(root, &format!("release: cm-testsuite v{v}"));
    git(root, &["tag", &format!("cm-testsuite-v{v}")]);
    sha
}
#[test]
fn stable_history_includes_rc_changes_but_excludes_later_commits() {
    let d = tempfile::tempdir().unwrap();
    let root = d.path();
    git(root, &["init", "-b", "main"]);
    release(root, "0.1.0");
    let alpha = commit(root, "feat(catalog): add alpha");
    release(root, "0.2.0-rc.1");
    let beta = commit(root, "fix(catalog): fix beta");
    let stable = release(root, "0.2.0");
    let later = commit(root, "feat(catalog): later change");
    git(root, &["update-ref", "refs/remotes/origin/main", &later]);
    let h = history::release_history(root, "cm-testsuite-v0.2.0").unwrap();
    assert!(h.commits.contains(&alpha) && h.commits.contains(&beta));
    assert!(!h.commits.contains(&later));
    assert!(!h.commits.contains(&stable));
    let rc = history::release_history(root, "cm-testsuite-v0.2.0-rc.1").unwrap();
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
    let d = tempfile::tempdir().unwrap();
    let root = d.path();
    git(root, &["init", "-b", "main"]);
    let first = commit(root, "feat: initial feature");
    release(root, "0.1.0");
    assert_eq!(
        history::release_history(root, "cm-testsuite-v0.1.0")
            .unwrap()
            .commits,
        vec![first.clone()]
    );
    git(root, &["update-ref", "refs/remotes/origin/main", &first]);
    assert!(history::tag_target(root, &Metadata::new("0.1.0").unwrap(), "HEAD").is_err());
}
