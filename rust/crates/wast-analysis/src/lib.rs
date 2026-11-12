//! Admit structurally valid WAST and provide explicitly incomplete syntax evidence.

mod evidence;

use anyhow::{Result, ensure};
use cm_catalog::Analysis;
use wast::{
    Wast,
    parser::{self, ParseBuffer},
};

pub const ANALYZER: &str = "cm-wast-analysis/0.1+wast-263.0.0";

/// Parse a nonempty WAST script and collect conservative feature/comment guidance.
/// Preserve expected-negative payloads; never execute assertions or infer conformance.
pub fn analyze(text: &str) -> Result<Analysis> {
    let buffer = ParseBuffer::new(text)?;
    let script = parser::parse::<Wast<'_>>(&buffer)?;
    ensure!(!script.directives.is_empty(), "empty WAST script");
    evidence::collect(text)
}

#[cfg(test)]
mod tests {
    use super::*;
    use cm_catalog::warnings::{Kind, SYNTAX_FEATURES, Warning};

    #[test]
    fn preserves_negative_tests() {
        for s in [
            "(module)",
            "(component)",
            "(assert_invalid (module (func (result i32))) \"type mismatch\")",
            "(assert_malformed (module quote \"(module broken\") \"bad\")",
        ] {
            assert!(analyze(s).is_ok(), "{s}");
        }
        assert!(analyze("(module").is_err());
    }

    #[test]
    fn strings_and_comments_are_not_feature_evidence() {
        let a=analyze(";; component resource shared v128 mentioned in prose\n(module (export \"component\" (func 0)) (func))").unwrap();
        assert!(a.syntax.is_empty());
        assert!(!a.complete);
        assert!(!a.warnings.iter().any(|x| x.contains("top-of-file")));
    }

    fn kinds(text: &str) -> Vec<Kind> {
        let analysis = analyze(text).unwrap();
        assert!(!analysis.complete);
        analysis
            .warnings
            .iter()
            .map(|stored| Warning::parse(stored).expect(stored).kind())
            .collect()
    }

    #[test]
    fn every_warning_is_a_known_kind() {
        const INTENT: &str = ";; Checks that addition wraps.\n";
        const ASSERT: &str = "(module)\n(assert_invalid (module (func (result i32))) \"x\")";
        // Inference is never complete, whatever the file contains.
        assert_eq!(
            kinds(&format!("{INTENT};; Rejected: no result.\n{ASSERT}")),
            [Kind::IncompleteFeatureInference]
        );
        assert_eq!(
            kinds(&format!("{INTENT}(module)")),
            [Kind::IncompleteFeatureInference]
        );
        // A short comment, or one after the first token, is not an intent comment.
        for text in [
            "(module)",
            ";; short\n(module)",
            "(module)\n;; A long comment after the module.",
        ] {
            assert_eq!(
                kinds(text),
                [Kind::IncompleteFeatureInference, Kind::MissingIntentComment],
                "{text}"
            );
        }
        // Assertions need a second comment anywhere; the intent comment is the first.
        assert_eq!(
            kinds(&format!("{INTENT}{ASSERT}")),
            [
                Kind::IncompleteFeatureInference,
                Kind::UnexplainedAssertions
            ]
        );
        assert_eq!(kinds(ASSERT), Kind::ALL[..3]);
    }

    #[test]
    fn guidance_names_every_evidence_keyword() {
        let analysis =
            analyze("(component)\n(module (memory 1 1 shared) (func (param v128)))").unwrap();
        let features: Vec<_> = analysis.syntax.iter().map(|e| e.feature.as_str()).collect();
        assert_eq!(features, ["component-model", "threads", "simd"]);
        for (keyword, _) in SYNTAX_FEATURES {
            assert!(
                Kind::IncompleteFeatureInference
                    .meaning()
                    .contains(&format!("`{keyword}`")),
                "{keyword}"
            );
        }
    }
}
