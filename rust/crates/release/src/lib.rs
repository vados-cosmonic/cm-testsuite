//! Repository release preparation and notifications, adapted from bytecodealliance/jco.
//! Network mutations are separate from planning and require an explicit caller action.

pub mod github;
pub mod history;

use anyhow::{Context, Result, ensure};
use cm_catalog::validate_revision;
use semver::Version;
use serde::Serialize;
use serde_json::Value;
use std::{fs, path::Path, process::Command};
use toml_edit::{DocumentMut, Item, value};

/// Project name reported in workflow outputs.
pub const PROJECT: &str = "cm-testsuite";

/// Prefix of every release tag; the canonical SemVer version follows it.
/// Keep in sync with `cliff.toml` and the tag filter in `release.yml`.
pub const TAG_PREFIX: &str = "cm-testsuite-v";

/// Exact `--version` output of the git-cliff release pinned by `bins/git-cliff`.
pub const CLIFF_VERSION: &str = "git-cliff 2.14.2";

/// Conventional Commit type that starts release commit subjects and PR titles.
/// Keep in sync with the skip rule in `cliff.toml` and the job filter in
/// `tag-release.yml`.
pub const RELEASE_COMMIT_PREFIX: &str = "release:";

/// Release PR title up to its version, as written by `create-release-pr.yml`.
pub const RELEASE_TITLE_PREFIX: &str = "release: cm-testsuite v";

/// Release PR branch name up to its tag, as written by `create-release-pr.yml`.
pub const RELEASE_BRANCH_PREFIX: &str = "prep-release-";

/// Label that `create-release-pr.yml` applies to release preparation PRs.
pub const RELEASE_LABEL: &str = "release-pr";

/// Workspace manifest that carries the single shared version.
pub const MANIFEST_PATH: &str = "rust/Cargo.toml";

/// Lockfile whose workspace package entries repeat the shared version.
pub const LOCK_PATH: &str = "rust/Cargo.lock";

/// Changelog whose latest section becomes the release notes.
pub const CHANGELOG_PATH: &str = "CHANGELOG.md";

/// First line of the changelog, written by the `cliff.toml` header.
const CHANGELOG_HEADING: &str = "# Changelog";

/// Parse a canonical SemVer version, rejecting a leading `v`, leading zeros,
/// and anything else that would not round-trip to the same text.
pub fn version(text: &str) -> Result<Version> {
    let v = Version::parse(text).context("expected a SemVer version without a leading v")?;
    ensure!(v.to_string() == text, "version must use canonical SemVer");
    Ok(v)
}

/// The catalog source identifier of a GitHub `owner/repository`.
pub(crate) fn source(repository: &str) -> String {
    format!("github:{repository}")
}

/// Validated identity of one release, derived only from its version.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct Metadata {
    /// Always [`PROJECT`].
    pub project: String,
    /// Canonical SemVer version, without a leading `v`.
    pub version: String,
    /// Git tag: [`TAG_PREFIX`] followed by the version.
    pub tag: String,
    /// Whether the version has a prerelease component.
    pub prerelease: bool,
    /// Prerelease channel, such as `rc` for `1.2.3-rc.1`; empty for stable versions.
    pub prerelease_tag: String,
}

impl Metadata {
    /// Validate a version and derive its release identity.
    pub fn new(text: &str) -> Result<Self> {
        let v = version(text)?;
        Ok(Self {
            project: PROJECT.into(),
            version: text.into(),
            tag: format!("{TAG_PREFIX}{text}"),
            prerelease: !v.pre.is_empty(),
            prerelease_tag: v.pre.as_str().split('.').next().unwrap_or("").into(),
        })
    }

    /// Derive the release identity from an existing release tag.
    pub fn from_tag(tag: &str) -> Result<Self> {
        Self::new(
            tag.strip_prefix(TAG_PREFIX)
                .context("not a cm-testsuite release tag")?,
        )
    }

    /// Render the fields as `key=value` lines in GitHub Actions output syntax.
    pub fn outputs(&self) -> String {
        format!(
            "project={}\nversion={}\ntag={}\nprerelease={}\nprerelease-tag={}\n",
            self.project, self.version, self.tag, self.prerelease, self.prerelease_tag
        )
    }
}

