//! Exact-commit release history and pinned changelog generation.
use crate::{CLIFF_VERSION, Metadata, TAG_PREFIX, release_notes, version};
use anyhow::{Context, Result, ensure};
use cm_catalog::validate_revision;
use std::{collections::BTreeSet, fs, path::Path, process::Command};

pub fn git(root: &Path, args: &[&str]) -> Result<String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(args)
        .output()?;
    ensure!(
        output.status.success(),
        "git failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(String::from_utf8(output.stdout)?)
}
pub fn resolve(root: &Path, reference: &str) -> Result<String> {
    ensure!(!reference.is_empty(), "empty release reference");
    let sha = git(
        root,
        &[
            "rev-parse",
            "--verify",
            "--end-of-options",
            &format!("{reference}^{{commit}}"),
        ],
    )?
    .trim()
    .to_owned();
    validate_revision(&sha)?;
    Ok(sha)
}

pub fn changelog(root: &Path, meta: &Metadata) -> Result<String> {
    let tool = root.join("git-cliff");
    let output = Command::new(&tool)
        .arg("--version")
        .output()
        .context("bootstrap DotSlash before preparing a release")?;
    ensure!(
        output.status.success() && String::from_utf8_lossy(&output.stdout).trim() == CLIFF_VERSION,
        "expected pinned {CLIFF_VERSION}"
    );
    let pattern = if meta.prerelease {
        "^cm-testsuite-v[0-9]+\\.[0-9]+\\.[0-9]+(?:[-+].*)?$"
    } else {
        "^cm-testsuite-v[0-9]+\\.[0-9]+\\.[0-9]+(?:\\+.*)?$"
    };
    let output = Command::new(tool)
        .arg("--repository")
        .arg(root)
        .arg("--config")
        .arg(root.join("cliff.toml"))
        .args([
            "--offline",
            "--no-exec",
            "--unreleased",
            "--tag",
            &meta.tag,
            "--tag-pattern",
            pattern,
        ])
        .output()?;
    ensure!(
        output.status.success(),
        "git-cliff failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(String::from_utf8(output.stdout)?)
}

/// Verify the exact commit's version and notes, not the runner's current main tip.
pub fn validate_commit(root: &Path, meta: &Metadata, sha: &str) -> Result<()> {
    validate_revision(sha)?;
    let manifest = git(root, &["show", &format!("{sha}:rust/Cargo.toml")])?;
    let doc: toml::Value = toml::from_str(&manifest)?;
    ensure!(
        doc["workspace"]["package"]["version"].as_str() == Some(meta.version.as_str()),
        "release version differs from tagged workspace"
    );
    let lock: toml::Value =
        toml::from_str(&git(root, &["show", &format!("{sha}:rust/Cargo.lock")])?)?;
    for package in lock["package"]
        .as_array()
        .context("missing locked packages")?
    {
        if package.get("source").is_none() {
            ensure!(
                package["version"].as_str() == Some(meta.version.as_str()),
                "release lockfile has stale workspace versions"
            );
        }
    }
    release_notes(
        &git(root, &["show", &format!("{sha}:CHANGELOG.md")])?,
        &meta.version,
    )?;
    Ok(())
}

pub fn tag_target(root: &Path, meta: &Metadata, reference: &str) -> Result<String> {
    let sha = resolve(root, reference)?;
    validate_commit(root, meta, &sha)?;
    let status = Command::new("git")
        .arg("-C")
        .arg(root)
        .args([
            "merge-base",
            "--is-ancestor",
            &sha,
            "refs/remotes/origin/main",
        ])
        .status()?;
    ensure!(
        status.success(),
        "release commit must be reachable from origin/main; fetch main first"
    );
    Ok(sha)
}

#[derive(Debug)]
pub struct ReleaseHistory {
    pub commits: Vec<String>,
    pub ancestors: BTreeSet<String>,
}
/// Stable releases include changes from prereleases; prereleases use the previous
/// reachable release of either kind. All ranges end at the requested tag.
pub fn release_history(root: &Path, tag: &str) -> Result<ReleaseHistory> {
    let meta = Metadata::from_tag(tag)?;
    let target = version(&meta.version)?;
    let sha = resolve(root, &format!("refs/tags/{tag}"))?;
    validate_commit(root, &meta, &sha)?;
    let mut previous = None;
    for tag in git(root, &["tag", "--merged", &sha])?.lines() {
        let Some(text) = tag.strip_prefix(TAG_PREFIX) else {
            continue;
        };
        let Ok(v) = version(text) else {
            continue;
        };
        if !v.cmp_precedence(&target).is_lt() || (!meta.prerelease && !v.pre.is_empty()) {
            continue;
        }
        if previous
            .as_ref()
            .is_none_or(|(old, _): &(semver::Version, String)| old.cmp_precedence(&v).is_lt())
        {
            previous = Some((v, tag.to_owned()));
        }
    }
    let range = previous
        .map(|(_, tag)| format!("refs/tags/{tag}..{sha}"))
        .unwrap_or_else(|| sha.clone());
    let commits = git(root, &["log", "--format=%H%x00%s", &range])?
        .lines()
        .filter_map(|line| line.split_once('\0'))
        .filter(|(_, subject)| !subject.starts_with("release:"))
        .map(|(sha, _)| sha.to_owned())
        .collect();
    let ancestors = git(root, &["rev-list", &sha])?
        .lines()
        .map(str::to_owned)
        .collect();
    Ok(ReleaseHistory { commits, ancestors })
}

pub fn validate_checkout(root: &Path, tag: &str) -> Result<Metadata> {
    let meta = Metadata::from_tag(tag)?;
    let sha = resolve(root, &format!("refs/tags/{tag}"))?;
    ensure!(
        sha == resolve(root, "HEAD")?,
        "checkout must match the release tag exactly"
    );
    validate_commit(root, &meta, &sha)?;
    let notes = release_notes(
        &fs::read_to_string(root.join("CHANGELOG.md"))?,
        &meta.version,
    )?;
    ensure!(!notes.trim().is_empty(), "empty release notes");
    Ok(meta)
}
