//! GitHub publication scenarios against the shared in-memory API model.

mod common;

use cm_catalog::sha256;
use cm_delivery::{Publication, branch_for, publish};
use common::github::*;
use serde_json::json;

#[test]
fn creates_updates_and_repeats_one_pr_without_content_churn() {
    let api = Mock::new();
    let p = plan("(module)");
    assert!(matches!(
        publish(&api, "github:example/target", &p).unwrap(),
        Publication::PullRequest(_)
    ));
    let first_count = api.0.borrow().commits.len();
    let main = api.0.borrow().refs["main"].clone();
    publish(&api, "github:example/target", &p).unwrap();
    assert_eq!(api.0.borrow().commits.len(), first_count);
    assert_eq!(api.0.borrow().prs.len(), 1);
    publish(&api, "github:example/target", &plan("(component)")).unwrap();
    let s = api.0.borrow();
    assert_eq!(s.prs.len(), 1);
    assert_eq!(s.refs["main"], main);
    let files = &s.files[&s.commits[&s.refs[&branch_for(&p)]].tree];
    assert_eq!(files["README"], "unrelated source context");
    assert_eq!(files["corpus/wast/example/source/a.wast"], "(component)");
}

#[test]
fn declined_identical_plan_is_not_reopened() {
    let api = Mock::new();
    let p = plan("(module)");
    publish(&api, "github:example/target", &p).unwrap();
    api.0.borrow_mut().prs[0]["state"] = json!("closed");
    assert_eq!(
        publish(&api, "github:example/target", &p).unwrap(),
        Publication::Declined
    );
    assert_eq!(api.0.borrow().prs.len(), 1);
}

#[test]
fn human_changes_on_proposal_branch_are_not_overwritten() {
    let api = Mock::new();
    let p = plan("(module)");
    publish(&api, "github:example/target", &p).unwrap();
    let branch = branch_for(&p);
    {
        let mut s = api.0.borrow_mut();
        let old = s.refs[&branch].clone();
        let mut files = s.files[&s.commits[&old].tree].clone();
        files.insert("README".into(), "human change".into());
        let id = commit(&mut s, files, "human edit".into(), vec![old]);
        s.refs.insert(branch.clone(), id);
    }
    let before = api.0.borrow().refs[&branch].clone();
    assert!(publish(&api, "github:example/target", &plan("(component)")).is_err());
    assert_eq!(api.0.borrow().refs[&branch], before);
}

#[test]
fn concurrent_default_branch_change_stops_before_ref_update() {
    let api = Mock::new();
    api.0.borrow_mut().advance_base = true;
    assert!(publish(&api, "github:example/target", &plan("(module)")).is_err());
    assert_eq!(api.0.borrow().refs.len(), 1);
    assert!(api.0.borrow().prs.is_empty());
}

#[test]
fn no_diff_does_not_create_a_pr() {
    let api = Mock::new();
    let mut p = plan("(module)");
    p.edits.clear();
    assert_eq!(
        publish(&api, "github:example/target", &p).unwrap(),
        Publication::NoChanges
    );
    assert!(api.0.borrow().prs.is_empty());
}

#[test]
fn withdrawn_changes_close_the_owned_proposal() {
    let api = Mock::new();
    let mut p = plan("(module)");
    p.guards.push(cm_exchange_plan::Guard {
        path: "README".into(),
        sha256: Some(sha256(b"unrelated source context")),
    });
    publish(&api, "github:example/target", &p).unwrap();
    p.edits.clear();
    assert!(matches!(
        publish(&api, "github:example/target", &p).unwrap(),
        Publication::Closed(_)
    ));
    assert_eq!(api.0.borrow().prs[0]["state"], "closed");
    assert_eq!(
        publish(&api, "github:example/target", &p).unwrap(),
        Publication::NoChanges
    );
}

#[test]
fn stale_empty_plan_must_not_withdraw_current_proposal() {
    let api = Mock::new();
    let p = plan("(module)");
    publish(&api, "github:example/target", &p).unwrap();
    let mut stale = p.clone();
    stale.edits.clear();
    stale.guards.push(cm_exchange_plan::Guard {
        path: "README".into(),
        sha256: Some(sha256(b"obsolete policy")),
    });
    let result = publish(&api, "github:example/target", &stale);
    assert!(
        result.is_err(),
        "Stale empty plan bypassed its guard and returned {result:?}"
    );
    assert_eq!(api.0.borrow().prs[0]["state"], "open");
}

#[test]
fn republishing_applied_plan_must_not_close_newer_proposal() {
    let api = Mock::new();
    let a = plan("(module)");
    publish(&api, "github:example/target", &a).unwrap();
    {
        let mut s = api.0.borrow_mut();
        let head = s.refs[&branch_for(&a)].clone();
        s.refs.insert("main".into(), head);
        s.prs[0]["state"] = json!("closed");
        s.prs[0]["merged_at"] = json!("2026-11-06T00:00:00Z");
    }
    let mut b = plan("(component)");
    b.edits[0].before = Some(sha256(b"(module)"));
    publish(&api, "github:example/target", &b).unwrap();
    assert_eq!(api.0.borrow().prs[1]["state"], "open");
    publish(&api, "github:example/target", &a).unwrap();
    assert_eq!(
        api.0.borrow().prs[1]["state"],
        "open",
        "Retry of applied plan A closed newer proposal B"
    );
}

#[test]
fn applied_plan_closes_only_its_own_proposal_despite_old_guards() {
    let api = Mock::new();
    let mut p = plan("(module)");
    p.guards.push(cm_exchange_plan::Guard {
        path: p.edits[0].path.clone(),
        sha256: None,
    });
    publish(&api, "github:example/target", &p).unwrap();
    {
        let mut s = api.0.borrow_mut();
        let head = s.refs[&branch_for(&p)].clone();
        s.refs.insert("main".into(), head);
    }
    assert!(matches!(
        publish(&api, "github:example/target", &p).unwrap(),
        Publication::Closed(_)
    ));
    assert_eq!(api.0.borrow().prs[0]["state"], "closed");
}

#[test]
fn automatic_withdrawal_is_not_a_human_decline() {
    let api = Mock::new();
    let p = plan("(module)");
    publish(&api, "github:example/target", &p).unwrap();
    let mut empty = p.clone();
    empty.edits.clear();
    assert!(matches!(
        publish(&api, "github:example/target", &empty).unwrap(),
        Publication::Closed(_)
    ));
    assert!(
        matches!(
            publish(&api, "github:example/target", &p).unwrap(),
            Publication::PullRequest(_)
        ),
        "Tool-withdrawn plan is suppressed as if a maintainer declined it"
    );
    assert_eq!(api.0.borrow().prs.len(), 2);
    api.0.borrow_mut().prs[1]["state"] = json!("closed");
    assert_eq!(
        publish(&api, "github:example/target", &p).unwrap(),
        Publication::Declined
    );
    assert_eq!(api.0.borrow().prs.len(), 2);
}
