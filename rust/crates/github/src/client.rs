//! Synchronous command boundary around Octocrab's asynchronous REST client.

use anyhow::{Context, Result, bail, ensure};
use octocrab::Octocrab;
use serde_json::Value;
use std::time::Duration;
use tokio::runtime::Runtime;

/// A non-success response with its status preserved for moved/deleted targets.
/// Response bodies and credentials are deliberately excluded from diagnostics.
#[derive(Debug)]
pub struct HttpStatus {
    pub method: String,
    pub path: String,
    pub status: u16,
}

impl std::fmt::Display for HttpStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "GitHub {} {} returned HTTP {}; check token permissions, rate limits, and branch state",
            self.method, self.path, self.status
        )
    }
}

impl std::error::Error for HttpStatus {}

/// Minimal exchange-facing interface, also implemented by offline test fixtures.
pub trait Api {
    /// Return `None` for an absent resource; propagate transport and permission errors.
    fn request(&self, method: &str, path: &str, body: Option<Value>) -> Result<Option<Value>>;
}

pub struct Client {
    client: Octocrab,
    runtime: Runtime,
}

impl Client {
    /// Build a client using caller-supplied credentials and a private single-thread runtime.
    pub fn new(token: Option<String>) -> Result<Self> {
        Self::at(None, token)
    }

    /// Build a client for another HTTPS origin, or for GitHub when `origin` is absent.
    /// The bearer credential is sent to that origin only.
    fn at(origin: Option<&str>, token: Option<String>) -> Result<Self> {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?;
        let client = {
            // Octocrab's buffered transport is initialized within a Tokio context.
            let _entered = runtime.enter();
            let mut builder = Octocrab::builder()
                .set_connect_timeout(Some(Duration::from_secs(60)))
                .set_read_timeout(Some(Duration::from_secs(60)))
                .set_write_timeout(Some(Duration::from_secs(60)));
            if let Some(origin) = origin {
                builder = builder.base_uri(origin)?;
            }
            if let Some(token) = token.filter(|value| !value.trim().is_empty()) {
                builder = builder.personal_token(token);
            }
            builder.build()?
        };
        Ok(Self { client, runtime })
    }
}

/// Fetch one JSON document from `origin` + `route`, presenting a bearer credential.
///
/// Used for token exchange, so failures name only the origin: neither the
/// credential nor the response body reaches diagnostics.
pub(crate) fn fetch_json(origin: &str, route: &str, bearer: &str) -> Result<Value> {
    ensure!(
        origin.starts_with("https://") && route.starts_with('/'),
        "token exchange requires an HTTPS endpoint"
    );
    let client = Client::at(Some(origin), Some(bearer.to_owned()))?;
    client.runtime.block_on(async {
        let request = client.client.get::<Value, _, ()>(route, None);
        match tokio::time::timeout(Duration::from_secs(60), request).await {
            Ok(Ok(value)) => Ok(value),
            Ok(Err(octocrab::Error::GitHub { source, .. })) => {
                bail!("{origin} answered HTTP {}", source.status_code.as_u16())
            }
            Ok(Err(_)) => bail!("request to {origin} failed"),
            Err(_) => bail!("request to {origin} exceeded 60 seconds"),
        }
    })
}

impl Api for Client {
    /// Send one bounded REST request; redirects and automatic retries are disabled.
    fn request(&self, method: &str, path: &str, body: Option<Value>) -> Result<Option<Value>> {
        ensure!(
            path.starts_with("/repos/") && !path.contains(['\r', '\n']),
            "invalid GitHub API path"
        );
        self.runtime.block_on(async {
            let request = async {
                match method {
                    "GET" => {
                        self.client
                            .get::<Value, _, Value>(path, body.as_ref())
                            .await
                    }
                    "POST" => self.client.post::<Value, Value>(path, body.as_ref()).await,
                    "PATCH" => {
                        self.client
                            .patch::<Value, _, Value>(path, body.as_ref())
                            .await
                    }
                    _ => bail!("unsupported GitHub API method: {method}"),
                }
                .map_err(anyhow::Error::from)
            };
            let response = tokio::time::timeout(Duration::from_secs(60), request)
                .await
                .context("GitHub request exceeded 60 seconds")?;
            match response {
                Ok(value) => Ok(Some(value)),
                Err(error) => {
                    // Release notifications distinguish moved/deleted targets from
                    // authentication and transport failures without parsing messages.
                    let Some(octocrab::Error::GitHub { source, .. }) =
                        error.downcast_ref::<octocrab::Error>()
                    else {
                        // Avoid including response bodies or caller credentials in errors.
                        bail!(
                            "GitHub {method} {path} failed; check connectivity, \
                             token permissions, rate limits, and branch state"
                        )
                    };
                    match source.status_code.as_u16() {
                        404 => Ok(None),
                        status => Err(HttpStatus {
                            method: method.into(),
                            path: path.into(),
                            status,
                        }
                        .into()),
                    }
                }
            }
        })
    }
}

/// Require an existing resource, adding its path to an otherwise ambiguous 404.
pub fn required(api: &dyn Api, method: &str, path: &str, body: Option<Value>) -> Result<Value> {
    api.request(method, path, body)?
        .with_context(|| format!("GitHub object not found: {path}"))
}
