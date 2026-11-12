//! Imports and deliveries across the catalog's file layouts: one file per
//! source, the shared file, and a mix of both.

mod common;

use cm_acquisition::{Snapshot, load_catalog_files};
use cm_catalog::{
    CATALOG_DIR, CATALOG_PATH, Catalog, Layout, RECEIPT_PATH, digest, records, source_path,
};
use cm_delivery::{Publication, apply, branch_for, publish};
use cm_exchange_plan::{Plan, Receipt, import};
use cm_test_support::github::FakeGitHub;
use common::exchange::*;
use serde_json::json;
use std::{fs, path::Path};

const A: &str = "github:example/a";
const B: &str = "github:example/b";
const A_FILE: &str = "metadata/catalog/example/a.toml";
const B_FILE: &str = "metadata/catalog/example/b.toml";
const A_TEST: &str = "corpus/wast/example/a/stable-name.wast";
const B_TEST: &str = "corpus/wast/example/b/stable-name.wast";

fn plan(root: &Path, snapshot: &Snapshot) -> Plan {
    import(root, std::slice::from_ref(snapshot)).unwrap()
}

fn paths(plan: &Plan) -> Vec<&str> {
    plan.edits.iter().map(|edit| edit.path.as_str()).collect()
}

fn guarded(plan: &Plan) -> Vec<&str> {
    plan.guards
        .iter()
        .map(|guard| guard.path.as_str())
        .collect()
}

/// Rewrite the catalog in `root` as the shared file alone.
fn inline(root: &Path) -> Catalog {
    let catalog = Catalog::load(root).unwrap();
    fs::remove_dir_all(root.join(CATALOG_DIR)).unwrap();
    let text = records::encode(&catalog).unwrap();
    fs::write(root.join(CATALOG_PATH), text).unwrap();
    assert_eq!(Catalog::load(root).unwrap(), catalog);
    catalog
}

#[test]
fn an_import_writes_only_the_file_of_its_source() {
    let d = tempfile::tempdir().unwrap();
    assert_eq!(source_path(A).unwrap(), A_FILE);
    setup(d.path(), &snapshot("a", "x.wast", "(module)"));
    let before = fs::read(d.path().join(A_FILE)).unwrap();

    let mut b = snapshot("b", "x.wast", "(component)");
    let p = plan(d.path(), &b);
    assert_eq!(paths(&p), [B_TEST, B_FILE]);
    assert_eq!(guarded(&p), [B_FILE]);
    assert_eq!(p.guards[0].sha256, None);
    // Catalog files are records: neither counted nor listed as test changes.
    assert_eq!(p.changes().to_string(), "1 added");
    assert!(!p.body().unwrap().contains("metadata/"));
    apply(d.path(), &p).unwrap();
    assert_eq!(fs::read(d.path().join(A_FILE)).unwrap(), before);
    assert!(!d.path().join(CATALOG_PATH).exists());
    assert!(plan(d.path(), &b).edits.is_empty());

    // A source that retires every test keeps its file, which records them.
    b.tests.clear();
    b.files.clear();
    let retiring = plan(d.path(), &b);
    assert_eq!(paths(&retiring), [B_TEST, B_FILE]);
    apply(d.path(), &retiring).unwrap();
    assert_eq!(fs::read(d.path().join(A_FILE)).unwrap(), before);
    let part: Catalog = records::decode(&fs::read(d.path().join(B_FILE)).unwrap()).unwrap();
    assert!(part.sources.contains_key(B) && part.tests.is_empty());
    assert_eq!(part.retired.len(), 1);
    assert_eq!(Catalog::load(d.path()).unwrap().only(B), part);
}

#[test]
fn plans_for_different_sources_apply_in_either_order() {
    let existing = snapshot("c", "x.wast", "(module)");
    let a = snapshot("a", "x.wast", "(module (func))");
    let b = snapshot("b", "x.wast", "(component)");
    let together = tempfile::tempdir().unwrap();
    setup(together.path(), &existing);
    let both = import(together.path(), &[a.clone(), b.clone()]).unwrap();
    apply(together.path(), &both).unwrap();
    let expected = Catalog::load(together.path()).unwrap();

    for a_first in [true, false] {
        let d = tempfile::tempdir().unwrap();
        setup(d.path(), &existing);
        // Both plans see the same catalog; neither knows of the other.
        let (for_a, for_b) = (plan(d.path(), &a), plan(d.path(), &b));
        let order = if a_first {
            [&for_a, &for_b]
        } else {
            [&for_b, &for_a]
        };
        for p in order {
            assert_eq!(apply(d.path(), p).unwrap(), 2);
        }
        let loaded = Catalog::load(d.path()).unwrap();
        assert_eq!(loaded, expected);
        assert_eq!(digest(&loaded).unwrap(), digest(&expected).unwrap());
        assert_eq!(load_catalog_files(d.path(), &loaded).unwrap().len(), 3);
        for snapshot in [&a, &b] {
            assert!(plan(d.path(), snapshot).edits.is_empty());
        }
    }
}

