//! Complete on-disk exchange lifecycles through serialized CLI records.

use crate::common::*;
use cm_catalog::{CATALOG_PATH, Catalog, RECEIPT_PATH, records};
use cm_exchange_plan::Receipt;
use std::fs;

#[test]
fn full_exchange_preserves_bytes_and_repeated_commands_are_noops() {
    let f = Exchange::new();
    f.import();
    f.receive();
    assert_eq!(
        fs::read(f.recipient.join(DELIVERED)).unwrap(),
        WAST.as_bytes()
    );
    assert_eq!(
        fs::read(f.source.join("tests/input.wast")).unwrap(),
        WAST.as_bytes()
    );
    assert_eq!(
        fs::read_to_string(f.recipient.join("README.md")).unwrap(),
        "Unrelated recipient content\n"
    );
    assert!(success(f.apply(&f.hub, "import.toml")).contains("Applied 0"));
    assert!(success(f.apply(&f.recipient, "receive.toml")).contains("Applied 0"));
    f.import();
    assert!(f.plan("import.toml").edits.is_empty());
    success(f.plan_receive());
    assert!(f.plan("receive.toml").edits.is_empty());
}

#[test]
fn plans_do_not_modify_catalog_or_recipient() {
    let f = Exchange::new();
    success(f.acquire("main", ".cm-testsuite.toml"));
    success(f.plan_import());
    assert!(!f.hub.join(CATALOG_PATH).exists());
    success(f.apply(&f.hub, "import.toml"));
    success(f.plan_receive());
    assert!(!f.recipient.join(DELIVERED).exists());
    assert!(!f.recipient.join(RECEIPT_PATH).exists());
}

#[test]
fn committed_updates_flow_through_to_existing_deliveries() {
    let f = Exchange::new();
    f.import();
    f.receive();
    f.update();
    f.import();
    f.receive();
    assert_eq!(
        fs::read(f.recipient.join(DELIVERED)).unwrap(),
        UPDATED.as_bytes()
    );
    let receipt: Receipt =
        records::decode(&fs::read(f.recipient.join(RECEIPT_PATH)).unwrap()).unwrap();
    assert_eq!(
        receipt.tests.values().next().unwrap().sha256,
        cm_catalog::sha256(UPDATED.as_bytes())
    );
    success(f.plan_receive());
    assert!(f.plan("receive.toml").edits.is_empty());
}

#[test]
fn source_file_moves_keep_received_identity_and_path() {
    let f = Exchange::new();
    f.import();
    f.receive();
    fs::rename(
        f.source.join("tests/input.wast"),
        f.source.join("tests/moved.wast"),
    )
    .unwrap();
    write(
        &f.source,
        ".cm-testsuite.toml",
        manifest("file:///tests/moved.wast", ""),
    );
    commit(&f.source);
    f.import();
    assert_eq!(f.plan("import.toml").edits.len(), 1);
    f.receive();
    assert_eq!(
        fs::read_to_string(f.recipient.join(DELIVERED)).unwrap(),
        WAST
    );
    assert!(
        f.plan("receive.toml")
            .edits
            .iter()
            .all(|e| e.path == RECEIPT_PATH)
    );
}

#[test]
fn retirement_requires_explicit_recipient_permission() {
    let f = Exchange::new();
    f.import();
    f.receive();
    write(&f.source, ".cm-testsuite.toml", "schema = 1\n");
    commit(&f.source);
    f.import();
    assert!(Catalog::load(&f.hub).unwrap().tests.is_empty());
    success(f.plan_receive());
    assert!(f.plan("receive.toml").edits.is_empty());
    assert!(f.recipient.join(DELIVERED).exists());
    write(
        &f.recipient,
        ".cm-testsuite.toml",
        policy("deletions = 'pull-request'"),
    );
    f.receive();
    assert!(!f.recipient.join(DELIVERED).exists());
    let receipt: Receipt =
        records::decode(&fs::read(f.recipient.join(RECEIPT_PATH)).unwrap()).unwrap();
    assert!(receipt.tests.is_empty());
    f.receive();
    assert!(f.plan("receive.toml").edits.is_empty());
}

