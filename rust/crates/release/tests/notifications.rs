//! Tagging and release notification planning against an in-memory GitHub API.

use anyhow::{Result, bail};
use cm_github::{Api, HttpStatus};
use cm_release::{
    Metadata,
    github::{self, COMMENT_MARKER, CommentEdit, NotificationPlan, ReleasePrBase},
    history::ReleaseHistory,
};
use serde_json::{Value, json};
use std::{
    cell::RefCell,
    collections::{BTreeMap, BTreeSet},
};

const REPO: &str = "acme/tests";
const AUTHOR: &str = "github-actions[bot]";
const RC_TAG: &str = "cm-testsuite-v0.2.0-rc.1";
const STABLE_TAG: &str = "cm-testsuite-v0.2.0";

/// Page size the client requests from listings.
const PAGE_SIZE: usize = 100;

/// Everything the fake repository holds.
struct State {
    /// The published release returned for any tag.
    release: Value,
    /// Pull requests by number.
    prs: BTreeMap<u64, Value>,
    /// Pull requests associated with each commit.
    associated: BTreeMap<String, Vec<Value>>,
    /// Comments by issue or PR number.
    comments: BTreeMap<u64, Vec<Value>>,
    /// Issue numbers that respond with this status instead of the issue.
    unavailable: BTreeMap<u64, u16>,
    /// Commits by PR number.
    pr_commits: BTreeMap<u64, Vec<Value>>,
    /// Number of mutating requests received.
    writes: usize,
    /// Commit the release tag points at, once created.
    tag: Option<String>,
}

/// In-memory stand-in for the GitHub REST endpoints the release code uses.
struct Mock(RefCell<State>);

impl Mock {
    fn new() -> Self {
        Self(RefCell::new(State {
            release: json!({"tag_name": RC_TAG, "draft": false, "prerelease": true}),
            prs: BTreeMap::new(),
            associated: BTreeMap::new(),
            comments: BTreeMap::new(),
            unavailable: BTreeMap::new(),
            pr_commits: BTreeMap::new(),
            writes: 0,
            tag: None,
        }))
    }

    /// Add a PR merged into main as `sha`, associated with that commit.
    fn add_pr(&self, number: u64, sha: &str, body: &str) {
        let mut s = self.0.borrow_mut();
        s.prs.insert(
            number,
            json!({
                "number": number,
                "title": "feat: change",
                "body": body,
                "merged_at": "2026-11-06",
                "merge_commit_sha": sha,
                "base": {"ref": "main", "repo": {"full_name": REPO}},
            }),
        );
        s.associated
            .entry(sha.into())
            .or_default()
            .push(json!({"number": number}));
    }

    /// Plan notifications for `tag` over the default history.
    fn plan(&self, tag: &str) -> Result<NotificationPlan> {
        github::notifications(self, REPO, tag, &history(), AUTHOR)
    }

    /// Plan notifications for `tag`, expecting success, and return the edits.
    fn edits(&self, tag: &str) -> Vec<CommentEdit> {
        self.plan(tag).unwrap().edits
    }

    fn apply(&self, edits: &[CommentEdit]) -> Result<()> {
        github::apply_notifications(self, REPO, AUTHOR, edits)
    }

    fn writes(&self) -> usize {
        self.0.borrow().writes
    }
}

/// Serve one page of a listing, as GitHub does for `page` and `per_page`.
fn page_of(items: Option<&Vec<Value>>, page: usize) -> Value {
    let items = items.map(Vec::as_slice).unwrap_or_default();
    json!(
        items
            .iter()
            .skip((page - 1) * PAGE_SIZE)
            .take(PAGE_SIZE)
            .collect::<Vec<_>>()
    )
}

