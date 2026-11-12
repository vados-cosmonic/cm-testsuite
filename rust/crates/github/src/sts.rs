//! Short-lived, repository-scoped tokens for GitHub Actions through Octo STS.
//!
//! A workflow proves its identity with a GitHub Actions OIDC token. Octo STS
//! exchanges that for a GitHub App token when the target repository's
//! `.github/chainguard/<identity>.sts.yaml` trust policy accepts the workflow.

use crate::client::{fetch_json, revoke};
use crate::{Api, Client};
use anyhow::{Context, Result, ensure};
use cm_catalog::validate_source;
use serde_json::Value;

/// The public Octo STS instance, which is also the OIDC audience it expects.
const DOMAIN: &str = "octo-sts.dev";

/// A workload that can request tokens for one named trust policy.
pub struct OctoSts {
    identity: String,
    request_url: String,
    request_token: String,
}

/// Split an absolute HTTPS URL into its origin and its path with query.
fn split(url: &str) -> Result<(&str, &str)> {
    let rest = url
        .strip_prefix("https://")
        .context("expected an HTTPS URL")?;
    let host = rest.find('/').context("URL has no path")?;
    ensure!(host > 0, "URL has no host");
    Ok(url.split_at("https://".len() + host))
}

/// Ask for an OIDC token whose audience is the exchange service.
fn audience_route(route: &str) -> String {
    let separator = if route.contains('?') { '&' } else { '?' };
    format!("{route}{separator}audience={DOMAIN}")
}

/// Name the repository and trust policy to exchange for.
fn exchange_route(repository: &str, identity: &str) -> String {
    format!("/sts/exchange?scope={repository}&scopes={repository}&identity={identity}")
}

impl OctoSts {
    /// Use the OIDC endpoint GitHub Actions exposes to jobs with `id-token: write`.
    pub fn from_actions(identity: &str) -> Result<Self> {
        ensure!(
            !identity.is_empty()
                && identity
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b"-_.".contains(&b)),
            "invalid Octo STS identity: {identity}"
        );
        let variable = |name: &str| {
            std::env::var(name)
                .ok()
                .filter(|value| !value.is_empty())
                .with_context(|| {
                    format!(
                        "{name} is not set; Octo STS needs a GitHub Actions job \
                         with the id-token: write permission"
                    )
                })
        };
        Ok(Self {
            identity: identity.to_owned(),
            request_url: variable("ACTIONS_ID_TOKEN_REQUEST_URL")?,
            request_token: variable("ACTIONS_ID_TOKEN_REQUEST_TOKEN")?,
        })
    }

    /// Mint a token for `github:owner/repository` under this identity's trust policy.
    fn token(&self, repository: &str) -> Result<String> {
        validate_source(repository)?;
        let name = repository.trim_start_matches("github:");
        let (origin, route) = split(&self.request_url)?;
        let oidc = fetch_json(origin, &audience_route(route), &self.request_token)
            .context("requesting a GitHub Actions identity token")?;
        let oidc = oidc["value"]
            .as_str()
            .context("identity token response has no value")?;
        let exchanged = fetch_json(
            &format!("https://{DOMAIN}"),
            &exchange_route(name, &self.identity),
            oidc,
        )
        .with_context(|| {
            format!(
                "exchanging a token for {name}; check that Octo STS is installed there \
                 and that .github/chainguard/{}.sts.yaml trusts this workflow",
                self.identity
            )
        })?;
        Ok(exchanged["token"]
            .as_str()
            .context("token exchange response has no token")?
            .to_owned())
    }

    /// Open a connection to `github:owner/repository` whose token is revoked
    /// when the connection is dropped.
    pub fn connect(&self, repository: &str) -> Result<Session> {
        let token = self.token(repository)?;
        Ok(Session {
            client: Client::new(Some(token.clone()))?,
            repository: repository.to_owned(),
            token,
        })
    }
}

/// A connection authorized by one minted token, which it revokes on drop so
/// the token does not outlive the work it was requested for.
pub struct Session {
    client: Client,
    repository: String,
    token: String,
}

impl Api for Session {
    fn request(&self, method: &str, path: &str, body: Option<Value>) -> Result<Option<Value>> {
        self.client.request(method, path, body)
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        // The work is already done, so a failed revocation is reported rather
        // than turned into a failure; the token then expires on its own.
        if let Err(error) = revoke(&self.token) {
            eprintln!(
                "warning: could not revoke the token for {}: {error:#}",
                self.repository
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_exchange_requests_from_actions_urls() {
        let url = "https://run.actions.example/id/token?api-version=2.0";
        let (origin, route) = split(url).unwrap();
        assert_eq!(origin, "https://run.actions.example");
        assert_eq!(
            audience_route(route),
            "/id/token?api-version=2.0&audience=octo-sts.dev"
        );
        assert_eq!(audience_route("/token"), "/token?audience=octo-sts.dev");
        assert_eq!(
            exchange_route("example/repo", "cm-testsuite"),
            "/sts/exchange?scope=example/repo&scopes=example/repo&identity=cm-testsuite"
        );
        for bad in [
            "http://plain.example/token",
            "https://nopath.example",
            "https:///x",
        ] {
            assert!(split(bad).is_err(), "{bad}");
        }
    }
}
