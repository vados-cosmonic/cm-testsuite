//! Exact-commit release history and pinned changelog generation.

use crate::{
    CHANGELOG_PATH, LOCK_PATH, MANIFEST_PATH, Metadata, RELEASE_COMMIT_PREFIX, TAG_PREFIX,
    ensure_local_packages, manifest_version, release_notes, version,
};
use anyhow::{Result, ensure};
use cm_acquisition::{LocalGit, Repository};
use cm_catalog::validate_revision;
use std::{collections::BTreeSet, fs, path::Path, process::Command};

/// Remote-tracking ref that every release commit must be reachable from.
const MAIN_REF: &str = "refs/remotes/origin/main";

/// Run a read-only Git command in `root` with ambient configuration disabled,
/// returning its standard output.
pub fn git(root: &Path, args: &[&str]) -> Result<String> {
    let repository = LocalGit {
        root: root.to_owned(),
    };
    Ok(String::from_utf8(repository.git(args)?)?)
}

/// Resolve a branch, tag, or revision in `root` to a full commit identifier.
pub fn resolve(root: &Path, reference: &str) -> Result<String> {
    LocalGit {
        root: root.to_owned(),
    }
    .resolve(reference)
}

/// Read a tracked file as committed at `sha`, ignoring the working tree.
fn show(root: &Path, sha: &str, path: &str) -> Result<String> {
    git(root, &["show", &format!("{sha}:{path}")])
}

/// Generate the changelog section for `meta` from the commits since the
/// previous release tag, offline and without running repository code.
///
/// `cliff` must launch the pinned git-cliff ([`crate::CLIFF_VERSION`]); the
/// caller verifies the pin, and this function only appends arguments.
pub fn changelog(root: &Path, meta: &Metadata, mut cliff: Command) -> Result<String> {
    // The previous release is the newest tag matching this pattern. A stable
    // release matches only stable tags, so its notes span every prerelease
    // since the last stable release; a prerelease also matches prereleases.
    let suffix = if meta.prerelease { "[-+]" } else { r"\+" };
    let pattern = format!(r"^{TAG_PREFIX}[0-9]+\.[0-9]+\.[0-9]+(?:{suffix}.*)?$");
    let output = cliff
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
            &pattern,
        ])
        .output()?;
    ensure!(
        output.status.success(),
        "git-cliff failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(String::from_utf8(output.stdout)?)
}

/// Verify the version, lockfile, and notes committed at exactly `sha`, not the
/// working tree or the runner's current main tip.
pub fn validate_commit(root: &Path, meta: &Metadata, sha: &str) -> Result<()> {
    validate_revision(sha)?;
    ensure!(
        manifest_version(&show(root, sha, MANIFEST_PATH)?)?.to_string() == meta.version,
        "release version differs from tagged workspace"
    );
    ensure_local_packages(&show(root, sha, LOCK_PATH)?.parse()?, &meta.version)?;
    release_notes(&show(root, sha, CHANGELOG_PATH)?, &meta.version)?;
    Ok(())
}

/// Resolve `reference` to the commit a new release tag may point at: it must
/// carry the release's version and notes and be reachable from main.
pub fn tag_target(root: &Path, meta: &Metadata, reference: &str) -> Result<String> {
    let sha = resolve(root, reference)?;
    validate_commit(root, meta, &sha)?;
    ensure_on_main(root, &sha)?;
    Ok(sha)
}

/// Releases are cut only from main, whether tagged by automation or a maintainer.
fn ensure_on_main(root: &Path, sha: &str) -> Result<()> {
    // Git exits unsuccessfully both when `sha` is not an ancestor and when the
    // local origin/main is missing; fetching main is the remedy for either.
    ensure!(
        git(root, &["merge-base", "--is-ancestor", sha, MAIN_REF]).is_ok(),
        "release commit must be reachable from origin/main; fetch main first"
    );
    Ok(())
}

/// The commits one release introduced, used to find the PRs and issues to notify.
#[derive(Debug)]
pub struct ReleaseHistory {
    /// The release this range starts after; `None` for the first release.
    pub previous: Option<String>,
    /// Commits after `previous` up to the release tag, newest first, without
    /// release preparation commits.
    pub commits: Vec<String>,
    /// Every commit reachable from the release tag, to tell whether a PR's
    /// merge commit shipped in this release.
    pub ancestors: BTreeSet<String>,
}

