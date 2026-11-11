//! Exercise every example project through the same command handlers used by the CLI.

use anyhow::{Context, Result, ensure};
use cm_catalog::{Catalog, Manifest, RECEIPT_PATH, records};
use cm_exchange_plan::Receipt;
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    process::Command,
};

use crate::io::load_plan;

#[derive(clap::Args)]
pub struct Args {
    #[arg(long, default_value = ".demo")]
    pub(crate) out: PathBuf,
}

/// Run acquisition, import, validation, and receiving for every example directory.
pub fn run(root: &Path, args: Args) -> Result<()> {
    let fixtures = std::env::var_os("CM_TESTSUITE_EXAMPLES")
        .map(PathBuf::from)
        .unwrap_or_else(|| root.join("examples"));
    let mut projects = fs::read_dir(&fixtures)
        .with_context(|| format!("example projects at {}", fixtures.display()))?
        .map(|entry| entry.map(|entry| entry.path()))
        .collect::<std::io::Result<Vec<_>>>()?;
    projects.retain(|path| path.is_dir());
    projects.sort();
    ensure!(
        projects.len() >= 2,
        "at least two example projects are required for exchange"
    );

    // All mutable state belongs to the output directory. Never initialize Git or
    // apply a delivery in the checked-in example directories themselves.
    fs::create_dir_all(&args.out)?;
    let out = args.out.canonicalize()?;
    let fixtures = fixtures.canonicalize()?;
    ensure!(
        !out.starts_with(&fixtures),
        "demo output must be outside examples"
    );
    let hub = out.join("central");
    fs::create_dir_all(&hub)?;
    let mut snapshots = Vec::new();
    let mut originals = BTreeMap::new();
    for fixture in &projects {
        let name = fixture
            .file_name()
            .context("missing example directory name")?
            .to_str()
            .context("example names must be UTF-8")?;
        let project = out.join(name);
        let files = inventory(fixture)?;
        let manifest = files
            .get(".cm-testsuite.toml")
            .context("every example needs a manifest")?;
        let manifest = Manifest::parse(std::str::from_utf8(manifest)?, ".cm-testsuite.toml")?;
        ensure!(
            !manifest.test.is_empty() && manifest.receive.enabled,
            "example {name} must contribute and receive tests"
        );
        if !project.exists() {
            for (path, bytes) in &files {
                let target = project.join(path);
                fs::create_dir_all(target.parent().unwrap())?;
                fs::write(target, bytes)?;
            }
            git(&project, &["init", "-b", "main"])?;
            git(&project, &["add", "."])?;
            git(&project, &["commit", "-m", "Example checked-in WAST"])?;
        }
        originals.insert(name.to_owned(), files);
        let snapshot = out.join(format!("{name}-snapshot.toml"));
        super::acquire::run(
            root,
            super::acquire::Args {
                source: format!("github:example/{name}"),
                reference: "main".into(),
                manifest: ".cm-testsuite.toml".into(),
                local: Some(project),
                out: snapshot.clone(),
            },
        )?;
        snapshots.push(snapshot);
    }

    // Serialize and reload plans at the command boundary, just as separate CLI
    // invocations do. Replanning must produce no edits after successful application.
    let import_plan = out.join("import-plan.toml");
    for _ in 0..2 {
        super::plan_import::run(
            &hub,
            super::plan_import::Args {
                snapshot: snapshots.clone(),
                out: import_plan.clone(),
            },
        )?;
        super::apply::run(
            &hub,
            super::apply::Args {
                plan: import_plan.clone(),
            },
        )?;
    }
    ensure!(
        load_plan(&import_plan)?.edits.is_empty(),
        "import rerun must be empty"
    );
    super::validate::run(&hub, super::validate::Args {})?;
    let catalog = Catalog::load(&hub)?;

    for (name, files) in &originals {
        let project = out.join(name);
        let plan = out.join(format!("{name}-receive-plan.toml"));
        for _ in 0..2 {
            super::plan_receive::run(
                &project,
                super::plan_receive::Args {
                    catalog_root: hub.clone(),
                    recipient: format!("github:example/{name}"),
                    manifest: ".cm-testsuite.toml".into(),
                    out: plan.clone(),
                },
            )?;
            super::apply::run(&project, super::apply::Args { plan: plan.clone() })?;
        }
        ensure!(
            load_plan(&plan)?.edits.is_empty(),
            "receiving rerun must be empty for {name}"
        );
        let receipt: Receipt = records::decode(&fs::read(project.join(RECEIPT_PATH))?)?;
        receipt.validate()?;
        ensure!(
            !receipt.tests.is_empty(),
            "example {name} must receive another project's tests"
        );
        for (id, received) in receipt.tests {
            let test = catalog
                .tests
                .get(&id)
                .context("receipt identity missing from catalog")?;
            ensure!(
                test.source != format!("github:example/{name}"),
                "example received its own test"
            );
            ensure!(
                fs::read(project.join(received.path))? == fs::read(hub.join(&test.destination))?,
                "delivered bytes differ from corpus"
            );
        }
        for (path, bytes) in files {
            ensure!(
                fs::read(project.join(path))? == *bytes,
                "original example file changed: {name}/{path}"
            );
        }
        ensure!(
            inventory(&fixtures.join(name))? == *files,
            "checked-in example changed: {name}"
        );
    }
    println!(
        "Demo complete: {} projects contributed and received tests; repeated plans were empty. Files: {}",
        projects.len(),
        out.display()
    );
    Ok(())
}

/// Inventory fixture bytes while rejecting symlinks and unexpected filesystem objects.
fn inventory(root: &Path) -> Result<BTreeMap<String, Vec<u8>>> {
    fn visit(root: &Path, path: &Path, files: &mut BTreeMap<String, Vec<u8>>) -> Result<()> {
        for entry in fs::read_dir(path)? {
            let entry = entry?;
            let kind = entry.file_type()?;
            ensure!(
                !kind.is_symlink(),
                "example fixtures must not contain symlinks"
            );
            if kind.is_dir() {
                visit(root, &entry.path(), files)?;
            } else {
                ensure!(kind.is_file(), "unsupported example file type");
                files.insert(
                    entry
                        .path()
                        .strip_prefix(root)?
                        .to_str()
                        .context("example paths must be UTF-8")?
                        .replace('\\', "/"),
                    fs::read(entry.path())?,
                );
            }
        }
        Ok(())
    }
    let mut files = BTreeMap::new();
    visit(root, root, &mut files)?;
    Ok(files)
}

/// Create local fixture commits without depending on the developer's Git identity.
fn git(root: &Path, args: &[&str]) -> Result<()> {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["-c", "commit.gpgsign=false", "-c", "core.autocrlf=false"])
        .args(args)
        .env("GIT_AUTHOR_NAME", "cm-testsuite example")
        .env("GIT_AUTHOR_EMAIL", "example@invalid")
        .env("GIT_COMMITTER_NAME", "cm-testsuite example")
        .env("GIT_COMMITTER_EMAIL", "example@invalid")
        .output()?;
    ensure!(
        output.status.success(),
        "demo git failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}
