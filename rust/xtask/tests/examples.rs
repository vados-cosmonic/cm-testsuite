//! Every checked-in example directory participates in the offline CLI exchange.

use std::{path::PathBuf, process::Command};

#[test]
fn all_example_projects_exchange_tests_repeatably() {
    let fixtures = std::env::var_os("CM_TESTSUITE_EXAMPLES")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../examples"));
    let output = tempfile::tempdir().unwrap();

    // Run twice against the same output to verify persisted snapshots and receipts.
    for _ in 0..2 {
        let result = Command::new(env!("CARGO_BIN_EXE_xtask"))
            .env("CM_TESTSUITE_EXAMPLES", &fixtures)
            .args(["demo", "--out"])
            .arg(output.path().join("exchange"))
            .output()
            .unwrap();
        assert!(
            result.status.success(),
            "{}\n{}",
            String::from_utf8_lossy(&result.stdout),
            String::from_utf8_lossy(&result.stderr)
        );
    }
}

#[test]
fn report_admission_keeps_comments_and_rejects_unknown_publishers() {
    use cm_catalog::{Catalog, digest, records};
    use cm_results::{Publishers, Report};
    use std::{collections::BTreeSet, fs};

    let root = tempfile::tempdir().unwrap();
    let publishers = root.path().join("publishers.toml");
    fs::write(
        &publishers,
        records::encode(&Publishers {
            schema: 1,
            publishers: BTreeSet::from(["example".into()]),
        })
        .unwrap(),
    )
    .unwrap();
    let report = Report {
        schema: 1,
        publisher: "example".into(),
        engine: "engine".into(),
        engine_version: "1".into(),
        run_id: "run".into(),
        catalog_sha256: digest(&Catalog::default()).unwrap(),
        reported_at: "2026-11-11T00:00:00Z".into(),
        platform: "test-host".into(),
        configuration: "default".into(),
        results: vec![],
    };
    let submitted = root.path().join("report.toml");
    fs::write(
        &submitted,
        format!(
            "# Harness configuration notes\n{}",
            records::encode(&report).unwrap()
        ),
    )
    .unwrap();
    let admit = || {
        Command::new(env!("CARGO_BIN_EXE_xtask"))
            .arg("--root")
            .arg(root.path())
            .arg("results-import")
            .arg("--report")
            .arg(&submitted)
            .arg("--publishers")
            .arg(&publishers)
            .output()
            .unwrap()
    };
    assert!(admit().status.success());
    let output = root
        .path()
        .join("reports")
        .join(&report.catalog_sha256)
        .join(format!("{}.toml", digest(&report).unwrap()));
    let accepted = fs::read_to_string(&output).unwrap();
    assert!(accepted.contains("# Harness configuration notes"));
    assert!(admit().status.success());
    assert_eq!(fs::read_to_string(output).unwrap(), accepted);

    fs::write(&publishers, "schema = 1\npublishers = []\n").unwrap();
    assert!(!admit().status.success());
}
