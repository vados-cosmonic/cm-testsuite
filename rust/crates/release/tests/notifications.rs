//! Tagging and release notification planning against the shared in-memory GitHub API.

use anyhow::Result;
use cm_release::{
    Metadata,
    github::{self, COMMENT_MARKER, CommentEdit, NotificationPlan, ReleasePrBase, TagOutcome},
    history::ReleaseHistory,
};
use cm_test_support::github::{FakeGitHub, comment};
use serde_json::{Value, json};
use std::collections::BTreeSet;

const REPO: &str = "acme/tests";
const AUTHOR: &str = "github-actions[bot]";
const RC_TAG: &str = "cm-testsuite-v0.2.0-rc.1";
const STABLE_TAG: &str = "cm-testsuite-v0.2.0";

/// A repository whose release candidate and stable release are both published.
fn repository() -> FakeGitHub {
    let api = FakeGitHub::new(REPO);
    api.add_release(RC_TAG, true);
    api.add_release(STABLE_TAG, false);
    api
}

/// Plan notifications for `tag` over the default history.
fn try_plan(api: &FakeGitHub, tag: &str) -> Result<NotificationPlan> {
    github::notifications(api, REPO, tag, &history(), AUTHOR)
}

/// Plan notifications for `tag`, expecting success, and return the edits.
fn planned_edits(api: &FakeGitHub, tag: &str) -> Vec<CommentEdit> {
    try_plan(api, tag).unwrap().edits
}

/// Write planned comments as the release workflow does.
fn apply(api: &FakeGitHub, edits: &[CommentEdit]) -> Result<()> {
    github::apply_notifications(api, REPO, AUTHOR, edits)
}

/// A commit identifier made of one repeated hex digit.
fn sha(digit: char) -> String {
    digit.to_string().repeat(40)
}

/// A release after 0.1.0 that contains commits `a` and `b`.
fn history() -> ReleaseHistory {
    ReleaseHistory {
        previous: Some("cm-testsuite-v0.1.0".into()),
        commits: vec![sha('a'), sha('b')],
        ancestors: BTreeSet::from([sha('a'), sha('b')]),
    }
}

