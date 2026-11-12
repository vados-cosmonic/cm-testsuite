//! One in-memory GitHub repository behind the [`Api`] boundary.
//!
//! [`FakeGitHub`] models just the REST endpoints the workspace calls: Git
//! objects and refs, pull requests, releases, and issue comments. Tests
//! arrange and inspect it through [`FakeGitHub::state`] and
//! [`FakeGitHub::state_mut`]; an endpoint it does not model is an error, so a
//! new request in production code fails loudly here.

use anyhow::{Context, Result, bail, ensure};
use base64::{Engine, engine::general_purpose::STANDARD};
use cm_catalog::{digest, sha256};
use cm_github::{Api, HttpStatus};
use percent_encoding::percent_decode_str;
use serde_json::{Value, json};
use std::{
    cell::{Ref, RefCell, RefMut},
    collections::{BTreeMap, BTreeSet},
};

/// Largest page the fake serves, like GitHub's `per_page` maximum.
pub const PAGE_SIZE: usize = 100;

/// File contents by repository path.
pub type Files = BTreeMap<String, String>;

/// An immutable commit in the fake repository.
#[derive(Clone)]
pub struct Commit {
    /// Identifier of the root tree.
    pub tree: String,
    /// Full commit message.
    pub message: String,
    /// Parent commit identifiers.
    pub parents: Vec<String>,
}

/// Everything the fake repository holds. Fields are public so tests can
/// arrange state GitHub would reach by other means and assert on the outcome.
#[derive(Default)]
pub struct State {
    /// Complete file inventory of each tree, by tree identifier.
    pub files: BTreeMap<String, Files>,
    /// Tree objects as the Git trees API returns them, by identifier.
    pub trees: BTreeMap<String, Value>,
    /// Blob contents by identifier.
    pub blobs: BTreeMap<String, String>,
    /// Commits by identifier.
    pub commits: BTreeMap<String, Commit>,
    /// Branch heads by branch name, without `refs/heads/`.
    pub refs: BTreeMap<String, String>,
    /// Tag targets by tag name, without `refs/tags/`. A tag may point at a
    /// commit the fake does not model.
    pub tags: BTreeMap<String, String>,
    /// Move `main` forward whenever a commit is created, to simulate a
    /// concurrent push.
    pub advance_base: bool,
    /// Pull requests as the pulls API returns them, in creation order.
    pub prs: Vec<Value>,
    /// Commits of each pull request, by PR number.
    pub pr_commits: BTreeMap<u64, Vec<Value>>,
    /// Pull requests associated with each commit, by commit identifier.
    pub commit_prs: BTreeMap<String, Vec<Value>>,
    /// Releases by tag name.
    pub releases: BTreeMap<String, Value>,
    /// Issue numbers that answer with this HTTP status instead of an issue.
    /// Every other number exists: as a PR when [`State::prs`] lists it, as the
    /// issue in [`State::issues`] with that number, or else as a bare issue.
    pub unavailable: BTreeMap<u64, u16>,
    /// Issues, created through the API or arranged, as the issues API returns them.
    pub issues: Vec<Value>,
    /// Comments of each issue or PR, by number.
    pub comments: BTreeMap<u64, Vec<Value>>,
    /// Number of mutating (non-`GET`) requests received.
    pub writes: usize,
}

impl State {
    /// Store the tree objects for `files` and return the root tree identifier.
    pub fn tree(&mut self, files: Files) -> String {
        let id = digest(&files).unwrap()[..40].to_owned();
        let mut entries = vec![];
        let mut directories: BTreeMap<String, Files> = BTreeMap::new();
        for (path, text) in &files {
            if let Some((directory, rest)) = path.split_once('/') {
                directories
                    .entry(directory.into())
                    .or_default()
                    .insert(rest.into(), text.clone());
            } else {
                let blob = sha256(text.as_bytes())[..40].to_owned();
                self.blobs.insert(blob.clone(), text.clone());
                entries.push(json!({
                    "path": path, "type": "blob", "mode": "100644", "sha": blob,
                }));
            }
        }
        for (directory, files) in directories {
            let child = self.tree(files);
            entries.push(json!({
                "path": directory, "type": "tree", "mode": "040000", "sha": child,
            }));
        }
        self.files.insert(id.clone(), files);
        self.trees
            .insert(id.clone(), json!({"tree": entries, "truncated": false}));
        id
    }

