//! Turn an "Enroll a repository" issue into its registry entry and pull request.
use crate::exchange::{Access, Credentials, github_name};
use crate::io::write;
use anyhow::{Context, Result};
use cm_catalog::{PARTICIPANTS_PATH, Participants};
use cm_github::Api;
use std::{fs, path::Path};

#[derive(clap::Args)]
pub struct Args {
    /// Number of the "Enroll a repository" issue to act on.
    #[arg(long, value_name = "NUMBER")]
    pub(crate) issue: u64,
    /// Add the entry to the registry in the checkout at --root.
    #[arg(long, conflicts_with = "publish")]
    pub(crate) apply: bool,
    /// Open or update the draft pull request that enrolls the repository and closes the issue.
    #[arg(long)]
    pub(crate) publish: bool,
    /// Catalog repository holding the issue and the registry; defaults to GITHUB_REPOSITORY.
    #[arg(long)]
    pub(crate) repository: Option<String>,
    #[command(flatten)]
    pub(crate) credentials: Credentials,
}

/// What to do with a reviewed enrollment.
#[derive(Clone, Copy)]
enum Mode {
    DryRun,
    Apply,
    Publish,
}

/// Review the issue, act on it, and describe the outcome.
///
/// A dry run and `Apply` edit the registry under `root`. `Publish` leaves the
/// checkout alone and edits the registry on the catalog's default branch.
fn enroll(api: &dyn Api, root: &Path, catalog: &str, issue: u64, mode: Mode) -> Result<String> {
    let enrollment = cm_delivery::review_enrollment(api, catalog, issue)?;
    let mut report = format!(
        "issue #{issue}: enroll {}\n",
        enrollment.participant.repository
    );
    for check in &enrollment.checks {
        let status = if check.passed { "ok" } else { "warning" };
        report.push_str(&format!("{status}: {}\n", check.summary));
    }
    if let Mode::Publish = mode {
        let url = cm_delivery::publish_enrollment(api, catalog, &enrollment)
            .with_context(|| format!("publishing the enrollment to {catalog}"))?;
        return Ok(format!("{report}pull request: {url}\n"));
    }

    let path = root.join(PARTICIPANTS_PATH);
    let current = fs::read_to_string(&path)
        .with_context(|| format!("participant registry {PARTICIPANTS_PATH}"))?;
    let updated = Participants::enroll(&current, &enrollment.participant)
        .with_context(|| format!("updating {PARTICIPANTS_PATH}"))?;
    // Enrolling appends, so the new entry is what follows the current text.
    let entry = updated.strip_prefix(current.as_str()).unwrap_or(&updated);
    report.push_str(&format!("{}\n", entry.trim()));
    if let Mode::Apply = mode {
        write(&path, &updated)?;
        report.push_str(&format!("added to {PARTICIPANTS_PATH}\n"));
    } else {
        report.push_str("dry run; pass --apply or --publish\n");
    }
    Ok(report)
}

/// Turn an "Enroll a repository" issue into its registry entry and pull request.
///
/// Without --apply or --publish this only reports what would change.
pub fn run(root: &Path, args: Args) -> Result<()> {
    let Args {
        issue,
        apply,
        publish,
        repository,
        credentials,
    } = args;

    let catalog = repository
        .or_else(|| std::env::var("GITHUB_REPOSITORY").ok())
        .map(|name| github_name(&name))
        .context("enroll needs --repository or GITHUB_REPOSITORY to name the catalog repository")?;
    let access = Access::new(&credentials, publish)?;
    let mode = match (apply, publish) {
        (true, _) => Mode::Apply,
        (_, true) => Mode::Publish,
        _ => Mode::DryRun,
    };
    print!("{}", enroll(&access.api(), root, &catalog, issue, mode)?);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use cm_test_support::github::FakeGitHubs;

    const CATALOG: &str = "github:example/catalog";
    const REGISTRY: &str = "# Enrolled repositories.\nschema = 1\n";
    const ENTRY: &str =
        "[[participant]]\nrepository = \"github:example/project\"\nmaintainers = [\"octo-cat\"]\n";
    const FORM: &str = "### Repository\n\nexample/project\n\n### Roles\n\n\
        - [x] Submit tests (the manifest declares `[[test]]` entries)\n\n\
        ### Maintainers\n\nocto-cat\n\n### Manifest path\n\n_No response_\n\n\
        ### Branch to read tests from\n\n_No response_\n\n### Setup\n\n\
        - [x] The manifest is committed on the branch named above.\n\
        - [x] I maintain this repository, or its maintainers have agreed to this request.\n\n\
        ### Anything else\n\n_No response_\n";

    /// A catalog with an enrollment issue for a project sharing one test,
    /// and a checkout of the catalog's registry.
    fn fixture() -> (FakeGitHubs, tempfile::TempDir) {
        let hub = FakeGitHubs::new(&["example/catalog", "example/project"]);
        hub.push("example/catalog", &[(PARTICIPANTS_PATH, REGISTRY)]);
        let manifest = "schema=1\n[[test]]\nname='smoke'\npath='file:///smoke.wast'\n\
                        requires=['component-model']\ncovers=['smoke']\n";
        hub.push("example/project", &[(".cm-testsuite.toml", manifest)]);
        hub.repository("example/catalog")
            .add_issue(3, "[Enroll] example/project", FORM);
        let root = tempfile::tempdir().unwrap();
        write(&root.path().join(PARTICIPANTS_PATH), REGISTRY).unwrap();
        (hub, root)
    }

    #[test]
    fn reports_by_default_and_writes_only_where_asked() {
        let (hub, root) = fixture();
        let registry = || fs::read_to_string(root.path().join(PARTICIPANTS_PATH)).unwrap();

        let report = enroll(&hub, root.path(), CATALOG, 3, Mode::DryRun).unwrap();
        assert!(report.starts_with("issue #3: enroll github:example/project\nok: "));
        assert!(report.contains(ENTRY), "{report}");
        assert!(report.ends_with("dry run; pass --apply or --publish\n"));
        assert_eq!(registry(), REGISTRY);
        assert_eq!(hub.writes(), 0);

        // Publishing proposes on GitHub and leaves the checkout alone.
        let report = enroll(&hub, root.path(), CATALOG, 3, Mode::Publish).unwrap();
        assert!(report.ends_with("pull request: https://example.invalid/pull/1\n"));
        assert_eq!(registry(), REGISTRY);

        // Applying edits the checkout only, and a second run finds it enrolled.
        let writes = hub.writes();
        enroll(&hub, root.path(), CATALOG, 3, Mode::Apply).unwrap();
        assert_eq!(registry(), format!("{REGISTRY}\n{ENTRY}"));
        assert_eq!(hub.writes(), writes);
        for mode in [Mode::DryRun, Mode::Apply] {
            let error = enroll(&hub, root.path(), CATALOG, 3, mode).unwrap_err();
            assert!(
                format!("{error:#}").contains("already enrolled"),
                "{error:#}"
            );
        }
        assert_eq!(registry(), format!("{REGISTRY}\n{ENTRY}"));
    }
}
