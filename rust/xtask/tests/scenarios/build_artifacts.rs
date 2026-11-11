//! Verify suite filtering, failure propagation, and missing compiler outputs.

use super::build_artifacts::{
    Bundle, TestBinary, export,
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
