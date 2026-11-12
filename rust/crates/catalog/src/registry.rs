//! Review-maintained list of repositories that take part in the exchange.
//!
//! Each repository is enrolled by its own file, `OWNER/REPO.toml` under
//! [`PARTICIPANTS_DIR`], so enrollments never edit the same lines.
//! [`PARTICIPANTS_PATH`] holds the registry-wide settings and may still list
//! repositories inline.

use crate::{SCHEMA, records, validate_path, validate_source};
use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeSet,
    fs, io,
    path::{Path, PathBuf},
};
use toml_edit::{Array, DocumentMut, value};

/// Registry-wide settings, and any repositories still enrolled inline.
pub const PARTICIPANTS_PATH: &str = "registry/participants.toml";
/// Directory of per-repository files: [`PARTICIPANTS_PATH`] without its extension.
pub const PARTICIPANTS_DIR: &str = "registry/participants";

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
    /// Enrolled repositories, ordered by name whatever file each one came from.
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

    /// Where this repository's file lives, relative to the catalog checkout:
    /// `OWNER/REPO.toml` under [`PARTICIPANTS_DIR`], spelled as `repository` is.
    pub fn path(&self) -> Result<String> {
        validate_source(&self.repository)?;
        let name = self.repository.trim_start_matches("github:");
        Ok(format!("{PARTICIPANTS_DIR}/{name}.toml"))
    }

    /// The contents of this repository's file. Defaults stay implicit.
    pub fn encode(&self) -> Result<String> {
        self.validate()?;
        let mut file = DocumentMut::new();
        file.insert("schema", value(i64::from(SCHEMA)));
        file.insert("repository", value(&self.repository));
        if self.manifest != default_manifest() {
            file.insert("manifest", value(&self.manifest));
        }
        if let Some(reference) = &self.reference {
            file.insert("reference", value(reference));
        }
        if !self.maintainers.is_empty() {
            let logins: Array = self.maintainers.iter().collect();
            file.insert("maintainers", value(logins));
        }
        Ok(file.to_string())
    }

    /// Decode the file found at `name`.toml under the registry directory.
    ///
    /// A file may only enroll the repository its path names, so a review of
    /// the path is a review of what gets enrolled.
    fn decode(name: &str, bytes: &[u8]) -> Result<Self> {
        let mut fields: toml::Table = records::decode(bytes)?;
        ensure!(
            fields.remove("schema") == Some(toml::Value::Integer(SCHEMA.into())),
            "unsupported participant schema"
        );
        let participant: Self = fields.try_into()?;
        participant.validate()?;
        ensure!(
            participant.repository == format!("github:{name}"),
            "expected repository github:{name}, found {}",
            participant.repository
        );
        Ok(participant)
    }
}

/// GitHub treats names that differ only in case as one repository.
fn key(participant: &Participant) -> String {
    participant.repository.to_ascii_lowercase()
}

/// List the `.toml` files below `directory` as their path under it without
/// the extension, skipping every other file. A missing directory holds none.
fn files(directory: &Path, prefix: &str, found: &mut Vec<(String, PathBuf)>) -> Result<()> {
    let entries = match fs::read_dir(directory) {
        Ok(entries) => entries,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error).context(directory.display().to_string()),
    };
    for entry in entries {
        let entry = entry?;
        let (path, kind) = (entry.path(), entry.file_type()?);
        ensure!(
            !kind.is_symlink(),
            "symlink in participant registry: {}",
            path.display()
        );
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if kind.is_dir() {
            files(&path, &format!("{prefix}{name}/"), found)?;
        } else if let Some(stem) = name.strip_suffix(".toml") {
            found.push((format!("{prefix}{stem}"), path));
        }
    }
    Ok(())
}

impl Participants {
    /// Decode and validate the registry file alone: its settings and inline entries.
    pub fn parse(bytes: &[u8]) -> Result<Self> {
        let mut registry: Self = records::decode(bytes)?;
        ensure!(
            registry.schema == SCHEMA,
            "unsupported participant registry schema"
        );
        for login in &registry.maintainers {
            validate_login(login)?;
        }
        registry.settle()?;
        Ok(registry)
    }

