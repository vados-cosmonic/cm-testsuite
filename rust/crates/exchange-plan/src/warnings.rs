//! Group a plan's warnings by kind and attach each kind's guidance once.

use crate::{Kind as PlanKind, Plan, plan::LISTED};
use cm_catalog::warnings::{Kind, Warning, subject};
use std::collections::{BTreeMap, BTreeSet};

/// One stored plan warning: the test and route it names, when it names them.
struct Entry<'a> {
    test: Option<&'a str>,
    route: Option<&'a str>,
    stored: &'a str,
}

/// Recognized warnings by kind, and the rest as stored.
fn grouped(plan: &Plan) -> (BTreeMap<Kind, Vec<Entry<'_>>>, Vec<&str>) {
    let mut kinds: BTreeMap<_, Vec<_>> = BTreeMap::new();
    let mut other = vec![];
    for stored in &plan.warnings {
        let (test, text) = subject(stored);
        match Warning::parse(text) {
            Some(warning) => kinds.entry(warning.kind()).or_default().push(Entry {
                test,
                route: warning.route(),
                stored,
            }),
            None => other.push(stored.as_str()),
        }
    }
    (kinds, other)
}

impl Plan {
    /// One line per warning kind for terminal output: its count, title, and reference.
    pub fn warning_summary(&self) -> Vec<String> {
        let (kinds, other) = grouped(self);
        let mut lines: Vec<_> = kinds
            .iter()
            .map(|(kind, entries)| {
                format!(
                    "warning[{}]: {} ({}); see {}",
                    kind.code(),
                    kind.title(),
                    entries.len(),
                    kind.reference()
                )
            })
            .collect();
        lines.extend(other.iter().map(|stored| format!("warning: {stored}")));
        lines
    }

    /// The collapsed description section: one heading per kind with its meaning
    /// and action, then the tests it names. Empty when the plan has no warnings.
    pub(crate) fn warning_section(&self) -> String {
        if self.warnings.is_empty() {
            return String::new();
        }
        let (kinds, other) = grouped(self);
        let mut out = format!(
            "\n<details>\n<summary>Warnings ({})</summary>\n\n\
             Warnings do not block this pull request.\n",
            self.warnings.len()
        );
        for (kind, entries) in &kinds {
            out.push_str(&format!(
                "\n**{}** ({})\n\n{} [`{}`]({})\n",
                kind.title(),
                entries.len(),
                kind.meaning(),
                kind.code(),
                kind.reference(),
            ));
            let (label, lines) = match kind {
                // Every test has this warning, so their identities say nothing.
                Kind::IncompleteFeatureInference => (
                    "Check these first. Evidence is a hint: a keyword in an assertion \
                     that expects rejection is not a requirement.",
                    self.undeclared_evidence(entries),
                ),
                // Every selected test has this warning; the routes are what differ.
                Kind::AcceptedIncompleteInference => {
                    ("Tests selected per route:", per_route(entries))
                }
                _ => ("Tests:", entries.iter().map(line).collect()),
            };
            if !lines.is_empty() {
                out.push_str(&format!("\n{label}\n"));
                list(&mut out, &lines);
            }
            out.push_str("\nWhat to do:\n\n");
            for action in kind.actions() {
                out.push_str(&format!("- {action}\n"));
            }
        }
        if !other.is_empty() {
            out.push_str(&format!("\n**Other warnings** ({})\n", other.len()));
            list(
                &mut out,
                &other.iter().map(|s| (*s).to_owned()).collect::<Vec<_>>(),
            );
        }
        out.push_str("\n</details>\n");
        out
    }