#[test]
fn restoring_retired_source_delivers_it_again() {
    let f = Exchange::new();
    write(
        &f.recipient,
        ".cm-testsuite.toml",
        policy("deletions = 'pull-request'"),
    );
    f.import();
    f.receive();
    write(&f.source, ".cm-testsuite.toml", "schema = 1\n");
    commit(&f.source);
    f.import();
    f.receive();
    write(
        &f.source,
        ".cm-testsuite.toml",
        manifest("file:///tests/input.wast", ""),
    );
    commit(&f.source);
    f.import();
    f.receive();
    assert!(Catalog::load(&f.hub).unwrap().retired.is_empty());
    assert_eq!(
        fs::read_to_string(f.recipient.join(DELIVERED)).unwrap(),
        WAST
    );
}

#[test]
fn manual_additions_and_updates_are_left_untouched() {
    let f = Exchange::new();
    f.import();
    write(
        &f.recipient,
        ".cm-testsuite.toml",
        policy("additions = 'manual'"),
    );
    success(f.plan_receive());
    assert!(f.plan("receive.toml").edits.is_empty());
    write(
        &f.recipient,
        ".cm-testsuite.toml",
        policy("updates = 'manual'"),
    );
    f.receive();
    f.update();
    f.import();
    success(f.plan_receive());
    assert!(f.plan("receive.toml").edits.is_empty());
    assert_eq!(
        fs::read_to_string(f.recipient.join(DELIVERED)).unwrap(),
        WAST
    );
}

#[test]
fn routes_filter_features_categories_and_unknown_analysis() {
    let f = Exchange::new();
    f.import();
    for rejected in [
        policy("").replace("supports = ['component-model']", "supports = []"),
        policy("") + "covers_any = ['unrelated']\n",
        policy("").replace("allow_unknown = true", "allow_unknown = false"),
    ] {
        write(&f.recipient, ".cm-testsuite.toml", rejected);
        success(f.plan_receive());
        assert!(f.plan("receive.toml").edits.is_empty());
    }
    write(
        &f.recipient,
        ".cm-testsuite.toml",
        policy("") + "covers_any = ['arithmetic']\n",
    );
    f.receive();
    assert!(f.recipient.join(DELIVERED).exists());
}

#[test]
fn commands_report_warning_kinds_with_guidance() {
    let stderr = |output: std::process::Output| {
        assert!(output.status.success());
        String::from_utf8(output.stderr).unwrap()
    };
    let reference = "https://github.com/vados-cosmonic/cm-testsuite/blob/main/docs/src/warnings.md";
    let f = Exchange::new();

    // The record on standard output is unchanged; guidance goes to standard error.
    let analyzed = cli(&f.hub)
        .arg("analyze")
        .arg(f.source.join("tests/input.wast"))
        .output()
        .unwrap();
    let record = String::from_utf8(analyzed.stdout.clone()).unwrap();
    assert!(record.contains("complete = false") && !record.contains("warning["));
    let notes = stderr(analyzed);
    assert!(notes.starts_with("warning[incomplete-feature-inference]: Feature inference"));
    assert!(notes.contains("Reported for every test; nothing is wrong with the file."));
    assert!(notes.contains("Source maintainer: list every capability"));
    assert!(notes.contains(&format!("See {reference}#incomplete-feature-inference")));
    assert!(!notes.contains("missing-intent-comment"));

    success(f.acquire("main", ".cm-testsuite.toml"));
    let planned = stderr(f.plan_import());
    assert!(planned.starts_with("warning[incomplete-feature-inference]: "));
    assert!(planned.contains(&format!(
        "(1); see {reference}#incomplete-feature-inference"
    )));
    // Stored plan warnings keep their text, which plan digests cover.
    assert_eq!(
        f.plan("import.toml").warnings,
        [format!(
            "{SOURCE}/nested/test: Feature inference is incomplete; \
             syntax evidence is not a complete execution requirement set."
        )]
    );
    success(f.apply(&f.hub, "import.toml"));

    assert!(stderr(f.plan_receive()).starts_with("warning[accepted-incomplete-inference]: "));
    write(
        &f.recipient,
        ".cm-testsuite.toml",
        policy("").replace("allow_unknown = true", "allow_unknown = false"),
    );
    assert!(stderr(f.plan_receive()).starts_with("warning[route-needs-allow-unknown]: "));
    assert_eq!(
        f.plan("receive.toml").warnings,
        [format!(
            "{SOURCE}/nested/test: route shared needs explicit allow_unknown \
             for incomplete feature inference"
        )]
    );
}

