//! Structural WAST admission and explicitly incomplete syntax evidence. No execution.
use anyhow::{Result, ensure};
use cm_catalog::{Analysis, Evidence};
use wast::{
    Wast,
    lexer::{Lexer, TokenKind},
    parser::{self, ParseBuffer},
};
pub const ANALYZER: &str = "cm-wast-analysis/0.1+wast-263.0.0";
pub fn analyze(text: &str) -> Result<Analysis> {
    let buffer = ParseBuffer::new(text)?;
    let script = parser::parse::<Wast<'_>>(&buffer)?;
    ensure!(!script.directives.is_empty(), "empty WAST script");
    let mut out=Analysis{analyzer:ANALYZER.into(),complete:false,syntax:vec![],warnings:vec!["Feature inference is incomplete; syntax evidence is not a complete execution requirement set.".into()]};
    let mut first = true;
    let mut has_intent = false;
    let mut assertions = false;
    let mut comments = 0;
    for token in Lexer::new(text).iter(0) {
        let token = token?;
        let value = &text[token.offset..token.offset + token.len as usize];
        match token.kind {
            TokenKind::Whitespace => {}
            TokenKind::LineComment | TokenKind::BlockComment => {
                comments += 1;
                if first
                    && value
                        .trim_matches([';', '(', ')', ' ', '\n', '\r', '\t'])
                        .len()
                        >= 12
                {
                    has_intent = true;
                }
            }
            TokenKind::Keyword => {
                first = false;
                let feature = match value {
                    "component" => Some("component-model"),
                    "resource" => Some("resources"),
                    "v128" => Some("simd"),
                    "shared" => Some("threads"),
                    "memory64" => Some("memory64"),
                    _ => None,
                };
                if let Some(feature) = feature {
                    out.syntax.push(Evidence {
                        feature: feature.into(),
                        byte_offset: token.offset,
                    });
                }
                assertions |= value.starts_with("assert_");
            }
            _ => first = false,
        }
    }
    if !has_intent {
        out.warnings
            .push("Add a top-of-file comment explaining this test's intent.".into());
    }
    if assertions && comments <= 1 {
        out.warnings
            .push("Review whether subtle assertions need nearby explanatory comments.".into());
    }
    Ok(out)
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