    /// Validate every entry, reject a repository enrolled twice, and order the
    /// entries by name so a run does not depend on where each one was read from.
    fn settle(&mut self) -> Result<()> {
        let mut seen = BTreeSet::new();
        for participant in &self.participant {
            participant.validate()?;
            ensure!(
                seen.insert(key(participant)),
                "repository enrolled twice: {}",
                participant.repository
            );
        }
        self.participant.sort_by_key(key);
        Ok(())
    }

    /// Load the enrolled repositories: the registry file at `path`, plus one
    /// file per repository in the directory named like it without the extension.
    pub fn load(path: &Path) -> Result<Self> {
        let bytes =
            fs::read(path).with_context(|| format!("participant registry {}", path.display()))?;
        let mut registry = Self::parse(&bytes)
            .with_context(|| format!("participant registry {}", path.display()))?;
        let mut found = vec![];
        files(&path.with_extension(""), "", &mut found)?;
        for (name, file) in found {
            let participant = fs::read(&file)
                .map_err(Into::into)
                .and_then(|bytes| Participant::decode(&name, &bytes))
                .with_context(|| format!("participant file {}", file.display()))?;
            registry.participant.push(participant);
        }
        registry
            .settle()
            .with_context(|| format!("participant registry {}", path.display()))?;
        Ok(registry)
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

        // GitHub names are case-insensitive, so a respelling is the same repository.
        for repeated in ["github:example/a", "github:Example/A"] {
            let twice = format!("{one}[[participant]]\nrepository = '{repeated}'\n");
            assert!(Participants::parse(twice.as_bytes()).is_err(), "{repeated}");
        }
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
            maintainers: vec![],
        }
    }

    /// A registry in a temporary checkout: the registry file, then other files
    /// by their path under the registry directory.
    fn registry(settings: &str, others: &[(&str, &str)]) -> (tempfile::TempDir, PathBuf) {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join(PARTICIPANTS_PATH);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, settings).unwrap();
        for (name, text) in others {
            let file = root.path().join(PARTICIPANTS_DIR).join(name);
            fs::create_dir_all(file.parent().unwrap()).unwrap();
            fs::write(file, text).unwrap();
        }
        (root, path)
    }

    fn load(settings: &str, others: &[(&str, &str)]) -> Result<Participants> {
        let (_root, path) = registry(settings, others);
        Participants::load(&path)
    }

    fn names(registry: &Participants) -> Vec<&str> {
        let entries = registry.participant.iter();
        entries.map(|p| p.repository.as_str()).collect()
    }

    #[test]
    fn loads_one_file_per_repository_in_name_order() {
        let custom = Participant {
            manifest: "ci/exchange.toml".into(),
            reference: Some("release/1.x".into()),
            maintainers: vec!["octo-cat".into(), "hubot".into()],
            ..participant("github:zeta/a")
        };
        let files: Vec<_> = [
            custom.clone(),
            participant("github:example/b"),
            participant("github:Other/a.b"),
        ]
        .iter()
        .map(|entry| {
            let path = entry.path().unwrap();
            let name = path.strip_prefix("registry/participants/").unwrap();
            (name.to_owned(), entry.encode().unwrap())
        })
        .collect();
        let mut files: Vec<_> = files
            .iter()
            .map(|(n, t)| (n.as_str(), t.as_str()))
            .collect();
        // Anything that is not a TOML file is left alone, wherever it sits.
        files.push(("README.md", "# Participants\n"));
        files.push(("example/.gitkeep", ""));
        files.push(("example/notes.txt", "repository = 'github:example/notes'\n"));

        let loaded = load("schema = 1\nmaintainers = ['lead']\n", &files).unwrap();
        assert_eq!(loaded.maintainers, ["lead"]);
        assert_eq!(
            names(&loaded),
            ["github:example/b", "github:Other/a.b", "github:zeta/a"]
        );
        assert_eq!(loaded.participant[2], custom);
        assert_eq!(
            custom.encode().unwrap(),
            "schema = 1\nrepository = \"github:zeta/a\"\nmanifest = \"ci/exchange.toml\"\n\
             reference = \"release/1.x\"\nmaintainers = [\"octo-cat\", \"hubot\"]\n"
        );
        assert_eq!(
            participant("github:example/b").encode().unwrap(),
            "schema = 1\nrepository = \"github:example/b\"\n"
        );
    }

    #[test]
    fn inline_entries_still_load_and_merge_with_files() {
        let inline = "schema = 1\n[[participant]]\nrepository = 'github:example/c'\n\
                      [[participant]]\nrepository = 'github:example/a'\n";
        // The directory is optional, and so are entries of either kind.
        assert_eq!(
            names(&load(inline, &[]).unwrap()),
            ["github:example/a", "github:example/c"]
        );
        assert!(load("schema = 1\n", &[]).unwrap().participant.is_empty());
        let only_readme = load("schema = 1\n", &[("README.md", "")]).unwrap();
        assert!(only_readme.participant.is_empty());

        let b = "schema = 1\nrepository = 'github:example/b'\n";
        assert_eq!(
            names(&load(inline, &[("example/b.toml", b)]).unwrap()),
            ["github:example/a", "github:example/b", "github:example/c"]
        );

        // Enrolled both ways, in either spelling, is enrolled twice.
        for (name, repository) in [("example/a", "example/a"), ("Example/A", "Example/A")] {
            let file = format!("schema = 1\nrepository = 'github:{repository}'\n");
            let error = load(inline, &[(&format!("{name}.toml"), &file)]).unwrap_err();
            assert!(format!("{error:#}").contains("enrolled twice"), "{error:#}");
        }
    }

    #[test]
    fn a_file_enrolls_only_the_repository_its_path_names() {
        let a = "schema = 1\nrepository = 'github:example/a'\n";
        for (name, text) in [
            ("example/b.toml", a),
            // The spelling must match too, so one repository has one possible path.
            ("Example/a.toml", a),
            ("a.toml", a),
            ("example/a/a.toml", a),
            ("example/a.toml", "repository = 'github:example/a'\n"),
            (
                "example/a.toml",
                "schema = 2\nrepository = 'github:example/a'\n",
            ),
            ("example/a.toml", "schema = 1\n"),
            (
                "example/a.toml",
                "schema = 1\nrepository = 'github:example/a'\nmanifst = 'x'\n",
            ),
            (
                "example/a.toml",
                "schema = 1\nrepository = 'github:example/a'\nreference = ' '\n",
            ),
            (
                "example/a.toml",
                "schema = 1\n[[participant]]\nrepository = 'github:example/a'\n",
            ),
        ] {
            let error = load("schema = 1\n", &[(name, text)]).unwrap_err();
            // The failing file is named, with the separators of the platform.
            let file = name.rsplit('/').next().unwrap();
            assert!(format!("{error:#}").contains(file), "{name}: {error:#}");
        }
        assert!(participant("example/a").path().is_err());
        assert!(participant("github:example/a/b").encode().is_err());
    }

    // Two spellings of one name can only sit side by side where file names are case-sensitive.
    #[cfg(target_os = "linux")]
    #[test]
    fn files_differing_only_in_case_enroll_one_repository_twice() {
        let files = [
            (
                "example/a.toml",
                "schema = 1\nrepository = 'github:example/a'\n",
            ),
            (
                "Example/A.toml",
                "schema = 1\nrepository = 'github:Example/A'\n",
            ),
        ];
        let error = load("schema = 1\n", &files).unwrap_err();
        assert!(format!("{error:#}").contains("enrolled twice"), "{error:#}");
    }

    #[cfg(unix)]
    #[test]
    fn symlinked_files_are_rejected() {
        let a = "schema = 1\nrepository = 'github:example/a'\n";
        let (root, path) = registry("schema = 1\n", &[("example/a.toml", a)]);
        let directory = root.path().join(PARTICIPANTS_DIR).join("example");
        std::os::unix::fs::symlink(directory.join("a.toml"), directory.join("b.toml")).unwrap();
        assert!(Participants::load(&path).is_err());
    }

    #[test]
    fn the_default_registry_directory_sits_beside_its_file() {
        assert_eq!(
            Path::new(PARTICIPANTS_PATH).with_extension(""),
            Path::new(PARTICIPANTS_DIR)
        );
        assert_eq!(
            participant("github:example/a").path().unwrap(),
            "registry/participants/example/a.toml"
        );
    }
}
