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
    let tool = root.join("bins/git-cliff");
    let output = Command::new("dotslash")
        .arg(&tool)
        .arg("--version")
        .output()
        .context("install DotSlash before preparing a release")?;
    ensure!(
        output.status.success() && String::from_utf8_lossy(&output.stdout).trim() == CLIFF_VERSION,
        "expected pinned {CLIFF_VERSION}"
    );
    let pattern = if meta.prerelease {
        "^cm-testsuite-v[0-9]+\\.[0-9]+\\.[0-9]+(?:[-+].*)?$"
    } else {
        "^cm-testsuite-v[0-9]+\\.[0-9]+\\.[0-9]+(?:\\+.*)?$"
    };
    let output = Command::new("dotslash")
        .arg(tool)
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
        doc.get("workspace")
            .and_then(|w| w.get("package"))
            .and_then(|p| p.get("version"))
            .and_then(toml::Value::as_str)
            == Some(meta.version.as_str()),
        "release version differs from tagged workspace"
    );
    let lock: toml::Value =
        toml::from_str(&git(root, &["show", &format!("{sha}:rust/Cargo.lock")])?)?;
    for package in lock
        .get("package")
        .and_then(toml::Value::as_array)
        .context("missing locked packages")?
    {
        if package.get("source").is_none() {
            ensure!(
                package.get("version").and_then(toml::Value::as_str) == Some(meta.version.as_str()),
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
    ensure_on_main(root, &sha)?;
    Ok(sha)
}

/// Releases are cut only from main, whether tagged by automation or a maintainer.
fn ensure_on_main(root: &Path, sha: &str) -> Result<()> {
    let status = Command::new("git")
        .arg("-C")
        .arg(root)
        .args([
            "merge-base",
            "--is-ancestor",
            sha,
            "refs/remotes/origin/main",
        ])
        .status()?;
    ensure!(
        status.success(),
        "release commit must be reachable from origin/main; fetch main first"
    );
    Ok(())
}

#[derive(Debug)]
pub struct ReleaseHistory {
    /// The release this range starts after; `None` for the first release.
    pub previous: Option<String>,
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
    let previous = previous.map(|(_, tag)| tag);
    let range = previous
        .as_ref()
        .map(|tag| format!("refs/tags/{tag}..{sha}"))
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
    Ok(ReleaseHistory {
        previous,
        commits,
        ancestors,
    })
}

pub fn validate_checkout(root: &Path, tag: &str) -> Result<Metadata> {
    let meta = Metadata::from_tag(tag)?;
    let sha = resolve(root, &format!("refs/tags/{tag}"))?;
    ensure!(
        sha == resolve(root, "HEAD")?,
        "checkout must match the release tag exactly"
    );
    validate_commit(root, &meta, &sha)?;
    ensure_on_main(root, &sha)?;
    let notes = release_notes(
        &fs::read_to_string(root.join("CHANGELOG.md"))?,
        &meta.version,
    )?;
    ensure!(!notes.trim().is_empty(), "empty release notes");
    Ok(meta)
}

/// Refuse a release whose merge also brought in commits that landed on main
/// after the release PR was prepared: its changelog would omit them, and the
/// next release starts after this tag, so no changelog would ever list them.
///
/// `base` is the main commit the PR's `release_commits` were prepared on.
/// Merge commits must have `base` as their first parent; squash merges must sit
/// directly on `base`; rebase merges must be `release_commits` release commits
/// directly on `base`.
pub fn ensure_merged_onto(
    root: &Path,
    merge_sha: &str,
    base: &str,
    release_commits: usize,
) -> Result<()> {
    validate_revision(merge_sha)?;
    validate_revision(base)?;
    ensure!(release_commits > 0, "release PR has no commits");
    let parents: Vec<String> = git(root, &["rev-list", "--parents", "-n", "1", merge_sha])?
        .split_whitespace()
        .skip(1)
        .map(str::to_owned)
        .collect();
    let exact = match parents.as_slice() {
        [first, _] | [first] if first == base => true,
        [_] if release_commits > 1 => {
            let count = (release_commits + 1).to_string();
            let chain = git(
                root,
                &[
                    "log",
                    "--first-parent",
                    "--format=%H %s",
                    "-n",
                    &count,
                    merge_sha,
                ],
            )?;
            let chain: Vec<_> = chain.lines().collect();
            chain.len() == release_commits + 1
                && chain[..release_commits].iter().all(|line| {
                    line.split_once(' ')
                        .is_some_and(|(_, s)| s.starts_with("release:"))
                })
                && chain[release_commits].split_once(' ').map(|(sha, _)| sha) == Some(base)
        }
        _ => false,
    };
    ensure!(
        exact,
        "commits landed on main after the release PR was prepared; \
         rerun create-release-pr so the changelog covers them"
    );
    Ok(())
}