    /// Imported tests whose syntax evidence names a feature their `requires` omits:
    /// the concrete place to start checking declarations.
    fn undeclared_evidence(&self, entries: &[Entry<'_>]) -> Vec<String> {
        if self.kind != PlanKind::Import {
            return vec![];
        }
        let warned: BTreeSet<_> = entries.iter().filter_map(|entry| entry.test).collect();
        let mut lines = vec![];
        for (id, test) in &self.provenance {
            let undeclared: BTreeSet<_> = test
                .analysis
                .syntax
                .iter()
                .map(|evidence| evidence.feature.as_str())
                .filter(|feature| !test.requires.contains(*feature))
                .collect();
            if warned.contains(id.as_str()) && !undeclared.is_empty() {
                let features: Vec<_> = undeclared.iter().map(|f| format!("`{f}`")).collect();
                lines.push(format!(
                    "`{id}`: syntax evidence for {}, absent from `requires`",
                    features.join(", ")
                ));
            }
        }
        lines
    }
}

/// Name the test, and the route when the warning concerns one.
fn line(entry: &Entry<'_>) -> String {
    match (entry.test, entry.route) {
        (Some(test), Some(route)) => format!("`{test}` (route `{route}`)"),
        (Some(test), None) => format!("`{test}`"),
        (None, _) => entry.stored.to_owned(),
    }
}

/// Count the tests each route selected.
fn per_route(entries: &[Entry<'_>]) -> Vec<String> {
    let mut routes: BTreeMap<_, usize> = BTreeMap::new();
    for entry in entries {
        *routes.entry(entry.route.unwrap_or_default()).or_default() += 1;
    }
    routes
        .iter()
        .map(|(route, count)| format!("route `{route}`: {count}"))
        .collect()
}

/// Append a bounded Markdown list, counting what it leaves out.
fn list(out: &mut String, lines: &[String]) {
    if !lines.is_empty() {
        out.push('\n');
    }
    for line in lines.iter().take(LISTED) {
        out.push_str(&format!("- {line}\n"));
    }
    if lines.len() > LISTED {
        out.push_str(&format!("- and {} more\n", lines.len() - LISTED));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cm_catalog::{Analysis, Evidence, SCHEMA, Test};

    const SOURCE: &str = "github:example/source";

    fn test(name: &str, requires: &[&str], evidence: &[&str]) -> Test {
        Test {
            source: SOURCE.into(),
            name: name.into(),
            revision: "a".repeat(40),
            source_path: format!("tests/{name}.wast"),
            manifest_revision: "a".repeat(40),
            sha256: "b".repeat(64),
            destination: format!("corpus/wast/example/source/{name}.wast"),
            summary: String::new(),
            requires: requires.iter().map(|s| (*s).to_owned()).collect(),
            covers: BTreeSet::new(),
            analysis: Analysis {
                syntax: evidence
                    .iter()
                    .map(|feature| Evidence {
                        feature: (*feature).into(),
                        byte_offset: 0,
                    })
                    .collect(),
                ..Analysis::default()
            },
        }
    }

    fn plan(kind: PlanKind, tests: Vec<Test>, warnings: Vec<String>) -> Plan {
        Plan {
            schema: SCHEMA,
            kind,
            subject: SOURCE.into(),
            edits: vec![],
            guards: vec![],
            warnings,
            exclusions: BTreeSet::new(),
            provenance: tests.into_iter().map(|test| (test.id(), test)).collect(),
        }
    }

    fn stored(name: &str, warning: Warning<'_>) -> String {
        format!("{SOURCE}/{name}: {warning}")
    }

    #[test]
    fn import_section_explains_each_kind_once() {
        let plan = plan(
            PlanKind::Import,
            vec![
                test("declared", &["component-model"], &["component-model"]),
                test(
                    "undeclared",
                    &["resources"],
                    &["simd", "component-model", "simd"],
                ),
            ],
            vec![
                stored("declared", Warning::IncompleteFeatureInference),
                stored("declared", Warning::MissingIntentComment),
                stored("undeclared", Warning::IncompleteFeatureInference),
                stored("undeclared", Warning::MissingIntentComment),
                stored("undeclared", Warning::UnexplainedAssertions),
                // Stored for a test this plan leaves unchanged.
                stored("unchanged", Warning::IncompleteFeatureInference),
            ],
        );
        let section = plan.warning_section();
        assert!(section.starts_with("\n<details>\n<summary>Warnings (6)</summary>\n"));
        assert!(section.ends_with("\n</details>\n"));

        for (kind, count) in [
            (Kind::IncompleteFeatureInference, 3),
            (Kind::MissingIntentComment, 2),
            (Kind::UnexplainedAssertions, 1),
        ] {
            let heading = format!("\n**{}** ({count})\n", kind.title());
            assert_eq!(section.matches(&heading).count(), 1, "{heading}");
            assert_eq!(section.matches(kind.meaning()).count(), 1);
            let actions: String = kind.actions().iter().map(|a| format!("- {a}\n")).collect();
            assert_eq!(
                section
                    .matches(&format!("\nWhat to do:\n\n{actions}"))
                    .count(),
                1
            );
            assert!(section.contains(&format!("[`{}`]({})", kind.code(), kind.reference())));
        }
        // The unconditional kind lists only tests whose evidence outruns `requires`.
        assert!(section.contains(&format!(
            "- `{SOURCE}/undeclared`: syntax evidence for `component-model`, `simd`, \
             absent from `requires`\n"
        )));
        assert!(!section.contains(&format!("- `{SOURCE}/declared`: syntax evidence")));
        assert!(!section.contains("unchanged"));
        assert!(section.contains(&format!(
            "\nTests:\n\n- `{SOURCE}/declared`\n- `{SOURCE}/undeclared`\n\nWhat to do:\n"
        )));
        // No stored sentence is repeated per test.
        assert!(!section.contains(&Warning::IncompleteFeatureInference.to_string()));
    }

    #[test]
    fn receive_section_names_routes() {
        let plan = plan(
            PlanKind::Receive,
            vec![test("a", &[], &["simd"])],
            vec![
                stored("a", Warning::AcceptedIncompleteInference { route: "wide" }),
                stored("b", Warning::AcceptedIncompleteInference { route: "wide" }),
                stored(
                    "c",
                    Warning::AcceptedIncompleteInference { route: "narrow" },
                ),
                stored("d", Warning::RouteNeedsAllowUnknown { route: "strict" }),
            ],
        );
        let section = plan.warning_section();
        assert!(section.contains(&format!("- `{SOURCE}/d` (route `strict`)\n")));
        assert!(
            section.contains(
                "\nTests selected per route:\n\n- route `narrow`: 1\n- route `wide`: 2\n"
            )
        );
        assert!(section.contains(Kind::RouteNeedsAllowUnknown.actions()[0]));
        // Receivers cannot change a source's `requires`.
        assert!(!section.contains("syntax evidence for"));
        assert_eq!(
            plan.warning_summary(),
            [
                format!(
                    "warning[route-needs-allow-unknown]: {} (1); see {}",
                    Kind::RouteNeedsAllowUnknown.title(),
                    Kind::RouteNeedsAllowUnknown.reference()
                ),
                format!(
                    "warning[accepted-incomplete-inference]: {} (3); see {}",
                    Kind::AcceptedIncompleteInference.title(),
                    Kind::AcceptedIncompleteInference.reference()
                ),
            ]
        );
    }

    #[test]
    fn long_lists_are_bounded_and_unknown_text_is_kept() {
        let mut warnings: Vec<_> = (0..LISTED + 50)
            .flat_map(|n| {
                let name = format!("t{n:03}");
                [
                    stored(&name, Warning::IncompleteFeatureInference),
                    stored(&name, Warning::MissingIntentComment),
                ]
            })
            .collect();
        warnings.push("a warning from another version".into());
        let plan = plan(PlanKind::Import, vec![], warnings);
        let section = plan.warning_section();
        assert!(section.contains(&format!(
            "<summary>Warnings ({})</summary>",
            2 * LISTED + 101
        )));
        assert!(section.contains(&format!("- `{SOURCE}/t099`\n- and 50 more\n")));
        assert!(!section.contains("t100"));
        assert!(section.contains("\n**Other warnings** (1)\n\n- a warning from another version\n"));
        assert_eq!(
            plan.warning_summary().last().unwrap(),
            "warning: a warning from another version"
        );
        // Guidance appears once per kind, so 300 warnings stay far below the description cap.
        assert!(section.len() < 10_000, "{}", section.len());
    }

    #[test]
    fn plans_without_warnings_have_no_section() {
        let plan = plan(PlanKind::Import, vec![], vec![]);
        assert_eq!(plan.warning_section(), "");
        assert!(plan.warning_summary().is_empty());
    }
}