#[test]
fn a_second_plan_for_the_same_source_is_stale() {
    let d = tempfile::tempdir().unwrap();
    setup(d.path(), &snapshot("a", "x.wast", "(module)"));
    let first = plan(d.path(), &snapshot("a", "x.wast", "(component)"));
    let second = plan(d.path(), &snapshot("a", "x.wast", "(module (func))"));
    // A plan that only advances records competes too.
    let moved = plan(d.path(), &snapshot("a", "y.wast", "(module)"));
    assert_eq!(paths(&moved), [A_FILE]);
    apply(d.path(), &first).unwrap();
    let applied = fs::read(d.path().join(A_FILE)).unwrap();
    for stale in [&second, &moved] {
        let error = apply(d.path(), stale).unwrap_err();
        assert!(error.to_string().contains("stale plan"), "{error:#}");
    }
    assert_eq!(fs::read(d.path().join(A_FILE)).unwrap(), applied);
    assert_eq!(
        fs::read_to_string(d.path().join(A_TEST)).unwrap(),
        "(component)"
    );
}

#[test]
fn a_source_moves_out_of_the_shared_file_when_it_is_imported() {
    let d = tempfile::tempdir().unwrap();
    let a = snapshot("a", "x.wast", "(module)");
    let b = snapshot("b", "x.wast", "(component)");
    apply(d.path(), &import(d.path(), &[a, b.clone()]).unwrap()).unwrap();
    let whole = inline(d.path());
    let shared = d.path().join(CATALOG_PATH);
    let header = "[sources.\"github:example/a\"]";
    let text = fs::read_to_string(&shared).unwrap();
    assert!(text.contains(header), "{text}");
    let noted = text.replace(header, &format!("{header} # Source note"));
    fs::write(&shared, format!("# Maintainer context\n{noted}")).unwrap();
    assert_eq!(Catalog::load(d.path()).unwrap(), whole);

    // An unchanged source stays where it is recorded.
    let updated = snapshot("a", "x.wast", "(module (func))");
    let p = plan(d.path(), &updated);
    assert_eq!(paths(&p), [A_TEST, CATALOG_PATH, A_FILE]);
    assert_eq!(guarded(&p), [A_FILE, CATALOG_PATH]);
    // Two sources leaving the shared file both rewrite it, so they still compete.
    let competing = plan(d.path(), &b);
    assert_eq!(paths(&competing), [CATALOG_PATH, B_FILE]);
    apply(d.path(), &p).unwrap();
    let error = apply(d.path(), &competing).unwrap_err();
    assert!(error.to_string().contains("stale plan"), "{error:#}");

    let layout = Layout::load(d.path()).unwrap();
    assert_eq!(layout.inline, whole.only(B));
    assert_eq!(layout.catalog.tests.len(), 2);
    assert_ne!(layout.catalog.only(A), whole.only(A));
    // Comments travel with the records they annotate.
    let own = fs::read_to_string(d.path().join(A_FILE)).unwrap();
    assert!(own.contains("# Source note"), "{own}");
    let rest = fs::read_to_string(&shared).unwrap();
    assert!(rest.contains("# Maintainer context"), "{rest}");
    assert!(
        !rest.contains("# Source note") && !rest.contains(A),
        "{rest}"
    );
    assert!(plan(d.path(), &updated).edits.is_empty());

    // Importing the other source, even unchanged, empties the shared file.
    let last = plan(d.path(), &b);
    assert_eq!(paths(&last), [CATALOG_PATH, B_FILE]);
    assert!(last.changes().is_empty());
    apply(d.path(), &last).unwrap();
    let layout = Layout::load(d.path()).unwrap();
    assert_eq!(layout.inline, Catalog::default());
    assert_eq!(layout.catalog.only(B), whole.only(B));
    assert!(shared.exists());
    let p = plan(d.path(), &b);
    assert!(p.edits.is_empty());
    assert_eq!(guarded(&p), [B_FILE]);
}

