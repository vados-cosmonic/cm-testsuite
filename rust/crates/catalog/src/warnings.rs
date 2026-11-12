//! Warning kinds: the text stored in records, and the guidance shown beside it.
//!
//! Analyses and plans store warnings as plain strings that digests cover, so
//! an existing kind's stored text must never change. Guidance is attached when
//! a warning is displayed, by recognizing the stored text.

use std::fmt;

/// Where each kind is documented, by anchor.
const REFERENCE: &str =
    "https://github.com/vados-cosmonic/cm-testsuite/blob/main/docs/src/warnings.md";

/// Keywords the analyzer records as syntax evidence, with the feature each suggests.
pub const SYNTAX_FEATURES: [(&str, &str); 5] = [
    ("component", "component-model"),
    ("resource", "resources"),
    ("v128", "simd"),
    ("shared", "threads"),
    ("memory64", "memory64"),
];

/// A kind of warning, in the order descriptions list them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Kind {
    IncompleteFeatureInference,
    MissingIntentComment,
    UnexplainedAssertions,
    RouteNeedsAllowUnknown,
    AcceptedIncompleteInference,
}

impl Kind {
    pub const ALL: [Self; 5] = [
        Self::IncompleteFeatureInference,
        Self::MissingIntentComment,
        Self::UnexplainedAssertions,
        Self::RouteNeedsAllowUnknown,
        Self::AcceptedIncompleteInference,
    ];

    /// Stable identifier, also the anchor of the kind's reference entry.
    pub fn code(self) -> &'static str {
        match self {
            Self::IncompleteFeatureInference => "incomplete-feature-inference",
            Self::MissingIntentComment => "missing-intent-comment",
            Self::UnexplainedAssertions => "unexplained-assertions",
            Self::RouteNeedsAllowUnknown => "route-needs-allow-unknown",
            Self::AcceptedIncompleteInference => "accepted-incomplete-inference",
        }
    }

    /// Short heading for a group of warnings of this kind.
    pub fn title(self) -> &'static str {
        match self {
            Self::IncompleteFeatureInference => "Feature requirements are not inferred",
            Self::MissingIntentComment => "No intent comment at the top of the file",
            Self::UnexplainedAssertions => "Assertions without explanatory comments",
            Self::RouteNeedsAllowUnknown => "Route withheld tests it otherwise matches",
            Self::AcceptedIncompleteInference => "Tests selected on declared requirements alone",
        }
    }

    /// What was observed and why it matters.
    pub fn meaning(self) -> &'static str {
        match self {
            Self::IncompleteFeatureInference => {
                "Reported for every test; nothing is wrong with the file. \
                 The analyzer parses a test and records a few keywords \
                 (`component`, `resource`, `v128`, `shared`, `memory64`) as syntax evidence. \
                 It never works out what an engine needs to run the test, \
                 so routing relies on the `requires` declared in the source manifest, \
                 and only routes with `allow_unknown = true` receive tests."
            }
            Self::MissingIntentComment => {
                "The file does not open with a comment of at least 12 characters, \
                 so reviewers and receiving repositories cannot tell what the test checks. \
                 Blocks neither import nor delivery."
            }
            Self::UnexplainedAssertions => {
                "The file has `assert_*` directives and fewer than two comments, \
                 counting the intent comment. \
                 The check counts comments; \
                 it does not judge whether the assertions need explaining. \
                 Blocks neither import nor delivery."
            }
            Self::RouteNeedsAllowUnknown => {
                "The route's `supports`, `covers_any`, and `exclude` accept these tests, \
                 but the route does not set `allow_unknown = true`, \
                 so they are not delivered through it. \
                 Feature inference is never complete, \
                 so such a route receives no tests at all."
            }
            Self::AcceptedIncompleteInference => {
                "Reported for every test a route with `allow_unknown = true` selects, \
                 changed by this plan or not. \
                 Selection used the `requires` declared by the test's author; \
                 it does not show that your engine supports or passes the test."
            }
        }
    }

    /// Who acts, and how: one entry per role, each opening with the role.
    pub fn actions(self) -> &'static [&'static str] {
        match self {
            Self::IncompleteFeatureInference => &[
                "Source maintainer: list every capability the test needs in its `requires`; \
                 no change to the test file removes this warning.",
                "Catalog reviewer: check each test's `requires` against its contents \
                 before merging.",
                "Receiving repository: set `allow_unknown = true` on a route to receive tests.",
            ],
            Self::MissingIntentComment => &[
                "Source maintainer: start the file with a comment that states \
                 what the test checks; the next import picks up the change.",
                "Catalog reviewer: ask the source maintainer; \
                 do not edit the copy under `corpus/wast/`, which must match its source.",
            ],
            Self::UnexplainedAssertions => &[
                "Source maintainer: comment the assertions whose expected result is not obvious; \
                 leave self-explanatory tests as they are.",
                "Catalog reviewer: no action is required.",
            ],
            Self::RouteNeedsAllowUnknown => &["Receiving repository: to receive these tests, \
                 set `allow_unknown = true` on the route \
                 once its `supports` lists only what your engine implements; \
                 to decline them, add their identities to `exclude`, \
                 which also ends this warning."],
            Self::AcceptedIncompleteInference => &[
                "Receiving repository: run the proposed tests on your engine before merging, \
                 and add any you decline to `exclude`; \
                 no manifest change is needed otherwise.",
            ],
        }
    }

    /// Link to the kind's reference entry.
    pub fn reference(self) -> String {
        format!("{REFERENCE}#{}", self.code())
    }
}

