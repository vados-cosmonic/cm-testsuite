//! Build docs and the separate engine-submitted results page.
use crate::io::write;
use anyhow::Result;
use cm_catalog::{Catalog, digest, records};
use std::{fs, path::Path};

#[derive(clap::Args)]
pub struct Args {}

/// Build docs and the separate engine-submitted results page.
pub fn run(root: &Path, args: Args) -> Result<()> {
    let _ = args;

    super::docs::run(root, super::docs::Args {})?;
    let catalog = Catalog::load(root)?;
    let directory = root.join("reports").join(digest(&catalog)?);
    let allowed = cm_results::Publishers::load(&root.join("registry/publishers.toml"))?;
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
        if path.extension().is_some_and(|e| e == "toml")
            && path.file_name().is_some_and(|n| n != "catalog.toml")
        {
            let r: cm_results::Report = records::decode(&fs::read(path)?)?;
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

    Ok(())
}
