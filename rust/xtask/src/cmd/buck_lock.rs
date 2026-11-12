//! Generate Buck's immutable crate inputs from the Cargo lockfile.
use crate::io::write;
use anyhow::{Context, Result, ensure};
use std::{fs, path::Path};

#[derive(clap::Args)]
pub struct Args {
    #[arg(long)]
    pub(crate) check: bool,
}

/// Generate Buck's immutable crate inputs from the Cargo lockfile.
pub fn run(root: &Path, args: Args) -> Result<()> {
    let Args { check } = args;

    let generated = buck_lock(&fs::read_to_string(root.join("rust/Cargo.lock"))?)?;
    let path = root.join(".buck/third-party/crates.bzl");
    if check {
        ensure!(
            fs::read_to_string(&path)? == generated,
            "Buck crate inputs differ from Cargo.lock; run xtask buck-lock"
        );
    } else {
        write(&path, &generated)?;
    }

    Ok(())
}

/// Render deterministic archive definitions and checksum maps from registry packages.
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
        out.push_str(&format!(
            "    native.http_file(name = {:?}, \
             urls = [\"https://static.crates.io/crates/{name}/{version}/download\"], \
             sha256 = \"{checksum}\", out = \"{full}.crate\", \
             visibility = [\"PUBLIC\"])\n",
            full.replace('+', "_"),
        ));
    }
    out.push_str("\nCRATES = {\n");
    for p in &packages {
        let name = p["name"].as_str().unwrap();
        let version = p["version"].as_str().unwrap();
        out.push_str(&format!(
            "    \"{name}-{version}\": \"//.buck/third-party:{name}-{}\",\n",
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
