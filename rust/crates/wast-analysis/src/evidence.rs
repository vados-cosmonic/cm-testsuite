//! Conservative syntax hints and comment guidance; never execution requirements.

use crate::ANALYZER;
use anyhow::Result;
use cm_catalog::{Analysis, Evidence};
use wast::lexer::{Lexer, TokenKind};

/// Inspect tokens without mistaking string/comment contents for feature usage.
pub(crate) fn collect(text: &str) -> Result<Analysis> {
    // Syntax hints are conservative: unknown requirements must stay visible.
    let mut out = Analysis {
        analyzer: ANALYZER.into(),
        complete: false,
        syntax: vec![],
        warnings: vec![
            "Feature inference is incomplete; syntax evidence is not a complete execution requirement set.".into(),
        ],
    };
    let mut first = true;
    let mut has_intent = false;
    let mut assertions = false;
    let mut comments = 0;

    // The lexer distinguishes actual keywords from comments and quoted strings.
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

    // Editorial warnings help authors explain intent without rejecting valid WAST.
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
