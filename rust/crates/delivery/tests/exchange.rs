//! Exchange behavior across imports, receiving policies, and catalog changes.

mod common;

use cm_acquisition::{acquire, load_catalog_files};
use cm_catalog::{Catalog, Manifest, digest, records, source_path};
use cm_delivery::apply;
use cm_exchange_plan::{Receipt, import, receive};
use common::exchange::*;
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
};

#[test]
fn independent_sources_same_name_and_retirement() {
    let d = tempfile::tempdir().unwrap();
    let a = snapshot("a", "x.wast", "(module)");
    let b = snapshot("b", "x.wast", "(component)");
    apply(d.path(), &import(d.path(), &[a.clone(), b]).unwrap()).unwrap();
    assert_eq!(Catalog::load(d.path()).unwrap().tests.len(), 2);
    let mut empty = a;
    empty.tests.clear();
    empty.files.clear();
    apply(d.path(), &import(d.path(), &[empty]).unwrap()).unwrap();
    let c = Catalog::load(d.path()).unwrap();
    assert_eq!(c.tests.len(), 1);
    assert_eq!(c.retired.len(), 1);
    assert!(c.tests.keys().all(|id| id.starts_with("github:example/b/")));
}

#[test]
fn per_file_pin_and_integrity_constraint() {
    let source = Source {
        manifest: format!(
            "schema=1\n[[test]]\nname='old'\npath='file:///a.wast'\nrevision='{}'\n[[test]]\nname='new'\npath='file:///a.wast'\n",
            "a".repeat(40)
        ),
        files: BTreeMap::from([("a.wast".into(), "(component)".into())]),
    };
    let s = acquire(&source, "github:example/a", "main", ".cm-testsuite.toml").unwrap();
    assert_ne!(
        s.tests["github:example/a/old"].sha256,
        s.tests["github:example/a/new"].sha256
    );
    let bad = Source {
        manifest: format!(
            "schema=1\n[[test]]\nname='bad'\npath='file:///a.wast'\nsha256='{}'\n",
            "0".repeat(64)
        ),
        files: source.files,
    };
    assert!(acquire(&bad, "github:example/a", "main", ".cm-testsuite.toml").is_err());
}

#[test]
fn stable_name_survives_source_file_move() {
    let d = tempfile::tempdir().unwrap();
    let before = snapshot("a", "old.wast", "(module)");
    let after = snapshot("a", "new.wast", "(module)");
    setup(d.path(), &before);
    let p = import(d.path(), &[after]).unwrap();
    assert_eq!(p.edits.len(), 1);
    assert_eq!(p.edits[0].path, source_path("github:example/a").unwrap());
    apply(d.path(), &p).unwrap();
    assert_eq!(
        Catalog::load(d.path())
            .unwrap()
            .tests
            .values()
            .next()
            .unwrap()
            .source_path,
        "new.wast"
    );
}

#[test]
fn exclusions_preserve_received_files_and_survive_content_updates() {
    let hub = tempfile::tempdir().unwrap();
    let target = tempfile::tempdir().unwrap();
    let a = snapshot("a", "x.wast", "(module)");
    setup(hub.path(), &a);
    policy(target.path(), "");
    apply(target.path(), &received(target.path(), hub.path())).unwrap();
    policy(target.path(), "exclude=['github:example/a/stable-name']");
    setup(hub.path(), &snapshot("a", "x.wast", "(component)"));
    assert!(received(target.path(), hub.path()).edits.is_empty());
    let receipt: Receipt =
        records::decode(&fs::read(target.path().join(".cm-testsuite-lock.toml")).unwrap()).unwrap();
    assert_eq!(
        fs::read_to_string(
            target
                .path()
                .join(&receipt.tests.values().next().unwrap().path)
        )
        .unwrap(),
        "(module)"
    );
}

