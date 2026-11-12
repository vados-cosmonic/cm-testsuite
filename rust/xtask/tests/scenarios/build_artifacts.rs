//! Verify suite filtering, failure propagation, and missing compiler outputs.

use super::build_artifacts::{
    Bundle, DocTest, TestBinary, export,
    run::{self, Suite},
};
use std::{fs, path::Path};

/// Make a relocatable bundle whose harness is this integration-test executable.
fn fixture(root: &Path, live: bool) {
    fs::create_dir_all(root).unwrap();
    fs::copy(
        std::env::current_exe().unwrap(),
        root.join(format!("harness{}", std::env::consts::EXE_SUFFIX)),
    )
    .unwrap();
    fs::write(
        root.join("manifest.json"),
        serde_json::to_vec(&Bundle {
            tests: vec![TestBinary {
                name: "fixture".into(),
                path: format!("harness{}", std::env::consts::EXE_SUFFIX),
                directory: ".".into(),
                integration: true,
                live,
            }],
            docs: vec![],
        })
        .unwrap(),
    )
    .unwrap();
}

/// Only an explicit child invocation runs this failing test.
#[test]
#[ignore = "failure fixture for the artifact runner"]
fn failure_fixture() {
    panic!("intentional child harness failure");
}

#[test]
fn relocated_harness_failure_reaches_the_buck_runner() {
    let temp = tempfile::tempdir().unwrap();
    let original = temp.path().join("original");
    let moved = temp.path().join("relocated bundle café");
    fixture(&original, false);
    fs::rename(original, &moved).unwrap();
    let output = super::common::cli(temp.path())
        .args([
            "buck-artifacts",
            "test",
            "--suite",
            "integration",
            "--bundle",
        ])
        .arg(&moved)
        .arg("--sdk")
        .arg(temp.path())
        .args(["--", "--ignored", "--exact", "scenarios::failure_fixture"])
        .output()
        .unwrap();
    super::common::failure(output, "failed test targets: fixture");
}

#[test]
fn ordinary_suites_exclude_live_publication_harnesses() {
    let temp = tempfile::tempdir().unwrap();
    fixture(temp.path(), true);
    // Delete the binary to ensure an accidental selection cannot silently pass.
    fs::remove_file(
        temp.path()
            .join(format!("harness{}", std::env::consts::EXE_SUFFIX)),
    )
    .unwrap();
    let error = run::run(temp.path(), temp.path(), Suite::Integration, &[]).unwrap_err();
    assert!(error.to_string().contains("no tests matched"));
}

#[test]
fn export_rejects_a_build_without_test_artifacts() {
    let temp = tempfile::tempdir().unwrap();
    fs::write(
        temp.path().join("artifacts.jsonl"),
        "{\"reason\":\"build-finished\",\"success\":true}\n",
    )
    .unwrap();
    let error = export::export(temp.path(), temp.path(), &temp.path().join("out")).unwrap_err();
    assert!(
        error
            .to_string()
            .contains("Cargo produced no test executables")
    );
}

#[test]
fn rustdoc_arguments_accept_mixed_separators_after_windows_relocation() {
    let bundle = Path::new(r"\\?\D:\build artifacts");
    let sdk = Path::new(r"\\?\UNC\server\sdk");
    assert_eq!(
        run::expanded(r"dependency=${BUNDLE}/target\debug\deps", bundle, sdk),
        r"dependency=D:\build artifacts/target\debug\deps",
    );
    assert_eq!(
        run::expanded("${SDK}/bin/rustc.exe", bundle, sdk),
        r"\\server\sdk/bin/rustc.exe"
    );
}

#[test]
#[ignore = "child harness for checking Cargo's package context"]
fn package_context_fixture() {
    assert_eq!(
        fs::read_to_string("fixture.txt").unwrap(),
        "package fixture"
    );
    let directory = std::env::current_dir().unwrap().canonicalize().unwrap();
    assert_eq!(
        Path::new(&std::env::var_os("CARGO_MANIFEST_DIR").unwrap())
            .canonicalize()
            .unwrap(),
        directory,
    );
    assert_eq!(
        Path::new(&std::env::var_os("CARGO_MANIFEST_PATH").unwrap())
            .canonicalize()
            .unwrap(),
        directory.join("Cargo.toml"),
    );
}

