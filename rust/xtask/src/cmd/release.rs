//! Typed adapters for release preparation, tagging, and notifications.

use crate::tools;
use anyhow::{Result, ensure};
use clap::Subcommand;
use cm_release::{CHANGELOG_PATH, CLIFF_VERSION, Metadata, github, history};
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
};

#[derive(clap::Args)]
pub struct Args {
    #[command(subcommand)]
    command: Release,
}

#[derive(Subcommand)]
pub enum Release {
    /// Validate a version and emit GitHub Actions outputs, without mutation.
    Metadata {
        /// SemVer version without a leading v, such as 0.2.0 or 0.2.0-rc.1.
        #[arg(long)]
        version: String,

        /// File to append the outputs to; GitHub Actions provides $GITHUB_OUTPUT.
        #[arg(long)]
        github_output: Option<PathBuf>,
    },

    /// Validate an automatic merged-PR event and select its exact merge commit,
    /// refusing merges that also brought in commits not covered by its changelog.
    FromEvent {
        /// JSON payload of the pull_request event; GitHub Actions provides
        /// $GITHUB_EVENT_PATH.
        #[arg(long)]
        event: PathBuf,

        /// GitHub repository the event must belong to, as OWNER/REPO.
        #[arg(long)]
        repository: String,

        /// File to append the outputs to; GitHub Actions provides $GITHUB_OUTPUT.
        #[arg(long)]
        github_output: Option<PathBuf>,
    },

    /// Update the workspace version, Cargo.lock, and changelog in a clean checkout.
    Prepare {
        /// Next SemVer version without a leading v; must follow the current one.
        #[arg(long)]
        version: String,

        /// Re-prepare the current, untagged workspace version (for example after
        /// tag-release refused its merge), regenerating its changelog section.
        #[arg(long)]
        allow_current_version: bool,

        /// File to append the outputs to; GitHub Actions provides $GITHUB_OUTPUT.
        #[arg(long)]
        github_output: Option<PathBuf>,
    },

    /// Check that HEAD, the tag, workspace version, lockfile, and notes agree.
    Validate {
        /// Existing release tag, such as cm-testsuite-v0.2.0.
        #[arg(long)]
        tag: String,

        /// File to append the outputs to; GitHub Actions provides $GITHUB_OUTPUT.
        #[arg(long)]
        github_output: Option<PathBuf>,
    },

    /// Extract the latest changelog section, requiring the expected version.
    ExtractLatestChangelogSection {
        /// SemVer version the latest changelog section must describe.
        #[arg(long)]
        version: String,

        /// File to write the extracted release notes to.
        #[arg(long)]
        out: PathBuf,
    },

    /// Plan a tag on a validated commit reachable from origin/main; --apply writes it.
    Tag {
        /// GitHub repository to tag, as OWNER/REPO.
        #[arg(long)]
        repository: String,

        /// SemVer version to tag, without a leading v.
        #[arg(long)]
        version: String,

        /// Commit, branch, or other local ref to tag.
        #[arg(long)]
        reference: String,

        /// Create the tag on GitHub; requires GITHUB_TOKEN.
        #[arg(long)]
        apply: bool,
    },

    /// Plan release comments for related issues/PRs; --apply sends them to GitHub.
    PostReleaseIssueUpdate {
        /// GitHub repository holding the release, as OWNER/REPO.
        #[arg(long)]
        repository: String,

        /// Published release tag, such as cm-testsuite-v0.2.0.
        #[arg(long)]
        tag: String,

        /// Login that owns the release comments; only its comments are updated.
        #[arg(long, default_value = "github-actions[bot]")]
        comment_author: String,

        /// Create or update the comments on GitHub; requires GITHUB_TOKEN.
        #[arg(long)]
        apply: bool,
    },
}

/// Print `key=value` workflow outputs and, when given a path, append them to
/// that GitHub Actions output file.
fn write_outputs(outputs: &str, github_output: Option<&Path>) -> Result<()> {
    print!("{outputs}");
    if let Some(path) = github_output {
        fs::OpenOptions::new()
            .append(true)
            .create(true)
            .open(path)?
            .write_all(outputs.as_bytes())?;
    }
    Ok(())
}

/// Emit a validated release's metadata and tell the workflow to proceed.
fn emit(meta: &Metadata, github_output: Option<&Path>) -> Result<()> {
    write_outputs(&format!("proceed=true\n{}", meta.outputs()), github_output)
}

/// Build a GitHub client, requiring caller credentials only for operations
/// that perform writes.
fn client(apply: bool) -> Result<cm_github::Client> {
    let token = std::env::var("GITHUB_TOKEN")
        .ok()
        .filter(|s| !s.trim().is_empty());
    ensure!(!apply || token.is_some(), "--apply requires GITHUB_TOKEN");
    cm_github::Client::new(token)
}

/// Decide whether a closed-PR event is a release to tag. A release emits its
/// metadata and the merge commit as `ref`; any other PR emits `proceed=false`.
fn from_event(
    root: &Path,
    event: &Path,
    repository: &str,
    github_output: Option<&Path>,
) -> Result<()> {
    let event = serde_json::from_slice(&fs::read(event)?)?;
    let Some(release) = cm_release::merged_release(&event, repository)? else {
        return write_outputs("proceed=false\n", github_output);
    };
    let base = github::release_pr_base(&client(false)?, repository, release.number)?;
    history::ensure_merged_onto(root, &release.merge_sha, &base.sha, base.commits)?;
    emit(&release.meta, github_output)?;
    write_outputs(&format!("ref={}\n", release.merge_sha), github_output)
}

/// Run the selected release operation with explicit mutation flags.
pub fn run(root: &Path, args: Args) -> Result<()> {
    match args.command {
        Release::Metadata {
            version,
            github_output,
        } => emit(&Metadata::new(&version)?, github_output.as_deref())?,

        Release::FromEvent {
            event,
            repository,
            github_output,
        } => from_event(root, &event, &repository, github_output.as_deref())?,

        Release::Prepare {
            version,
            allow_current_version,
            github_output,
        } => {
            let cliff = tools::pinned(root, "git-cliff", CLIFF_VERSION)?;
            let meta = cm_release::prepare(root, &version, allow_current_version, cliff)?;
            emit(&meta, github_output.as_deref())?;
        }

        Release::Validate { tag, github_output } => emit(
            &history::validate_checkout(root, &tag)?,
            github_output.as_deref(),
        )?,

        Release::ExtractLatestChangelogSection { version, out } => {
            let changelog = fs::read_to_string(root.join(CHANGELOG_PATH))?;
            fs::write(out, cm_release::release_notes(&changelog, &version)?)?;
        }

        Release::Tag {
            repository,
            version,
            reference,
            apply,
        } => {
            let meta = Metadata::new(&version)?;
            let sha = history::tag_target(root, &meta, &reference)?;
            let create = github::create_tag(&client(apply)?, &repository, &meta, &sha, apply)?;
            println!(
                "{}",
                serde_json::json!({"tag": meta.tag, "sha": sha, "apply": apply, "create": create})
            );
        }

        Release::PostReleaseIssueUpdate {
            repository,
            tag,
            comment_author,
            apply,
        } => {
            let history = history::release_history(root, &tag)?;
            let api = client(apply)?;
            let plan = github::notifications(&api, &repository, &tag, &history, &comment_author)?;
            for warning in &plan.warnings {
                eprintln!("warning: {warning}");
            }
            println!("{}", cm_catalog::json(&plan.edits)?);
            if apply {
                github::apply_notifications(&api, &repository, &comment_author, &plan.edits)?;
            }
        }
    }

    Ok(())
}
