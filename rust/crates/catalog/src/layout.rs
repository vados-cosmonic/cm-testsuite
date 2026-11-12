//! Where the catalog lives in a checkout.
//!
//! Each source repository has its own file, `OWNER/REPO.toml` under
//! [`CATALOG_DIR`], holding its snapshot record, its tests, and its retired
//! identities, so imports of different sources never edit the same file.
//! [`CATALOG_PATH`] is still read and may hold any number of sources.

use crate::{CATALOG_PATH, Catalog, checked_path, records, validate_source};
use anyhow::{Context, Result, ensure};
use std::{
    fs, io,
    path::{Path, PathBuf},
};

/// Directory of per-source files: [`CATALOG_PATH`] without its extension.
pub const CATALOG_DIR: &str = "metadata/catalog";

/// Where a source's records live, relative to the catalog checkout:
/// `OWNER/REPO.toml` under [`CATALOG_DIR`], spelled as the source is.
pub fn source_path(source: &str) -> Result<String> {
    validate_source(source)?;
    let name = source.trim_start_matches("github:");
    Ok(format!("{CATALOG_DIR}/{name}.toml"))
}

/// The source whose records `path` may hold, when it is a per-source file.
pub fn path_source(path: &str) -> Option<String> {
    let name = path
        .strip_prefix(CATALOG_DIR)?
        .strip_prefix('/')?
        .strip_suffix(".toml")?;
    let source = format!("github:{name}");
    validate_source(&source).is_ok().then_some(source)
}

/// Whether an identity names a test of `source`.
fn owns(source: &str, id: &str) -> bool {
    id.strip_prefix(source)
        .is_some_and(|name| name.starts_with('/'))
}

/// The catalog of a checkout, and the part of it [`CATALOG_PATH`] holds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Layout {
    /// Every record: [`CATALOG_PATH`] merged with the per-source files.
    pub catalog: Catalog,
    /// The records of [`CATALOG_PATH`] alone; empty when the file is absent.
    pub inline: Catalog,
}

impl Catalog {
    /// Whether this catalog records anything of `source`.
    pub fn holds(&self, source: &str) -> bool {
        let mut ids = self.tests.keys().chain(self.retired.keys());
        self.sources.contains_key(source) || ids.any(|id| owns(source, id))
    }

    /// The records of one source: what its own file holds.
    pub fn only(&self, source: &str) -> Self {
        let mut part = self.clone();
        part.sources.retain(|name, _| name == source);
        part.tests.retain(|id, _| owns(source, id));
        part.retired.retain(|id, _| owns(source, id));
        part
    }

    /// Drop every record of `source`.
    pub fn remove(&mut self, source: &str) {
        self.sources.remove(source);
        self.tests.retain(|id, _| !owns(source, id));
        self.retired.retain(|id, _| !owns(source, id));
    }

    /// Decode the file found at `name`.toml under [`CATALOG_DIR`].
    ///
    /// A file may only record the source its path names, so two files never
    /// describe the same test and a review of the path is a review of the source.
    fn decode_source(name: &str, bytes: &[u8]) -> Result<Self> {
        let part: Self = records::decode(bytes)?;
        let source = format!("github:{name}");
        ensure!(
            part.sources.len() == 1 && part.sources.contains_key(&source),
            "expected the one source {source}, found {:?}",
            part.sources.keys().collect::<Vec<_>>()
        );
        for id in part.tests.keys().chain(part.retired.keys()) {
            ensure!(owns(&source, id), "{id} does not belong to {source}");
        }
        part.validate()?;
        Ok(part)
    }