#[test]
fn exported_harness_preserves_package_context_after_relocation() {
    let temp = tempfile::tempdir().unwrap();
    let work = temp.path().join("work");
    let source = work.join("source");
    let package = source.join("crates/example");
    fs::create_dir_all(&package).unwrap();
    fs::write(package.join("Cargo.toml"), "[package]\nname = 'example'\n").unwrap();
    fs::write(package.join("fixture.txt"), "package fixture").unwrap();
    // A target's source need not be below its owning package directory.
    fs::create_dir_all(source.join("shared-tests")).unwrap();
    let src_path = source.join("shared-tests/context.rs");
    fs::write(&src_path, "").unwrap();
    let executable = work.join(format!("harness{}", std::env::consts::EXE_SUFFIX));
    fs::copy(std::env::current_exe().unwrap(), &executable).unwrap();
    fs::write(
        work.join("artifacts.jsonl"),
        serde_json::to_vec(&serde_json::json!({
            "reason": "compiler-artifact",
            "manifest_path": package.join("Cargo.toml"),
            "filenames": [],
            "executable": executable,
            "profile": {"test": true},
            "target": {"name": "context", "kind": ["test"], "src_path": src_path},
        }))
        .unwrap(),
    )
    .unwrap();
    fs::create_dir_all(work.join("target/debug/build")).unwrap();
    fs::copy(
        std::env::var_os("CM_TESTSUITE_XTASK")
            .unwrap_or_else(|| env!("CARGO_BIN_EXE_xtask").into()),
        work.join(format!(
            "target/debug/xtask{}",
            std::env::consts::EXE_SUFFIX
        )),
    )
    .unwrap();
    fs::create_dir_all(work.join("doctests")).unwrap();
    fs::write(
        work.join("doctests/example.json"),
        serde_json::to_vec(&DocTest {
            args: vec![],
            directory: source.to_str().unwrap().into(),
            env: Default::default(),
        })
        .unwrap(),
    )
    .unwrap();
    let original = temp.path().join("original");
    export::export(&work, temp.path(), &original).unwrap();
    let moved = temp.path().join("relocated bundle café");
    fs::rename(original, &moved).unwrap();
    fs::remove_dir_all(work).unwrap();
    let output = super::common::cli(temp.path())
        .current_dir(temp.path())
        .args([
            "buck-artifacts",
            "test",
            "--suite",
            "integration",
            "--bundle",
        ])
        .arg(&moved)
        .arg("--sdk")
        .arg(temp.path())
        .args([
            "--",
            "--ignored",
            "--exact",
            "scenarios::package_context_fixture",
        ])
        .output()
        .unwrap();
    super::common::success(output);
}

#[test]
#[ignore = "child harness for checking offline credentials"]
fn offline_environment_fixture() {
    assert!(std::env::var_os("GH_TOKEN").is_none());
    assert!(std::env::var_os("GITHUB_TOKEN").is_none());
}

#[test]
fn documentation_suite_removes_publication_credentials() {
    let temp = tempfile::tempdir().unwrap();
    let sdk = temp.path().join("sdk");
    fs::create_dir_all(sdk.join("bin")).unwrap();
    // Stand in for rustdoc with a harness that checks its inherited environment.
    fs::copy(
        std::env::current_exe().unwrap(),
        sdk.join(format!("bin/rustdoc{}", std::env::consts::EXE_SUFFIX)),
    )
    .unwrap();
    fs::write(
        temp.path().join("manifest.json"),
        serde_json::to_vec(&Bundle {
            tests: vec![],
            docs: vec![DocTest {
                args: vec![
                    "--ignored".into(),
                    "--exact".into(),
                    "scenarios::offline_environment_fixture".into(),
                ],
                directory: "${BUNDLE}".into(),
                env: Default::default(),
            }],
        })
        .unwrap(),
    )
    .unwrap();
    let output = super::common::cli(temp.path())
        .env("GH_TOKEN", "dummy-gh-token")
        .env("GITHUB_TOKEN", "dummy-github-token")
        .args(["buck-artifacts", "test", "--suite", "doc", "--bundle"])
        .arg(temp.path())
        .arg("--sdk")
        .arg(sdk)
        .output()
        .unwrap();
    super::common::success(output);
}