/// A warning as stored in an analysis or a plan; `Display` gives the stored text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Warning<'a> {
    IncompleteFeatureInference,
    MissingIntentComment,
    UnexplainedAssertions,
    RouteNeedsAllowUnknown { route: &'a str },
    AcceptedIncompleteInference { route: &'a str },
}

const INCOMPLETE: &str = "Feature inference is incomplete; \
                          syntax evidence is not a complete execution requirement set.";
const INTENT: &str = "Add a top-of-file comment explaining this test's intent.";
const ASSERTIONS: &str = "Review whether subtle assertions need nearby explanatory comments.";
const NEEDS_PREFIX: &str = "route ";
const NEEDS_SUFFIX: &str = " needs explicit allow_unknown for incomplete feature inference";
const ACCEPTED_PREFIX: &str = "accepted incomplete feature inference via route ";

impl<'a> Warning<'a> {
    /// Recognize stored text; `None` for text no kind produces.
    pub fn parse(text: &'a str) -> Option<Self> {
        match text {
            INCOMPLETE => Some(Self::IncompleteFeatureInference),
            INTENT => Some(Self::MissingIntentComment),
            ASSERTIONS => Some(Self::UnexplainedAssertions),
            _ => {
                if let Some(route) = text.strip_prefix(ACCEPTED_PREFIX) {
                    Some(Self::AcceptedIncompleteInference { route })
                } else {
                    text.strip_prefix(NEEDS_PREFIX)?
                        .strip_suffix(NEEDS_SUFFIX)
                        .map(|route| Self::RouteNeedsAllowUnknown { route })
                }
            }
        }
    }

    pub fn kind(self) -> Kind {
        match self {
            Self::IncompleteFeatureInference => Kind::IncompleteFeatureInference,
            Self::MissingIntentComment => Kind::MissingIntentComment,
            Self::UnexplainedAssertions => Kind::UnexplainedAssertions,
            Self::RouteNeedsAllowUnknown { .. } => Kind::RouteNeedsAllowUnknown,
            Self::AcceptedIncompleteInference { .. } => Kind::AcceptedIncompleteInference,
        }
    }

    /// The receiving route a warning names, if any.
    pub fn route(self) -> Option<&'a str> {
        match self {
            Self::RouteNeedsAllowUnknown { route }
            | Self::AcceptedIncompleteInference { route } => Some(route),
            _ => None,
        }
    }
}

impl fmt::Display for Warning<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::IncompleteFeatureInference => f.write_str(INCOMPLETE),
            Self::MissingIntentComment => f.write_str(INTENT),
            Self::UnexplainedAssertions => f.write_str(ASSERTIONS),
            Self::RouteNeedsAllowUnknown { route } => {
                write!(f, "{NEEDS_PREFIX}{route}{NEEDS_SUFFIX}")
            }
            Self::AcceptedIncompleteInference { route } => write!(f, "{ACCEPTED_PREFIX}{route}"),
        }
    }
}

