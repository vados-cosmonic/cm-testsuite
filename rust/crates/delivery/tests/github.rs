//! GitHub publication scenarios against the shared in-memory API model.

mod common;

use cm_catalog::sha256;
use cm_delivery::{Offer, Publication, Review, branch_for, publish, publish_on};
use cm_test_support::github::FakeGitHub;
use common::github::plan;
use serde_json::json;

#[test]
fn creates_updates_and_repeats_one_pr_without_content_churn() {
    let api = FakeGitHub::new("example/target");
    let p = plan("(module)");
    assert!(matches!(
        publish(&api, "github:example/target", &p).unwrap(),
        Publication::PullRequest(_)
    ));
    let first_count = api.state().commits.len();
    let main = api.state().refs["main"].clone();
    publish(&api, "github:example/target", &p).unwrap();
    assert_eq!(api.state().commits.len(), first_count);
    assert_eq!(api.state().prs.len(), 1);
    publish(&api, "github:example/target", &plan("(component)")).unwrap();
    let s = api.state();
    assert_eq!(s.prs.len(), 1);
    assert_eq!(s.refs["main"], main);
    let files = &s.files[&s.commits[&s.refs[&branch_for(&p)]].tree];
    assert_eq!(files["README"], "unrelated source context");
    assert_eq!(files["corpus/wast/example/source/a.wast"], "(component)");
}

#[test]
fn declined_identical_plan_is_not_reopened() {
    let api = FakeGitHub::new("example/target");
    let p = plan("(module)");
    publish(&api, "github:example/target", &p).unwrap();
    api.state_mut().prs[0]["state"] = json!("closed");
    assert_eq!(
        publish(&api, "github:example/target", &p).unwrap(),
        Publication::Declined
    );
    assert_eq!(api.state().prs.len(), 1);
}

#[test]
fn declined_identical_plan_is_opened_again_on_request() {
    let api = FakeGitHub::new("example/target");
    let p = plan("(module)");
    let offer = |review, recreate| {
        let offer = Offer { review, recreate };
        publish_on(&api, "github:example/target", &p, &branch_for(&p), offer).unwrap()
    };
    let close = |index: usize| api.state_mut().prs[index]["state"] = json!("closed");
    publish(&api, "github:example/target", &p).unwrap();
    close(0);
    let commits = api.state().commits.len();

    // The request opens a second pull request from the branch the first one left.
    let again = offer(Review::Ready, true);
    assert!(matches!(again, Publication::PullRequest(_)), "{again:?}");
    {
        let state = api.state();
        assert_eq!(state.prs.len(), 2);
        assert_eq!(state.prs[0]["state"], "closed");
        assert_eq!(state.prs[1]["state"], "open");
        assert_eq!(state.prs[1]["draft"], false);
        assert_eq!(state.commits.len(), commits);
    }

    // While it is open, later runs update it whether or not they repeat the request.
    assert_eq!(offer(Review::Draft, false), again);
    assert_eq!(offer(Review::Draft, true), again);
    assert_eq!(api.state().prs.len(), 2);

    // Closing it declines the plan again, and each request opens one more.
    close(1);
    assert_eq!(offer(Review::Draft, false), Publication::Declined);
    assert!(matches!(
        offer(Review::Draft, true),
        Publication::PullRequest(_)
    ));
    let state = api.state();
    assert_eq!(state.prs.len(), 3);
    assert_eq!(state.prs[2]["draft"], true);
}

#[test]
fn human_changes_on_proposal_branch_are_not_overwritten() {
    let api = FakeGitHub::new("example/target");
    let p = plan("(module)");
    publish(&api, "github:example/target", &p).unwrap();
    let branch = branch_for(&p);
    {
        let mut s = api.state_mut();
        let old = s.refs[&branch].clone();
        let mut files = s.files[&s.commits[&old].tree].clone();
        files.insert("README".into(), "human change".into());
        let id = s.commit(files, "human edit", vec![old]);
        s.refs.insert(branch.clone(), id);
    }
    let before = api.state().refs[&branch].clone();
    assert!(publish(&api, "github:example/target", &plan("(component)")).is_err());
    assert_eq!(api.state().refs[&branch], before);
}

#[test]
fn concurrent_default_branch_change_stops_before_ref_update() {
    let api = FakeGitHub::new("example/target");
    api.state_mut().advance_base = true;
    assert!(publish(&api, "github:example/target", &plan("(module)")).is_err());
    assert_eq!(api.state().refs.len(), 1);
    assert!(api.state().prs.is_empty());
}

#[test]
fn no_diff_does_not_create_a_pr() {
    let api = FakeGitHub::new("example/target");
    let mut p = plan("(module)");
    p.edits.clear();
    assert_eq!(
        publish(&api, "github:example/target", &p).unwrap(),
        Publication::NoChanges
    );
    assert!(api.state().prs.is_empty());
}

#[test]
fn withdrawn_changes_close_the_owned_proposal() {
    let api = FakeGitHub::new("example/target");
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
    assert_eq!(api.state().prs[0]["state"], "closed");
    assert_eq!(
        publish(&api, "github:example/target", &p).unwrap(),
        Publication::NoChanges
    );
}

#[test]
fn stale_empty_plan_must_not_withdraw_current_proposal() {
    let api = FakeGitHub::new("example/target");
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
    assert_eq!(api.state().prs[0]["state"], "open");
}

#[test]
fn republishing_applied_plan_must_not_close_newer_proposal() {
    let api = FakeGitHub::new("example/target");
    let a = plan("(module)");
    publish(&api, "github:example/target", &a).unwrap();
    {
        let mut s = api.state_mut();
        let head = s.refs[&branch_for(&a)].clone();
        s.refs.insert("main".into(), head);
        s.prs[0]["state"] = json!("closed");
        s.prs[0]["merged_at"] = json!("2026-11-06T00:00:00Z");
    }
    let mut b = plan("(component)");
    b.edits[0].before = Some(sha256(b"(module)"));
    publish(&api, "github:example/target", &b).unwrap();
    assert_eq!(api.state().prs[1]["state"], "open");
    publish(&api, "github:example/target", &a).unwrap();
    assert_eq!(
        api.state().prs[1]["state"],
        "open",
        "Retry of applied plan A closed newer proposal B"
    );
}

#[test]
fn applied_plan_closes_only_its_own_proposal_despite_old_guards() {
    let api = FakeGitHub::new("example/target");
    let mut p = plan("(module)");
    p.guards.push(cm_exchange_plan::Guard {
        path: p.edits[0].path.clone(),
        sha256: None,
    });
    publish(&api, "github:example/target", &p).unwrap();
    {
        let mut s = api.state_mut();
        let head = s.refs[&branch_for(&p)].clone();
        s.refs.insert("main".into(), head);
    }
    assert!(matches!(
        publish(&api, "github:example/target", &p).unwrap(),
        Publication::Closed(_)
    ));
    assert_eq!(api.state().prs[0]["state"], "closed");
}

#[test]
fn automatic_withdrawal_is_not_a_human_decline() {
    let api = FakeGitHub::new("example/target");
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
    assert_eq!(api.state().prs.len(), 2);
    api.state_mut().prs[1]["state"] = json!("closed");
    assert_eq!(
        publish(&api, "github:example/target", &p).unwrap(),
        Publication::Declined
    );
    assert_eq!(api.state().prs.len(), 2);
}