#[test]
fn disabled_receiving_rejects_plan_without_output() {
    let f = Exchange::new();
    f.import();
    write(
        &f.recipient,
        ".cm-testsuite.toml",
        policy("").replace("enabled = true", "enabled = false"),
    );
    failure(f.plan_receive(), "has not opted into receiving");
    assert!(!f.artifact("receive.toml").exists());
}

#[test]
fn nested_recipient_manifest_resolves_relative_destination() {
    let f = Exchange::new();
    f.import();
    write(
        &f.recipient,
        "config/receive.toml",
        policy("").replace("file:///tests/shared", "file://received"),
    );
    success(
        cli(&f.recipient)
            .args([
                "plan-receive",
                "--recipient",
                RECIPIENT,
                "--manifest",
                "config/receive.toml",
                "--catalog-root",
            ])
            .arg(&f.hub)
            .arg("--out")
            .arg(f.artifact("receive.toml"))
            .output()
            .unwrap(),
    );
    success(f.apply(&f.recipient, "receive.toml"));
    assert_eq!(
        fs::read_to_string(
            f.recipient
                .join("config/received/example/source/nested/test.wast")
        )
        .unwrap(),
        WAST
    );
    assert!(f.recipient.join(RECEIPT_PATH).exists());
}

#[test]
fn route_move_preserves_unmanaged_neighbors() {
    let f = Exchange::new();
    f.import();
    f.receive();
    write(&f.recipient, "tests/shared/unmanaged.wast", "(component)\n");
    write(
        &f.recipient,
        ".cm-testsuite.toml",
        policy("").replace("tests/shared", "tests/new location"),
    );
    f.receive();
    assert!(!f.recipient.join(DELIVERED).exists());
    assert_eq!(
        fs::read_to_string(
            f.recipient
                .join("tests/new location/example/source/nested/test.wast")
        )
        .unwrap(),
        WAST
    );
    assert_eq!(
        fs::read_to_string(f.recipient.join("tests/shared/unmanaged.wast")).unwrap(),
        "(component)\n"
    );
}

#[test]
fn stale_policy_and_receipt_block_delivery_without_partial_writes() {
    let f = Exchange::new();
    f.import();
    success(f.plan_receive());
    write(
        &f.recipient,
        ".cm-testsuite.toml",
        policy("updates = 'manual'"),
    );
    failure(f.apply(&f.recipient, "receive.toml"), "stale plan");
    assert!(!f.recipient.join(DELIVERED).exists());
    success(f.plan_receive());
    write(
        &f.recipient,
        RECEIPT_PATH,
        "# Concurrent receipt\nschema = 1\n",
    );
    failure(f.apply(&f.recipient, "receive.toml"), "stale plan");
    assert!(!f.recipient.join(DELIVERED).exists());
    assert_eq!(
        fs::read_to_string(f.recipient.join(RECEIPT_PATH)).unwrap(),
        "# Concurrent receipt\nschema = 1\n"
    );
}

