//! Sole command entrypoint. Domain behavior lives in the workspace library crates.
use anyhow::{Context, Result, ensure};
use clap::{Parser, Subcommand};
use cm_catalog::{Catalog, Manifest, digest, json, sha256};
use cm_exchange_plan::Plan;
use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
    process::Command,
};
#[derive(Parser)]
#[command(about = "Curate and exchange checked-in WAST snapshots. Never executes WAST.")]
struct Cli {
    #[arg(long, global = true, default_value = ".")]
    root: PathBuf,
    #[command(subcommand)]
    command: Task,
}
#[derive(Subcommand)]
enum Task {
    /// Read an opted-in manifest and its files from immutable Git snapshots.
    Acquire {
        #[arg(long)]
        source: String,
        #[arg(long, default_value = "main")]
        reference: String,
        #[arg(long, default_value = ".cm-testsuite.toml")]
        manifest: String,
        #[arg(long)]
        local: Option<PathBuf>,
        #[arg(long)]
        out: PathBuf,
    },
    /// Prepare a reviewed catalog/corpus change; does not mutate the repository.
    PlanImport {
        #[arg(long, required = true)]
        snapshot: Vec<PathBuf>,
        #[arg(long)]
        out: PathBuf,
    },
    /// Apply a reviewed plan after checking all file and policy baselines.
    Apply {
        #[arg(long)]
        plan: PathBuf,
    },
    /// Verify catalog digests and structurally parse every WAST; never execute it.
    Validate,
    /// Parse a WAT/WAST file and show conservative syntax evidence and warnings.
    Analyze { file: PathBuf },
    /// Plan delivery using the recipient manifest in --root and a curated catalog.
    PlanReceive {
        #[arg(long)]
        catalog_root: PathBuf,
        #[arg(long)]
        recipient: String,
        #[arg(long, default_value = ".cm-testsuite.toml")]
        manifest: String,
        #[arg(long)]
        out: PathBuf,
    },
    /// Explicitly create/update a draft PR. Requires caller-supplied GITHUB_TOKEN.
    Publish {
        #[arg(long)]
        repository: String,
        #[arg(long)]
        plan: PathBuf,
    },
    /// Admit an engine's JSON report against this exact catalog and approved publishers.
    ResultsImport {
        #[arg(long)]
        report: PathBuf,
        #[arg(long, default_value = "registry/publishers.json")]
        publishers: PathBuf,
    },
    /// Build the mdBook with the pinned repository launcher.
    Docs,
    /// Build docs and the separate engine-submitted results page.
    Site,
    /// Generate Buck's immutable crate inputs from the Cargo lockfile.
    BuckLock {
        #[arg(long)]
        check: bool,
    },
    /// Run a complete offline two-source collection and receiving example.
    Demo {
        #[arg(long, default_value = ".demo")]
        out: PathBuf,
    },
}
fn write(path: &Path, text: &str) -> Result<()> {
    if let Some(parent) = path.parent().filter(|x| !x.as_os_str().is_empty()) {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, text)?;
    Ok(())
}
fn load_plan(path: &Path) -> Result<Plan> {
    let plan: Plan = serde_json::from_slice(&fs::read(path)?)?;
    plan.validate()?;
    Ok(plan)
}
fn main() -> Result<()> {
    let cli = Cli::parse();
    fs::create_dir_all(&cli.root)?;
    let root = cli.root.canonicalize()?;
    match cli.command {
        Task::Acquire {
            source,
            reference,
            manifest,
            local,
            out,
        } => {
            let snapshot = if let Some(path) = local {
                cm_acquisition::acquire(
                    &cm_acquisition::LocalGit { root: path },
                    &source,
                    &reference,
                    &manifest,
                )?
            } else {
                let api = cm_github::Client::new(std::env::var("GITHUB_TOKEN").ok());
                cm_acquisition::acquire(
                    &cm_acquisition::GitHub {
                        api: &api,
                        source: &source,
                    },
                    &source,
                    &reference,
                    &manifest,
                )?
            };
            write(&out, &json(&snapshot)?)?;
            println!(
                "Acquired {} tests at {}",
                snapshot.tests.len(),
                snapshot.metadata.revision
            );
        }
        Task::PlanImport { snapshot, out } => {
            let snapshots = snapshot
                .iter()
                .map(|p| cm_acquisition::load_snapshot(p))
                .collect::<Result<Vec<_>>>()?;
            let plan = cm_exchange_plan::import(&root, &snapshots)?;
            write(&out, &json(&plan)?)?;
            println!("Planned {} file changes", plan.edits.len());
        }
        Task::Apply { plan } => {
            let count = cm_delivery::apply(&root, &load_plan(&plan)?)?;
            println!("Applied {count} file changes");
        }
        Task::Validate => {
            let catalog = Catalog::load(&root)?;
            let files = cm_acquisition::load_catalog_files(&root, &catalog)?;
            for text in files.values() {
                cm_wast_analysis::analyze(text)?;
            }
            println!(
                "Validated {} WAST files; catalog {}",
                catalog.tests.len(),
                digest(&catalog)?
            );
        }
        Task::Analyze { file } => println!(
            "{}",
            json(&cm_wast_analysis::analyze(&fs::read_to_string(file)?)?)?
        ),
        Task::PlanReceive {
            catalog_root,
            recipient,
            manifest,
            out,
        } => {
            let catalog = Catalog::load(&catalog_root)?;
            let files = cm_acquisition::load_catalog_files(&catalog_root, &catalog)?;
            let plan = cm_exchange_plan::receive(&root, &recipient, &manifest, &catalog, &files)?;
            write(&out, &json(&plan)?)?;
            println!("Planned {} receiving changes", plan.edits.len());
        }
        Task::Publish { repository, plan } => {
            let token = std::env::var("GITHUB_TOKEN")
                .context("publish requires GITHUB_TOKEN supplied by the caller")?;
            ensure!(!token.is_empty(), "GITHUB_TOKEN is empty");
            let api = cm_github::Client::new(Some(token));
            println!(
                "{:?}",
                cm_delivery::publish(&api, &repository, &load_plan(&plan)?)?
            );
        }
        Task::ResultsImport { report, publishers } => {
            let catalog = Catalog::load(&root)?;
            let allowed: BTreeSet<String> =
                serde_json::from_slice(&fs::read(root.join(publishers))?)?;
            let report: cm_results::Report = serde_json::from_slice(&fs::read(report)?)?;
            report.validate(&catalog, &allowed)?;
            let directory = root.join("reports").join(digest(&catalog)?);
            let text = json(&report)?;
            let output = directory.join(format!("{}.json", sha256(text.as_bytes())));
            write(&output, &text)?;
            write(&directory.join("catalog.json"), &json(&catalog)?)?;
            println!("Accepted {}", output.display());
        }
        Task::Docs => docs(&root)?,
        Task::Site => {
            docs(&root)?;
            let catalog = Catalog::load(&root)?;
            let directory = root.join("reports").join(digest(&catalog)?);
            let allowed: BTreeSet<String> =
                serde_json::from_slice(&fs::read(root.join("registry/publishers.json"))?)?;
            let mut paths = if directory.exists() {
                fs::read_dir(directory)?
                    .map(|e| e.map(|x| x.path()))
                    .collect::<std::io::Result<Vec<_>>>()?
            } else {
                vec![]
            };
            paths.sort();
            let mut reports = vec![];
            for path in paths {
                if path.extension().is_some_and(|e| e == "json")
                    && path.file_name().is_some_and(|n| n != "catalog.json")
                {
                    let r: cm_results::Report = serde_json::from_slice(&fs::read(path)?)?;
                    r.validate(&catalog, &allowed)?;
                    reports.push(r);
                }
            }
            write(
                &root.join("site-out/results/index.html"),
                &cm_results::render(&catalog, &reports)?,
            )?;
            write(
                &root.join("site-out/index.html"),
                "<!doctype html><html lang=\"en\"><meta charset=\"utf-8\"><title>cm-testsuite</title><h1>cm-testsuite</h1><p>A curated exchange of checked-in Component Model WAST tests.</p><ul><li><a href=\"docs/\">Documentation</a></li><li><a href=\"results/\">Engine-reported results</a></li></ul></html>\n",
            )?;
        }
        Task::BuckLock { check } => {
            let generated = buck_lock(&fs::read_to_string(root.join("rust/Cargo.lock"))?)?;
            let path = root.join("third-party/crates.bzl");
            if check {
                ensure!(
                    fs::read_to_string(&path)? == generated,
                    "Buck crate inputs differ from Cargo.lock; run xtask buck-lock"
                );
            } else {
                write(&path, &generated)?;
            }
        }
        Task::Demo { out } => demo(&out)?,
    }
    Ok(())
}
fn docs(root: &Path) -> Result<()> {
    let version = Command::new("dotslash")
        .arg(root.join("mdbook"))
        .arg("--version")
        .output()
        .context("DotSlash is required; install it and ensure dotslash is on PATH")?;
    ensure!(
        version.status.success()
            && String::from_utf8_lossy(&version.stdout).trim() == "mdbook v0.5.4",
        "expected pinned mdBook 0.5.4"
    );
    let status = Command::new("dotslash")
        .arg(root.join("mdbook"))
        .arg("build")
        .arg(root.join("docs"))
        .arg("--dest-dir")
        .arg(root.join("site-out/docs"))
        .status()?;
    ensure!(status.success(), "mdBook build failed");
    Ok(())
}
fn buck_lock(text: &str) -> Result<String> {
    let lock: toml::Value = toml::from_str(text)?;
    let mut packages = lock["package"]
        .as_array()
        .context("missing lock packages")?
        .iter()
        .filter(|p| {
            p.get("source")
                .and_then(|x| x.as_str())
                .is_some_and(|s| s.starts_with("registry+"))
        })
        .collect::<Vec<_>>();
    packages.sort_by_key(|p| (p["name"].as_str().unwrap(), p["version"].as_str().unwrap()));
    let mut out = String::from(
        "# Generated from rust/Cargo.lock by xtask buck-lock. Do not edit.\n\ndef pinned_crates():\n",
    );
    for p in &packages {
        let name = p["name"].as_str().context("missing crate name")?;
        let version = p["version"].as_str().context("missing version")?;
        let checksum = p["checksum"].as_str().context("missing crate checksum")?;
        let full = format!("{name}-{version}");
        out.push_str(&format!("    native.http_archive(name = {:?}, urls = [\"https://static.crates.io/crates/{name}/{version}/download\"], sha256 = \"{checksum}\", strip_prefix = \"{full}\", type = \"tar.gz\", visibility = [\"PUBLIC\"])\n",full.replace('+',"_")));
    }
    out.push_str("\nCRATES = {\n");
    for p in &packages {
        let name = p["name"].as_str().unwrap();
        let version = p["version"].as_str().unwrap();
        out.push_str(&format!(
            "    \"{name}-{version}\": \"//third-party:{name}-{}\",\n",
            version.replace('+', "_")
        ));
    }
    out.push_str("}\n\nCHECKSUMS = {\n");
    for p in packages {
        out.push_str(&format!(
            "    \"{}-{}\": \"{}\",\n",
            p["name"].as_str().unwrap(),
            p["version"].as_str().unwrap(),
            p["checksum"].as_str().unwrap()
        ));
    }
    out.push_str("}\n");
    Ok(out)
}
fn git(root: &Path, args: &[&str]) -> Result<()> {
    let out = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(args)
        .env("GIT_AUTHOR_NAME", "cm-testsuite example")
        .env("GIT_AUTHOR_EMAIL", "example@invalid")
        .env("GIT_COMMITTER_NAME", "cm-testsuite example")
        .env("GIT_COMMITTER_EMAIL", "example@invalid")
        .output()?;
    ensure!(
        out.status.success(),
        "demo git failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    Ok(())
}
fn demo(out: &Path) -> Result<()> {
    fs::create_dir_all(out)?;
    let out = out.canonicalize()?;
    let hub = out.join("central");
    let recipient = out.join("recipient");
    fs::create_dir_all(&hub)?;
    fs::create_dir_all(&recipient)?;
    let mut snapshots = vec![];
    for (name, text) in [
        (
            "one",
            ";; Intent: a minimal component can be represented.\n(component)\n",
        ),
        (
            "two",
            ";; Intent: preserve a deliberately invalid core module.\n(assert_invalid (module (func (result i32))) \"type mismatch\")\n",
        ),
    ] {
        let upstream = out.join(format!("source-{name}"));
        if !upstream.exists() {
            fs::create_dir(&upstream)?;
            write(
                &upstream.join(".cm-testsuite.toml"),
                "schema = 1\n[[test]]\nname = 'example'\npath = 'file:///example.wast'\nrequires = ['component-model']\ncovers = ['smoke']\n",
            )?;
            write(&upstream.join("example.wast"), text)?;
            git(&upstream, &["init", "-b", "main"])?;
            git(&upstream, &["add", "."])?;
            git(&upstream, &["commit", "-m", "Example checked-in WAST"])?;
        }
        snapshots.push(cm_acquisition::acquire(
            &cm_acquisition::LocalGit { root: upstream },
            &format!("github:example/{name}"),
            "main",
            ".cm-testsuite.toml",
        )?);
    }
    let plan = cm_exchange_plan::import(&hub, &snapshots)?;
    write(&out.join("import-plan.json"), &json(&plan)?)?;
    cm_delivery::apply(&hub, &plan)?;
    ensure!(
        cm_exchange_plan::import(&hub, &snapshots)?.edits.is_empty(),
        "import rerun must be empty"
    );
    let manifest = recipient.join(".cm-testsuite.toml");
    if !manifest.exists() {
        write(
            &manifest,
            "schema = 1\n[receive]\nenabled = true\n[[receive.route]]\nname = 'smoke'\ndestination = 'file:///tests/shared'\nsupports = ['component-model']\nallow_unknown = true\n",
        )?;
    }
    let catalog = Catalog::load(&hub)?;
    let files = cm_acquisition::load_catalog_files(&hub, &catalog)?;
    let receiving = cm_exchange_plan::receive(
        &recipient,
        "github:example/recipient",
        ".cm-testsuite.toml",
        &catalog,
        &files,
    )?;
    write(&out.join("receive-plan.json"), &json(&receiving)?)?;
    cm_delivery::apply(&recipient, &receiving)?;
    ensure!(
        cm_exchange_plan::receive(
            &recipient,
            "github:example/recipient",
            ".cm-testsuite.toml",
            &catalog,
            &files
        )?
        .edits
        .is_empty(),
        "receiving rerun must be empty"
    );
    let parsed = Manifest::parse(&fs::read_to_string(manifest)?, ".cm-testsuite.toml")?;
    ensure!(parsed.receive.enabled, "demo recipient must opt in");
    println!(
        "Demo complete: two committed sources -> curated catalog -> receiving folder. Both reruns produced no changes. Files: {}",
        out.display()
    );
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn offline_flow_is_repeatable() {
        let d = tempfile::tempdir().unwrap();
        demo(&d.path().join("demo")).unwrap();
        demo(&d.path().join("demo")).unwrap();
    }
}