    /// Load the catalog of a checkout, whichever files hold its records.
    pub fn load(root: &Path) -> Result<Self> {
        Ok(Layout::load(root)?.catalog)
    }
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
        ensure!(!kind.is_symlink(), "symlink in catalog: {}", path.display());
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

impl Layout {
    /// Read [`CATALOG_PATH`] and every per-source file under [`CATALOG_DIR`].
    ///
    /// Both are optional. A source lives in exactly one of them, and the merged
    /// catalog is validated as a whole, so the result does not depend on which.
    pub fn load(root: &Path) -> Result<Self> {
        let inline = match fs::read(checked_path(root, CATALOG_PATH)?) {
            Ok(bytes) => records::decode::<Catalog>(&bytes)
                .and_then(|inline| inline.validate().map(|()| inline))
                .with_context(|| format!("catalog file {CATALOG_PATH}"))?,
            Err(error) if error.kind() == io::ErrorKind::NotFound => Catalog::default(),
            Err(error) => return Err(error).context(CATALOG_PATH),
        };
        let mut catalog = inline.clone();
        let mut found = vec![];
        files(&checked_path(root, CATALOG_DIR)?, "", &mut found)?;
        found.sort();
        for (name, file) in found {
            let part = fs::read(&file)
                .map_err(Into::into)
                .and_then(|bytes| Catalog::decode_source(&name, &bytes))
                .with_context(|| format!("catalog file {}", file.display()))?;
            ensure!(
                !inline.holds(&format!("github:{name}")),
                "source github:{name} is recorded in both {CATALOG_PATH} and {}",
                file.display()
            );
            catalog.sources.extend(part.sources);
            catalog.tests.extend(part.tests);
            catalog.retired.extend(part.retired);
        }
        catalog
            .validate()
            .with_context(|| format!("catalog under {CATALOG_DIR}"))?;
        Ok(Self { catalog, inline })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Analysis, SCHEMA, SourceSnapshot, Test, corpus_path, digest};
    use std::collections::BTreeSet;

    const A: &str = "github:example/a";
    const B: &str = "github:Other/b.c";

    /// Record `source` with the named tests and retired identities.
    fn record(catalog: &mut Catalog, source: &str, tests: &[&str], retired: &[&str]) {
        let snapshot = SourceSnapshot {
            revision: "a".repeat(40),
            manifest_path: ".cm-testsuite.toml".into(),
            manifest_sha256: "b".repeat(64),
        };
        catalog.sources.insert(source.into(), snapshot);
        for name in tests {
            let test = Test {
                source: source.into(),
                name: (*name).into(),
                revision: "a".repeat(40),
                source_path: format!("tests/{name}.wast"),
                manifest_revision: "a".repeat(40),
                sha256: "c".repeat(64),
                destination: corpus_path(source, name).unwrap(),
                summary: format!("Check {name}"),
                requires: BTreeSet::from(["component-model".into()]),
                covers: BTreeSet::new(),
                analysis: Analysis::default(),
            };
            catalog.tests.insert(test.id(), test);
        }
        for name in retired {
            let id = format!("{source}/{name}");
            catalog.retired.insert(id, "d".repeat(64));
        }
    }

    fn catalog(sources: &[(&str, &[&str], &[&str])]) -> Catalog {
        let mut catalog = Catalog::default();
        for (source, tests, retired) in sources {
            record(&mut catalog, source, tests, retired);
        }
        catalog
    }

    /// Both sources, with nested names, a retirement, and a source without tests.
    fn both() -> Catalog {
        catalog(&[(A, &["one", "nested/two"], &[]), (B, &[], &["gone"])])
    }

    fn text(catalog: &Catalog) -> String {
        records::encode(catalog).unwrap()
    }

    /// A checkout holding the shared file, when given, and other files by
    /// their path under the catalog directory.
    fn checkout(inline: Option<&str>, others: &[(&str, &str)]) -> tempfile::TempDir {
        let root = tempfile::tempdir().unwrap();
        let write = |path: PathBuf, text: &str| {
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, text).unwrap();
        };
        if let Some(inline) = inline {
            write(root.path().join(CATALOG_PATH), inline);
        }
        for (name, text) in others {
            // Join one component at a time, so names use the platform's separators.
            let directory = root.path().join(CATALOG_DIR);
            write(
                name.split('/').fold(directory, |p, part| p.join(part)),
                text,
            );
        }
        root
    }

    fn load(inline: Option<&str>, others: &[(&str, &str)]) -> Result<Catalog> {
        Catalog::load(checkout(inline, others).path())
    }

