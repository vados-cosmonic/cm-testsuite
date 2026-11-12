//! Throwaway local Git repositories that ignore the developer's environment.

use std::{path::Path, process::Command};

/// Run Git in `root` without user hooks, signing, line-ending conversion, global
/// configuration, or the developer's identity. Panics with Git's diagnostics on
/// failure and returns standard output without its trailing newline.
pub fn git(root: &Path, args: &[&str]) -> String {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args([
            "-c",
            "core.autocrlf=false",
            "-c",
            "core.hooksPath=",
            "-c",
            "commit.gpgsign=false",
            "-c",
            "user.name=cm-testsuite tests",
            "-c",
            "user.email=tests@example.invalid",
        ])
        .args(args)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env(
            "GIT_CONFIG_GLOBAL",
            if cfg!(windows) { "NUL" } else { "/dev/null" },
        )
        // A test run from a Git hook must not act on the hook's repository.
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .env_remove("GIT_INDEX_FILE")
        .output()
        .expect("git must be installed to run these tests");
    assert!(
        output.status.success(),
        "git {args:?} failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout)
        .expect("git output must be UTF-8")
        .trim_end()
        .to_owned()
}

/// Commit every change in the working tree, or an empty commit when there is
/// none, and return the new commit identifier.
pub fn commit(root: &Path, message: &str) -> String {
    git(root, &["add", "--all"]);
    git(root, &["commit", "--quiet", "--allow-empty", "-m", message]);
    git(root, &["rev-parse", "HEAD"])
}