/// A merged release preparation PR that automation may tag.
#[derive(Debug)]
pub struct MergedRelease {
    /// Release identity taken from the PR title.
    pub meta: Metadata,
    /// Commit that the merge produced on main; the commit to tag.
    pub merge_sha: String,
    /// Pull request number.
    pub number: u64,
}

/// Read a `pull_request` closed event. Only same-repository release preparation
/// PRs merged into main may auto-tag.
///
/// Returns `None` for PRs that are unmerged or not titled as a release, and an
/// error for a release-titled PR that lacks the expected branch, label, or base.
pub fn merged_release(event: &Value, repository: &str) -> Result<Option<MergedRelease>> {
    cm_catalog::validate_source(&source(repository))?;
    let pr = &event["pull_request"];
    let title = pr["title"].as_str().unwrap_or("");
    if pr["merged"] != true || !title.starts_with(RELEASE_COMMIT_PREFIX) {
        return Ok(None);
    }
    ensure!(pr["base"]["ref"] == "main", "release PR must target main");
    ensure!(
        pr["head"]["repo"]["full_name"] == repository
            && pr["base"]["repo"]["full_name"] == repository,
        "release PR must originate in this repository"
    );
    ensure!(
        pr["labels"]
            .as_array()
            .context("missing PR labels")?
            .iter()
            .any(|l| l["name"] == RELEASE_LABEL),
        "release PR needs the {RELEASE_LABEL} label"
    );
    let text = title
        .strip_prefix(RELEASE_TITLE_PREFIX)
        .with_context(|| format!("expected {RELEASE_TITLE_PREFIX}VERSION"))?;
    let meta = Metadata::new(text)?;
    ensure!(
        pr["head"]["ref"] == format!("{RELEASE_BRANCH_PREFIX}{}", meta.tag),
        "unexpected release branch"
    );
    let sha = pr["merge_commit_sha"]
        .as_str()
        .context("missing release merge commit")?;
    validate_revision(sha)?;
    Ok(Some(MergedRelease {
        meta,
        merge_sha: sha.into(),
        number: pr["number"].as_u64().context("missing release PR number")?,
    }))
}

/// Read `workspace.package.version` from a parsed workspace manifest.
fn document_version(manifest: &DocumentMut) -> Result<Version> {
    version(
        manifest
            .get("workspace")
            .and_then(|w| w.get("package"))
            .and_then(|p| p.get("version"))
            .and_then(Item::as_str)
            .context("missing workspace version")?,
    )
}

/// Read the shared workspace version from the text of the workspace manifest.
pub fn manifest_version(manifest: &str) -> Result<Version> {
    document_version(&manifest.parse()?)
}

/// Read the shared workspace version from the checkout at `root`.
pub fn workspace_version(root: &Path) -> Result<Version> {
    manifest_version(&fs::read_to_string(root.join(MANIFEST_PATH))?)
}

/// Lockfile packages without a `source` are workspace members rather than
/// registry or Git dependencies.
fn is_local_package(package: &toml_edit::Table) -> bool {
    !package.contains_key("source")
}

/// Require at least one workspace package in the lockfile, all at `expected`.
pub fn ensure_local_packages(lock: &DocumentMut, expected: &str) -> Result<()> {
    let mut local = lock
        .get("package")
        .and_then(Item::as_array_of_tables)
        .context("missing locked packages")?
        .iter()
        .filter(|package| is_local_package(package))
        .peekable();
    ensure!(local.peek().is_some(), "no workspace packages in lockfile");
    ensure!(
        local.all(|package| package.get("version").and_then(Item::as_str) == Some(expected)),
        "lockfile workspace packages must all be at version {expected}"
    );
    Ok(())
}

