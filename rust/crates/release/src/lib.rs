//! Repository release preparation and notifications, adapted from bytecodealliance/jco.
//! Network mutations are separate from planning and require an explicit caller action.
pub mod github;
pub mod history;

use anyhow::{Context, Result, ensure};
use cm_catalog::validate_revision;
use semver::Version;
use serde::Serialize;
use serde_json::Value;
use std::{fs, path::Path};
use toml_edit::{DocumentMut, value};

pub const PROJECT: &str = "cm-testsuite";
pub const TAG_PREFIX: &str = "cm-testsuite-v";
pub const CLIFF_VERSION: &str = "git-cliff 2.14.2";

pub fn version(text: &str) -> Result<Version> {
    let v = Version::parse(text).context("expected a SemVer version without a leading v")?;
    ensure!(v.to_string() == text, "version must use canonical SemVer");
    Ok(v)
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct Metadata {
    pub project: String,
    pub version: String,
    pub tag: String,
    pub prerelease: bool,
    pub prerelease_tag: String,
}
impl Metadata {
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
    pub fn from_tag(tag: &str) -> Result<Self> {
        Self::new(
            tag.strip_prefix(TAG_PREFIX)
                .context("not a cm-testsuite release tag")?,
        )
    }
    pub fn outputs(&self) -> String {
        format!(
            "proceed=true\nproject={}\nversion={}\ntag={}\nprerelease={}\nprerelease-tag={}\n",
            self.project, self.version, self.tag, self.prerelease, self.prerelease_tag
        )
    }
}

/// Only same-repository release preparation PRs merged into main may auto-tag.
#[derive(Debug)]
pub struct MergedRelease {
    pub meta: Metadata,
    pub merge_sha: String,
    pub number: u64,
}

pub fn merged_release(event: &Value, repository: &str) -> Result<Option<MergedRelease>> {
    cm_catalog::validate_source(&format!("github:{repository}"))?;
    let pr = &event["pull_request"];
    if pr["merged"] != true || !pr["title"].as_str().unwrap_or("").starts_with("release:") {
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
            .any(|l| l["name"] == "release-pr"),
        "release PR needs the release-pr label"
    );
    let text = pr["title"]
        .as_str()
        .unwrap()
        .strip_prefix("release: cm-testsuite v")
        .context("expected release: cm-testsuite vVERSION")?;
    let meta = Metadata::new(text)?;
    ensure!(
        pr["head"]["ref"] == format!("prep-release-{}", meta.tag),
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

pub fn workspace_version(root: &Path) -> Result<Version> {
    let doc: toml::Value = toml::from_str(&fs::read_to_string(root.join("rust/Cargo.toml"))?)?;
    version(
        doc.get("workspace")
            .and_then(|w| w.get("package"))
            .and_then(|p| p.get("version"))
            .and_then(toml::Value::as_str)
            .context("missing workspace version")?,
    )
}

/// Preserve TOML formatting and update only local workspace packages in the lockfile.
pub fn bumped_manifests(manifest: &str, lock: &str, next: &str) -> Result<(String, String)> {
    let mut doc = manifest.parse::<DocumentMut>()?;
    let current = version(
        doc.get("workspace")
            .and_then(|w| w.get("package"))
            .and_then(|p| p.get("version"))
            .and_then(toml_edit::Item::as_str)
            .context("missing workspace version")?,
    )?;
    let next_version = version(next)?;
    ensure!(
        current.cmp_precedence(&next_version).is_lt(),
        "next version must be after current version {current}"
    );
    let mut lock = lock.parse::<DocumentMut>()?;
    let packages = lock["package"]
        .as_array_of_tables_mut()
        .context("missing locked packages")?;
    let mut changed = 0;
    for package in packages.iter_mut() {
        if package.get("source").is_none() {
            ensure!(
                package.get("version").and_then(toml_edit::Item::as_str)
                    == Some(current.to_string().as_str()),
                "local package versions must match the workspace"
            );
            package["version"] = value(next);
            changed += 1;
        }
    }
    ensure!(changed > 0, "no workspace packages in lockfile");
    doc["workspace"]["package"]["version"] = value(next);
    Ok((doc.to_string(), lock.to_string()))
}

/// Extract exactly the requested latest release, including a final section at EOF.
pub fn release_notes(changelog: &str, expected: &str) -> Result<String> {
    version(expected)?;
    ensure!(
        changelog
            .lines()
            .next()
            .is_some_and(|s| s.eq_ignore_ascii_case("# Changelog")),
        "missing changelog heading"
    );
    let mut section = Vec::new();
    let mut found = false;
    for line in changelog.lines().skip(1) {
        if let Some(heading) = line.strip_prefix("## [") {
            if found {
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
            found = true;
        }
        if found {
            section.push(line);
        }
    }
    ensure!(found, "missing changelog release {expected}");
    Ok(section.join("\n").trim_end().to_owned() + "\n")
}

/// Prepend generated notes (which carry the heading) to the existing sections,
/// separated by a blank line.
fn combine_changelogs(generated: &str, previous: &str) -> Result<String> {
    let older = previous
        .strip_prefix("# Changelog\n")
        .context("missing changelog heading")?
        .trim();
    Ok(if older.is_empty() {
        format!("{}\n", generated.trim_end())
    } else {
        format!("{}\n\n{older}\n", generated.trim_end())
    })
}

pub fn prepare(root: &Path, next: &str) -> Result<Metadata> {
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
    let manifest_path = root.join("rust/Cargo.toml");
    let lock_path = root.join("rust/Cargo.lock");
    let (manifest, lock) = bumped_manifests(
        &fs::read_to_string(&manifest_path)?,
        &fs::read_to_string(&lock_path)?,
        next,
    )?;
    let generated = history::changelog(root, &meta)?;
    release_notes(&generated, next)?;
    let changelog_path = root.join("CHANGELOG.md");
    let previous = fs::read_to_string(&changelog_path)?;
    let combined = combine_changelogs(&generated, &previous)?;
    // Complete validation and changelog generation before changing tracked files.
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
        let lock = "version = 4\n[[package]]\nname='xtask'\nversion='0.1.0'\n[[package]]\nname='dependency'\nversion='0.1.0'\nsource='registry+https://example.test'\n";
        let (m, l) = bumped_manifests(manifest, lock, "0.2.0-rc.1").unwrap();
        assert!(m.starts_with("# keep"));
        let l: toml::Value = toml::from_str(&l).unwrap();
        assert_eq!(l["package"][0]["version"].as_str(), Some("0.2.0-rc.1"));
        assert_eq!(l["package"][1]["version"].as_str(), Some("0.1.0"));
        for next in ["0.1.0", "0.1.0+build", "0.0.9"] {
            assert!(bumped_manifests(manifest, lock, next).is_err());
        }
    }
    #[test]
    fn malformed_manifests_are_errors_not_panics() {
        let lock = "version = 4\n[[package]]\nname='xtask'\nversion='0.1.0'\n";
        assert!(bumped_manifests("[workspace]\n", lock, "0.2.0").is_err());
        let manifest = "[workspace.package]\nversion = \"0.1.0\"\n";
        assert!(
            bumped_manifests(
                manifest,
                "version = 4\n[[package]]\nname='xtask'\n",
                "0.2.0"
            )
            .is_err()
        );
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir_all(dir.path().join("rust")).unwrap();
        fs::write(dir.path().join("rust/Cargo.toml"), "[workspace]\n").unwrap();
        assert!(workspace_version(dir.path()).is_err());
    }
    #[test]
    fn changelog_sections_are_separated_by_a_blank_line() {
        let generated = "# Changelog\n\n## [0.2.0] - 2026-11-06\n\n- new\n";
        assert_eq!(
            combine_changelogs(generated, "# Changelog\n").unwrap(),
            generated
        );
        assert_eq!(
            combine_changelogs(
                generated,
                "# Changelog\n\n## [0.1.0] - 2026-10-01\n\n- old\n"
            )
            .unwrap(),
            "# Changelog\n\n## [0.2.0] - 2026-11-06\n\n- new\n\n## [0.1.0] - 2026-10-01\n\n- old\n"
        );
        assert!(combine_changelogs(generated, "no heading\n").is_err());
    }
    #[test]
    fn changelog_checks_version_and_handles_eof() {
        let text = "# Changelog\n\n## [Unreleased]\n\n## [1.2.0-rc.1] - 2026-11-06\n\n### Fixes\n- fixed\n";
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
    fn merged_release_requires_trusted_metadata_and_uses_merge_sha() {
        let mut event = json!({"pull_request":{"merged":true,"title":"release: cm-testsuite v0.2.0","labels":[{"name":"release-pr"}],"head":{"ref":"prep-release-cm-testsuite-v0.2.0","repo":{"full_name":"example/tests"}},"base":{"ref":"main","repo":{"full_name":"example/tests"}},"merge_commit_sha":"a".repeat(40),"number":12}});
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