    /// Record a commit holding exactly `files` and return its identifier.
    pub fn commit(&mut self, files: Files, message: &str, parents: Vec<String>) -> String {
        let tree = self.tree(files);
        let id = digest(&(&tree, message, &parents)).unwrap()[..40].to_owned();
        self.commits.insert(
            id.clone(),
            Commit {
                tree,
                message: message.to_owned(),
                parents,
            },
        );
        id
    }

    /// The files of the commit at `id`.
    pub fn files_at(&self, id: &str) -> &Files {
        &self.files[&self.commits[id].tree]
    }

    /// The pull request numbered `number`. Panics when there is none.
    pub fn pr_mut(&mut self, number: u64) -> &mut Value {
        self.prs
            .iter_mut()
            .find(|pr| pr["number"] == number)
            .unwrap_or_else(|| panic!("no pull request #{number}"))
    }

    /// The comment with identifier `id`, on whichever issue holds it.
    fn comment_mut(&mut self, id: u64) -> Result<&mut Value> {
        self.comments
            .values_mut()
            .flatten()
            .find(|comment| comment["id"] == id)
            .with_context(|| format!("missing comment {id}"))
    }

    /// Simulate another contributor pushing to `main`.
    fn push_concurrent_commit(&mut self) {
        let old = self.refs["main"].clone();
        let mut files = self.files_at(&old).clone();
        files.insert("new-default-file".into(), "new".into());
        let newer = self.commit(files, "concurrent base update", vec![old]);
        self.refs.insert("main".into(), newer);
    }
}

/// A comment as the issue comments API returns it.
pub fn comment(id: u64, body: &str, author: &str) -> Value {
    json!({"id": id, "body": body, "user": {"login": author}})
}

/// Serve one page of a listing, as GitHub does for `page` and `per_page`.
fn page_of(items: Option<&Vec<Value>>, page: usize) -> Value {
    let items = items.map(Vec::as_slice).unwrap_or_default();
    json!(
        items
            .iter()
            .skip(page.saturating_sub(1) * PAGE_SIZE)
            .take(PAGE_SIZE)
            .collect::<Vec<_>>()
    )
}

/// An in-memory stand-in for one GitHub repository.
pub struct FakeGitHub {
    /// `owner/repository`, as GitHub reports it in `full_name`.
    repository: String,
    state: RefCell<State>,
}

impl FakeGitHub {
    /// Create the repository `owner/name` with one commit on its default
    /// branch, `main`.
    pub fn new(repository: &str) -> Self {
        let mut state = State::default();
        let files = Files::from([("README".into(), "unrelated source context".into())]);
        let initial = state.commit(files, "initial", vec![]);
        state.refs.insert("main".into(), initial);
        Self {
            repository: repository.to_owned(),
            state: RefCell::new(state),
        }
    }

    /// Inspect the repository.
    pub fn state(&self) -> Ref<'_, State> {
        self.state.borrow()
    }

    /// Arrange the repository. Release the borrow before the next request.
    pub fn state_mut(&self) -> RefMut<'_, State> {
        self.state.borrow_mut()
    }

    /// Number of mutating requests received so far.
    pub fn writes(&self) -> usize {
        self.state().writes
    }

    /// Add a PR that was merged into `main` as commit `sha`, and associate it
    /// with that commit.
    pub fn add_merged_pr(&self, number: u64, sha: &str, body: &str) {
        let mut state = self.state_mut();
        state.prs.push(json!({
            "number": number,
            "title": "feat: change",
            "body": body,
            "state": "closed",
            "merged_at": "2026-11-06T00:00:00Z",
            "merge_commit_sha": sha,
            "base": {"ref": "main", "repo": {"full_name": self.repository}},
        }));
        state
            .commit_prs
            .entry(sha.into())
            .or_default()
            .push(json!({"number": number}));
    }

    /// Add an open issue numbered `number`, as someone would file it.
    pub fn add_issue(&self, number: u64, title: &str, body: &str) {
        self.state_mut().issues.push(json!({
            "number": number,
            "title": title,
            "body": body,
            "state": "open",
            "html_url": format!("https://example.invalid/issues/{number}"),
        }));
    }

    /// Publish (or replace) the release for `tag`.
    pub fn add_release(&self, tag: &str, prerelease: bool) {
        self.state_mut().releases.insert(
            tag.into(),
            json!({"tag_name": tag, "draft": false, "prerelease": prerelease}),
        );
    }
}