#[test]
fn removal_requires_retirement_and_permission() {
    let hub = tempfile::tempdir().unwrap();
    let target = tempfile::tempdir().unwrap();
    let mut a = snapshot("a", "x.wast", "(module)");
    setup(hub.path(), &a);
    policy(target.path(), "");
    apply(target.path(), &received(target.path(), hub.path())).unwrap();
    let empty = Catalog::default();
    assert!(
        receive(
            target.path(),
            "github:example/recipient",
            ".cm-testsuite.toml",
            &empty,
            &BTreeMap::new()
        )
        .unwrap()
        .edits
        .is_empty()
    );
    a.tests.clear();
    a.files.clear();
    setup(hub.path(), &a);
    assert!(received(target.path(), hub.path()).edits.is_empty());
    policy(target.path(), "deletions='pull-request'");
    let p = received(target.path(), hub.path());
    assert!(p.edits.iter().any(|e| e.after.is_none()));
    apply(target.path(), &p).unwrap();
    assert!(received(target.path(), hub.path()).edits.is_empty());
}

#[test]
fn local_divergence_blocks_update_and_route_move() {
    let hub = tempfile::tempdir().unwrap();
    let target = tempfile::tempdir().unwrap();
    setup(hub.path(), &snapshot("a", "x.wast", "(module)"));
    policy(target.path(), "");
    apply(target.path(), &received(target.path(), hub.path())).unwrap();
    let policy_path = target.path().join(".cm-testsuite.toml");
    let changed = fs::read_to_string(&policy_path)
        .unwrap()
        .replace("tests/shared", "tests/moved");
    fs::write(&policy_path, changed).unwrap();
    let p = received(target.path(), hub.path());
    assert!(p.edits.iter().any(|e| e.after.is_none()));
    apply(target.path(), &p).unwrap();
    let receipt: Receipt =
        records::decode(&fs::read(target.path().join(".cm-testsuite-lock.toml")).unwrap()).unwrap();
    fs::write(
        target
            .path()
            .join(&receipt.tests.values().next().unwrap().path),
        "local changes",
    )
    .unwrap();
    setup(hub.path(), &snapshot("a", "x.wast", "(component)"));
    let c = Catalog::load(hub.path()).unwrap();
    assert!(
        receive(
            target.path(),
            "github:example/recipient",
            ".cm-testsuite.toml",
            &c,
            &load_catalog_files(hub.path(), &c).unwrap()
        )
        .is_err()
    );
}

#[test]
fn ambiguous_routes_and_incomplete_inference_are_explicit() {
    let hub = tempfile::tempdir().unwrap();
    let target = tempfile::tempdir().unwrap();
    let c = setup(hub.path(), &snapshot("a", "x.wast", "(module)"));
    policy(target.path(), "");
    let path = target.path().join(".cm-testsuite.toml");
    let text = fs::read_to_string(&path).unwrap();
    fs::write(
        &path,
        text.replace("allow_unknown=true", "allow_unknown=false"),
    )
    .unwrap();
    assert!(received(target.path(), hub.path()).edits.is_empty());
    fs::write(&path,text+"\n[[receive.route]]\nname='duplicate'\ndestination='file:///tests/other'\nsupports=['component-model']\nallow_unknown=true\n").unwrap();
    assert!(
        receive(
            target.path(),
            "github:example/recipient",
            ".cm-testsuite.toml",
            &c,
            &load_catalog_files(hub.path(), &c).unwrap()
        )
        .is_err()
    );
}

#[test]
fn unavailable_source_cannot_become_deletion() {
    let d = tempfile::tempdir().unwrap();
    setup(d.path(), &snapshot("a", "x.wast", "(module)"));
    let before = fs::read(d.path().join(source_path("github:example/a").unwrap())).unwrap();
    assert!(acquire(&Gone, "github:example/a", "main", ".cm-testsuite.toml").is_err());
    assert_eq!(
        fs::read(d.path().join(source_path("github:example/a").unwrap())).unwrap(),
        before
    );
}