impl Api for Mock {
    fn request(&self, method: &str, path: &str, body: Option<Value>) -> Result<Option<Value>> {
        let p = path.strip_prefix("/repos/acme/tests").unwrap();
        let (p, query) = p.split_once('?').unwrap_or((p, ""));
        let parts: Vec<_> = p.trim_start_matches('/').split('/').collect();
        let mut s = self.0.borrow_mut();
        let page = query
            .split('&')
            .find_map(|kv| kv.strip_prefix("page="))
            .unwrap_or("1")
            .parse::<usize>()?;
        let result = match (method, parts.as_slice()) {
            ("GET", ["releases", "tags", _]) => s.release.clone(),
            ("GET", ["commits", sha, "pulls"]) => page_of(s.associated.get(*sha), page),
            ("GET", ["pulls", number, "commits"]) => {
                page_of(s.pr_commits.get(&number.parse()?), page)
            }
            ("GET", ["pulls", number]) => s.prs[&number.parse::<u64>()?].clone(),
            ("GET", ["issues", number]) => {
                let number = number.parse::<u64>()?;
                match s.unavailable.get(&number) {
                    Some(404) => return Ok(None),
                    Some(&status) => {
                        return Err(HttpStatus {
                            method: method.into(),
                            path: path.into(),
                            status,
                        }
                        .into());
                    }
                    None if s.prs.contains_key(&number) => {
                        json!({"number": number, "pull_request": {}})
                    }
                    None => json!({"number": number}),
                }
            }
            ("GET", ["issues", number, "comments"]) => {
                page_of(s.comments.get(&number.parse()?), page)
            }
            ("GET", ["issues", "comments", id]) => s
                .comments
                .values()
                .flatten()
                .find(|c| c["id"] == id.parse::<u64>().unwrap())
                .unwrap()
                .clone(),
            ("PATCH", ["issues", "comments", id]) => {
                s.writes += 1;
                let c = s
                    .comments
                    .values_mut()
                    .flatten()
                    .find(|c| c["id"] == id.parse::<u64>().unwrap())
                    .unwrap();
                c["body"] = body.unwrap()["body"].clone();
                c.clone()
            }
            ("POST", ["issues", number, "comments"]) => {
                s.writes += 1;
                let c = json!({
                    "id": 1000 + s.writes,
                    "body": body.unwrap()["body"],
                    "user": {"login": AUTHOR},
                });
                s.comments
                    .entry(number.parse()?)
                    .or_default()
                    .push(c.clone());
                c
            }
            ("GET", ["git", "ref", "tags", _]) => match &s.tag {
                Some(sha) => json!({"object": {"sha": sha}}),
                None => return Ok(None),
            },
            ("GET", ["commits", _]) => json!({"sha": s.tag}),
            ("POST", ["git", "refs"]) => {
                s.writes += 1;
                s.tag = Some(body.unwrap()["sha"].as_str().unwrap().into());
                json!({})
            }
            _ => bail!("unexpected {method} {path}"),
        };
        Ok(Some(result))
    }
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

/// A comment as returned by the issue comments API.
fn comment(id: u64, body: &str, author: &str) -> Value {
    json!({"id": id, "body": body, "user": {"login": author}})
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
    let api = Mock::new();
    assert!(api.edits(RC_TAG).is_empty());
    api.add_pr(1, &sha('a'), "Fixes #7 and closes #7");
    api.add_pr(2, &sha('a'), "No issue");
    api.0
        .borrow_mut()
        .associated
        .insert(sha('b'), vec![json!({"number": 1})]);
    let edits = api.edits(RC_TAG);
    assert_eq!(
        edits.iter().map(|e| e.number).collect::<Vec<_>>(),
        vec![2, 7]
    );
    assert!(edits.iter().all(|e| {
        e.body
            .contains("https://github.com/acme/tests/releases/tag/")
    }));
    assert_eq!(api.writes(), 0);
    api.apply(&edits).unwrap();
    assert!(api.edits(RC_TAG).is_empty());
    assert_eq!(api.writes(), 2);
}

#[test]
fn excludes_unmerged_foreign_and_not_yet_released_prs() {
    let api = Mock::new();
    for number in 1..=3 {
        api.add_pr(number, &sha('a'), "");
    }
    {
        let mut s = api.0.borrow_mut();
        s.prs.get_mut(&1).unwrap()["merged_at"] = Value::Null;
        s.prs.get_mut(&2).unwrap()["base"]["repo"]["full_name"] = json!("other/repo");
        s.prs.get_mut(&3).unwrap()["merge_commit_sha"] = json!(sha('c'));
    }
    assert!(api.edits(RC_TAG).is_empty());
}

#[test]
fn release_preparation_prs_are_not_notified() {
    // With merge commits, the release PR's merge commit is not a `release:`
    // commit, so it stays in the history and leads back to the release PR.
    let api = Mock::new();
    api.add_pr(1, &sha('a'), "");
    api.add_pr(2, &sha('b'), "");
    api.0.borrow_mut().prs.get_mut(&2).unwrap()["title"] = json!("release: cm-testsuite v0.2.0");
    let edits = api.edits(RC_TAG);
    assert_eq!(edits.iter().map(|e| e.number).collect::<Vec<_>>(), vec![1]);
}

#[test]
fn paginates_and_updates_only_owned_comments_without_losing_releases() {
    let api = Mock::new();
    api.add_pr(1, &sha('a'), "");
    let mut comments = (0..100)
        .map(|id| comment(id, COMMENT_MARKER, "someone-else"))
        .collect::<Vec<_>>();
    let previous =
        format!("Previous release:\n- [old](https://example.test)\n\n{COMMENT_MARKER}\n");
    comments.push(comment(101, &previous, AUTHOR));
    api.0.borrow_mut().comments.insert(1, comments);

    let edits = api.edits(RC_TAG);
    assert_eq!(edits[0].comment_id, Some(101));
    api.apply(&edits).unwrap();

    // A later stable release keeps both earlier links.
    api.0.borrow_mut().release =
        json!({"tag_name": STABLE_TAG, "draft": false, "prerelease": false});
    let edits = api.edits(STABLE_TAG);
    assert!(edits[0].body.contains(RC_TAG));
    assert!(edits[0].body.contains("https://example.test"));
    api.apply(&edits).unwrap();

    // Rerunning the older release changes nothing.
    api.0.borrow_mut().release = json!({"tag_name": RC_TAG, "draft": false, "prerelease": true});
    assert!(api.edits(RC_TAG).is_empty());
    assert_eq!(api.0.borrow().comments[&1][0]["body"], COMMENT_MARKER);
}

#[test]
fn rejects_draft_releases_and_concurrently_edited_comments() {
    let api = Mock::new();
    api.add_pr(1, &sha('a'), "");
    api.0.borrow_mut().release["draft"] = json!(true);
    assert!(api.plan(RC_TAG).is_err());
    api.0.borrow_mut().release["draft"] = json!(false);
    api.0
        .borrow_mut()
        .comments
        .insert(1, vec![comment(101, COMMENT_MARKER, AUTHOR)]);
    let edits = api.edits(RC_TAG);
    api.0.borrow_mut().comments.get_mut(&1).unwrap()[0]["body"] = json!("changed");
    assert!(api.apply(&edits).is_err());
    assert_eq!(api.writes(), 0);
}

#[test]
fn tagging_is_explicit_idempotent_and_never_moves_existing_tags() {
    let api = Mock::new();
    let meta = Metadata::new("0.2.0").unwrap();
    assert!(github::create_tag(&api, REPO, &meta, &sha('a'), false).unwrap());
    assert_eq!(api.writes(), 0);
    assert!(github::create_tag(&api, REPO, &meta, &sha('a'), true).unwrap());
    assert!(!github::create_tag(&api, REPO, &meta, &sha('a'), true).unwrap());
    assert!(github::create_tag(&api, REPO, &meta, &sha('b'), true).is_err());
    assert_eq!(api.writes(), 1);
}

#[test]
fn skips_missing_moved_and_deleted_targets_and_names_linked_prs() {
    let api = Mock::new();
    api.add_pr(
        1,
        &sha('a'),
        "Fixes #7, fixes #8, fixes #9, fixes #10, fixes #2",
    );
    // #2 is a PR outside this release, so it is only a linked target.
    api.add_pr(2, &sha('c'), "");
    api.0
        .borrow_mut()
        .unavailable
        .extend([(7, 404), (8, 301), (9, 410)]);
    let plan = api.plan(RC_TAG).unwrap();
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
    assert_eq!(
        plan.warnings,
        [
            "skipping #7: not found",
            "skipping #8: moved or deleted (HTTP 301)",
            "skipping #9: moved or deleted (HTTP 410)",
        ]
    );
    api.0.borrow_mut().unavailable.insert(10, 500);
    assert!(api.plan(RC_TAG).is_err());
}

#[test]
fn first_release_skips_notifications() {
    let api = Mock::new();
    api.add_pr(1, &sha('a'), "Fixes #7");
    let first = ReleaseHistory {
        previous: None,
        ..history()
    };
    let plan = github::notifications(&api, REPO, RC_TAG, &first, AUTHOR).unwrap();
    assert!(plan.edits.is_empty());
    assert_eq!(plan.warnings.len(), 1);
    assert!(plan.warnings[0].contains("first release"));
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
    let api = Mock::new();
    let set = |commits: Vec<Value>| {
        api.0.borrow_mut().pr_commits.insert(5, commits);
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
