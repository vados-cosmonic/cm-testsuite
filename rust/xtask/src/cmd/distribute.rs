//! Propose catalog updates to every enrolled repository that receives tests.
use crate::exchange::{participants, planned, published, require_token, settle, token};
use anyhow::Result;
use cm_catalog::{Catalog, PARTICIPANTS_PATH};
use cm_delivery::{Delivery, Failure};
use std::path::{Path, PathBuf};

#[derive(clap::Args)]
pub struct Args {
    /// Registry of enrolled repositories, relative to --root.
    #[arg(long, default_value = PARTICIPANTS_PATH)]
    pub(crate) registry: PathBuf,
    /// Check only this enrolled repository; repeat to name several.
    #[arg(long)]
    pub(crate) only: Vec<String>,
    /// Open, update, or withdraw each repository's single draft pull request.
    #[arg(long)]
    pub(crate) publish: bool,
    /// File failures as one issue in this repository instead of failing the run.
    #[arg(long, conflicts_with = "only")]
    pub(crate) report_to: Option<String>,
}

/// Propose catalog updates to every enrolled repository that receives tests.
///
/// Without --publish this only reports what each repository would receive.
pub fn run(root: &Path, args: Args) -> Result<()> {
    let Args {
        registry: path,
        only,
        publish,
        report_to,
    } = args;

    let registry = participants(root, &path, &only)?;
    let token = token();
    if publish && !registry.participant.is_empty() {
        require_token(&token)?;
    }
    let catalog = Catalog::load(root)?;
    let files = cm_acquisition::load_catalog_files(root, &catalog)?;
    let api = cm_github::Client::new(token)?;

    // One repository's failure must not keep the others from their updates.
    let mut failures = vec![];
    for participant in &registry.participant {
        let repository = &participant.repository;
        match cm_delivery::distribute(&api, participant, &catalog, &files, publish) {
            Ok(Delivery::NotReceiving) => println!("{repository}: does not receive tests"),
            Ok(Delivery::Planned(plan)) => println!("{repository}: {}", planned(&plan)),
            Ok(Delivery::Published(plan, publication)) => {
                println!("{repository}: {}", published(&plan, &publication));
            }
            Err(error) => {
                println!("{repository}: failed: {error:#}");
                failures.push(Failure {
                    repository: repository.clone(),
                    error: format!("{error:#}"),
                    contacts: participant.maintainers.clone(),
                });
            }
        }
    }
    if registry.participant.is_empty() {
        println!("no repositories are enrolled in {}", path.display());
    } else if !publish {
        println!("dry run; pass --publish to open or update pull requests");
    }

    settle(
        "distribute",
        report_to.as_deref(),
        &failures,
        &registry.maintainers,
    )
}
