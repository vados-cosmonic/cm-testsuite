//! Record Cargo's rustdoc test commands without executing tests during a Buck build.
//! Version probes are forwarded to the pinned rustdoc. Only package-related Cargo
//! environment variables are recorded; credentials and user settings are excluded.

use anyhow::{Context, Result, ensure};
use serde::Serialize;
use std::{collections::BTreeMap, env, fs, path::PathBuf, process::Command};

#[derive(Serialize)]
struct Invocation {
    args: Vec<String>,
    directory: String,
    env: BTreeMap<String, String>,
}

/// Capture test invocations and preserve rustdoc's behavior for compiler probes.
fn main() -> Result<()> {
    let args: Vec<String> = env::args().skip(1).collect();
    if !args.iter().any(|arg| arg == "--test") {
        let status = Command::new(env::var_os("CM_REAL_RUSTDOC").context("CM_REAL_RUSTDOC")?)
            .args(args)
            .status()?;
        ensure!(status.success(), "rustdoc probe failed: {status}");
        return Ok(());
    }

    let name = args
        .windows(2)
        .find(|pair| pair[0] == "--crate-name")
        .map(|pair| &pair[1])
        .context("rustdoc test command lacks --crate-name")?;
    ensure!(
        name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_'),
        "invalid crate name"
    );
    let directory = PathBuf::from(env::var_os("CM_DOC_CAPTURE").context("CM_DOC_CAPTURE")?);
    fs::create_dir_all(&directory)?;
    let invocation = Invocation {
        directory: env::current_dir()?
            .to_str()
            .context("UTF-8 workspace path")?
            .into(),
        env: env::vars()
            .filter(|(key, _)| {
                key.starts_with("CARGO_PKG_")
                    || matches!(
                        key.as_str(),
                        "CARGO_MANIFEST_DIR" | "CARGO_MANIFEST_PATH" | "OUT_DIR"
                    )
            })
            .collect(),
        args: args.clone(),
    };
    fs::write(
        directory.join(format!("{name}.json")),
        serde_json::to_vec(&invocation)?,
    )?;
    Ok(())
}