/// Split a plan warning into the test identity it names, if any, and the stored text.
pub fn subject(entry: &str) -> (Option<&str>, &str) {
    match entry.split_once(": ") {
        Some((id, text)) if crate::validate_id(id).is_ok() => (Some(id), text),
        _ => (None, entry),
    }
}

/// Explain one stored warning for terminal output: its kind, meaning, action, and reference.
/// Unrecognized text is returned as it is.
pub fn explain(text: &str) -> String {
    match Warning::parse(text) {
        Some(warning) => {
            let kind = warning.kind();
            let mut out = format!(
                "warning[{}]: {text}\n  {}",
                kind.code(),
                plain(kind.meaning())
            );
            for action in kind.actions() {
                out.push_str(&format!("\n  {}", plain(action)));
            }
            out + &format!("\n  See {}", kind.reference())
        }
        None => format!("warning: {text}"),
    }
}

/// Drop Markdown code marks and put each sentence on its own line for terminal output.
fn plain(text: &str) -> String {
    text.replace('`', "").replace(". ", ".\n  ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    fn samples() -> [Warning<'static>; 5] {
        [
            Warning::IncompleteFeatureInference,
            Warning::MissingIntentComment,
            Warning::UnexplainedAssertions,
            Warning::RouteNeedsAllowUnknown { route: "wasi/p2" },
            Warning::AcceptedIncompleteInference { route: "wasi/p2" },
        ]
    }

    #[test]
    fn stored_text_is_stable_and_recognized() {
        // Digests of catalogs, snapshots, and plans cover these exact strings.
        let stored = [
            "Feature inference is incomplete; syntax evidence is not a complete execution requirement set.",
            "Add a top-of-file comment explaining this test's intent.",
            "Review whether subtle assertions need nearby explanatory comments.",
            "route wasi/p2 needs explicit allow_unknown for incomplete feature inference",
            "accepted incomplete feature inference via route wasi/p2",
        ];
        for ((warning, text), kind) in samples().into_iter().zip(stored).zip(Kind::ALL) {
            assert_eq!(warning.to_string(), text);
            assert_eq!(Warning::parse(text), Some(warning));
            assert_eq!(warning.kind(), kind);
        }
        assert_eq!(samples()[0].route(), None);
        assert_eq!(samples()[3].route(), Some("wasi/p2"));
        assert_eq!(Warning::parse("incomplete inference"), None);
        assert_eq!(Warning::parse("route x needs something else"), None);
    }

    #[test]
    fn every_kind_says_who_acts() {
        let codes: BTreeSet<_> = Kind::ALL.iter().map(|kind| kind.code()).collect();
        assert_eq!(codes.len(), Kind::ALL.len());
        for kind in Kind::ALL {
            assert!(!kind.title().is_empty() && !kind.meaning().is_empty());
            assert!(!kind.actions().is_empty(), "{}", kind.code());
            for action in kind.actions() {
                let (actor, instruction) = action.split_once(": ").unwrap();
                assert!(
                    [
                        "Source maintainer",
                        "Catalog reviewer",
                        "Receiving repository"
                    ]
                    .contains(&actor),
                    "{action}"
                );
                assert!(instruction.ends_with('.'));
            }
            assert!(kind.reference().ends_with(&format!("#{}", kind.code())));
        }
    }

    #[test]
    fn plan_entries_split_into_test_and_text() {
        assert_eq!(
            subject("github:example/source/values/u32-add: route x: y"),
            (Some("github:example/source/values/u32-add"), "route x: y")
        );
        assert_eq!(subject("no identity: here"), (None, "no identity: here"));
        assert_eq!(subject("plain"), (None, "plain"));
    }

    #[test]
    fn explanations_name_the_kind_and_keep_unknown_text() {
        let text = Warning::MissingIntentComment.to_string();
        let explained = explain(&text);
        assert!(explained.starts_with(&format!("warning[missing-intent-comment]: {text}\n")));
        assert!(explained.contains("Source maintainer: start the file with a comment"));
        assert!(explained.ends_with("warnings.md#missing-intent-comment"));
        assert!(!explained.contains('`'));
        assert_eq!(explain("something new"), "warning: something new");
    }
}