#[test]
fn result_identity_digests_and_duplicates_are_checked() {
    let hub = tempfile::tempdir().unwrap();
    let c = setup(hub.path(), &snapshot("a", "x.wast", "(module)"));
    let (id, test) = c.tests.iter().next().unwrap();
    let mut r = cm_results::Report {
        schema: 1,
        publisher: "test-engine".into(),
        engine: "test-engine".into(),
        engine_version: "1".into(),
        run_id: "abc".into(),
        catalog_sha256: digest(&c).unwrap(),
        reported_at: "2026-11-05T00:00:00Z".into(),
        platform: "linux".into(),
        configuration: "default".into(),
        results: vec![cm_results::Observation {
            id: id.clone(),
            sha256: test.sha256.clone(),
            status: cm_results::Status::Pass,
            diagnostic: "<script>alert(1)</script>".into(),
        }],
    };
    let publishers = BTreeSet::from(["test-engine".into()]);
    r.validate(&c, &publishers).unwrap();
    assert!(
        !cm_results::render(&c, &[r.clone()])
            .unwrap()
            .contains("<script>")
    );
    r.results.push(r.results[0].clone());
    assert!(r.validate(&c, &publishers).is_err());
    r.results.pop();
    r.results[0].sha256 = "0".repeat(64);
    assert!(r.validate(&c, &publishers).is_err());
    r.results.clear();
    r.catalog_sha256 = "0".repeat(64);
    assert!(r.validate(&c, &publishers).is_err());
}

#[test]
fn incoming_paths_cannot_be_declared_as_authoritative() {
    let m = "schema=1\n[[test]]\nname='loop'\npath='file:///received/a.wast'\n[receive]\nenabled=true\n[[receive.route]]\nname='default'\ndestination='file:///received'\n";
    Manifest::parse(m, ".cm-testsuite.toml").unwrap();
    let s = Source {
        manifest: m.into(),
        files: BTreeMap::from([("received/a.wast".into(), "(module)".into())]),
    };
    assert!(acquire(&s, "github:example/a", "main", ".cm-testsuite.toml").is_err());
}

#[test]
fn duplicate_sources_do_not_mutate() {
    let d = tempfile::tempdir().unwrap();
    let a = snapshot("a", "x.wast", "(module)");
    assert!(import(d.path(), &[a.clone(), a]).is_err());
    assert!(!d.path().join("metadata").exists());
}

#[test]
fn snapshot_tampering_is_rejected() {
    let mut s = snapshot("a", "x.wast", "(module)");
    s.files
        .values_mut()
        .next()
        .unwrap()
        .push_str("\n(component)");
    assert!(s.validate().is_err());
    assert!(records::encode(&s).is_ok());
}

#[test]
fn route_exclusion_must_preserve_retired_file() {
    let hub = tempfile::tempdir().unwrap();
    let target = tempfile::tempdir().unwrap();
    let mut a = snapshot("a", "x.wast", "(module)");
    setup(hub.path(), &a);
    policy(target.path(), "deletions='pull-request'");
    apply(target.path(), &received(target.path(), hub.path())).unwrap();
    let path = target.path().join(".cm-testsuite.toml");
    let text = fs::read_to_string(&path).unwrap();
    fs::write(&path, text + "exclude=['github:example/a/stable-name']\n").unwrap();
    a.tests.clear();
    a.files.clear();
    setup(hub.path(), &a);
    let p = received(target.path(), hub.path());
    assert!(
        p.edits.is_empty(),
        "Route-excluded test was scheduled for deletion: {:?}",
        p.edits
    );
}