#[test]
fn deliveries_do_not_depend_on_the_layout() {
    let hub = tempfile::tempdir().unwrap();
    let target = tempfile::tempdir().unwrap();
    let a = snapshot("a", "x.wast", "(module)");
    let b = snapshot("b", "x.wast", "(component)");
    apply(hub.path(), &import(hub.path(), &[a.clone(), b]).unwrap()).unwrap();
    inline(hub.path());
    // One source in the shared file, the other in its own.
    apply(hub.path(), &plan(hub.path(), &a)).unwrap();
    let layout = Layout::load(hub.path()).unwrap();
    assert!(layout.inline.holds(B) && !layout.inline.holds(A));

    policy(target.path(), "");
    let delivery = received(target.path(), hub.path());
    assert_eq!(delivery.changes().added, 2);
    apply(target.path(), &delivery).unwrap();
    let receipt = || -> Receipt {
        records::decode(&fs::read(target.path().join(RECEIPT_PATH)).unwrap()).unwrap()
    };
    assert_eq!(receipt().catalog_sha256, digest(&layout.catalog).unwrap());

    // Moving records between files changes neither the catalog nor deliveries.
    inline(hub.path());
    assert!(received(target.path(), hub.path()).edits.is_empty());
    assert_eq!(
        digest(&Catalog::load(hub.path()).unwrap()).unwrap(),
        receipt().catalog_sha256
    );
}

/// Land a plan on the default branch as a merge of its pull request would,
/// keeping whatever the branch gained since the plan was made.
fn merge(api: &FakeGitHub, plan: &Plan) {
    let mut state = api.state_mut();
    let main = state.refs["main"].clone();
    let mut files = state.files_at(&main).clone();
    for edit in &plan.edits {
        match &edit.after {
            Some(text) => files.insert(edit.path.clone(), text.clone()),
            None => files.remove(&edit.path),
        };
    }
    let merged = state.commit(files, "merge", vec![main]);
    state.refs.insert("main".into(), merged);
    let branch = branch_for(plan);
    for pr in state.prs.iter_mut().filter(|pr| pr["head"] == branch) {
        pr["state"] = json!("closed");
        pr["merged_at"] = json!("2026-11-12T00:00:00Z");
    }
}

#[test]
fn open_proposals_for_different_sources_land_in_either_order() {
    let local = tempfile::tempdir().unwrap();
    let a = snapshot("a", "x.wast", "(module)");
    let b = snapshot("b", "x.wast", "(component)");
    let (for_a, for_b) = (plan(local.path(), &a), plan(local.path(), &b));
    let rival = plan(local.path(), &snapshot("a", "x.wast", "(module (func))"));
    let target = "github:example/catalog";

    for a_first in [true, false] {
        let api = FakeGitHub::new("example/catalog");
        let (first, second) = if a_first {
            (&for_a, &for_b)
        } else {
            (&for_b, &for_a)
        };
        for p in [first, second] {
            let opened = publish(&api, target, p).unwrap();
            assert!(matches!(opened, Publication::PullRequest(_)), "{opened:?}");
        }
        assert_eq!(api.state().prs.len(), 2);

        // The other proposal is still current once the first has landed.
        merge(&api, first);
        let kept = publish(&api, target, second).unwrap();
        assert!(matches!(kept, Publication::PullRequest(_)), "{kept:?}");
        assert_eq!(api.state().prs.len(), 2);
        merge(&api, second);
        for p in [first, second] {
            assert_eq!(publish(&api, target, p).unwrap(), Publication::NoChanges);
        }

        // The default branch now holds a valid catalog of both sources.
        let checkout = tempfile::tempdir().unwrap();
        let state = api.state();
        for (path, text) in state.files_at(&state.refs["main"]) {
            let file = checkout.path().join(path);
            fs::create_dir_all(file.parent().unwrap()).unwrap();
            fs::write(file, text).unwrap();
        }
        drop(state);
        let catalog = Catalog::load(checkout.path()).unwrap();
        assert_eq!(catalog.sources.keys().collect::<Vec<_>>(), [A, B]);
        assert_eq!(
            load_catalog_files(checkout.path(), &catalog).unwrap().len(),
            2
        );

        // A plan for a source that has since landed is still refused.
        let error = publish(&api, target, &rival).unwrap_err();
        assert!(
            error.to_string().contains("remote plan baseline changed"),
            "{error:#}"
        );
    }
}