impl Api for FakeGitHub {
    fn request(&self, method: &str, path: &str, body: Option<Value>) -> Result<Option<Value>> {
        let route = path
            .strip_prefix("/repos/")
            .and_then(|rest| rest.strip_prefix(self.repository.as_str()))
            .filter(|route| route.is_empty() || route.starts_with(['/', '?']))
            .with_context(|| format!("unexpected repository in {path}"))?;
        let (route, query) = route.split_once('?').unwrap_or((route, ""));
        // Callers percent-encode single segments such as tag names.
        let segments = route
            .split('/')
            .filter(|segment| !segment.is_empty())
            .map(|segment| percent_decode_str(segment).decode_utf8())
            .collect::<Result<Vec<_>, _>>()?;
        let segments: Vec<&str> = segments.iter().map(AsRef::as_ref).collect();
        let page = query
            .split('&')
            .find_map(|pair| pair.strip_prefix("page="))
            .unwrap_or("1")
            .parse::<usize>()?;
        let data = body.unwrap_or(Value::Null);
        let mut s = self.state.borrow_mut();
        if method != "GET" {
            s.writes += 1;
        }

        let found = match (method, segments.as_slice()) {
            // Repository.
            ("GET", []) => json!({"default_branch": "main"}),

            // Commits and Git objects.
            ("GET", ["commits", sha, "pulls"]) => page_of(s.commit_prs.get(*sha), page),
            ("GET", ["commits", name]) => {
                let id = match s.tags.get(*name) {
                    Some(target) => target.clone(),
                    None => {
                        let id = s.refs.get(*name).cloned().unwrap_or((*name).to_owned());
                        ensure!(s.commits.contains_key(&id), "missing commit");
                        id
                    }
                };
                json!({"sha": id})
            }
            ("GET", ["compare", range]) => {
                let (base, head) = range.split_once("...").context("invalid compare range")?;
                let (old, new) = (s.files_at(base), s.files_at(head));
                let paths: BTreeSet<_> = old.keys().chain(new.keys()).collect();
                let changed: Vec<_> = paths
                    .into_iter()
                    .filter(|path| old.get(*path) != new.get(*path))
                    .map(|path| json!({"filename": path}))
                    .collect();
                json!({"files": changed})
            }
            ("GET", ["git", "commits", id]) => {
                let commit = s.commits.get(*id).context("missing commit")?;
                json!({"tree": {"sha": commit.tree}, "message": commit.message})
            }
            ("GET", ["git", "trees", id]) => s.trees.get(*id).cloned().context("missing tree")?,
            ("GET", ["git", "blobs", id]) => {
                let text = s.blobs.get(*id).context("missing blob")?;
                json!({"encoding": "base64", "content": STANDARD.encode(text)})
            }
            ("POST", ["git", "trees"]) => {
                let mut files = s.files[data["base_tree"].as_str().unwrap()].clone();
                for entry in data["tree"].as_array().unwrap() {
                    let path = entry["path"].as_str().unwrap();
                    // An entry without content deletes the path.
                    if let Some(text) = entry["content"].as_str() {
                        files.insert(path.into(), text.into());
                    } else {
                        files.remove(path);
                    }
                }
                json!({"sha": s.tree(files)})
            }
            ("POST", ["git", "commits"]) => {
                let files = s.files[data["tree"].as_str().unwrap()].clone();
                let parents = data["parents"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|parent| parent.as_str().unwrap().to_owned())
                    .collect();
                let sha = s.commit(files, data["message"].as_str().unwrap(), parents);
                if s.advance_base {
                    s.push_concurrent_commit();
                }
                json!({"sha": sha})
            }

            // Branch and tag refs. Branch names may contain slashes.
            ("GET", ["git", "ref", "heads", branch @ ..]) => match s.refs.get(&branch.join("/")) {
                Some(id) => json!({"object": {"sha": id}}),
                None => return Ok(None),
            },
            ("GET", ["git", "ref", "tags", tag]) => match s.tags.get(*tag) {
                Some(id) => json!({"object": {"sha": id}}),
                None => return Ok(None),
            },
            ("POST", ["git", "refs"]) => {
                let name = data["ref"].as_str().context("missing ref name")?;
                let id = data["sha"].as_str().context("missing ref target")?;
                if let Some(branch) = name.strip_prefix("refs/heads/") {
                    ensure!(!s.refs.contains_key(branch), "branch already exists");
                    s.refs.insert(branch.into(), id.into());
                } else if let Some(tag) = name.strip_prefix("refs/tags/") {
                    ensure!(!s.tags.contains_key(tag), "tag already exists");
                    s.tags.insert(tag.into(), id.into());
                } else {
                    bail!("unsupported ref: {name}");
                }
                json!({"object": {"sha": id}})
            }
            ("PATCH", ["git", "refs", "heads", branch @ ..]) => {
                let branch = branch.join("/");
                let old = s.refs.get(&branch).context("missing branch")?;
                let id = data["sha"].as_str().unwrap();
                ensure!(
                    data["force"] == false && s.commits[id].parents.contains(old),
                    "non-fast-forward branch update"
                );
                s.refs.insert(branch, id.into());
                json!({"object": {"sha": id}})
            }

            // Pull requests.
            ("GET", ["pulls"]) => {
                // `head=owner:branch` narrows the listing to one proposal branch.
                let head = query
                    .split('&')
                    .find_map(|pair| pair.strip_prefix("head="))
                    .map(|value| {
                        percent_encoding::percent_decode_str(value)
                            .decode_utf8_lossy()
                            .into_owned()
                    });
                let branch = head
                    .as_deref()
                    .map(|head| head.split_once(':').map_or(head, |(_, branch)| branch));
                json!(
                    s.prs
                        .iter()
                        .filter(|pr| !query.contains("state=open") || pr["state"] == "open")
                        .filter(|pr| branch.is_none_or(|branch| pr["head"] == branch))
                        .collect::<Vec<_>>()
                )
            }
            ("GET", ["pulls", number]) => {
                let number = number.parse::<u64>()?;
                match s.prs.iter().find(|pr| pr["number"] == number) {
                    Some(pr) => pr.clone(),
                    None => return Ok(None),
                }
            }
            ("GET", ["pulls", number, "commits"]) => {
                page_of(s.pr_commits.get(&number.parse()?), page)
            }
            ("POST", ["pulls"]) => {
                let number = s.prs.iter().filter_map(|pr| pr["number"].as_u64()).max();
                let number = number.unwrap_or(0) + 1;
                let mut pr = data;
                pr["number"] = json!(number);
                pr["html_url"] = json!(format!("https://example.invalid/pull/{number}"));
                pr["state"] = json!("open");
                pr["merged_at"] = Value::Null;
                s.prs.push(pr.clone());
                pr
            }
            ("PATCH", ["pulls", number]) => {
                let pr = s.pr_mut(number.parse()?);
                for (key, value) in data.as_object().context("expected PR fields")? {
                    pr[key] = value.clone();
                }
                pr.clone()
            }

            // Releases.
            ("GET", ["releases", "tags", tag]) => match s.releases.get(*tag) {
                Some(release) => release.clone(),
                None => return Ok(None),
            },

            // Issues and their comments. The issues API also serves PRs.
            ("GET", ["issues", "comments", id]) => s.comment_mut(id.parse()?)?.clone(),
            ("PATCH", ["issues", "comments", id]) => {
                let comment = s.comment_mut(id.parse()?)?;
                comment["body"] = data["body"].clone();
                comment.clone()
            }
            ("GET", ["issues"]) => {
                let listed: Vec<_> = s
                    .issues
                    .iter()
                    .filter(|issue| !query.contains("state=open") || issue["state"] == "open")
                    .cloned()
                    .collect();
                page_of(Some(&listed), page)
            }
            ("POST", ["issues"]) => {
                // Issues and pull requests share one number sequence.
                let numbers = s.issues.iter().chain(&s.prs);
                let number = numbers.filter_map(|item| item["number"].as_u64()).max();
                let number = number.unwrap_or(0) + 1;
                let mut issue = data;
                issue["number"] = json!(number);
                issue["html_url"] = json!(format!("https://example.invalid/issues/{number}"));
                issue["state"] = json!("open");
                s.issues.push(issue.clone());
                issue
            }
            ("PATCH", ["issues", number]) => {
                let number = number.parse::<u64>()?;
                let issue = s
                    .issues
                    .iter_mut()
                    .find(|issue| issue["number"] == number)
                    .with_context(|| format!("missing issue {number}"))?;
                for (key, value) in data.as_object().context("expected issue fields")? {
                    issue[key] = value.clone();
                }
                issue.clone()
            }
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
                    None if s.prs.iter().any(|pr| pr["number"] == number) => {
                        json!({"number": number, "pull_request": {}})
                    }
                    None => match s.issues.iter().find(|issue| issue["number"] == number) {
                        Some(issue) => issue.clone(),
                        None => json!({"number": number}),
                    },
                }
            }
            ("GET", ["issues", number, "comments"]) => {
                page_of(s.comments.get(&number.parse()?), page)
            }
            ("POST", ["issues", number, "comments"]) => {
                // The fake authenticates every request as the Actions bot.
                let id = 1000 + s.writes as u64;
                let created = comment(id, data["body"].as_str().unwrap(), "github-actions[bot]");
                s.comments
                    .entry(number.parse()?)
                    .or_default()
                    .push(created.clone());
                created
            }