/// Collect the history of the release at `tag`.
///
/// Stable releases include changes from prereleases; prereleases use the previous
/// reachable release of either kind. All ranges end at the requested tag.
pub fn release_history(root: &Path, tag: &str) -> Result<ReleaseHistory> {
    let meta = Metadata::from_tag(tag)?;
    let target = version(&meta.version)?;
    let sha = resolve(root, &format!("refs/tags/{tag}"))?;
    validate_commit(root, &meta, &sha)?;
    let reachable_tags = git(root, &["tag", "--merged", &sha])?;
    let previous = reachable_tags
        .lines()
        .filter_map(|candidate| {
            let release = version(candidate.strip_prefix(TAG_PREFIX)?).ok()?;
            Some((release, candidate))
        })
        // Only earlier releases count, and a stable release skips prerelease
        // tags so that its range covers the prereleases leading up to it.
        .filter(|(release, _)| {
            release.cmp_precedence(&target).is_lt() && (meta.prerelease || release.pre.is_empty())
        })
        .max_by(|(a, _), (b, _)| a.cmp_precedence(b))
        .map(|(_, tag)| tag.to_owned());
    let range = match &previous {
        Some(previous) => format!("refs/tags/{previous}..{sha}"),
        // The first release's range is the entire history.
        None => sha.clone(),
    };
    // One `<sha> NUL <subject>` record per line.
    let commits = git(root, &["log", "--format=%H%x00%s", &range])?
        .lines()
        .filter_map(|line| line.split_once('\0'))
        .filter(|(_, subject)| !subject.starts_with(RELEASE_COMMIT_PREFIX))
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

/// Check that the checkout at `root` is exactly the release tagged `tag`: HEAD
/// is the tagged commit, it is on main, and its committed and working-tree
/// version, lockfile, and release notes agree with the tag.
pub fn validate_checkout(root: &Path, tag: &str) -> Result<Metadata> {
    let meta = Metadata::from_tag(tag)?;
    let sha = resolve(root, &format!("refs/tags/{tag}"))?;
    ensure!(
        sha == resolve(root, "HEAD")?,
        "checkout must match the release tag exactly"
    );
    validate_commit(root, &meta, &sha)?;
    ensure_on_main(root, &sha)?;
    // The release notes are later extracted from the working tree, so it must
    // hold valid notes as well as the commit.
    release_notes(
        &fs::read_to_string(root.join(CHANGELOG_PATH))?,
        &meta.version,
    )?;
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
    // `rev-list --parents` prints the commit followed by its parents.
    let parents: Vec<String> = git(root, &["rev-list", "--parents", "-n", "1", merge_sha])?
        .split_whitespace()
        .skip(1)
        .map(str::to_owned)
        .collect();
    let landed_on_base = match parents.as_slice() {
        // A merge commit whose first parent is the base, or a squash commit
        // (or single rebased commit) sitting directly on it.
        [first, _] | [first] if first == base => true,
        // A rebase merge of several commits: the PR's commits replayed in a row.
        [_] if release_commits > 1 => rebased_onto(root, merge_sha, base, release_commits)?,
        _ => false,
    };
    ensure!(
        landed_on_base,
        "commits landed on main after the release PR was prepared; \
         rerun create-release-pr so the changelog covers them"
    );
    Ok(())
}

/// Whether the `release_commits` first-parent commits ending at `tip` are all
/// release commits and the commit just below them is `base`.
fn rebased_onto(root: &Path, tip: &str, base: &str, release_commits: usize) -> Result<bool> {
    let count = (release_commits + 1).to_string();
    // One `<sha> <subject>` line per commit, newest first: the release commits
    // followed by the commit they were replayed onto.
    let log = git(
        root,
        &["log", "--first-parent", "--format=%H %s", "-n", &count, tip],
    )?;
    let chain: Vec<(&str, &str)> = log
        .lines()
        .filter_map(|line| line.split_once(' '))
        .collect();
    let Some((&(below, _), replayed)) = chain.split_last() else {
        return Ok(false);
    };
    Ok(replayed.len() == release_commits
        && replayed
            .iter()
            .all(|(_, subject)| subject.starts_with(RELEASE_COMMIT_PREFIX))
        && below == base)
}