#[test]
fn retirement_exclusion_follows_route_after_destination_moves() {
    let hub = tempfile::tempdir().unwrap();
    let target = tempfile::tempdir().unwrap();
    let mut a = snapshot("a", "x.wast", "(module)");
    setup(hub.path(), &a);
    policy(target.path(), "deletions='pull-request'");
    apply(target.path(), &received(target.path(), hub.path())).unwrap();
    let path = target.path().join(".cm-testsuite.toml");
    let text = fs::read_to_string(&path)
        .unwrap()
        .replace("tests/shared", "tests/moved");
    fs::write(&path, text + "exclude=['github:example/a/stable-name']\n").unwrap();
    a.tests.clear();
    a.files.clear();
    setup(hub.path(), &a);
    assert!(received(target.path(), hub.path()).edits.is_empty());
}

#[test]
fn legacy_receipts_honor_exclusions_before_and_after_route_moves() {
    for moved in [false, true] {
        let hub = tempfile::tempdir().unwrap();
        let target = tempfile::tempdir().unwrap();
        let mut a = snapshot("a", "x.wast", "(module)");
        setup(hub.path(), &a);
        policy(target.path(), "deletions='pull-request'");
        apply(target.path(), &received(target.path(), hub.path())).unwrap();
        let receipt_path = target.path().join(".cm-testsuite-lock.toml");
        let mut receipt: toml::Value = records::decode(&fs::read(&receipt_path).unwrap()).unwrap();
        for (_, test) in receipt["tests"].as_table_mut().unwrap().iter_mut() {
            test.as_table_mut().unwrap().remove("route");
        }
        fs::write(&receipt_path, records::encode(&receipt).unwrap()).unwrap();
        let path = target.path().join(".cm-testsuite.toml");
        let mut text = fs::read_to_string(&path).unwrap();
        if moved {
            text = text.replace("tests/shared", "tests/moved");
        }
        fs::write(&path, text + "exclude=['github:example/a/stable-name']\n").unwrap();
        a.tests.clear();
        a.files.clear();
        setup(hub.path(), &a);
        assert!(received(target.path(), hub.path()).edits.is_empty());
    }
}

#[test]
fn unrelated_route_exclusion_does_not_block_retirement() {
    for legacy in [false, true] {
        let hub = tempfile::tempdir().unwrap();
        let target = tempfile::tempdir().unwrap();
        let mut a = snapshot("a", "x.wast", "(module)");
        setup(hub.path(), &a);
        policy(target.path(), "deletions='pull-request'");
        apply(target.path(), &received(target.path(), hub.path())).unwrap();
        if legacy {
            let path = target.path().join(".cm-testsuite-lock.toml");
            let mut receipt: toml::Value = records::decode(&fs::read(&path).unwrap()).unwrap();
            for (_, test) in receipt["tests"].as_table_mut().unwrap().iter_mut() {
                test.as_table_mut().unwrap().remove("route");
            }
            fs::write(&path, records::encode(&receipt).unwrap()).unwrap();
        }
        let path = target.path().join(".cm-testsuite.toml");
        let text = fs::read_to_string(&path).unwrap();
        fs::write(&path, text + "\n[[receive.route]]\nname='other'\ndestination='file:///tests/other'\nexclude=['github:example/a/stable-name']\n").unwrap();
        a.tests.clear();
        a.files.clear();
        setup(hub.path(), &a);
        let p = received(target.path(), hub.path());
        assert!(
            p.edits
                .iter()
                .any(|e| e.path.ends_with(".wast") && e.after.is_none())
        );
        apply(target.path(), &p).unwrap();
        assert!(received(target.path(), hub.path()).edits.is_empty());
    }
}

