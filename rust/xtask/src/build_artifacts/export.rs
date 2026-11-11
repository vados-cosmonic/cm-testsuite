//! Publish a compact, relocatable build bundle from Cargo's machine-readable output.

use super::{Bundle, DocTest, TestBinary};
use anyhow::{Context, Result, ensure};
use serde::Deserialize;
use std::{
    fs,
    io::{BufRead, BufReader},
    path::{Path, PathBuf},
};

#[derive(Deserialize)]
struct Artifact {
    filenames: Vec<PathBuf>,
    executable: Option<PathBuf>,
    target: Target,
    profile: Profile,
}

#[derive(Deserialize)]
struct Target {
    name: String,
    kind: Vec<String>,
    src_path: PathBuf,
}

#[derive(Deserialize)]
struct Profile {
    test: bool,
}

/// Copy a declared file tree, dereferencing source symlinks and preserving executability.
fn copy_tree(source: &Path, destination: &Path) -> Result<()> {
    fs::create_dir_all(destination)?;
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let target = destination.join(entry.file_name());
        if entry.path().is_dir() {
            copy_tree(&entry.path(), &target)?;
        } else {
            fs::copy(entry.path(), target)?;
        }
    }
    Ok(())
}

/// Replace both native and slash-separated Windows paths in Cargo command arguments.
fn tokenized(value: &str, work: &Path, sdk: &Path) -> String {
    let mut value = value.to_owned();
    for (path, token) in [(work, "${BUNDLE}"), (sdk, "${SDK}")] {
        let path = path.to_string_lossy();
        // canonicalize() adds this Windows prefix; Cargo generally omits it.
        value = value.replace(path.as_ref(), token);
        let path = path.strip_prefix("\\\\?\\").unwrap_or(&path);
        value = value
            .replace(path, token)
            .replace(&path.replace('\\', "/"), token)
            .replace(&path.replace('/', "\\"), token);
    }
    value
}

/// Export test executables, documentation inputs, and the CLI from one compilation.
pub fn export(work: &Path, sdk: &Path, out: &Path) -> Result<()> {
    ensure!(
        !out.exists(),
        "artifact output already exists: {}",
        out.display()
    );
    let input_work = work.to_owned();
    let input_sdk = sdk.to_owned();
    let work = work.canonicalize()?;
    let sdk = sdk.canonicalize()?;
    fs::create_dir_all(out)?;
    let mut bundle = Bundle::default();
    let deps = out.join("target/debug/deps");
    fs::create_dir_all(&deps)?;

    // Cargo is authoritative about target names and enabled test harnesses. Using
    // its JSON stream avoids guessing executable hashes or maintaining target lists.
    let messages = BufReader::new(fs::File::open(work.join("artifacts.jsonl"))?);
    for line in messages.lines() {
        let message: serde_json::Value = serde_json::from_str(&line?)?;
        if message["reason"] != "compiler-artifact" {
            continue;
        }
        let artifact: Artifact = serde_json::from_value(message)?;
        for path in &artifact.filenames {
            if path.extension().is_some_and(|ext| {
                matches!(ext.to_str(), Some("rlib" | "so" | "dylib" | "dll" | "lib"))
            }) {
                fs::copy(
                    path,
                    deps.join(path.file_name().context("library filename")?),
                )?;
            }
        }
        // Workspace libraries are reported with an unhashed top-level .rlib and
        // a hashed .rmeta. Rustdoc refers to the matching hashed .rlib in deps.
        for path in &artifact.filenames {
            if path.extension().is_some_and(|ext| ext == "rmeta") {
                let library = path.with_extension("rlib");
                if library.is_file() {
                    fs::copy(
                        &library,
                        deps.join(library.file_name().context("library filename")?),
                    )?;
                }
            }
        }
        let Some(executable) = artifact.executable else {
            continue;
        };
        if !artifact.profile.test {
            continue;
        }
        let filename = executable.file_name().context("test executable name")?;
        let path = Path::new("tests").join(filename);
        fs::create_dir_all(out.join("tests"))?;
        fs::copy(&executable, out.join(&path))?;
        bundle.tests.push(TestBinary {
            name: artifact
                .target
                .src_path
                .canonicalize()?
                .strip_prefix(work.join("source"))?
                .to_string_lossy()
                .replace('\\', "/"),
            path: path
                .to_str()
                .context("UTF-8 executable path")?
                .replace('\\', "/"),
            integration: artifact.target.kind.iter().any(|kind| kind == "test"),
            live: artifact.target.name == "github_live",
        });
    }
    ensure!(
        !bundle.tests.is_empty(),
        "Cargo produced no test executables"
    );
    bundle.tests.sort_by(|a, b| a.name.cmp(&b.name));
    bundle.tests.dedup_by(|a, b| a.path == b.path);

    // Rustdoc needs source and link libraries, but none of Cargo's fingerprint,
    // incremental, or registry cache. Native build-script outputs supply -L paths.
    copy_tree(&work.join("source"), &out.join("source"))?;
    copy_tree(
        &work.join("target/debug/build"),
        &out.join("target/debug/build"),
    )?;
    let executable = format!("xtask{}", std::env::consts::EXE_SUFFIX);
    fs::copy(
        work.join("target/debug").join(&executable),
        out.join(&executable),
    )?;

    // Preserve Cargo's exact rustdoc arguments instead of reimplementing feature
    // flags, dependency renames, compiler options, or package environments.
    let mut paths = fs::read_dir(work.join("doctests"))?
        .map(|entry| entry.map(|entry| entry.path()))
        .collect::<std::io::Result<Vec<_>>>()?;
    paths.sort();
    for path in paths {
        let mut doc: DocTest = serde_json::from_slice(&fs::read(path)?)?;
        doc.args = doc
            .args
            .iter()
            .map(|arg| tokenized(&tokenized(arg, &work, &sdk), &input_work, &input_sdk))
            .collect();
        doc.directory = tokenized(
            &tokenized(&doc.directory, &work, &sdk),
            &input_work,
            &input_sdk,
        );
        doc.env = doc
            .env
            .into_iter()
            .map(|(key, value)| {
                (
                    key,
                    tokenized(&tokenized(&value, &work, &sdk), &input_work, &input_sdk),
                )
            })
            .collect();
        bundle.docs.push(doc);
    }
    ensure!(
        !bundle.docs.is_empty(),
        "Cargo produced no documentation test commands"
    );
    fs::write(
        out.join("manifest.json"),
        serde_json::to_vec_pretty(&bundle)?,
    )?;
    Ok(())
}
