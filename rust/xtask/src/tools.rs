//! Launch repository tools through their checksum-pinned DotSlash descriptors.
use anyhow::{Context, Result, ensure};
use std::{path::Path, process::Command};

/// Prepare a command for the DotSlash tool `bins/<name>`, after checking that
/// its `--version` output is exactly `expected_version`.
pub fn pinned(root: &Path, name: &str, expected_version: &str) -> Result<Command> {
    let launcher = root.join("bins").join(name);
    let version = Command::new("dotslash")
        .arg(&launcher)
        .arg("--version")
        .output()
        .context("DotSlash is required; install it and ensure dotslash is on PATH")?;
    ensure!(
        version.status.success()
            && String::from_utf8_lossy(&version.stdout).trim() == expected_version,
        "expected bins/{name} to be the pinned {expected_version}"
    );
    let mut command = Command::new("dotslash");
    command.arg(launcher);
    Ok(command)
}
