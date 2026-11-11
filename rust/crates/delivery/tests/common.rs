//! Shared repository fixtures and in-memory GitHub API support for delivery tests.
// Each integration-test binary uses only its own subset of these helpers.
#![allow(dead_code)]

pub mod exchange {
    use anyhow::Result;
    use cm_acquisition::{Repository, Snapshot, acquire, load_catalog_files};
    use cm_catalog::Catalog;
    use cm_delivery::apply;
    use cm_exchange_plan::{import, receive};
    use std::{collections::BTreeMap, fs, path::Path};
    /// Repository fixture that fails before any snapshot can be acquired.
    pub struct Gone;
    impl Repository for Gone {
        fn resolve(&self, _: &str) -> Result<String> {
            anyhow::bail!("network unavailable")
        }
        fn read(&self, _: &str, _: &str) -> Result<Vec<u8>> {
            unreachable!()
        }
    }

    pub struct Source {
        pub manifest: String,
        pub files: BTreeMap<String, String>,
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
    /// Acquire one committed fixture test with stable provenance.
    pub fn snapshot(name: &str, path: &str, text: &str) -> Snapshot {
        let source = Source {
            manifest: format!(
                "schema=1\n\n[[test]]\nname='stable-name'\npath='file:///{path}'\n\
                 requires=['component-model']\ncovers=['smoke']\n",
            ),
            files: BTreeMap::from([(path.into(), text.into())]),
        };
        acquire(
            &source,
            &format!("github:example/{name}"),
            "main",
            ".cm-testsuite.toml",
        )
        .unwrap()
    }
    /// Import a fixture snapshot and return the resulting catalog.
    pub fn setup(root: &Path, snapshot: &Snapshot) -> Catalog {
        apply(root, &import(root, std::slice::from_ref(snapshot)).unwrap()).unwrap();
        Catalog::load(root).unwrap()
    }
    /// Write a receiving policy with scenario-specific overrides.
    pub fn policy(root: &Path, extra: &str) {
        fs::write(root.join(".cm-testsuite.toml"),format!("schema=1\n[receive]\nenabled=true\n{extra}\n[[receive.route]]\nname='smoke'\ndestination='file:///tests/shared'\nsupports=['component-model']\nallow_unknown=true\n")).unwrap();
    }
    /// Plan receiving from the current fixture catalog.
    pub fn received(root: &Path, catalog_root: &Path) -> cm_exchange_plan::Plan {
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
}

pub mod github {
    use anyhow::{Result, bail, ensure};
    use cm_catalog::{digest, sha256};
    use cm_exchange_plan::{Edit, Kind, Plan};
    use cm_github::Api;
    use serde_json::{Value, json};
    use std::{
        cell::RefCell,
        collections::{BTreeMap, BTreeSet},
    };
    #[derive(Clone)]
    pub struct Commit {
        pub tree: String,
        pub message: String,
        pub parents: Vec<String>,
    }
    #[derive(Default)]
    pub struct State {
        pub files: BTreeMap<String, BTreeMap<String, String>>,
        pub trees: BTreeMap<String, Value>,
        pub blobs: BTreeMap<String, String>,
        pub commits: BTreeMap<String, Commit>,
        pub refs: BTreeMap<String, String>,
        pub prs: Vec<Value>,
        pub advance_base: bool,
    }
    /// Build Git tree objects for the mock repository.
    pub fn tree(s: &mut State, files: BTreeMap<String, String>) -> String {
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
    /// Record an immutable mock commit and its file inventory.
    pub fn commit(
        s: &mut State,
        files: BTreeMap<String, String>,
        message: String,
        parents: Vec<String>,
    ) -> String {
        let tree = tree(s, files);
        let id =
            digest(&(tree.clone(), message.clone(), parents.clone())).unwrap()[..40].to_owned();
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
    pub struct Mock(pub RefCell<State>);
    impl Default for Mock {
        fn default() -> Self {
            Self::new()
        }
    }

    impl Mock {
        pub fn new() -> Self {
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
    /// Create the standard managed proposal used by publication scenarios.
    pub fn plan(text: &str) -> Plan {
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
}
