use anyhow::{Result, bail};
use cm_acquisition::{Repository, Snapshot, acquire, load_catalog_files};
use cm_catalog::{CATALOG_PATH, Catalog, Manifest, digest, json};
use cm_delivery::apply;
use cm_exchange_plan::{Receipt, import, receive};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::Path,
};
struct Source {
    manifest: String,
    files: BTreeMap<String, String>,
}
impl Repository for Source {
    fn resolve(&self, r: &str) -> Result<String> {
        Ok(if r == "old" || r == "a".repeat(40) {
            "a".repeat(40)
        } else {
            "b".repeat(40)
        })
    }
    fn read(&self, r: &str, p: &str) -> Result<Vec<u8>> {
        if p == ".cm-testsuite.toml" {
            return Ok(self.manifest.as_bytes().to_vec());
        }
        if r == "a".repeat(40) {
            return Ok(b";; Old revision intent\n(module)\n".to_vec());
        }
        self.files
            .get(p)
            .map(|x| x.as_bytes().to_vec())
            .ok_or_else(|| anyhow::anyhow!("missing file"))
    }
}
fn snapshot(name: &str, path: &str, text: &str) -> Snapshot {
    acquire(&Source{manifest:format!("schema=1\n[[test]]\nname='stable-name'\npath='file:///{path}'\nrequires=['component-model']\ncovers=['smoke']\n"),files:BTreeMap::from([(path.into(),text.into())])},&format!("github:example/{name}"),"main",".cm-testsuite.toml").unwrap()
}
fn setup(root: &Path, snapshot: &Snapshot) -> Catalog {
    apply(root, &import(root, std::slice::from_ref(snapshot)).unwrap()).unwrap();
    Catalog::load(root).unwrap()
}
fn policy(root: &Path, extra: &str) {
    fs::write(root.join(".cm-testsuite.toml"),format!("schema=1\n[receive]\nenabled=true\n{extra}\n[[receive.route]]\nname='smoke'\ndestination='file:///tests/shared'\nsupports=['component-model']\nallow_unknown=true\n")).unwrap();
}
fn received(root: &Path, catalog_root: &Path) -> cm_exchange_plan::Plan {
    let c = Catalog::load(catalog_root).unwrap();
    receive(
        root,
        "github:example/recipient",
        ".cm-testsuite.toml",
        &c,
        &load_catalog_files(catalog_root, &c).unwrap(),
    )
    .unwrap()
}
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
    assert_eq!(p.edits[0].path, CATALOG_PATH);
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
        serde_json::from_slice(&fs::read(target.path().join(".cm-testsuite-lock.json")).unwrap())
            .unwrap();
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
        serde_json::from_slice(&fs::read(target.path().join(".cm-testsuite-lock.json")).unwrap())
            .unwrap();
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
    let before = fs::read(d.path().join(CATALOG_PATH)).unwrap();
    struct Gone;
    impl Repository for Gone {
        fn resolve(&self, _: &str) -> Result<String> {
            bail!("network unavailable")
        }
        fn read(&self, _: &str, _: &str) -> Result<Vec<u8>> {
            unreachable!()
        }
    }
    assert!(acquire(&Gone, "github:example/a", "main", ".cm-testsuite.toml").is_err());
    assert_eq!(fs::read(d.path().join(CATALOG_PATH)).unwrap(), before);
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
    assert!(!d.path().join(CATALOG_PATH).exists());
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
    assert!(json(&s).is_ok());
}
