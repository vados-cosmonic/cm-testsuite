//! An in-memory GitHub object/API model. No credentials, network, or remote mutation.
use anyhow::{Result, bail, ensure};
use cm_catalog::{digest, sha256};
use cm_delivery::{Publication, branch_for, publish};
use cm_exchange_plan::{Edit, Kind, Plan};
use cm_github::Api;
use serde_json::{Value, json};
use std::{
    cell::RefCell,
    collections::{BTreeMap, BTreeSet},
};
#[derive(Clone)]
struct Commit {
    tree: String,
    message: String,
    parents: Vec<String>,
}
#[derive(Default)]
struct State {
    files: BTreeMap<String, BTreeMap<String, String>>,
    trees: BTreeMap<String, Value>,
    blobs: BTreeMap<String, String>,
    commits: BTreeMap<String, Commit>,
    refs: BTreeMap<String, String>,
    prs: Vec<Value>,
    advance_base: bool,
}
fn tree(s: &mut State, files: BTreeMap<String, String>) -> String {
    let id = digest(&files).unwrap()[..40].to_owned();
    let mut entries = vec![];
    let mut dirs: BTreeMap<String, BTreeMap<String, String>> = BTreeMap::new();
    for (path, text) in &files {
        if let Some((dir, rest)) = path.split_once('/') {
            dirs.entry(dir.into())
                .or_default()
                .insert(rest.into(), text.clone());
        } else {
            let blob = sha256(text.as_bytes())[..40].to_owned();
            s.blobs.insert(blob.clone(), text.clone());
            entries.push(json!({"path":path,"type":"blob","mode":"100644","sha":blob}));
        }
    }
    for (dir, files) in dirs {
        let child = tree(s, files);
        entries.push(json!({"path":dir,"type":"tree","mode":"040000","sha":child}));
    }
    s.files.insert(id.clone(), files);
    s.trees
        .insert(id.clone(), json!({"tree":entries,"truncated":false}));
    id
}
fn commit(
    s: &mut State,
    files: BTreeMap<String, String>,
    message: String,
    parents: Vec<String>,
) -> String {
    let tree = tree(s, files);
    let id = digest(&(tree.clone(), message.clone(), parents.clone())).unwrap()[..40].to_owned();
    s.commits.insert(
        id.clone(),
        Commit {
            tree,
            message,
            parents,
        },
    );
    id
}
struct Mock(RefCell<State>);
impl Mock {
    fn new() -> Self {
        let mut s = State::default();
        let id = commit(
            &mut s,
            BTreeMap::from([("README".into(), "unrelated source context".into())]),
            "initial".into(),
            vec![],
        );
        s.refs.insert("main".into(), id);
        Self(RefCell::new(s))
    }
}
impl Api for Mock {
    fn request(&self, method: &str, path: &str, body: Option<Value>) -> Result<Option<Value>> {
        let mut s = self.0.borrow_mut();
        let p = path
            .strip_prefix("/repos/example/target")
            .ok_or_else(|| anyhow::anyhow!("unexpected repo"))?;
        let data = body.unwrap_or(Value::Null);
        let result = match (method, p) {
            ("GET", "") => json!({"default_branch":"main"}),
            ("GET", p) if p.starts_with("/commits/") => {
                let name = &p[9..];
                let id = s.refs.get(name).cloned().unwrap_or(name.into());
                ensure!(s.commits.contains_key(&id), "missing commit");
                json!({"sha":id})
            }
            ("GET", p) if p.starts_with("/git/commits/") => {
                let c = s
                    .commits
                    .get(&p[13..])
                    .ok_or_else(|| anyhow::anyhow!("missing commit"))?;
                json!({"tree":{"sha":c.tree},"message":c.message})
            }
            ("GET", p) if p.starts_with("/git/trees/") => s
                .trees
                .get(&p[11..])
                .cloned()
                .ok_or_else(|| anyhow::anyhow!("missing tree"))?,
            ("GET", p) if p.starts_with("/git/blobs/") => {
                use base64::{Engine, engine::general_purpose::STANDARD};
                let text = s
                    .blobs
                    .get(&p[11..])
                    .ok_or_else(|| anyhow::anyhow!("missing blob"))?;
                json!({"encoding":"base64","content":STANDARD.encode(text)})
            }
            ("GET", p) if p.starts_with("/git/ref/heads/") => {
                let Some(id) = s.refs.get(&p[15..]) else {
                    return Ok(None);
                };
                json!({"object":{"sha":id}})
            }
            ("GET", p) if p.starts_with("/pulls?") => json!(
                s.prs
                    .iter()
                    .filter(|pr| !p.contains("state=open") || pr["state"] == "open")
                    .collect::<Vec<_>>()
            ),
            ("GET", p) if p.starts_with("/compare/") => {
                let (a, b) = p[9..].split_once("...").unwrap();
                let old = &s.files[&s.commits[a].tree];
                let new = &s.files[&s.commits[b].tree];
                let paths: BTreeSet<_> = old.keys().chain(new.keys()).collect();
                let changed: Vec<_> = paths
                    .into_iter()
                    .filter(|p| old.get(*p) != new.get(*p))
                    .map(|p| json!({"filename":p}))
                    .collect();
                json!({"files":changed})
            }
            ("POST", "/git/trees") => {
                let mut files = s.files[data["base_tree"].as_str().unwrap()].clone();
                for e in data["tree"].as_array().unwrap() {
                    let path = e["path"].as_str().unwrap();
                    if let Some(text) = e["content"].as_str() {
                        files.insert(path.into(), text.into());
                    } else {
                        files.remove(path);
                    }
                }
                json!({"sha":tree(&mut s,files)})
            }
            ("POST", "/git/commits") => {
                let files = s.files[data["tree"].as_str().unwrap()].clone();
                let parents = data["parents"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|p| p.as_str().unwrap().to_owned())
                    .collect();
                let sha = commit(
                    &mut s,
                    files,
                    data["message"].as_str().unwrap().into(),
                    parents,
                );
                if s.advance_base {
                    let old = s.refs["main"].clone();
                    let mut f = s.files[&s.commits[&old].tree].clone();
                    f.insert("new-default-file".into(), "new".into());
                    let newer = commit(&mut s, f, "concurrent base update".into(), vec![old]);
                    s.refs.insert("main".into(), newer);
                }
                json!({"sha":sha})
            }
            ("POST", "/git/refs") => {
                let branch = data["ref"]
                    .as_str()
                    .unwrap()
                    .strip_prefix("refs/heads/")
                    .unwrap();
                ensure!(!s.refs.contains_key(branch), "branch already exists");
                let id = data["sha"].as_str().unwrap().to_owned();
                s.refs.insert(branch.into(), id.clone());
                json!({"object":{"sha":id}})
            }
            ("PATCH", p) if p.starts_with("/git/refs/heads/") => {
                let branch = &p[16..];
                let old = s
                    .refs
                    .get(branch)
                    .ok_or_else(|| anyhow::anyhow!("missing branch"))?;
                let id = data["sha"].as_str().unwrap();
                ensure!(
                    data["force"] == false && s.commits[id].parents.contains(old),
                    "non-fast-forward branch update"
                );
                s.refs.insert(branch.into(), id.into());
                json!({"object":{"sha":id}})
            }
            ("POST", "/pulls") => {
                let number = s.prs.len() + 1;
                let mut p = data;
                p["number"] = json!(number);
                p["html_url"] = json!(format!("https://example.invalid/pull/{number}"));
                p["state"] = json!("open");
                p["merged_at"] = Value::Null;
                s.prs.push(p.clone());
                p
            }
            ("PATCH", p) if p.starts_with("/pulls/") => {
                let index = p[7..].parse::<usize>()? - 1;
                for (k, v) in data.as_object().unwrap() {
                    s.prs[index][k] = v.clone();
                }
                s.prs[index].clone()
            }
            _ => bail!("unexpected API request: {method} {path}"),
        };
        Ok(Some(result))
    }
}
fn plan(text: &str) -> Plan {
    Plan {
        schema: 1,
        kind: Kind::Import,
        subject: "github:example/source".into(),
        edits: vec![Edit {
            path: "corpus/wast/example/source/a.wast".into(),
            before: None,
            after: Some(text.into()),
        }],
        guards: vec![],
        warnings: vec![],
        exclusions: BTreeSet::new(),
        provenance: std::collections::BTreeMap::new(),
    }
}
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