    /// The path under the catalog directory of the file that records `source`.
    fn file(source: &str) -> String {
        let path = source_path(source).unwrap();
        path.strip_prefix("metadata/catalog/").unwrap().to_owned()
    }

    #[test]
    fn every_layout_loads_the_same_catalog_with_the_same_digest() {
        let whole = both();
        let (a, b) = (text(&whole.only(A)), text(&whole.only(B)));
        let (a_file, b_file) = (file(A), file(B));
        let empty = text(&Catalog::default());
        let mut rest = whole.clone();
        rest.remove(B);
        assert_eq!(rest, whole.only(A));

        type Files<'a> = Vec<(&'a str, &'a str)>;
        let layouts: [(Option<&str>, Files); 5] = [
            (Some(&text(&whole)), vec![]),
            (None, vec![(&a_file, &a), (&b_file, &b)]),
            (Some(&empty), vec![(&a_file, &a), (&b_file, &b)]),
            (Some(&a), vec![(&b_file, &b)]),
            // Anything that is not a TOML file is left alone, wherever it sits.
            (
                Some(&b),
                vec![
                    (&a_file, &a),
                    ("README.md", "# Catalog\n"),
                    ("example/.gitkeep", ""),
                    ("example/notes.txt", "schema = 2\n"),
                ],
            ),
        ];
        for (inline, others) in layouts {
            let loaded = load(inline, &others).unwrap();
            assert_eq!(loaded, whole);
            assert_eq!(digest(&loaded).unwrap(), digest(&whole).unwrap());
        }

        // The shared file and the directory are both optional.
        for (inline, others) in [
            (None, vec![]),
            (Some(empty.as_str()), vec![("README.md", "")]),
        ] {
            assert_eq!(load(inline, &others).unwrap(), Catalog::default());
        }
        let root = checkout(Some(&a), &[(&b_file, &b)]);
        let layout = Layout::load(root.path()).unwrap();
        assert_eq!(layout.inline, whole.only(A));
        assert_eq!(layout.catalog, whole);
    }

    #[test]
    fn a_file_records_only_the_source_its_path_names() {
        let whole = both();
        let a = text(&whole.only(A));
        let mut foreign_test = whole.only(A);
        record(&mut foreign_test, B, &["stray"], &[]);
        foreign_test.sources.remove(B);
        let mut foreign_retirement = whole.only(A);
        let stray = format!("{B}/stray");
        foreign_retirement.retired.insert(stray, "d".repeat(64));
        let mut sourceless = whole.only(A);
        sourceless.sources.clear();
        sourceless.tests.clear();

        for (name, text) in [
            ("example/b.toml", a.clone()),
            // The spelling must match too, so one source has one possible path.
            ("Example/a.toml", a.clone()),
            ("a.toml", a.clone()),
            ("example/a/a.toml", a.clone()),
            ("example/a.toml", text(&whole)),
            ("example/a.toml", text(&foreign_test)),
            ("example/a.toml", text(&foreign_retirement)),
            ("example/a.toml", text(&sourceless)),
            ("example/a.toml", a.replace("schema = 1", "schema = 2")),
            ("example/a.toml", format!("participant = 'x'\n{a}")),
            ("example/a.toml", "schema = 1\n".into()),
        ] {
            let error = load(None, &[(name, &text)]).unwrap_err();
            // The failing file is named, with the separators of the platform.
            let file = name.rsplit('/').next().unwrap();
            assert!(format!("{error:#}").contains(file), "{name}: {error:#}");
        }
        let error = load(None, &[("example/a.toml", &text(&foreign_test))]).unwrap_err();
        assert!(
            format!("{error:#}").contains("does not belong to github:example/a"),
            "{error:#}"
        );
    }