/// Set the workspace version to `next` in the manifest and in every workspace
/// package of the lockfile, preserving TOML formatting and registry packages.
///
/// `allow_current` re-prepares the current, untagged version, for example after
/// tag-release refused its merge; both texts are then returned unchanged.
pub fn bumped_manifests(
    manifest: &str,
    lock: &str,
    next: &str,
    allow_current: bool,
) -> Result<(String, String)> {
    let mut manifest_doc = manifest.parse::<DocumentMut>()?;
    let mut lock_doc = lock.parse::<DocumentMut>()?;
    let current = document_version(&manifest_doc)?;
    let next_version = version(next)?;
    let reprepare = current == next_version;
    ensure!(
        !reprepare || allow_current,
        "version {current} is already current; pass --allow-current-version to re-prepare it"
    );
    ensure!(
        reprepare || current.cmp_precedence(&next_version).is_lt(),
        "next version must be after current version {current}"
    );
    ensure_local_packages(&lock_doc, &current.to_string())?;
    if reprepare {
        return Ok((manifest.to_owned(), lock.to_owned()));
    }
    manifest_doc["workspace"]["package"]["version"] = value(next);
    let packages = lock_doc["package"]
        .as_array_of_tables_mut()
        .context("missing locked packages")?;
    for package in packages.iter_mut().filter(|p| is_local_package(p)) {
        package["version"] = value(next);
    }
    Ok((manifest_doc.to_string(), lock_doc.to_string()))
}

/// Return the changelog after its `# Changelog` heading line. Every reader of
/// the changelog shares this rule, so a file that validates can also be edited.
fn changelog_sections(changelog: &str) -> Result<&str> {
    let (heading, sections) = changelog.split_once('\n').unwrap_or((changelog, ""));
    ensure!(
        heading.trim_end() == CHANGELOG_HEADING,
        "missing changelog heading"
    );
    Ok(sections)
}

/// Extract the latest release section, heading included, requiring it to be
/// the `expected` version and to list at least one change. The section ends at
/// the next release heading or at the end of the file.
pub fn release_notes(changelog: &str, expected: &str) -> Result<String> {
    version(expected)?;
    let mut section = Vec::new();
    for line in changelog_sections(changelog)?.lines() {
        if let Some(heading) = line.strip_prefix("## [") {
            if !section.is_empty() {
                break;
            }
            let release = heading
                .split_once(']')
                .context("invalid changelog section")?
                .0;
            if release.eq_ignore_ascii_case("unreleased") {
                continue;
            }
            ensure!(
                release == expected,
                "latest changelog version {release} does not match {expected}"
            );
        } else if section.is_empty() {
            // Nothing before the first release heading belongs to a release.
            continue;
        }
        section.push(line);
    }
    ensure!(!section.is_empty(), "missing changelog release {expected}");
    // The first line is the heading itself; a release needs notes below it.
    ensure!(
        section[1..].iter().any(|line| !line.trim().is_empty()),
        "changelog release {expected} has no notes"
    );
    Ok(section.join("\n").trim_end().to_owned() + "\n")
}

/// Prepend generated notes (which carry the changelog heading) to the existing
/// release sections, separated by a blank line.
///
/// `replace` names a version being re-prepared: its stale section is dropped
/// from the existing sections so the generated notes take its place.
fn combine_changelogs(generated: &str, previous: &str, replace: Option<&str>) -> Result<String> {
    let mut older = changelog_sections(previous)?.trim().to_owned();
    if let Some(version) = replace {
        // Called only for its checks: the section to replace must exist and be
        // the latest one, so an already released section is never rewritten.
        release_notes(previous, version)?;
        let heading = format!("## [{version}]");
        let mut skipping = false;
        let mut kept = Vec::new();
        for line in older.lines() {
            if line.starts_with(&heading) {
                skipping = true;
            } else if line.starts_with("## [") {
                skipping = false;
            }
            if !skipping {
                kept.push(line);
            }
        }
        older = kept.join("\n").trim().to_owned();
    }
    Ok(if older.is_empty() {
        format!("{}\n", generated.trim_end())
    } else {
        format!("{}\n\n{older}\n", generated.trim_end())
    })
}

