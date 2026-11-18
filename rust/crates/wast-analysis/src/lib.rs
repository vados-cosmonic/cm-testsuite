//! Admit structurally valid WAST and provide explicitly incomplete syntax evidence.

mod evidence;
mod metadata;
#[cfg(test)]
mod metadata_tests;

use anyhow::{Result, ensure};
use cm_catalog::Analysis;
use wast::{
    Wast,
    parser::{self, ParseBuffer},
};

pub const ANALYZER: &str = "cm-wast-analysis/0.2+wast-263.0.0";

/// Parse a nonempty WAST script and collect conservative feature/comment guidance.
/// Preserve expected-negative payloads; never execute assertions or infer conformance.
pub fn analyze(text: &str) -> Result<Analysis> {
    let buffer = ParseBuffer::new(text)?;
    let script = parser::parse::<Wast<'_>>(&buffer)?;
    ensure!(!script.directives.is_empty(), "empty WAST script");
    let (metadata, tokens) = metadata::extract(text)?;
    Ok(evidence::collect(text, &tokens, metadata))
}

#[cfg(test)]
mod tests {
    use super::*;
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
}