    #[test]
    fn a_source_is_recorded_in_one_place_whatever_its_case() {
        let whole = both();
        let (a, b) = (text(&whole.only(A)), text(&whole.only(B)));
        let both_places = load(Some(&text(&whole)), &[(&file(A), &a)]).unwrap_err();
        assert!(
            format!("{both_places:#}").contains("recorded in both"),
            "{both_places:#}"
        );

        // A retirement alone ties a source to the shared file.
        let mut retirement = Catalog::default();
        let gone = format!("{B}/gone-earlier");
        retirement.retired.insert(gone, "d".repeat(64));
        let error = load(Some(&text(&retirement)), &[(&file(B), &b)]).unwrap_err();
        assert!(
            format!("{error:#}").contains("recorded in both"),
            "{error:#}"
        );

        // GitHub names are case-insensitive, so a respelling is the same source.
        let respelled = catalog(&[("github:Example/A", &["one"], &["gone"])]);
        for inline in [respelled.clone(), {
            let mut retired = respelled.clone();
            retired.sources.clear();
            retired.tests.clear();
            retired
        }] {
            let error = load(Some(&text(&inline)), &[(&file(A), &a)]).unwrap_err();
            assert!(format!("{error:#}").contains("spelled"), "{error:#}");
        }
        let mut merged = whole;
        record(&mut merged, "github:Example/A", &[], &[]);
        assert!(merged.validate().is_err());
    }

    // Two spellings of one name can only sit side by side where file names are case-sensitive.
    #[cfg(target_os = "linux")]
    #[test]
    fn files_differing_only_in_case_record_one_source_twice() {
        let respelled = "github:Example/A";
        let files = [
            (file(A), text(&both().only(A))),
            (
                file(respelled),
                text(&catalog(&[(respelled, &["one"], &[])])),
            ),
        ];
        let files: Vec<_> = files
            .iter()
            .map(|(n, t)| (n.as_str(), t.as_str()))
            .collect();
        let error = load(None, &files).unwrap_err();
        assert!(format!("{error:#}").contains("spelled"), "{error:#}");
    }

    #[cfg(unix)]
    #[test]
    fn symlinked_files_and_directories_are_rejected() {
        use std::os::unix::fs::symlink;
        let a = text(&both().only(A));
        let root = checkout(None, &[(&file(A), &a)]);
        let directory = root.path().join(CATALOG_DIR);
        Catalog::load(root.path()).unwrap();
        symlink(directory.join("example"), directory.join("linked")).unwrap();
        assert!(Catalog::load(root.path()).is_err());
        fs::remove_file(directory.join("linked")).unwrap();
        let example = directory.join("example");
        symlink(example.join("a.toml"), example.join("b.toml")).unwrap();
        assert!(Catalog::load(root.path()).is_err());

        let elsewhere = checkout(None, &[(&file(A), &a)]);
        let root = tempfile::tempdir().unwrap();
        fs::create_dir_all(root.path().join("metadata")).unwrap();
        symlink(
            elsewhere.path().join(CATALOG_DIR),
            root.path().join(CATALOG_DIR),
        )
        .unwrap();
        assert!(Catalog::load(root.path()).is_err());
    }

    #[test]
    fn source_files_sit_beside_the_shared_file() {
        assert_eq!(
            Path::new(CATALOG_PATH).with_extension(""),
            Path::new(CATALOG_DIR)
        );
        assert_eq!(source_path(A).unwrap(), "metadata/catalog/example/a.toml");
        assert_eq!(source_path(B).unwrap(), "metadata/catalog/Other/b.c.toml");
        for source in [A, B] {
            let path = source_path(source).unwrap();
            assert_eq!(path_source(&path).as_deref(), Some(source));
        }
        assert!(source_path("example/a").is_err());
        assert!(source_path("github:example/a/b").is_err());
        for other in [
            CATALOG_PATH,
            "metadata/catalog/a.toml",
            "metadata/catalog/example/a/b.toml",
            "metadata/catalog/example/a.txt",
            "metadata/catalog/example/../a.toml",
            "metadata/catalogs/example/a.toml",
            "corpus/wast/example/a.toml",
        ] {
            assert_eq!(path_source(other), None, "{other}");
        }
        assert_eq!(both().schema, SCHEMA);
    }
}
