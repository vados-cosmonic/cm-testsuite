//! Typed adapters for release preparation, tagging, and notifications.

use anyhow::{Result, ensure};
use clap::Subcommand;
use cm_release::{Metadata, github, history};
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
        #[arg(long)]
        version: String,
        #[arg(long)]
        github_output: Option<PathBuf>,
    },
    /// Validate an automatic merged-PR event and select its exact merge commit,
    /// refusing merges that also brought in commits not covered by its changelog.
    FromEvent {
        #[arg(long)]
        event: PathBuf,
        #[arg(long)]
        repository: String,
        #[arg(long)]
        github_output: Option<PathBuf>,
    },
    /// Update the workspace version, Cargo.lock, and changelog in a clean checkout.
    Prepare {
        #[arg(long)]
        version: String,
        /// Re-prepare the current, untagged workspace version (for example after
        /// tag-release refused its merge), regenerating its changelog section.
        #[arg(long)]
        allow_current_version: bool,
        #[arg(long)]
        github_output: Option<PathBuf>,
    },
    /// Check that HEAD, the tag, workspace version, lockfile, and notes agree.
    Validate {
        #[arg(long)]
        tag: String,
        #[arg(long)]
        github_output: Option<PathBuf>,
    },
    /// Extract the latest changelog section, requiring the expected version.
    ExtractLatestChangelogSection {
        #[arg(long)]
        version: String,
        #[arg(long)]
        out: PathBuf,
    },
    /// Print whether a version is a prerelease.
    SemverIsPrerelease { version: String },
    /// Print the prerelease channel, or an empty string for a stable version.
    SemverGetPrerelease { version: String },
    /// Compare two versions using SemVer precedence (ignoring build metadata).
    SemverLt { current: String, next: String },
    /// Plan a tag on a validated commit reachable from origin/main; --apply writes it.
    Tag {
        #[arg(long)]
        repository: String,
        #[arg(long)]
        version: String,
        #[arg(long)]
        reference: String,
        #[arg(long)]
        apply: bool,
    },
    /// Plan release comments for related issues/PRs; --apply sends them to GitHub.
    PostReleaseIssueUpdate {
        #[arg(long)]
        repository: String,
        #[arg(long)]
        tag: String,
        #[arg(long, default_value = "github-actions[bot]")]
        comment_author: String,
        #[arg(long)]
        apply: bool,
    },
}
/// Print validated workflow metadata and optionally append Actions outputs.
fn emit(meta: &Metadata, output: Option<&Path>) -> Result<()> {
    print!("{}", meta.outputs());
    if let Some(path) = output {
        fs::OpenOptions::new()
            .append(true)
            .create(true)
            .open(path)?
            .write_all(meta.outputs().as_bytes())?;
    }
    Ok(())
}
/// Require caller credentials only for operations that perform writes.
fn client(apply: bool) -> Result<cm_github::Client> {
    let token = std::env::var("GITHUB_TOKEN")
        .ok()
        .filter(|s| !s.trim().is_empty());
    ensure!(!apply || token.is_some(), "--apply requires GITHUB_TOKEN");
    cm_github::Client::new(token)
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
        } => {
            let event = serde_json::from_slice(&fs::read(event)?)?;
            if let Some(release) = cm_release::merged_release(&event, &repository)? {
                let (base, count) =
                    github::release_pr_base(&client(false)?, &repository, release.number)?;
                history::ensure_merged_onto(root, &release.merge_sha, &base, count)?;
                let (meta, sha) = (release.meta, release.merge_sha);
                emit(&meta, github_output.as_deref())?;
                println!("ref={sha}");
                if let Some(path) = github_output {
                    writeln!(
                        fs::OpenOptions::new()
                            .append(true)
                            .create(true)
                            .open(path)?,
                        "ref={sha}"
                    )?;
                }
            } else {
                println!("proceed=false");
                if let Some(path) = github_output {
                    writeln!(
                        fs::OpenOptions::new()
                            .append(true)
                            .create(true)
                            .open(path)?,
                        "proceed=false"
                    )?;
                }
            }
        }
        Release::Prepare {
            version,
            allow_current_version,
            github_output,
        } => emit(
            &cm_release::prepare(root, &version, allow_current_version)?,
            github_output.as_deref(),
        )?,
        Release::Validate { tag, github_output } => emit(
            &history::validate_checkout(root, &tag)?,
            github_output.as_deref(),
        )?,
        Release::ExtractLatestChangelogSection { version, out } => {
            let notes = cm_release::release_notes(
                &fs::read_to_string(root.join("CHANGELOG.md"))?,
                &version,
            )?;
            fs::write(out, notes)?;
        }
        Release::SemverIsPrerelease { version } => {
            println!("{}", Metadata::new(&version)?.prerelease)
        }
        Release::SemverGetPrerelease { version } => {
            println!("{}", Metadata::new(&version)?.prerelease_tag)
        }
        Release::SemverLt { current, next } => println!(
            "{}",
            cm_release::version(&current)?
                .cmp_precedence(&cm_release::version(&next)?)
                .is_lt()
        ),
        Release::Tag {
            repository,
            version,
            reference,
            apply,
        } => {
            let meta = Metadata::new(&version)?;
            let sha = history::tag_target(root, &meta, &reference)?;
            let changed = github::create_tag(&client(apply)?, &repository, &meta, &sha, apply)?;
            println!(
                "{}",
                serde_json::json!({"tag":meta.tag,"sha":sha,"apply":apply,"create":changed})
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
            let edits = github::notifications(&api, &repository, &tag, &history, &comment_author)?;
            println!("{}", cm_catalog::json(&edits)?);
            if apply {
                github::apply_notifications(&api, &repository, &comment_author, &edits)?;
            }
        }
    }
    Ok(())
}