#[test]
fn retirement_must_protect_authoritative_paths() {
    let hub = tempfile::tempdir().unwrap();
    let target = tempfile::tempdir().unwrap();
    let mut a = snapshot("a", "x.wast", "(module)");
    setup(hub.path(), &a);
    policy(target.path(), "deletions='pull-request'");
    apply(target.path(), &received(target.path(), hub.path())).unwrap();
    fs::write(target.path().join(".cm-testsuite.toml"),
        "schema=1\n[[test]]\nname='own-test'\npath='file:///tests/shared/example/a/stable-name.wast'\n[receive]\nenabled=true\ndeletions='pull-request'\n[[receive.route]]\nname='smoke'\ndestination='file:///tests/new'\nsupports=['component-model']\nallow_unknown=true\n"
    ).unwrap();
    a.tests.clear();
    a.files.clear();
    setup(hub.path(), &a);
    let c = Catalog::load(hub.path()).unwrap();
    let result = receive(
        target.path(),
        "github:example/recipient",
        ".cm-testsuite.toml",
        &c,
        &BTreeMap::new(),
    );
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("authoritative test")
    );
    assert_eq!(
        fs::read_to_string(
            target
                .path()
                .join("tests/shared/example/a/stable-name.wast")
        )
        .unwrap(),
        "(module)"
    );
}

#[test]
fn route_move_must_protect_authoritative_paths() {
    let hub = tempfile::tempdir().unwrap();
    let target = tempfile::tempdir().unwrap();
    setup(hub.path(), &snapshot("a", "x.wast", "(module)"));
    policy(target.path(), "");
    apply(target.path(), &received(target.path(), hub.path())).unwrap();
    fs::write(target.path().join(".cm-testsuite.toml"),
        "schema=1\n[[test]]\nname='own-test'\npath='file:///tests/shared/example/a/stable-name.wast'\n[receive]\nenabled=true\n[[receive.route]]\nname='smoke'\ndestination='file:///tests/new'\nsupports=['component-model']\nallow_unknown=true\n"
    ).unwrap();
    let c = Catalog::load(hub.path()).unwrap();
    let result = receive(
        target.path(),
        "github:example/recipient",
        ".cm-testsuite.toml",
        &c,
        &load_catalog_files(hub.path(), &c).unwrap(),
    );
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("authoritative test")
    );
    assert_eq!(
        fs::read_to_string(
            target
                .path()
                .join("tests/shared/example/a/stable-name.wast")
        )
        .unwrap(),
        "(module)"
    );
    assert!(
        !target
            .path()
            .join("tests/new/example/a/stable-name.wast")
            .exists()
    );
}

#[test]
fn catalog_and_receipt_updates_preserve_maintainer_comments() {
    let hub = tempfile::tempdir().unwrap();
    let target = tempfile::tempdir().unwrap();
    let first = snapshot("source", "test.wast", ";; Original test intent\n(module)\n");
    setup(hub.path(), &first);
    policy(target.path(), "");
    apply(target.path(), &received(target.path(), hub.path())).unwrap();

    // Comments belong to maintainers and must survive real metadata reconciliation.
    for path in [
        hub.path()
            .join(source_path("github:example/source").unwrap()),
        target.path().join(cm_catalog::RECEIPT_PATH),
    ] {
        let text = fs::read_to_string(&path).unwrap();
        fs::write(
            &path,
            format!(
                "# Maintainer context\n{}",
                text.replace("schema = 1", "schema = 1 # Keep this explanation")
            ),
        )
        .unwrap();
    }
    assert!(
        import(hub.path(), std::slice::from_ref(&first))
            .unwrap()
            .edits
            .is_empty()
    );
    assert!(received(target.path(), hub.path()).edits.is_empty());

    let updated = snapshot(
        "source",
        "test.wast",
        ";; Updated test intent\n(module (func))\n",
    );
    apply(hub.path(), &import(hub.path(), &[updated]).unwrap()).unwrap();
    apply(target.path(), &received(target.path(), hub.path())).unwrap();
    for path in [
        hub.path()
            .join(source_path("github:example/source").unwrap()),
        target.path().join(cm_catalog::RECEIPT_PATH),
    ] {
        let text = fs::read_to_string(path).unwrap();
        assert!(text.contains("# Maintainer context"));
        assert!(text.contains("# Keep this explanation"));
    }
}
