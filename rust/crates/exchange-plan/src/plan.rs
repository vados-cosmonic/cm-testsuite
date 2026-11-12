//! Serializable proposals, validation, and review descriptions.

use anyhow::{Result, ensure};
use cm_catalog::*;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Edit {
    pub path: String,
    pub before: Option<String>,
    pub after: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Guard {
    pub path: String,
    pub sha256: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Kind {
    Import,
    Receive,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Plan {
    pub schema: u32,
    pub kind: Kind,
    pub subject: String,
    pub edits: Vec<Edit>,
    pub guards: Vec<Guard>,
    pub warnings: Vec<String>,
    pub exclusions: BTreeSet<String>,
    pub provenance: BTreeMap<String, Test>,
}

impl Plan {
    /// Reject unsupported schemas, unsafe edits, and inconsistent provenance.
    pub fn validate(&self) -> Result<()> {
        ensure!(self.schema == SCHEMA, "unsupported plan schema");
        ensure!(!self.subject.is_empty(), "empty plan subject");
        let mut paths = BTreeSet::new();
        for e in &self.edits {
            validate_path(&e.path)?;
            ensure!(paths.insert(&e.path), "duplicate edit path");
            ensure!(e.after.is_some() || e.before.is_some(), "empty deletion");
            if let Some(s) = &e.before {
                validate_sha(s)?;
            }
            match self.kind {
                Kind::Import => ensure!(
                    is_catalog(&e.path) || e.path.starts_with("corpus/wast/"),
                    "import path outside corpus/catalog"
                ),
                Kind::Receive => ensure!(
                    e.path == RECEIPT_PATH || e.path.ends_with(".wast"),
                    "unexpected receiving path"
                ),
            };
        }
        for g in &self.guards {
            validate_path(&g.path)?;
            if let Some(s) = &g.sha256 {
                validate_sha(s)?;
            }
        }
        for (id, test) in &self.provenance {
            validate_id(id)?;
            ensure!(*id == test.id(), "plan provenance identity mismatch");
            validate_revision(&test.revision)?;
            validate_sha(&test.sha256)?;
            validate_path(&test.source_path)?;
        }
        for id in &self.exclusions {
            validate_id(id)?;
        }
        Ok(())
    }

    /// Compute the canonical proposal identity used for decline suppression.
    pub fn key(&self) -> Result<String> {
        digest(self)
    }

    /// Count proposed test-file changes, leaving out the plan's own records.
    pub fn changes(&self) -> Change {
        let mut change = Change::default();
        for edit in self.edits.iter().filter(|edit| !is_record(&edit.path)) {
            match (&edit.before, &edit.after) {
                (None, _) => change.added += 1,
                (Some(_), Some(_)) => change.updated += 1,
                (Some(_), None) => change.removed += 1,
            }
        }
        change
    }

    /// Title one proposal per repository so later plans update the same pull request.
    pub fn title(&self) -> String {
        match self.kind {
            // Imports land in the catalog repository, which lints pull request titles.
            Kind::Import => {
                let sources: Vec<_> = self
                    .subject
                    .split(", ")
                    .map(|source| source.trim_start_matches("github:"))
                    .collect();
                let named = format!("feat(corpus): import tests from {}", sources.join(" and "));
                // The title lint caps headers at 100 characters; long names fall back to a count.
                if sources.len() <= 2 && named.len() <= 100 {
                    named
                } else {
                    format!(
                        "feat(corpus): import tests from {} repositories",
                        sources.len()
                    )
                }
            }
            Kind::Receive => "Update tests received from cm-testsuite".into(),
        }
    }

    /// Describe the proposal for review: one summary line, one row per test,
    /// and collapsed sections for declining tests and for warnings.
    pub fn body(&self) -> Result<String> {
        let changes = self.changes();
        let mut body = match self.kind {
            Kind::Import => format!(
                "Imports test snapshots from {} into the catalog: {changes}.\n",
                self.subject
            ),
            Kind::Receive => format!(
                "Updates the tests this repository receives through cm-testsuite: {changes}.\n"
            ),
        };

        // Pair each test with its file edits. A move pairs with two of them.
        let mut listed = BTreeSet::new();
        let mut rows = vec![];
        for (id, test) in &self.provenance {
            let suffix = format!("/{}.wast", id.trim_start_matches("github:"));
            let edits: Vec<_> = self
                .edits
                .iter()
                .filter(|edit| edit.path.ends_with(&suffix))
                .collect();
            listed.extend(edits.iter().map(|edit| edit.path.as_str()));
            let added = edits.iter().any(|e| e.before.is_none());
            let removed = edits.iter().any(|e| e.after.is_none());
            let updated = edits
                .iter()
                .any(|e| e.before.is_some() && e.after.is_some());
            let change = match (added, updated, removed) {
                (true, _, true) => "Move",
                (true, _, false) => "Add",
                (false, true, _) => "Update",
                (false, false, true) => "Remove",
                (false, false, false) => "Record",
            };
            rows.push(format!(
                "| {change} | `{id}` | `{}` | {} |\n",
                &test.revision[..12],
                cell(&test.summary)
            ));
        }
        if !rows.is_empty() {
            body.push_str(
                "\n| Change | Test | Source revision | Summary |\n| --- | --- | --- | --- |\n",
            );
            body.extend(rows.iter().take(LISTED).map(String::as_str));
            if rows.len() > LISTED {
                body.push_str(&format!(
                    "\n{} more tests are not listed; see the changed files.\n",
                    rows.len() - LISTED
                ));
            }
        }

        // Retirements and moves of tests whose provenance is unchanged.
        let others: Vec<_> = self
            .edits
            .iter()
            .filter(|edit| !is_record(&edit.path) && !listed.contains(edit.path.as_str()))
            .collect();
        if !others.is_empty() {
            body.push_str("\nOther files:\n\n");
            for edit in others.iter().take(LISTED) {
                body.push_str(&format!(
                    "- {} `{}`\n",
                    match (&edit.before, &edit.after) {
                        (None, _) => "Add",
                        (Some(_), Some(_)) => "Update",
                        (Some(_), None) => "Remove",
                    },
                    edit.path
                ));
            }
            if others.len() > LISTED {
                body.push_str(&format!("- and {} more\n", others.len() - LISTED));
            }
        }

        if !self.exclusions.is_empty() {
            body.push_str(
                "\n<details>\n<summary>Decline tests permanently</summary>\n\n\
                 Add their identities to `[receive].exclude` in your manifest, \
                 keeping existing entries:\n\n```toml\nexclude = [\n",
            );
            for id in &self.exclusions {
                body.push_str(&format!("  {id:?},\n"));
            }
            body.push_str("]\n```\n\n</details>\n");
        }
        if !self.warnings.is_empty() {
            body.push_str(&format!(
                "\n<details>\n<summary>Warnings ({})</summary>\n\n",
                self.warnings.len()
            ));
            for warning in self.warnings.iter().take(LISTED) {
                body.push_str(&format!("- {warning}\n"));
            }
            if self.warnings.len() > LISTED {
                body.push_str(&format!("- and {} more\n", self.warnings.len() - LISTED));
            }
            body.push_str("\n</details>\n");
        }

        // Publication reads this line back to recognize a declined proposal.
        body.push_str(&format!("\nPlan SHA-256: `{}`\n", self.key()?));
        Ok(body)
    }
}

/// Most entries a description lists before pointing at the changed files.
const LISTED: usize = 100;

/// Whether a path is a record the tool maintains rather than a test file.
fn is_record(path: &str) -> bool {
    is_catalog(path) || path == RECEIPT_PATH
}

/// Whether a path holds catalog records: the shared file or one source's own.
fn is_catalog(path: &str) -> bool {
    path == CATALOG_PATH || path_source(path).is_some()
}

/// Keep author-supplied text inside one Markdown table cell.
fn cell(text: &str) -> String {
    text.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .replace('|', "\\|")
}

/// Proposed test-file changes by kind.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Change {
    pub added: usize,
    pub updated: usize,
    pub removed: usize,
}

impl Change {
    /// Whether the plan changes no test file.
    pub fn is_empty(&self) -> bool {
        self.added + self.updated + self.removed == 0
    }
}

impl std::fmt::Display for Change {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let parts: Vec<_> = [
            (self.added, "added"),
            (self.updated, "updated"),
            (self.removed, "removed"),
        ]
        .into_iter()
        .filter(|(count, _)| *count > 0)
        .map(|(count, kind)| format!("{count} {kind}"))
        .collect();
        if parts.is_empty() {
            write!(f, "no test changes")
        } else {
            write!(f, "{}", parts.join(", "))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test(name: &str, summary: &str) -> Test {
        Test {
            source: "github:example/source".into(),
            name: name.into(),
            revision: "a".repeat(40),
            source_path: format!("tests/{name}.wast"),
            manifest_revision: "a".repeat(40),
            sha256: "b".repeat(64),
            destination: format!("corpus/wast/example/source/{name}.wast"),
            summary: summary.into(),
            requires: BTreeSet::new(),
            covers: BTreeSet::new(),
            analysis: Analysis::default(),
        }
    }

    fn receive_plan() -> Plan {
        let edit = |path: &str, before: Option<&str>, after: Option<&str>| Edit {
            path: path.into(),
            before: before.map(|_| "c".repeat(64)),
            after: after.map(str::to_owned),
        };
        let tests = [
            test("added", "Adds | a\ntest"),
            test("updated", "Updates a test"),
            test("moved", "Moves a test"),
        ];
        Plan {
            schema: SCHEMA,
            kind: Kind::Receive,
            subject: "github:example/recipient".into(),
            edits: vec![
                edit(RECEIPT_PATH, Some("old"), Some("new")),
                edit("shared/example/source/added.wast", None, Some("(module)")),
                edit(
                    "shared/example/source/updated.wast",
                    Some("old"),
                    Some("(module)"),
                ),
                edit("old/example/source/moved.wast", Some("old"), None),
                edit("shared/example/source/moved.wast", None, Some("(module)")),
                edit("shared/example/source/retired.wast", Some("old"), None),
            ],
            guards: vec![],
            warnings: vec!["github:example/source/added: incomplete inference".into()],
            exclusions: tests.iter().map(Test::id).collect(),
            provenance: tests.into_iter().map(|test| (test.id(), test)).collect(),
        }
    }

    #[test]
    fn counts_test_files_but_not_records() {
        let changes = receive_plan().changes();
        assert_eq!(
            changes,
            Change {
                added: 2,
                updated: 1,
                removed: 2
            }
        );
        assert_eq!(changes.to_string(), "2 added, 1 updated, 2 removed");
        assert_eq!(Change::default().to_string(), "no test changes");
    }

    #[test]
    fn body_lists_each_test_once_with_its_change() {
        let plan = receive_plan();
        let body = plan.body().unwrap();
        assert!(body.starts_with(
            "Updates the tests this repository receives through cm-testsuite: \
             2 added, 1 updated, 2 removed.\n"
        ));
        let id = "github:example/source";
        assert!(body.contains(&format!(
            "| Add | `{id}/added` | `aaaaaaaaaaaa` | Adds \\| a test |"
        )));
        assert!(body.contains(&format!("| Update | `{id}/updated` |")));
        assert!(body.contains(&format!("| Move | `{id}/moved` |")));
        assert!(body.contains("Other files:\n\n- Remove `shared/example/source/retired.wast`\n"));
        assert!(!body.contains(RECEIPT_PATH));
        assert!(body.contains("<summary>Warnings (1)</summary>"));
        assert!(body.contains(&format!("  \"{id}/added\",\n")));
        assert!(body.ends_with(&format!("\nPlan SHA-256: `{}`\n", plan.key().unwrap())));
    }

    #[test]
    fn imports_edit_only_the_corpus_and_catalog_records() {
        let mut plan = receive_plan();
        plan.kind = Kind::Import;
        let edit = |path: &str| Edit {
            path: path.into(),
            before: None,
            after: Some(String::new()),
        };
        plan.edits = vec![
            edit("corpus/wast/example/source/added.wast"),
            edit(CATALOG_PATH),
            edit("metadata/catalog/example/source.toml"),
        ];
        plan.validate().unwrap();
        assert_eq!(plan.changes().to_string(), "1 added");
        assert!(!plan.body().unwrap().contains("Other files"));
        for outside in [
            "metadata/other.toml",
            "metadata/catalog/source.toml",
            "metadata/catalog/example/source/extra.toml",
            "metadata/catalog/example/source.txt",
            "metadata/catalog/README.md",
            RECEIPT_PATH,
        ] {
            plan.edits = vec![edit(outside)];
            assert!(plan.validate().is_err(), "{outside}");
        }
    }

    #[test]
    fn titles_name_the_proposal_not_the_destination() {
        let mut plan = receive_plan();
        assert_eq!(plan.title(), "Update tests received from cm-testsuite");
        plan.kind = Kind::Import;
        plan.subject = "github:example/a, github:example/b".into();
        assert_eq!(
            plan.title(),
            "feat(corpus): import tests from example/a and example/b"
        );
        plan.subject = "github:a/a, github:b/b, github:c/c".into();
        assert_eq!(
            plan.title(),
            "feat(corpus): import tests from 3 repositories"
        );
    }
}
