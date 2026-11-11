//! Filesystem failure recovery and protection of paths outside managed ownership.

use crate::common::*;
use cm_catalog::records;
use cm_exchange_plan::{Edit, Kind, Plan};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
};

/// Serialize a proposal so failure cases use the public CLI parser too.
fn plan(f: &Exchange, edits: Vec<Edit>) {
    let plan = Plan {
        schema: 1,
        kind: Kind::Import,
        subject: SOURCE.into(),
        edits,
        guards: vec![],
        warnings: vec![],
        exclusions: BTreeSet::new(),
        provenance: BTreeMap::new(),
    };
    fs::write(f.artifact("import.toml"), records::encode(&plan).unwrap()).unwrap();
}

/// Describe a new managed file for application and rollback scenarios.
fn addition(path: &str) -> Edit {
    Edit {
        path: path.into(),
        before: None,
        after: Some(WAST.into()),
    }
}

/// All completed operations must release staging resources and their writer lock.
fn assert_clean(f: &Exchange) {
    assert!(!f.hub.join(".cm-testsuite-write-lock").exists());
    assert!(fs::read_dir(&f.hub).unwrap().all(|e| {
        !e.unwrap()
            .file_name()
            .to_string_lossy()
            .starts_with(".cm-testsuite-stage-")
    }));
}

#[test]
fn existing_writer_lock_prevents_mutation_and_is_preserved() {
    let f = Exchange::new();
    plan(&f, vec![addition("corpus/wast/new.wast")]);
    write(&f.hub, ".cm-testsuite-write-lock", "another writer");
    failure(
        f.apply(&f.hub, "import.toml"),
        "another apply may be active",
    );
    assert!(!f.hub.join("corpus").exists());
    assert_eq!(
        fs::read_to_string(f.hub.join(".cm-testsuite-write-lock")).unwrap(),
        "another writer"
    );
}

#[test]
fn duplicate_or_outside_paths_reject_the_entire_plan() {
    let f = Exchange::new();
    for path in [
        "README.md",
        "../outside.wast",
        ".git/config",
        "corpus/wast/new.wast",
    ] {
        plan(&f, vec![addition("corpus/wast/new.wast"), addition(path)]);
        assert!(
            !f.apply(&f.hub, "import.toml").status.success(),
            "accepted {path}"
        );
        assert!(!f.hub.join("corpus").exists());
        assert_clean(&f);
    }
}

#[test]
fn directory_collision_preserves_its_contents() {
    let f = Exchange::new();
    write(&f.hub, "corpus/wast/new.wast/keep.txt", "keep");
    plan(&f, vec![addition("corpus/wast/new.wast")]);
    assert!(!f.apply(&f.hub, "import.toml").status.success());
    assert_eq!(
        fs::read_to_string(f.hub.join("corpus/wast/new.wast/keep.txt")).unwrap(),
        "keep"
    );
    assert_clean(&f);
}

#[test]
fn later_parent_conflict_rolls_back_an_earlier_addition() {
    let f = Exchange::new();
    // Both paths start absent; the first write turns the second parent into a file.
    plan(
        &f,
        vec![
            addition("corpus/wast/parent.wast"),
            addition("corpus/wast/parent.wast/child.wast"),
        ],
    );
    assert!(!f.apply(&f.hub, "import.toml").status.success());
    assert!(!f.hub.join("corpus/wast/parent.wast").exists());
    assert_clean(&f);
}

#[test]
fn failed_write_restores_an_earlier_update() {
    let f = Exchange::new();
    write(&f.hub, "corpus/wast/original.wast", "original bytes");
    plan(
        &f,
        vec![
            Edit {
                path: "corpus/wast/original.wast".into(),
                before: Some(cm_catalog::sha256(b"original bytes")),
                after: Some(WAST.into()),
            },
            addition("corpus/wast/parent.wast"),
            addition("corpus/wast/parent.wast/child.wast"),
        ],
    );
    assert!(!f.apply(&f.hub, "import.toml").status.success());
    assert_eq!(
        fs::read_to_string(f.hub.join("corpus/wast/original.wast")).unwrap(),
        "original bytes"
    );
    assert!(!f.hub.join("corpus/wast/parent.wast").exists());
    assert_clean(&f);
}

#[test]
fn failed_write_restores_an_earlier_deletion() {
    let f = Exchange::new();
    write(&f.hub, "corpus/wast/original.wast", "original bytes");
    plan(
        &f,
        vec![
            Edit {
                path: "corpus/wast/original.wast".into(),
                before: Some(cm_catalog::sha256(b"original bytes")),
                after: None,
            },
            addition("corpus/wast/parent.wast"),
            addition("corpus/wast/parent.wast/child.wast"),
        ],
    );
    assert!(!f.apply(&f.hub, "import.toml").status.success());
    assert_eq!(
        fs::read_to_string(f.hub.join("corpus/wast/original.wast")).unwrap(),
        "original bytes"
    );
    assert_clean(&f);
}

#[cfg(unix)]
#[test]
fn symlinked_destination_ancestor_cannot_write_outside_recipient() {
    let f = Exchange::new();
    f.import();
    let outside = f.artifact("outside");
    fs::create_dir(&outside).unwrap();
    std::os::unix::fs::symlink(&outside, f.recipient.join("tests")).unwrap();
    failure(f.plan_receive(), "symlink in managed path");
    assert_eq!(fs::read_dir(outside).unwrap().count(), 0);
}

#[cfg(unix)]
#[test]
fn symlink_inserted_after_planning_is_rejected_at_apply() {
    let f = Exchange::new();
    f.import();
    success(f.plan_receive());
    let outside = f.artifact("outside");
    fs::create_dir(&outside).unwrap();
    std::os::unix::fs::symlink(&outside, f.recipient.join("tests")).unwrap();
    failure(
        f.apply(&f.recipient, "receive.toml"),
        "symlink in managed path",
    );
    assert_eq!(fs::read_dir(outside).unwrap().count(), 0);
    assert!(!f.recipient.join(cm_catalog::RECEIPT_PATH).exists());
}