/// Bump the workspace version and prepend generated release notes in the clean
/// checkout at `root`, returning the prepared release's identity.
///
/// `cliff` launches the pinned git-cliff; see [`history::changelog`]. Nothing
/// is written until every check and the changelog generation have succeeded.
pub fn prepare(root: &Path, next: &str, allow_current: bool, cliff: Command) -> Result<Metadata> {
    let meta = Metadata::new(next)?;
    ensure!(
        history::git(root, &["status", "--porcelain", "--untracked-files=normal"])?
            .trim()
            .is_empty(),
        "release preparation requires a clean checkout"
    );
    ensure!(
        history::git(root, &["tag", "--list", &meta.tag])?
            .trim()
            .is_empty(),
        "release tag already exists"
    );
    let manifest_path = root.join(MANIFEST_PATH);
    let lock_path = root.join(LOCK_PATH);
    let changelog_path = root.join(CHANGELOG_PATH);
    let manifest = fs::read_to_string(&manifest_path)?;
    let reprepare = manifest_version(&manifest)?.to_string() == next;
    let (manifest, lock) = bumped_manifests(
        &manifest,
        &fs::read_to_string(&lock_path)?,
        next,
        allow_current,
    )?;
    let generated = history::changelog(root, &meta, cliff)?;
    release_notes(&generated, next)?;
    let previous = fs::read_to_string(&changelog_path)?;
    let combined = combine_changelogs(&generated, &previous, reprepare.then_some(next))?;
    fs::write(manifest_path, manifest)?;
    fs::write(lock_path, lock)?;
    fs::write(changelog_path, combined)?;
    Ok(meta)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn strict_versions_and_prereleases() {
        for bad in ["v1.2.3", "01.2.3", "1.2", "1.2.3-01", "1.2.3\nother=bad"] {
            assert!(Metadata::new(bad).is_err());
        }
        let m = Metadata::new("1.2.3-rc.1+build.4").unwrap();
        assert!(m.prerelease);
        assert_eq!(m.prerelease_tag, "rc");
        assert_eq!(Metadata::from_tag(&m.tag).unwrap(), m);
        assert!(
            version("1.2.3-rc.1")
                .unwrap()
                .cmp_precedence(&version("1.2.3").unwrap())
                .is_lt()
        );
    }

    #[test]
    fn workspace_bump_preserves_comments_and_registry_versions() {
        let manifest = "# keep\n[workspace.package]\nversion = \"0.1.0\"\n";
        let lock = "\
version = 4
[[package]]
name='xtask'
version='0.1.0'
[[package]]
name='dependency'
version='0.1.0'
source='registry+https://example.test'
";
        let (m, l) = bumped_manifests(manifest, lock, "0.2.0-rc.1", false).unwrap();
        assert!(m.starts_with("# keep"));
        let l: DocumentMut = l.parse().unwrap();
        assert_eq!(l["package"][0]["version"].as_str(), Some("0.2.0-rc.1"));
        assert_eq!(l["package"][1]["version"].as_str(), Some("0.1.0"));
        for next in ["0.1.0", "0.1.0+build", "0.0.9"] {
            assert!(bumped_manifests(manifest, lock, next, false).is_err());
        }
    }

    #[test]
    fn current_version_is_reprepared_only_when_allowed() {
        let manifest = "# keep\n[workspace.package]\nversion  =  \"0.2.0\"\n";
        let lock = "version = 4\n[[package]]\nname='xtask'\nversion='0.2.0'\n";
        let err = bumped_manifests(manifest, lock, "0.2.0", false).unwrap_err();
        assert!(err.to_string().contains("--allow-current-version"));
        assert_eq!(
            bumped_manifests(manifest, lock, "0.2.0", true).unwrap(),
            (manifest.to_owned(), lock.to_owned())
        );
        // The option permits the current version, never an older one.
        assert!(bumped_manifests(manifest, lock, "0.1.0", true).is_err());
        assert!(bumped_manifests(manifest, lock, "0.3.0", true).is_ok());
    }

    #[test]
    fn reprepared_changelog_replaces_the_stale_section() {
        let generated = "# Changelog\n\n## [0.2.0] - 2026-11-07\n\n- early\n- late\n";
        let stale = "\
# Changelog

## [0.2.0] - 2026-11-06

- early

## [0.1.0] - 2026-10-01

- old
";
        assert_eq!(
            combine_changelogs(generated, stale, Some("0.2.0")).unwrap(),
            "\
# Changelog

## [0.2.0] - 2026-11-07

- early
- late

## [0.1.0] - 2026-10-01

- old
"
        );
        // Only the latest section may be replaced.
        assert!(combine_changelogs(generated, stale, Some("0.1.0")).is_err());
    }

    #[test]
    fn malformed_manifests_are_errors_not_panics() {
        let lock = "version = 4\n[[package]]\nname='xtask'\nversion='0.1.0'\n";
        assert!(bumped_manifests("[workspace]\n", lock, "0.2.0", false).is_err());
        let manifest = "[workspace.package]\nversion = \"0.1.0\"\n";
        for bad_lock in [
            "version = 4\n",
            "version = 4\n[[package]]\nname='xtask'\n",
            "version = 4\n[[package]]\nname='dep'\nversion='0.1.0'\nsource='registry'\n",
        ] {
            assert!(bumped_manifests(manifest, bad_lock, "0.2.0", false).is_err());
        }
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir_all(dir.path().join("rust")).unwrap();
        fs::write(dir.path().join(MANIFEST_PATH), "[workspace]\n").unwrap();
        assert!(workspace_version(dir.path()).is_err());
    }

    #[test]
    fn changelog_sections_are_separated_by_a_blank_line() {
        let generated = "# Changelog\n\n## [0.2.0] - 2026-11-06\n\n- new\n";
        assert_eq!(
            combine_changelogs(generated, "# Changelog\n", None).unwrap(),
            generated
        );
        assert_eq!(
            combine_changelogs(
                generated,
                "# Changelog\n\n## [0.1.0] - 2026-10-01\n\n- old\n",
                None
            )
            .unwrap(),
            "# Changelog\n\n## [0.2.0] - 2026-11-06\n\n- new\n\n## [0.1.0] - 2026-10-01\n\n- old\n"
        );
        assert!(combine_changelogs(generated, "no heading\n", None).is_err());
    }

    #[test]
    fn changelog_checks_version_and_handles_eof() {
        let text = "\
# Changelog

## [Unreleased]

## [1.2.0-rc.1] - 2026-11-06

### Fixes
- fixed
";
        assert!(
            release_notes(text, "1.2.0-rc.1")
                .unwrap()
                .ends_with("- fixed\n")
        );
        assert!(release_notes(text, "1.1.0").is_err());
        let two = text.to_owned() + "\n## [1.1.0] - 2026-10-01\nold\n";
        assert!(!release_notes(&two, "1.2.0-rc.1").unwrap().contains("old"));
        assert!(release_notes("# Changelog\n", "1.0.0").is_err());
    }

    #[test]
    fn changelog_heading_rule_is_shared_by_reading_and_editing() {
        let generated = "# Changelog\n\n## [0.2.0] - 2026-11-06\n\n- new\n";
        // CRLF endings validate and can be edited alike.
        let crlf = "# Changelog\r\n\r\n## [0.1.0] - 2026-10-01\r\n\r\n- old\r\n";
        assert!(release_notes(crlf, "0.1.0").is_ok());
        assert!(combine_changelogs(generated, crlf, None).is_ok());
        // A differently cased heading is refused by both.
        let lower = "# changelog\n\n## [0.1.0] - 2026-10-01\n\n- old\n";
        assert!(release_notes(lower, "0.1.0").is_err());
        assert!(combine_changelogs(generated, lower, None).is_err());
    }

    #[test]
    fn release_sections_without_notes_are_rejected() {
        for empty in [
            "# Changelog\n\n## [0.2.0] - 2026-11-06\n",
            "# Changelog\n\n## [0.2.0] - 2026-11-06\n\n  \n",
            "# Changelog\n\n## [0.2.0] - 2026-11-06\n\n## [0.1.0] - 2026-10-01\n\n- old\n",
        ] {
            let error = release_notes(empty, "0.2.0").unwrap_err();
            assert!(error.to_string().contains("has no notes"), "{error}");
        }
    }

    #[test]
    fn merged_release_requires_trusted_metadata_and_uses_merge_sha() {
        let mut event = json!({"pull_request": {
            "merged": true,
            "title": "release: cm-testsuite v0.2.0",
            "labels": [{"name": "release-pr"}],
            "head": {
                "ref": "prep-release-cm-testsuite-v0.2.0",
                "repo": {"full_name": "example/tests"},
            },
            "base": {"ref": "main", "repo": {"full_name": "example/tests"}},
            "merge_commit_sha": "a".repeat(40),
            "number": 12,
        }});
        assert_eq!(
            merged_release(&event, "example/tests")
                .unwrap()
                .unwrap()
                .merge_sha,
            "a".repeat(40)
        );
        event["pull_request"]["head"]["repo"]["full_name"] = json!("fork/tests");
        assert!(merged_release(&event, "example/tests").is_err());
        event["pull_request"]["merged"] = json!(false);
        assert!(merged_release(&event, "example/tests").unwrap().is_none());
    }
}
