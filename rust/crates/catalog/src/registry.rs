//! Review-maintained list of repositories that take part in the exchange.

use crate::{SCHEMA, records, validate_path, validate_source};
use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeSet, fs, path::Path};

pub const PARTICIPANTS_PATH: &str = "registry/participants.toml";

/// One enrolled repository. Its own manifest decides whether it shares tests,
/// receives them, or both; enrollment only makes the exchange look at it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Participant {
    /// `github:owner/repository`.
    pub repository: String,
    /// Tracked manifest path inside the repository.
    #[serde(default = "default_manifest")]
    pub manifest: String,
    /// Branch, tag, or commit to gather from; the default branch when absent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reference: Option<String>,
    /// GitHub logins to mention when this repository fails a scheduled run.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub maintainers: Vec<String>,
}

fn default_manifest() -> String {
    ".cm-testsuite.toml".into()
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Participants {
    pub schema: u32,
    /// GitHub logins of the exchange's maintainers, mentioned on every failure report.
    #[serde(default)]
    pub maintainers: Vec<String>,
    #[serde(default)]
    pub participant: Vec<Participant>,
}

/// Accept only what GitHub allows in a login, so a mention cannot carry markup.
fn validate_login(login: &str) -> Result<()> {
    ensure!(
        !login.is_empty()
            && login.len() <= 39
            && login
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'-')
            && !login.starts_with('-')
            && !login.ends_with('-'),
        "invalid GitHub login: {login}"
    );
    Ok(())
}

impl Participants {
    /// Decode and validate a registry, rejecting repeated repositories.
    pub fn parse(bytes: &[u8]) -> Result<Self> {
        let registry: Self = records::decode(bytes)?;
        ensure!(
            registry.schema == SCHEMA,
            "unsupported participant registry schema"
        );
        let mut seen = BTreeSet::new();
        for participant in &registry.participant {
            validate_source(&participant.repository)?;
            validate_path(&participant.manifest)?;
            if let Some(reference) = &participant.reference {
                ensure!(!reference.trim().is_empty(), "empty participant reference");
            }
            ensure!(
                seen.insert(&participant.repository),
                "repository enrolled twice: {}",
                participant.repository
            );
        }
        let contacts = registry.participant.iter().flat_map(|p| &p.maintainers);
        for login in registry.maintainers.iter().chain(contacts) {
            validate_login(login)?;
        }
        Ok(registry)
    }

    /// Load the enrolled repositories from a versioned TOML registry.
    pub fn load(path: &Path) -> Result<Self> {
        let bytes =
            fs::read(path).with_context(|| format!("participant registry {}", path.display()))?;
        Self::parse(&bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn applies_defaults_and_rejects_duplicates_and_typos() {
        let one = "schema = 1\n[[participant]]\nrepository = 'github:example/a'\n";
        let parsed = Participants::parse(one.as_bytes()).unwrap().participant;
        assert_eq!(parsed[0].manifest, ".cm-testsuite.toml");
        assert_eq!(parsed[0].reference, None);
        let empty = Participants::parse(b"schema = 1\n").unwrap();
        assert!(empty.participant.is_empty() && empty.maintainers.is_empty());

        let contacts = format!("maintainers = ['octo-cat']\n{one}maintainers = ['hubot']\n");
        let parsed = Participants::parse(contacts.as_bytes()).unwrap();
        assert_eq!(parsed.maintainers, ["octo-cat"]);
        assert_eq!(parsed.participant[0].maintainers, ["hubot"]);

        let twice = format!("{one}[[participant]]\nrepository = 'github:example/a'\n");
        assert!(Participants::parse(twice.as_bytes()).is_err());
        for bad in [
            "schema = 2\n",
            "schema = 1\nmaintainers = ['@octocat']\n",
            "schema = 1\nmaintainers = ['a b']\n",
            "schema = 1\n[[participant]]\nrepository = 'example/a'\n",
            "schema = 1\n[[participant]]\nrepository = 'github:example/a'\nmanifst = 'x'\n",
            "schema = 1\n[[participant]]\nrepository = 'github:example/a'\nmanifest = '../x'\n",
            "schema = 1\n[[participant]]\nrepository = 'github:example/a'\nmaintainers = ['x/y']\n",
        ] {
            assert!(Participants::parse(bad.as_bytes()).is_err(), "{bad}");
        }
    }
}