#[test]
fn corruption_and_missing_catalog_files_block_validation_and_delivery() {
    let f = Exchange::new();
    f.import();
    let path = Catalog::load(&f.hub)
        .unwrap()
        .tests
        .values()
        .next()
        .unwrap()
        .destination
        .clone();
    write(&f.hub, &path, "(module (func))");
    assert!(
        !cli(&f.hub)
            .arg("validate")
            .output()
            .unwrap()
            .status
            .success()
    );
    assert!(!f.plan_receive().status.success());
    fs::remove_file(f.hub.join(&path)).unwrap();
    assert!(
        !cli(&f.hub)
            .arg("validate")
            .output()
            .unwrap()
            .status
            .success()
    );
    assert!(!f.plan_receive().status.success());
    assert!(!f.artifact("receive.toml").exists());
}

#[test]
fn edited_or_deleted_received_file_is_not_silently_replaced() {
    let f = Exchange::new();
    f.import();
    f.receive();
    f.update();
    f.import();
    write(&f.recipient, DELIVERED, "local work");
    failure(f.plan_receive(), "divergence");
    assert_eq!(
        fs::read_to_string(f.recipient.join(DELIVERED)).unwrap(),
        "local work"
    );
    fs::remove_file(f.recipient.join(DELIVERED)).unwrap();
    failure(f.plan_receive(), "divergence");
    assert!(!f.recipient.join(DELIVERED).exists());
}

#[test]
fn unowned_collision_preserves_even_byte_identical_existing_files() {
    let f = Exchange::new();
    f.import();
    write(&f.recipient, DELIVERED, WAST);
    failure(f.plan_receive(), "unowned collision");
    assert!(!f.recipient.join(RECEIPT_PATH).exists());
    assert_eq!(
        fs::read_to_string(f.recipient.join(DELIVERED)).unwrap(),
        WAST
    );
}

#[test]
fn tampered_snapshot_is_rejected_before_import_plan_is_written() {
    let f = Exchange::new();
    success(f.acquire("main", ".cm-testsuite.toml"));
    let mut snapshot = f.snapshot();
    *snapshot.files.values_mut().next().unwrap() = UPDATED.into();
    fs::write(
        f.artifact("snapshot.toml"),
        records::encode(&snapshot).unwrap(),
    )
    .unwrap();
    assert!(!f.plan_import().status.success());
    assert!(!f.artifact("import.toml").exists());
    assert!(!f.hub.join(CATALOG_PATH).exists());
}

#[test]
fn independent_sources_share_names_without_overwriting_or_retiring_each_other() {
    let a = Exchange::new();
    let b = Exchange::new();
    success(a.acquire("main", ".cm-testsuite.toml"));
    success(
        cli(&a.hub)
            .args(["acquire", "--source", "github:example/other", "--local"])
            .arg(&b.source)
            .arg("--out")
            .arg(a.artifact("other.toml"))
            .output()
            .unwrap(),
    );
    success(
        cli(&a.hub)
            .args(["plan-import", "--snapshot"])
            .arg(a.artifact("snapshot.toml"))
            .arg("--snapshot")
            .arg(a.artifact("other.toml"))
            .arg("--out")
            .arg(a.artifact("import.toml"))
            .output()
            .unwrap(),
    );
    success(a.apply(&a.hub, "import.toml"));
    a.receive();
    let other = "tests/shared/example/other/nested/test.wast";
    assert_eq!(fs::read_to_string(a.recipient.join(other)).unwrap(), WAST);
    assert_eq!(Catalog::load(&a.hub).unwrap().tests.len(), 2);

    // Refreshing one producer must not retire the other producer's inventory.
    a.update();
    a.import();
    a.receive();
    assert_eq!(Catalog::load(&a.hub).unwrap().tests.len(), 2);
    assert_eq!(fs::read_to_string(a.recipient.join(other)).unwrap(), WAST);
    assert_eq!(
        fs::read_to_string(a.recipient.join(DELIVERED)).unwrap(),
        UPDATED
    );
}
