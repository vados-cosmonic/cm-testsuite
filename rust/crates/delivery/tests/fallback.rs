//! Registry fallback selection, failure isolation, and adoption by upstream.

use cm_acquisition::load_catalog_files;
use cm_catalog::{CATALOG_PATH, Catalog, Participant, Participants, sha256};
use cm_delivery::{Delivery, Offer, Source, apply, distribute, gather};
use cm_github::{Api, HttpStatus};
use cm_test_support::github::FakeGitHubs;
use serde_json::{Value, json};
use std::fs;

const SOURCE: &str = "github:example/source";
const PATH: &str = "registry/manifests/source.toml";
const RULES: &str = "schema=1\n[[gather]]\npath='file:///tests'\nrecursive=true\n";

fn fixture() -> (FakeGitHubs, tempfile::TempDir, Vec<Participant>) {
    let hub = FakeGitHubs::new(&["example/source"]);
    hub.push(
        "example/source",
        &[
            ("tests/a.wast", ";; intent\n(module)\n"),
            ("tests/nested/b.wast", ";; nested intent\n(component)\n"),
        ],
    );
    let root = tempfile::tempdir().unwrap();
    fs::create_dir_all(root.path().join("registry/manifests")).unwrap();
    fs::write(root.path().join(PATH), RULES).unwrap();
    let participants = Participants::parse(
        format!("schema=1\n[[participant]]\nrepository='{SOURCE}'\nfallback_manifest='{PATH}'\n")
            .as_bytes(),
    )
    .unwrap()
    .participant;
    (hub, root, participants)
}

#[test]
fn registry_fallback_is_repeatable_and_native_adoption_updates_provenance_only() {
    let (hub, root, participants) = fixture();
    let gathered = gather(&hub, root.path(), &participants).unwrap();
    assert!(matches!(
        gathered.sources[0].1,
        Source::Acquired { tests: 2, .. }
    ));
    let plan = gathered.plan.unwrap();
    assert!(plan.warnings.iter().any(|w| w.contains("catalog fallback")));
    apply(root.path(), &plan).unwrap();
    let old = Catalog::load(root.path()).unwrap();
    assert_eq!(old.sources[SOURCE].fallback_manifest.as_deref(), Some(PATH));
    assert_eq!(
        old.sources[SOURCE].manifest_sha256,
        sha256(RULES.as_bytes())
    );
    let repeated = gather(&hub, root.path(), &participants)
        .unwrap()
        .plan
        .unwrap();
    assert!(repeated.edits.is_empty());
    hub.push("example/source", &[("README", "unrelated update")]);
    assert!(
        gather(&hub, root.path(), &participants)
            .unwrap()
            .plan
            .unwrap()
            .edits
            .is_empty()
    );

    hub.push("example/source", &[(".cm-testsuite.toml", RULES)]);
    fs::remove_file(root.path().join(PATH)).unwrap();
    let adopted = gather(&hub, root.path(), &participants)
        .unwrap()
        .plan
        .unwrap();
    assert_eq!(adopted.edits.len(), 1);
    assert_eq!(adopted.edits[0].path, CATALOG_PATH);
    assert!(adopted.guards.iter().all(|g| g.path != PATH));
    apply(root.path(), &adopted).unwrap();
    let new = Catalog::load(root.path()).unwrap();
    assert_eq!(new.sources[SOURCE].fallback_manifest, None);
    assert_eq!(
        old.tests.keys().collect::<Vec<_>>(),
        new.tests.keys().collect::<Vec<_>>()
    );
    assert!(new.retired.is_empty());
    assert!(
        new.tests
            .values()
            .all(|t| t.manifest_revision.as_ref() == Some(&new.sources[SOURCE].revision))
    );
    assert!(
        gather(&hub, root.path(), &participants)
            .unwrap()
            .plan
            .unwrap()
            .edits
            .is_empty()
    );
    assert_eq!(hub.writes(), 0);
}

#[test]
fn bad_fallback_or_upstream_keeps_catalog_while_valid_empty_upstream_retires_tests() {
    let (hub, root, participants) = fixture();
    apply(
        root.path(),
        &gather(&hub, root.path(), &participants)
            .unwrap()
            .plan
            .unwrap(),
    )
    .unwrap();
    let before = fs::read(root.path().join(CATALOG_PATH)).unwrap();
    for configuration in ["bad TOML", "schema=1\n[[gather]]\npath='file:///absent'\n"] {
        fs::write(root.path().join(PATH), configuration).unwrap();
        let gathered = gather(&hub, root.path(), &participants).unwrap();
        assert!(matches!(gathered.sources[0].1, Source::Failed(_)));
        assert!(gathered.plan.is_none());
        assert_eq!(fs::read(root.path().join(CATALOG_PATH)).unwrap(), before);
    }
    fs::write(root.path().join(PATH), RULES).unwrap();
    hub.push(
        "example/source",
        &[(".cm-testsuite.toml", "invalid upstream manifest")],
    );
    assert!(matches!(
        gather(&hub, root.path(), &participants).unwrap().sources[0].1,
        Source::Failed(_)
    ));
    hub.push("example/source", &[(".cm-testsuite.toml", "schema=1\n")]);
    let native_empty = gather(&hub, root.path(), &participants)
        .unwrap()
        .plan
        .unwrap();
    assert_eq!(native_empty.changes().removed, 2);
    apply(root.path(), &native_empty).unwrap();
    assert!(Catalog::load(root.path()).unwrap().tests.is_empty());
}

