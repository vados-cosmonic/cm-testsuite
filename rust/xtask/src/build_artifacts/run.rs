//! Execute compiled test suites without invoking Cargo or mutating Buck inputs.

use super::Bundle;
use anyhow::{Context, Result, ensure};
use std::{env, fs, path::Path, process::Command};

#[derive(Clone, Copy, clap::ValueEnum)]
pub enum Suite {
    Unit,
    Integration,
    Doc,
    Live,
}

/// Render Windows verbatim roots as ordinary paths before appending recorded suffixes.
fn argument_root(path: &Path) -> String {
    let value = path.to_string_lossy();
    if let Some(rest) = value.strip_prefix(r"\\?\UNC\") {
        format!(r"\\{rest}")
    } else {
        value.strip_prefix(r"\\?\").unwrap_or(&value).to_owned()
    }
}

/// Expand recorded roots without mixing forward slashes into a Windows verbatim path.
pub(crate) fn expanded(value: &str, bundle: &Path, sdk: &Path) -> String {
    value
        .replace("${BUNDLE}", &argument_root(bundle))
        .replace("${SDK}", &argument_root(sdk))
}

/// Execute every selected harness, retaining failures instead of stopping at the first.
pub fn run(bundle: &Path, sdk: &Path, suite: Suite, args: &[String]) -> Result<()> {
    let bundle = bundle.canonicalize()?;
    let sdk = sdk.canonicalize()?;
    let manifest: Bundle = serde_json::from_slice(&fs::read(bundle.join("manifest.json"))?)?;
    let suffix = env::consts::EXE_SUFFIX;
    let mut failures = Vec::new();
    let mut count = 0;

    // Live GitHub mutations require both an explicit suite and the established opt-in.
    if matches!(suite, Suite::Live) {
        ensure!(
            env::var("CM_TESTSUITE_LIVE_GITHUB").as_deref() == Ok("1"),
            "set CM_TESTSUITE_LIVE_GITHUB=1 to run the live GitHub suite"
        );
    }
    for test in &manifest.tests {
        let selected = match suite {
            Suite::Unit => !test.integration,
            Suite::Integration => test.integration && !test.live,
            Suite::Live => test.live,
            Suite::Doc => false,
        };
        if !selected {
            continue;
        }
        count += 1;
        println!("Running {}", test.name);
        let directory = bundle.join(&test.directory);
        let mut command = Command::new(bundle.join(&test.path));
        command
            .current_dir(&directory)
            .args(args)
            .env("CARGO_MANIFEST_DIR", &directory)
            .env("CARGO_MANIFEST_PATH", directory.join("Cargo.toml"))
            .env("CM_TESTSUITE_XTASK", bundle.join(format!("xtask{suffix}")))
            .env("CM_TESTSUITE_EXAMPLES", bundle.join("source/examples"));
        if !matches!(suite, Suite::Live) {
            command.env_remove("GITHUB_TOKEN").env_remove("GH_TOKEN");
        }
        if !command
            .status()
            .with_context(|| format!("run {}", test.name))?
            .success()
        {
            failures.push(test.name.clone());
        }
    }

    // Rustdoc compiles each documentation example when the test runs. All crate
    // dependencies were compiled in the build action; no Cargo invocation or
    // registry download occurs here. Rustdoc's own temporary outputs stay outside
    // the immutable bundle, and the downloaded SDK supplies its compiler.
    if matches!(suite, Suite::Doc) {
        for doc in &manifest.docs {
            count += 1;
            let name = doc
                .args
                .windows(2)
                .find(|pair| pair[0] == "--crate-name")
                .map(|pair| pair[1].as_str())
                .unwrap_or("documentation");
            let directory = expanded(&doc.directory, &bundle, &sdk);
            let mut command = Command::new(sdk.join(format!("bin/rustdoc{suffix}")));
            command
                .current_dir(&directory)
                .args(doc.args.iter().map(|arg| expanded(arg, &bundle, &sdk)))
                .envs(
                    doc.env
                        .iter()
                        .map(|(key, value)| (key, expanded(value, &bundle, &sdk))),
                )
                .env("RUSTC", sdk.join(format!("bin/rustc{suffix}")))
                .env_remove("GITHUB_TOKEN")
                .env_remove("GH_TOKEN");
            let mut paths = vec![sdk.join("bin")];
            paths.extend(env::split_paths(&env::var_os("PATH").unwrap_or_default()));
            command.env("PATH", env::join_paths(paths)?);
            for arg in args {
                command.arg("--test-args").arg(arg);
            }
            println!("Running documentation tests for {name}");
            if !command.status().context("run rustdoc")?.success() {
                failures.push(name.to_owned());
            }
        }
    }
    ensure!(count > 0, "no tests matched the requested suite");
    ensure!(
        failures.is_empty(),
        "failed test targets: {}",
        failures.join(", ")
    );
    println!("All {count} test targets passed.");
    Ok(())
}
