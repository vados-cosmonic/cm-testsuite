//! Review-maintained list of repositories that take part in the exchange.

use crate::{SCHEMA, records, validate_path, validate_source};
use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeSet, fs, path::Path};
use toml_edit::{Array, ArrayOfTables, DocumentMut, Item, Table, value};

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
pub fn validate_login(login: &str) -> Result<()> {
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

impl Participant {
    /// Check the repository, manifest path, reference, and contacts of one entry.
    pub fn validate(&self) -> Result<()> {
        validate_source(&self.repository)?;
        validate_path(&self.manifest)?;
        if let Some(reference) = &self.reference {
            ensure!(!reference.trim().is_empty(), "empty participant reference");
        }
        self.maintainers
            .iter()
            .try_for_each(|login| validate_login(login))
    }
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
            participant.validate()?;
            ensure!(
                seen.insert(&participant.repository),
                "repository enrolled twice: {}",
                participant.repository
            );
        }
        for login in &registry.maintainers {
            validate_login(login)?;
        }
        Ok(registry)
    }

    /// Append `participant` to the registry `text`, keeping its comments and entries.
    ///
    /// The result is validated like any registry, and a repository that is
    /// already enrolled is rejected whatever the case of its name.
    pub fn enroll(text: &str, participant: &Participant) -> Result<String> {
        participant.validate()?;
        let current = Self::parse(text.as_bytes())?;
        ensure!(
            !current
                .participant
                .iter()
                .any(|p| p.repository.eq_ignore_ascii_case(&participant.repository)),
            "repository is already enrolled: {}",
            participant.repository
        );
        let mut entry = Table::new();
        entry.insert("repository", value(&participant.repository));
        if participant.manifest != default_manifest() {
            entry.insert("manifest", value(&participant.manifest));
        }
        if let Some(reference) = &participant.reference {
            entry.insert("reference", value(reference));
        }
        if !participant.maintainers.is_empty() {
            let logins: Array = participant.maintainers.iter().collect();
            entry.insert("maintainers", value(logins));
        }
        let mut document = text.parse::<DocumentMut>()?;
        let entries = document
            .entry("participant")
            .or_insert(Item::ArrayOfTables(ArrayOfTables::new()))
            .as_array_of_tables_mut()
            .context("participant entries must be [[participant]] tables")?;
        entry.decor_mut().set_prefix("\n");
        entries.push(entry);
        let updated = document.to_string();
        Self::parse(updated.as_bytes())?;
        Ok(updated)
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

    fn participant(repository: &str) -> Participant {
        Participant {
            repository: repository.into(),
            manifest: default_manifest(),
            reference: None,
            maintainers: vec!["octo-cat".into()],
        }
    }

    #[test]
    fn enrolling_appends_an_entry_and_keeps_comments_and_entries() {
        let template = "# Enrolled repositories.\n#\n# [[participant]]\n\
                        # repository = \"github:OWNER/REPO\"\nschema = 1\n";
        let first = Participants::enroll(template, &participant("github:example/a")).unwrap();
        assert_eq!(
            first,
            format!(
                "{template}\n[[participant]]\nrepository = \"github:example/a\"\n\
                 maintainers = [\"octo-cat\"]\n"
            )
        );

        // Defaults stay implicit; only what differs from them is written.
        let custom = Participant {
            manifest: "ci/exchange.toml".into(),
            reference: Some("release/1.x".into()),
            maintainers: vec!["octo-cat".into(), "hubot".into()],
            ..participant("github:example/b")
        };
        let annotated = first.replace("[[participant]]", "[[participant]] # the first");
        let second = Participants::enroll(&annotated, &custom).unwrap();
        assert_eq!(
            second,
            format!(
                "{annotated}\n[[participant]]\nrepository = \"github:example/b\"\n\
                 manifest = \"ci/exchange.toml\"\nreference = \"release/1.x\"\n\
                 maintainers = [\"octo-cat\", \"hubot\"]\n"
            )
        );
        let parsed = Participants::parse(second.as_bytes()).unwrap().participant;
        assert_eq!(parsed, [participant("github:example/a"), custom]);
    }

    #[test]
    fn enrolling_rejects_repeats_and_invalid_entries() {
        let registry = "schema = 1\n[[participant]]\nrepository = 'github:example/a'\n";
        let enroll = |entry: &Participant| Participants::enroll(registry, entry);
        // GitHub names are case-insensitive, so a respelling is the same repository.
        for repeated in ["github:example/a", "github:Example/A"] {
            let error = enroll(&participant(repeated)).unwrap_err();
            assert!(error.to_string().contains("already enrolled"), "{error}");
        }
        let with = |change: fn(&mut Participant)| {
            let mut entry = participant("github:example/b");
            change(&mut entry);
            entry
        };
        for bad in [
            participant("example/b"),
            participant("github:example/b/c"),
            participant("github:example/b\"\n[[participant]]"),
            with(|entry| entry.manifest = "../outside.toml".into()),
            with(|entry| entry.reference = Some(" ".into())),
            with(|entry| entry.maintainers = vec!["@octo-cat".into()]),
        ] {
            assert!(enroll(&bad).is_err(), "{bad:?}");
        }
        assert!(Participants::enroll("schema = 2\n", &participant("github:example/b")).is_err());
    }
}