#[test]
fn central_config_never_authorizes_receiving_but_upstream_can_opt_in() {
    let (hub, root, participants) = fixture();
    let receiving = "schema=1\n[receive]\nenabled=true\n[[receive.route]]\nname='all'\ndestination='file:///shared'\nallow_unknown=true\n";
    fs::write(root.path().join(PATH), receiving).unwrap();
    let catalog = Catalog::default();
    let files = load_catalog_files(root.path(), &catalog).unwrap();
    assert!(matches!(
        distribute(
            &hub,
            &participants[0],
            &catalog,
            &files,
            Some(Offer::default())
        )
        .unwrap(),
        Delivery::NotReceiving
    ));
    assert_eq!(hub.writes(), 0);
    hub.push(
        "example/source",
        &[(".cm-testsuite.toml", "invalid manifest")],
    );
    assert!(distribute(&hub, &participants[0], &catalog, &files, None).is_err());
    hub.push("example/source", &[(".cm-testsuite.toml", receiving)]);
    assert!(matches!(
        distribute(&hub, &participants[0], &catalog, &files, None).unwrap(),
        Delivery::Planned(_)
    ));
    assert_eq!(hub.writes(), 0);
}

struct Unavailable<'a> {
    hub: &'a FakeGitHubs,
    status: u16,
}

impl Api for Unavailable<'_> {
    fn request(
        &self,
        method: &str,
        path: &str,
        body: Option<Value>,
    ) -> anyhow::Result<Option<Value>> {
        if path.contains("/git/trees/") {
            if self.status == 404 {
                return Ok(None);
            }
            return Err(HttpStatus {
                method: method.into(),
                path: path.into(),
                status: self.status,
            }
            .into());
        }
        self.hub.request(method, path, body)
    }
}

#[test]
fn unreadable_trees_are_errors_not_missing_manifests() {
    let (hub, root, participants) = fixture();
    for status in [403, 404, 429, 500] {
        let api = Unavailable { hub: &hub, status };
        let gathered = gather(&api, root.path(), &participants).unwrap();
        let Source::Failed(error) = &gathered.sources[0].1 else {
            panic!("used fallback after HTTP {status}");
        };
        assert!(
            error.to_string().contains(if status == 404 {
                "GitHub object not found"
            } else {
                "HTTP"
            }),
            "{error:#}"
        );
        assert!(gathered.plan.is_none());
        assert!(
            distribute(
                &api,
                &participants[0],
                &Catalog::default(),
                &Default::default(),
                None
            )
            .is_err()
        );
    }
}

#[test]
fn incomplete_trees_nonregular_manifests_and_missing_blobs_do_not_enable_fallback() {
    for mode in ["truncated", "120000", "160000", "040000", "missing-blob"] {
        let (hub, root, participants) = fixture();
        hub.push("example/source", &[(".cm-testsuite.toml", RULES)]);
        {
            let mut state = hub.repository("example/source").state_mut();
            let revision = state.refs["main"].clone();
            let tree = state.commits[&revision].tree.clone();
            if mode == "truncated" {
                state.trees.get_mut(&tree).unwrap()["truncated"] = json!(true);
            } else {
                let entry = state.trees.get_mut(&tree).unwrap()["tree"]
                    .as_array_mut()
                    .unwrap()
                    .iter_mut()
                    .find(|e| e["path"] == ".cm-testsuite.toml")
                    .unwrap();
                if mode == "missing-blob" {
                    let sha = entry["sha"].as_str().unwrap().to_owned();
                    state.blobs.remove(&sha);
                } else {
                    entry["mode"] = json!(mode);
                    entry["type"] = json!(match mode {
                        "160000" => "commit",
                        "040000" => "tree",
                        _ => "blob",
                    });
                }
            }
        }
        let gathered = gather(&hub, root.path(), &participants).unwrap();
        assert!(
            matches!(gathered.sources[0].1, Source::Failed(_)),
            "accepted {mode}"
        );
        assert!(gathered.plan.is_none());
    }
}

#[test]
fn catalog_rejects_falsely_attributed_upstream_manifest_revisions() {
    let (hub, root, participants) = fixture();
    apply(
        root.path(),
        &gather(&hub, root.path(), &participants)
            .unwrap()
            .plan
            .unwrap(),
    )
    .unwrap();
    let mut catalog = Catalog::load(root.path()).unwrap();
    let test = catalog.tests.values_mut().next().unwrap();
    test.manifest_revision = Some(test.revision.clone());
    assert!(catalog.validate().is_err());
}
