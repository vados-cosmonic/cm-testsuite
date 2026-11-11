//! Typed entrypoints for Buck's build-artifact export and test execution.

use crate::build_artifacts::{export, run};
use anyhow::Result;
use std::path::PathBuf;

#[derive(clap::Args)]
pub struct Args {
    #[command(subcommand)]
    command: Command,
}

#[derive(clap::Subcommand)]
enum Command {
    /// Publish executables, link libraries, and recorded doctests from a Cargo build.
    Export {
        #[arg(long)]
        work: PathBuf,
        #[arg(long)]
        sdk: PathBuf,
        #[arg(long)]
        out: PathBuf,
    },
    /// Execute an artifact suite without invoking Cargo or recompiling dependencies.
    Test {
        #[arg(long)]
        bundle: PathBuf,
        #[arg(long)]
        sdk: PathBuf,
        #[arg(long, value_enum)]
        suite: run::Suite,
        #[arg(last = true)]
        args: Vec<String>,
    },
}

/// Dispatch build preparation separately from explicit test execution.
pub fn run(args: Args) -> Result<()> {
    match args.command {
        Command::Export { work, sdk, out } => export::export(&work, &sdk, &out),
        Command::Test {
            bundle,
            sdk,
            suite,
            args,
        } => run::run(&bundle, &sdk, suite, &args),
    }
}
