use anyhow::{Result, bail};
use cm_github::{Api, HttpStatus};
use cm_release::{
    Metadata,
    github::{self, COMMENT_MARKER},
    history::ReleaseHistory,
};
use serde_json::{Value, json};
use std::{
    cell::RefCell,
    collections::{BTreeMap, BTreeSet},
};
const REPO: &str = "acme/tests";
const AUTHOR: &str = "github-actions[bot]";
struct State {
    release: Value,
    prs: BTreeMap<u64, Value>,
    associated: BTreeMap<String, Vec<Value>>,
    comments: BTreeMap<u64, Vec<Value>>,
    // Issue numbers that respond with this status instead of the issue.
    unavailable: BTreeMap<u64, u16>,
    pr_commits: BTreeMap<u64, Vec<Value>>,
    writes: usize,
    tag: Option<String>,
}
struct Mock(RefCell<State>);
impl Mock {
    fn new() -> Self {
        Self(RefCell::new(State {
            release: json!({"tag_name":"cm-testsuite-v0.2.0-rc.1","draft":false,"prerelease":true}),
            prs: BTreeMap::new(),
            associated: BTreeMap::new(),
            comments: BTreeMap::new(),
            unavailable: BTreeMap::new(),
            pr_commits: BTreeMap::new(),
            writes: 0,
            tag: None,
        }))
    }
    fn add_pr(&self, number: u64, sha: &str, body: &str) {
        let mut s = self.0.borrow_mut();
        s.prs.insert(number, json!({"number":number,"body":body,"merged_at":"2026-11-06","merge_commit_sha":sha,"base":{"ref":"main","repo":{"full_name":REPO}}}));
        s.associated
            .entry(sha.into())
            .or_default()
            .push(json!({"number":number}));
    }
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
            ("GET", ["commits", sha, "pulls"]) => json!(
                s.associated
                    .get(*sha)
                    .cloned()
                    .unwrap_or_default()
                    .into_iter()
                    .skip((page - 1) * 100)
                    .take(100)
                    .collect::<Vec<_>>()
            ),
            ("GET", ["pulls", number, "commits"]) => json!(
                s.pr_commits
                    .get(&number.parse::<u64>()?)
                    .cloned()
                    .unwrap_or_default()
                    .into_iter()
                    .skip((page - 1) * 100)
                    .take(100)
                    .collect::<Vec<_>>()
            ),
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
                        json!({"number":number,"pull_request":{}})
                    }
                    None => json!({"number":number}),
                }
            }
            ("GET", ["issues", number, "comments"]) => json!(
                s.comments
                    .get(&number.parse::<u64>()?)
                    .cloned()
                    .unwrap_or_default()
                    .into_iter()
                    .skip((page - 1) * 100)
                    .take(100)
                    .collect::<Vec<_>>()
            ),
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
                let c = json!({"id":1000+s.writes,"body":body.unwrap()["body"],"user":{"login":AUTHOR}});
                s.comments
                    .entry(number.parse()?)
                    .or_default()
                    .push(c.clone());
                c
            }
            ("GET", ["git", "ref", "tags", _]) => match &s.tag {
                Some(sha) => json!({"object":{"sha":sha}}),
                None => return Ok(None),
            },
            ("GET", ["commits", _]) => json!({"sha":s.tag}),
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
fn history() -> ReleaseHistory {
    ReleaseHistory {
        previous: Some("cm-testsuite-v0.1.0".into()),
        commits: vec!["a".repeat(40), "b".repeat(40)],
        ancestors: BTreeSet::from(["a".repeat(40), "b".repeat(40)]),
    }
}
#[test]
fn parses_only_real_same_repository_closing_references() {
    let body = "Fixes #12, resolves acme/tests#13. CLOSED https://github.com/acme/tests/issues/14\nFixes other/tests#99\nFixes https://github.com/acme/tests-other/issues/88\nFixes #0\nFixes #12abc\nnotfixes #15\n`Fixes #16`\n```text\nFixes #17\n```\n";
    assert_eq!(
        github::closed_issues(body, REPO),
        BTreeSet::from([12, 13, 14])
    );
}
#[test]
fn no_prs_multiple_prs_and_duplicate_commits_are_handled() {
    let api = Mock::new();
    assert!(
        github::notifications(&api, REPO, "cm-testsuite-v0.2.0-rc.1", &history(), AUTHOR)
            .unwrap()
            .is_empty()
    );
    api.add_pr(1, &"a".repeat(40), "Fixes #7 and closes #7");
    api.add_pr(2, &"a".repeat(40), "No issue");
    api.0
        .borrow_mut()
        .associated
        .insert("b".repeat(40), vec![json!({"number":1})]);
    let edits =
        github::notifications(&api, REPO, "cm-testsuite-v0.2.0-rc.1", &history(), AUTHOR).unwrap();
    assert_eq!(
        edits.iter().map(|e| e.number).collect::<Vec<_>>(),
        vec![2, 7]
    );
    assert!(edits.iter().all(|e| {
        e.body
            .contains("https://github.com/acme/tests/releases/tag/")
    }));
    assert_eq!(api.0.borrow().writes, 0);
    github::apply_notifications(&api, REPO, AUTHOR, &edits).unwrap();
    assert!(
        github::notifications(&api, REPO, "cm-testsuite-v0.2.0-rc.1", &history(), AUTHOR)
            .unwrap()
            .is_empty()
    );
    assert_eq!(api.0.borrow().writes, 2);
}
#[test]
fn excludes_unmerged_foreign_and_not_yet_released_prs() {
    let api = Mock::new();
    for number in 1..=3 {
        api.add_pr(number, &"a".repeat(40), "");
    }
    {
        let mut s = api.0.borrow_mut();
        s.prs.get_mut(&1).unwrap()["merged_at"] = Value::Null;
        s.prs.get_mut(&2).unwrap()["base"]["repo"]["full_name"] = json!("other/repo");
        s.prs.get_mut(&3).unwrap()["merge_commit_sha"] = json!("c".repeat(40));
    }
    assert!(
        github::notifications(&api, REPO, "cm-testsuite-v0.2.0-rc.1", &history(), AUTHOR)
            .unwrap()
            .is_empty()
    );
}
#[test]
fn paginates_and_updates_only_owned_comments_without_losing_releases() {
    let api = Mock::new();
    api.add_pr(1, &"a".repeat(40), "");
    let mut comments = (0..100)
        .map(|id| json!({"id":id,"body":COMMENT_MARKER,"user":{"login":"someone-else"}}))
        .collect::<Vec<_>>();
    comments.push(json!({"id":101,"body":format!("Previous release:\n- [old](https://example.test)\n\n{COMMENT_MARKER}\n"),"user":{"login":AUTHOR}}));
    api.0.borrow_mut().comments.insert(1, comments);
    let edits =
        github::notifications(&api, REPO, "cm-testsuite-v0.2.0-rc.1", &history(), AUTHOR).unwrap();
    assert_eq!(edits[0].comment_id, Some(101));
    github::apply_notifications(&api, REPO, AUTHOR, &edits).unwrap();
    api.0.borrow_mut().release =
        json!({"tag_name":"cm-testsuite-v0.2.0","draft":false,"prerelease":false});
    let edits =
        github::notifications(&api, REPO, "cm-testsuite-v0.2.0", &history(), AUTHOR).unwrap();
    assert!(edits[0].body.contains("cm-testsuite-v0.2.0-rc.1"));
    assert!(edits[0].body.contains("https://example.test"));
    github::apply_notifications(&api, REPO, AUTHOR, &edits).unwrap();
    api.0.borrow_mut().release =
        json!({"tag_name":"cm-testsuite-v0.2.0-rc.1","draft":false,"prerelease":true});
    assert!(
        github::notifications(&api, REPO, "cm-testsuite-v0.2.0-rc.1", &history(), AUTHOR)
            .unwrap()
            .is_empty()
    );
    assert_eq!(api.0.borrow().comments[&1][0]["body"], COMMENT_MARKER);
}
#[test]
fn rejects_draft_releases_and_concurrently_edited_comments() {
    let api = Mock::new();
    api.add_pr(1, &"a".repeat(40), "");
    api.0.borrow_mut().release["draft"] = json!(true);
    assert!(
        github::notifications(&api, REPO, "cm-testsuite-v0.2.0-rc.1", &history(), AUTHOR).is_err()
    );
    api.0.borrow_mut().release["draft"] = json!(false);
    api.0.borrow_mut().comments.insert(
        1,
        vec![json!({"id":101,"body":COMMENT_MARKER,"user":{"login":AUTHOR}})],
    );
    let edits =
        github::notifications(&api, REPO, "cm-testsuite-v0.2.0-rc.1", &history(), AUTHOR).unwrap();
    api.0.borrow_mut().comments.get_mut(&1).unwrap()[0]["body"] = json!("changed");
    assert!(github::apply_notifications(&api, REPO, AUTHOR, &edits).is_err());
    assert_eq!(api.0.borrow().writes, 0);
}
#[test]
fn tagging_is_explicit_idempotent_and_never_moves_existing_tags() {
    let api = Mock::new();
    let meta = Metadata::new("0.2.0").unwrap();
    assert!(github::create_tag(&api, REPO, &meta, &"a".repeat(40), false).unwrap());
    assert_eq!(api.0.borrow().writes, 0);
    assert!(github::create_tag(&api, REPO, &meta, &"a".repeat(40), true).unwrap());
    assert!(!github::create_tag(&api, REPO, &meta, &"a".repeat(40), true).unwrap());
    assert!(github::create_tag(&api, REPO, &meta, &"b".repeat(40), true).is_err());
    assert_eq!(api.0.borrow().writes, 1);
}
#[test]
fn skips_missing_moved_and_deleted_targets_and_names_linked_prs() {
    let api = Mock::new();
    api.add_pr(
        1,
        &"a".repeat(40),
        "Fixes #7, fixes #8, fixes #9, fixes #10, fixes #2",
    );
    // #2 is a PR outside this release, so it is only a linked target.
    api.add_pr(2, &"c".repeat(40), "");
    api.0
        .borrow_mut()
        .unavailable
        .extend([(7, 404), (8, 301), (9, 410)]);
    let edits =
        github::notifications(&api, REPO, "cm-testsuite-v0.2.0-rc.1", &history(), AUTHOR).unwrap();
    assert_eq!(
        edits.iter().map(|e| e.number).collect::<Vec<_>>(),
        vec![2, 10]
    );
    assert!(edits[0].body.starts_with("The changes linked to this PR"));
    assert!(
        edits[1]
            .body
            .starts_with("The changes linked to this issue")
    );
    api.0.borrow_mut().unavailable.insert(10, 500);
    assert!(
        github::notifications(&api, REPO, "cm-testsuite-v0.2.0-rc.1", &history(), AUTHOR).is_err()
    );
}
#[test]
fn first_release_skips_notifications() {
    let api = Mock::new();
    api.add_pr(1, &"a".repeat(40), "Fixes #7");
    let first = ReleaseHistory {
        previous: None,
        ..history()
    };
    assert!(
        github::notifications(&api, REPO, "cm-testsuite-v0.2.0-rc.1", &first, AUTHOR)
            .unwrap()
            .is_empty()
    );
}
fn pr_commit(sha: char, parents: &[char], message: &str) -> Value {
    json!({
        "sha": sha.to_string().repeat(40),
        "commit": {"message": message},
        "parents": parents
            .iter()
            .map(|p| json!({"sha": p.to_string().repeat(40)}))
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
        ("0".repeat(40), 2)
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