            _ => bail!("unexpected API request: {method} {path}"),
        };
        Ok(Some(found))
    }
}

/// Several in-memory repositories behind one [`Api`], for flows that read
/// from some repositories and publish to others.
#[derive(Default)]
pub struct FakeGitHubs {
    repositories: Vec<FakeGitHub>,
}

impl FakeGitHubs {
    /// Create each `owner/name` repository with one commit on `main`.
    pub fn new(repositories: &[&str]) -> Self {
        Self {
            repositories: repositories
                .iter()
                .map(|name| FakeGitHub::new(name))
                .collect(),
        }
    }

    /// The repository `owner/name`. Panics when it was not created.
    pub fn repository(&self, name: &str) -> &FakeGitHub {
        self.repositories
            .iter()
            .find(|repository| repository.repository == name)
            .unwrap_or_else(|| panic!("no repository {name}"))
    }

    /// Commit `files` on top of the current `main` of `owner/name`, replacing
    /// same-named files and keeping the rest.
    pub fn push(&self, name: &str, files: &[(&str, &str)]) {
        let mut state = self.repository(name).state_mut();
        let old = state.refs["main"].clone();
        let mut all = state.files_at(&old).clone();
        for (path, text) in files {
            all.insert((*path).into(), (*text).into());
        }
        let new = state.commit(all, "push", vec![old]);
        state.refs.insert("main".into(), new);
    }

    /// Number of mutating requests received across every repository.
    pub fn writes(&self) -> usize {
        self.repositories.iter().map(FakeGitHub::writes).sum()
    }
}

impl Api for FakeGitHubs {
    fn request(&self, method: &str, path: &str, body: Option<Value>) -> Result<Option<Value>> {
        let rest = path.strip_prefix("/repos/").unwrap_or_default();
        let repository = self.repositories.iter().find(|repository| {
            rest.strip_prefix(repository.repository.as_str())
                .is_some_and(|route| route.is_empty() || route.starts_with(['/', '?']))
        });
        // An unknown repository answers like GitHub does: not found.
        match repository {
            Some(repository) => repository.request(method, path, body),
            None => Ok(None),
        }
    }
}
