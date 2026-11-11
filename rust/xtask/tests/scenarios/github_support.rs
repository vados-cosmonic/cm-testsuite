//! Disposable private GitHub repositories, bounded polling, and verified cleanup.

use anyhow::{Context, Result, bail, ensure};
use cm_exchange_plan::Plan;
use cm_github::{Client, read_file, repo_path, required, resolve, tree_sha};
use serde_json::{Value, json};
use std::{
    io::Write,
    process::{Command, Stdio},
    thread,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

/// Repository identity captured from creation, never accepted from user input.
#[derive(Clone)]
pub struct Remote {
    pub source: String,
    full_name: String,
    id: u64,
    pub branch: String,
}

/// Own credentials and only repositories successfully created by this test run.
pub struct Sandbox {
    pub api: Client,
    token: String,
    owner: String,
    prefix: String,
    created: Vec<Remote>,
}

impl Sandbox {
    /// Require deliberate opt-in; use an explicit token or the logged-in GitHub CLI.
    pub fn new() -> Result<Self> {
        ensure!(
            std::env::var("CM_TESTSUITE_LIVE_GITHUB").as_deref() == Ok("1"),
            "set CM_TESTSUITE_LIVE_GITHUB=1 to create and delete private test repositories"
        );
        let token = match std::env::var("GH_TOKEN").or_else(|_| std::env::var("GITHUB_TOKEN")) {
            Ok(token) if !token.trim().is_empty() => token,
            _ => {
                let output = Command::new("gh")
                    .args(["auth", "token", "--hostname", "github.com"])
                    .output()
                    .context("install gh and run gh auth login")?;
                ensure!(
                    output.status.success(),
                    "gh auth token failed; log in to github.com"
                );
                String::from_utf8(output.stdout)?.trim().to_owned()
            }
        };
        let api = Client::new(Some(token.clone()))?;
        let stamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
        let mut sandbox = Self {
            api,
            token,
            owner: String::new(),
            prefix: format!("cm-testsuite-e2e-{stamp}-{}", std::process::id()),
            created: vec![],
        };
        sandbox.check_classic_token_scopes()?;
        let user = sandbox.gh_api("GET", "/user", None)?;
        sandbox.owner = user["login"]
            .as_str()
            .context("missing authenticated login")?
            .into();
        Ok(sandbox)
    }

    /// Keep tokens in child environment variables; never in arguments or diagnostics.
    fn command(&self) -> Command {
        let mut command = Command::new("gh");
        command
            .env("GH_TOKEN", &self.token)
            .env("GH_HOST", "github.com")
            .env("GH_PROMPT_DISABLED", "1")
            .env_remove("GITHUB_TOKEN");
        command
    }

    /// Fail before creation when a classic token cannot clean up private fixtures.
    fn check_classic_token_scopes(&self) -> Result<()> {
        let output = self
            .command()
            .args(["api", "--hostname", "github.com", "--include", "/user"])
            .output()?;
        ensure!(
            output.status.success(),
            "GitHub authentication check failed"
        );
        let response = String::from_utf8(output.stdout)?;
        let scopes = response
            .lines()
            .filter_map(|line| line.split_once(':'))
            .find(|(name, _)| name.eq_ignore_ascii_case("x-oauth-scopes"))
            .map(|(_, value)| value.split(',').map(str::trim).collect::<Vec<_>>());
        // Fine-grained tokens do not expose OAuth scopes; GitHub enforces their
        // repository administration permissions at each lifecycle operation.
        if let Some(scopes) = scopes.filter(|scopes| scopes.iter().any(|s| !s.is_empty())) {
            ensure!(
                scopes.contains(&"repo") && scopes.contains(&"delete_repo"),
                "live tests require repo and delete_repo scopes; no repositories have been created"
            );
        }
        Ok(())
    }

    /// Use gh only for repository lifecycle and simulated maintainer operations.
    fn gh_api(&self, method: &str, path: &str, body: Option<Value>) -> Result<Value> {
        let mut command = self.command();
        command.args(["api", "--hostname", "github.com", "--method", method, path]);
        if body.is_some() {
            command.args(["--input", "-"]);
        }
        let mut child = command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;
        if let Some(body) = body {
            child
                .stdin
                .take()
                .context("missing stdin")?
                .write_all(&serde_json::to_vec(&body)?)?;
        }
        let output = child.wait_with_output()?;
        ensure!(
            output.status.success(),
            "GitHub {method} {path} failed; check repository permissions and rate limits"
        );
        if output.stdout.is_empty() {
            Ok(Value::Null)
        } else {
            Ok(serde_json::from_slice(&output.stdout)?)
        }
    }

    /// Create under the authenticated user's namespace with an unguessable run prefix.
    pub fn create(&mut self, role: &str) -> Result<Remote> {
        ensure!(
            ["source", "catalog", "recipient"].contains(&role),
            "invalid fixture role"
        );
        let name = format!("{}-{role}", self.prefix);
        let response = self.gh_api("POST", "/user/repos", Some(json!({
            "name": name, "private": true, "auto_init": true,
            "description": "Temporary cm-testsuite live E2E fixture; safe to remove after its run"
        })))?;
        let full_name = format!("{}/{name}", self.owner);
        let remote = Remote {
            source: format!("github:{full_name}"),
            full_name,
            id: response["id"]
                .as_u64()
                .context("missing created repository ID")?,
            branch: response["default_branch"]
                .as_str()
                .context("missing default branch")?
                .into(),
        };
        self.created.push(remote.clone());
        eprintln!(
            "Created temporary repository https://github.com/{}",
            remote.full_name
        );
        // Auto-initialization can finish just after the creation API returns.
        self.wait_for(|| Ok(resolve(&self.api, &remote.source, &remote.branch).is_ok()))?;
        Ok(remote)
    }

    /// Retry only read assertions, with a fixed deadline; never repeat a mutation.
    pub fn wait_for(&self, mut check: impl FnMut() -> Result<bool>) -> Result<()> {
        for _ in 0..30 {
            if check()? {
                return Ok(());
            }
            thread::sleep(Duration::from_secs(1));
        }
        bail!("GitHub state did not become visible within 30 seconds")
    }

    /// Seed/update producer and policy files using the actual GitHub Git data API.
    pub fn commit_files(&self, repo: &Remote, files: &[(&str, &str)]) -> Result<()> {
        self.require_owned(repo)?;
        let prefix = repo_path(&repo.source)?;
        let base = resolve(&self.api, &repo.source, &repo.branch)?;
        let entries: Vec<_> = files.iter().map(|(path, content)|
            json!({"path": path, "content": content, "mode": "100644", "type": "blob"})).collect();
        let tree = required(
            &self.api,
            "POST",
            &format!("{prefix}/git/trees"),
            Some(json!({"base_tree": tree_sha(&self.api, &repo.source, &base)?, "tree": entries})),
        )?;
        let commit = required(
            &self.api,
            "POST",
            &format!("{prefix}/git/commits"),
            Some(
                json!({"message": "test: update isolated fixture", "tree": tree["sha"], "parents": [base]}),
            ),
        )?;
        required(
            &self.api,
            "PATCH",
            &format!("{prefix}/git/refs/heads/{}", repo.branch),
            Some(json!({"sha": commit["sha"], "force": false})),
        )?;
        self.wait_for(|| Ok(resolve(&self.api, &repo.source, &repo.branch)? == commit["sha"]))
    }

    /// Compare exact bytes on the remote default branch after a completed operation.
    pub fn assert_file(&self, repo: &Remote, path: &str, bytes: Option<&[u8]>) -> Result<()> {
        self.wait_for(|| {
            let revision = resolve(&self.api, &repo.source, &repo.branch)?;
            Ok(read_file(&self.api, &repo.source, &revision, path)?.as_deref() == bytes)
        })
    }

    /// Publish twice to verify one draft PR, then simulate review and merge it.
    pub fn publish_and_merge(&self, repo: &Remote, plan: &Plan) -> Result<()> {
        self.require_owned(repo)?;
        let first = cm_delivery::publish(&self.api, &repo.source, plan)?;
        let cm_delivery::Publication::PullRequest(url) = &first else {
            bail!("expected a draft PR, got {first:?}");
        };
        ensure!(
            cm_delivery::publish(&self.api, &repo.source, plan)? == first,
            "publication created a second proposal"
        );
        let number = url.rsplit('/').next().context("missing PR number")?;
        let path = format!("/repos/{}/pulls/{number}", repo.full_name);
        let pr = self.gh_api("GET", &path, None)?;
        ensure!(
            pr["draft"] == true && pr["state"] == "open",
            "proposal must be an open draft"
        );
        let output = self.command().args(["pr", "ready", url]).output()?;
        ensure!(output.status.success(), "could not mark fixture PR ready");
        self.wait_for(|| Ok(!self.gh_api("GET", &path, None)?["mergeable"].is_null()))?;
        let merged = self.gh_api(
            "PUT",
            &format!("{path}/merge"),
            Some(json!({"merge_method": "merge", "sha": pr["head"]["sha"]})),
        )?;
        ensure!(merged["merged"] == true, "fixture PR did not merge");
        for edit in &plan.edits {
            self.assert_file(repo, &edit.path, edit.after.as_deref().map(str::as_bytes))?;
        }
        Ok(())
    }

    /// Restrict mutation to the exact repository names and IDs this run created.
    fn require_owned(&self, repo: &Remote) -> Result<()> {
        ensure!(
            self.created
                .iter()
                .any(|owned| owned.id == repo.id && owned.full_name == repo.full_name),
            "repository was not created by this run"
        );
        let current = self.gh_api("GET", &format!("/repos/{}", repo.full_name), None)?;
        ensure!(
            current["id"] == repo.id
                && current["full_name"] == repo.full_name
                && current["private"] == true,
            "temporary repository identity changed; refusing mutation"
        );
        Ok(())
    }

    /// Delete only verified temporary repositories, retaining failed entries for recovery.
    pub fn cleanup(&mut self) -> Result<()> {
        let mut failures = vec![];
        for repo in self.created.clone().into_iter().rev() {
            let result = self.require_owned(&repo).and_then(|()| {
                self.gh_api("DELETE", &format!("/repos/{}", repo.full_name), None)
                    .map(|_| ())
            });
            match result {
                Ok(()) => self.created.retain(|r| r.id != repo.id),
                Err(_) => failures.push(repo.full_name),
            }
        }
        ensure!(
            failures.is_empty(),
            "cleanup failed; remove these test repositories manually: {}",
            failures.join(", ")
        );
        Ok(())
    }
}

impl Drop for Sandbox {
    /// Attempt cleanup on assertion failures too, without masking the original panic.
    fn drop(&mut self) {
        if let Err(error) = self.cleanup() {
            eprintln!("{error}");
        }
    }
}
