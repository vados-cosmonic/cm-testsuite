//! Parse command-line arguments and dispatch to typed command handlers.

mod build_artifacts;
mod cmd;
mod exchange;
mod io;
mod tools;

use anyhow::Result;
use clap::{Parser, Subcommand};
use std::{fs, path::PathBuf};

#[derive(Parser)]
#[command(about = "Gather and redistribute checked-in WAST snapshots without executing them.")]
struct Cli {
    #[arg(long, global = true, default_value = ".")]
    root: PathBuf,
    #[command(subcommand)]
    command: Task,
}

#[derive(Subcommand)]
enum Task {
    /// Export or run the compiled artifacts used by Buck.
    #[command(hide = true)]
    BuckArtifacts(cmd::buck_artifacts::Args),

    /// Repository release preparation, tagging, and post-release updates.
    Release(cmd::release::Args),

    /// Run commitlint over an exact Git range.
    LintCommits(cmd::lint_commits::Args),

    /// Read an opted-in manifest and its files from immutable Git snapshots.
    Acquire(cmd::acquire::Args),

    /// Prepare a reviewed catalog/corpus change; does not mutate the repository.
    PlanImport(cmd::plan_import::Args),

    /// Apply a reviewed plan after checking all file and policy baselines.
    Apply(cmd::apply::Args),

    /// Verify catalog digests and structurally parse every WAST; never execute it.
    Validate(cmd::validate::Args),

    /// Parse a WAT/WAST file and show conservative syntax evidence and warnings.
    Analyze(cmd::analyze::Args),

    /// Plan delivery using the recipient manifest in --root and a curated catalog.
    PlanReceive(cmd::plan_receive::Args),

    /// Explicitly create/update a draft PR. Requires caller-supplied GITHUB_TOKEN.
    Publish(cmd::publish::Args),

    /// Gather updates from enrolled repositories into one catalog import.
    Gather(cmd::gather::Args),

    /// Propose catalog updates to every enrolled repository that receives tests.
    Distribute(cmd::distribute::Args),

    /// Admit an engine's TOML report against this exact catalog and approved publishers.
    ResultsImport(cmd::results_import::Args),

    /// Build the mdBook with the pinned repository launcher.
    Docs(cmd::docs::Args),

    /// Build docs and the separate engine-submitted results page.
    Site(cmd::site::Args),

    /// Generate Buck's immutable crate inputs from the Cargo lockfile.
    BuckLock(cmd::buck_lock::Args),

    /// Run every example project through an offline collection and receiving flow.
    Demo(cmd::demo::Args),
}

/// Resolve the repository once before invoking the selected command.
fn main() -> Result<()> {
    let cli = Cli::parse();
    fs::create_dir_all(&cli.root)?;
    let root = cli.root.canonicalize()?;

    match cli.command {
        Task::BuckArtifacts(args) => cmd::buck_artifacts::run(args),
        Task::Release(args) => cmd::release::run(&root, args),
        Task::LintCommits(args) => cmd::lint_commits::run(&root, args),
        Task::Acquire(args) => cmd::acquire::run(&root, args),
        Task::PlanImport(args) => cmd::plan_import::run(&root, args),
        Task::Apply(args) => cmd::apply::run(&root, args),
        Task::Validate(args) => cmd::validate::run(&root, args),
        Task::Analyze(args) => cmd::analyze::run(&root, args),
        Task::PlanReceive(args) => cmd::plan_receive::run(&root, args),
        Task::Publish(args) => cmd::publish::run(&root, args),
        Task::Gather(args) => cmd::gather::run(&root, args),
        Task::Distribute(args) => cmd::distribute::run(&root, args),
        Task::ResultsImport(args) => cmd::results_import::run(&root, args),
        Task::Docs(args) => cmd::docs::run(&root, args),
        Task::Site(args) => cmd::site::run(&root, args),
        Task::BuckLock(args) => cmd::buck_lock::run(&root, args),
        Task::Demo(args) => cmd::demo::run(&root, args),
    }
}