#[test]
fn parses_only_real_same_repository_closing_references() {
    let body = "\
Fixes #12, resolves acme/tests#13. CLOSED https://github.com/acme/tests/issues/14
Fixes other/tests#99
Fixes https://github.com/acme/tests-other/issues/88
Fixes #0
Fixes #12abc
Fixes #+18
notfixes #15
Fixes `code` #19
`Fixes #16`
```text
Fixes #17
```
(**Closes:** [#20]) and Resolved ACME/Tests#21;
";
    assert_eq!(
        github::closed_issues(body, REPO),
        BTreeSet::from([12, 13, 14, 20, 21])
    );
}

#[test]
fn no_prs_multiple_prs_and_duplicate_commits_are_handled() {
    let api = repository();
    assert!(planned_edits(&api, RC_TAG).is_empty());
    api.add_merged_pr(1, &sha('a'), "Fixes #7 and closes #7");
    api.add_merged_pr(2, &sha('a'), "No issue");
    // The same PR is also associated with a second commit of the release.
    api.state_mut()
        .commit_prs
        .insert(sha('b'), vec![json!({"number": 1})]);
    let edits = planned_edits(&api, RC_TAG);
    assert_eq!(
        edits.iter().map(|e| e.number).collect::<Vec<_>>(),
        vec![2, 7]
    );
    assert!(edits.iter().all(|e| {
        e.body
            .contains("https://github.com/acme/tests/releases/tag/")
    }));
    assert_eq!(api.writes(), 0);
    apply(&api, &edits).unwrap();
    assert!(planned_edits(&api, RC_TAG).is_empty());
    assert_eq!(api.writes(), 2);
}

#[test]
fn excludes_unmerged_foreign_and_not_yet_released_prs() {
    let api = repository();
    for number in 1..=3 {
        api.add_merged_pr(number, &sha('a'), "");
    }
    {
        let mut s = api.state_mut();
        s.pr_mut(1)["merged_at"] = Value::Null;
        s.pr_mut(2)["base"]["repo"]["full_name"] = json!("other/repo");
        s.pr_mut(3)["merge_commit_sha"] = json!(sha('c'));
    }
    assert!(planned_edits(&api, RC_TAG).is_empty());
}

#[test]
fn release_preparation_prs_are_not_notified() {
    // With merge commits, the release PR's merge commit is not a `release:`
    // commit, so it stays in the history and leads back to the release PR.
    let api = repository();
    api.add_merged_pr(1, &sha('a'), "");
    api.add_merged_pr(2, &sha('b'), "");
    api.state_mut().pr_mut(2)["title"] = json!("release: cm-testsuite v0.2.0");
    let edits = planned_edits(&api, RC_TAG);
    assert_eq!(edits.iter().map(|e| e.number).collect::<Vec<_>>(), vec![1]);
}

#[test]
fn paginates_and_updates_only_owned_comments_without_losing_releases() {
    let api = repository();
    api.add_merged_pr(1, &sha('a'), "");
    let mut comments = (0..100)
        .map(|id| comment(id, COMMENT_MARKER, "someone-else"))
        .collect::<Vec<_>>();
    let previous =
        format!("Previous release:\n- [old](https://example.test)\n\n{COMMENT_MARKER}\n");
    comments.push(comment(101, &previous, AUTHOR));
    api.state_mut().comments.insert(1, comments);

    let edits = planned_edits(&api, RC_TAG);
    assert_eq!(edits[0].comment_id, Some(101));
    apply(&api, &edits).unwrap();

    // A later stable release keeps both earlier links.
    let edits = planned_edits(&api, STABLE_TAG);
    assert!(edits[0].body.contains(RC_TAG));
    assert!(edits[0].body.contains("https://example.test"));
    apply(&api, &edits).unwrap();

    // Rerunning the older release changes nothing.
    assert!(planned_edits(&api, RC_TAG).is_empty());
    assert_eq!(api.state().comments[&1][0]["body"], COMMENT_MARKER);
}

#[test]
fn rejects_draft_releases_and_concurrently_edited_comments() {
    let api = repository();
    api.add_merged_pr(1, &sha('a'), "");
    api.state_mut().releases.get_mut(RC_TAG).unwrap()["draft"] = json!(true);
    assert!(try_plan(&api, RC_TAG).is_err());
    // An unpublished tag has no release at all.
    assert!(try_plan(&api, "cm-testsuite-v0.3.0").is_err());
    api.add_release(RC_TAG, true);
    api.state_mut()
        .comments
        .insert(1, vec![comment(101, COMMENT_MARKER, AUTHOR)]);
    let edits = planned_edits(&api, RC_TAG);
    api.state_mut().comments.get_mut(&1).unwrap()[0]["body"] = json!("changed");
    assert!(apply(&api, &edits).is_err());
    assert_eq!(api.writes(), 0);
}

#[test]
fn tagging_is_explicit_idempotent_and_never_moves_existing_tags() {
    let api = repository();
    let meta = Metadata::new("0.2.0").unwrap();
    let tag = |sha: &str, apply| github::create_tag(&api, REPO, &meta, sha, apply);
    assert_eq!(tag(&sha('a'), false).unwrap(), TagOutcome::Planned);
    assert_eq!(api.writes(), 0);
    assert_eq!(tag(&sha('a'), true).unwrap(), TagOutcome::Created);
    assert_eq!(tag(&sha('a'), true).unwrap(), TagOutcome::Unchanged);
    assert_eq!(tag(&sha('a'), false).unwrap(), TagOutcome::Unchanged);
    assert!(tag(&sha('b'), true).is_err());
    assert_eq!(api.writes(), 1);
    assert_eq!(api.state().tags[STABLE_TAG], sha('a'));
}

#[test]
fn skips_missing_moved_and_deleted_targets_and_names_linked_prs() {
    let api = repository();
    api.add_merged_pr(
        1,
        &sha('a'),
        "Fixes #7, fixes #8, fixes #9, fixes #10, fixes #2",
    );
    // #2 is a PR outside this release, so it is only a linked target.
    api.add_merged_pr(2, &sha('c'), "");
    api.state_mut()
        .unavailable
        .extend([(7, 404), (8, 301), (9, 410)]);
    let plan = try_plan(&api, RC_TAG).unwrap();
    assert_eq!(
        plan.edits.iter().map(|e| e.number).collect::<Vec<_>>(),
        vec![2, 10]
    );
    assert!(
        plan.edits[0]
            .body
            .starts_with("The changes linked to this PR")
    );
    assert!(
        plan.edits[1]
            .body
            .starts_with("The changes linked to this issue")
    );
    // Every skipped target is reported to the caller rather than printed.
    let moved = "it gets no release comment, \
                 so announce the release by hand where a moved issue now lives";
    assert_eq!(
        plan.warnings,
        [
            "skipping #7: not found; a released pull request names it with a closing keyword, \
             probably by mistake, and no action is needed"
                .to_owned(),
            format!("skipping #8: moved or deleted (HTTP 301); {moved}"),
            format!("skipping #9: moved or deleted (HTTP 410); {moved}"),
        ]
    );
    api.state_mut().unavailable.insert(10, 500);
    assert!(try_plan(&api, RC_TAG).is_err());
}

#[test]
fn first_release_skips_notifications() {
    let api = repository();
    api.add_merged_pr(1, &sha('a'), "Fixes #7");
    let first = ReleaseHistory {
        previous: None,
        ..history()
    };
    let plan = github::notifications(&api, REPO, RC_TAG, &first, AUTHOR).unwrap();
    assert!(plan.edits.is_empty());
    assert_eq!(plan.warnings.len(), 1);
    assert!(plan.warnings[0].contains("first release; skipping notifications, because"));
    assert!(plan.warnings[0].ends_with("no action is needed"));
}

/// A commit as returned by the PR commits API.
fn pr_commit(digit: char, parents: &[char], message: &str) -> Value {
    json!({
        "sha": sha(digit),
        "commit": {"message": message},
        "parents": parents
            .iter()
            .map(|p| json!({"sha": sha(*p)}))
            .collect::<Vec<_>>(),
    })
}

#[test]
fn release_pr_base_requires_one_chain_of_release_commits() {
    let api = repository();
    let set = |commits: Vec<Value>| {
        api.state_mut().pr_commits.insert(5, commits);
    };
    set(vec![
        pr_commit('1', &['0'], "release: cm-testsuite v0.2.0"),
        pr_commit('2', &['1'], "release: cm-testsuite v0.2.0\n\nRegenerated"),
    ]);
    assert_eq!(
        github::release_pr_base(&api, REPO, 5).unwrap(),
        ReleasePrBase {
            sha: sha('0'),
            commits: 2
        }
    );
    set(vec![
        pr_commit('1', &['0'], "release: cm-testsuite v0.2.0"),
        pr_commit('2', &['1'], "fix(release): tweak notes"),
    ]);
    assert!(github::release_pr_base(&api, REPO, 5).is_err());
    set(vec![
        pr_commit('1', &['0'], "release: cm-testsuite v0.2.0"),
        pr_commit('2', &['1', '9'], "release: Merge branch main"),
    ]);
    assert!(github::release_pr_base(&api, REPO, 5).is_err());
    set(vec![
        pr_commit('1', &['0'], "release: cm-testsuite v0.2.0"),
        pr_commit('2', &['8'], "release: cm-testsuite v0.2.0"),
    ]);
    assert!(github::release_pr_base(&api, REPO, 5).is_err());
    set(vec![]);
    assert!(github::release_pr_base(&api, REPO, 5).is_err());
}
