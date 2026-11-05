//! Small synchronous GitHub REST boundary shared by acquisition and delivery.
//! Credentials are supplied by the caller and are never serialized or logged.
use anyhow::{Context, Result, bail, ensure};
use base64::{Engine, engine::general_purpose::STANDARD};
use cm_catalog::{validate_path, validate_revision, validate_source};
use percent_encoding::{NON_ALPHANUMERIC, utf8_percent_encode};
use serde_json::Value;
use std::time::Duration;
pub fn encode(s: &str) -> String {
    utf8_percent_encode(s, NON_ALPHANUMERIC).to_string()
}
pub trait Api {
    fn request(&self, method: &str, path: &str, body: Option<Value>) -> Result<Option<Value>>;
}
pub struct Client {
    agent: ureq::Agent,
    token: Option<String>,
}
impl Client {
    pub fn new(token: Option<String>) -> Self {
        Self {
            agent: ureq::Agent::config_builder()
                .timeout_global(Some(Duration::from_secs(60)))
                .max_redirects(0)
                .http_status_as_error(false)
                .build()
                .into(),
            token: token.filter(|value| !value.trim().is_empty()),
        }
    }
}
impl Api for Client {
    fn request(&self, method: &str, path: &str, body: Option<Value>) -> Result<Option<Value>> {
        ensure!(
            path.starts_with("/repos/") && !path.contains(['\r', '\n']),
            "invalid GitHub API path"
        );
        let mut req = ureq::http::Request::builder()
            .method(method)
            .uri(format!("https://api.github.com{path}"))
            .header("User-Agent", "cm-testsuite/0.1")
            .header("Accept", "application/vnd.github+json")
            .header("X-GitHub-Api-Version", "2022-11-28");
        if let Some(token) = &self.token {
            req = req.header("Authorization", format!("Bearer {token}"));
        }
        let payload = body
            .map(|v| serde_json::to_vec(&v))
            .transpose()?
            .unwrap_or_default();
        if !payload.is_empty() {
            req = req.header("Content-Type", "application/json");
        }
        let mut res = self.agent.run(req.body(payload)?)?;
        let status = res.status().as_u16();
        if status == 404 {
            return Ok(None);
        }
        ensure!(
            (200..300).contains(&status),
            "GitHub {method} {path} returned HTTP {status}; check token permissions, rate limits, and branch state"
        );
        if status == 204 {
            return Ok(Some(Value::Null));
        }
        Ok(Some(res.body_mut().read_json()?))
    }
}
pub fn required(api: &dyn Api, method: &str, path: &str, body: Option<Value>) -> Result<Value> {
    api.request(method, path, body)?
        .with_context(|| format!("GitHub object not found: {path}"))
}
pub fn repo_path(source: &str) -> Result<String> {
    validate_source(source)?;
    Ok(format!(
        "/repos/{}",
        source.strip_prefix("github:").unwrap()
    ))
}
pub fn resolve(api: &dyn Api, source: &str, reference: &str) -> Result<String> {
    ensure!(!reference.is_empty(), "empty reference");
    let v = required(
        api,
        "GET",
        &format!("{}/commits/{}", repo_path(source)?, encode(reference)),
        None,
    )?;
    let sha = v["sha"].as_str().context("missing commit SHA")?.to_owned();
    validate_revision(&sha)?;
    Ok(sha)
}
pub fn tree_sha(api: &dyn Api, source: &str, revision: &str) -> Result<String> {
    validate_revision(revision)?;
    let v = required(
        api,
        "GET",
        &format!("{}/git/commits/{revision}", repo_path(source)?),
        None,
    )?;
    Ok(v["tree"]["sha"]
        .as_str()
        .context("missing commit tree")?
        .to_owned())
}
/// Traverse trees explicitly: the contents API can follow symlinks, which we reject.
pub fn read_file(
    api: &dyn Api,
    source: &str,
    revision: &str,
    path: &str,
) -> Result<Option<Vec<u8>>> {
    validate_path(path)?;
    validate_revision(revision)?;
    let base = repo_path(source)?;
    let mut tree = tree_sha(api, source, revision)?;
    let parts: Vec<_> = path.split('/').collect();
    for (i, part) in parts.iter().enumerate() {
        let data = required(api, "GET", &format!("{base}/git/trees/{tree}"), None)?;
        ensure!(
            data["truncated"] != true,
            "GitHub tree response truncated; refusing incomplete acquisition"
        );
        let entries = data["tree"].as_array().context("missing tree entries")?;
        let Some(entry) = entries.iter().find(|x| x["path"].as_str() == Some(part)) else {
            return Ok(None);
        };
        let sha = entry["sha"].as_str().context("missing entry SHA")?;
        if i + 1 < parts.len() {
            ensure!(
                entry["type"] == "tree" && entry["mode"] == "040000",
                "non-directory in source path: {path}"
            );
            tree = sha.to_owned();
            continue;
        }
        ensure!(
            entry["type"] == "blob" && (entry["mode"] == "100644" || entry["mode"] == "100755"),
            "source is not a regular file: {path}"
        );
        let blob = required(api, "GET", &format!("{base}/git/blobs/{sha}"), None)?;
        ensure!(
            blob["encoding"] == "base64",
            "unsupported GitHub blob encoding"
        );
        let raw = blob["content"]
            .as_str()
            .context("missing blob content")?
            .chars()
            .filter(|x| !x.is_whitespace())
            .collect::<String>();
        let bytes = STANDARD.decode(raw)?;
        ensure!(
            bytes.len() <= 8 * 1024 * 1024,
            "file exceeds 8 MiB draft limit"
        );
        return Ok(Some(bytes));
    }
    bail!("empty source path")
}
