//! Conservative syntax hints and comment guidance; never execution requirements.

use crate::ANALYZER;
use anyhow::Result;
use cm_catalog::warnings::{SYNTAX_FEATURES, Warning};
use cm_catalog::{Analysis, Evidence};
use wast::lexer::{Lexer, TokenKind};

/// Inspect tokens without mistaking string/comment contents for feature usage.
pub(crate) fn collect(text: &str) -> Result<Analysis> {
    // Syntax hints are conservative: unknown requirements must stay visible.
    let mut out = Analysis {
        analyzer: ANALYZER.into(),
        complete: false,
        syntax: vec![],
        warnings: vec![Warning::IncompleteFeatureInference.to_string()],
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
                let feature = SYNTAX_FEATURES
                    .iter()
                    .find(|(keyword, _)| *keyword == value);
                if let Some((_, feature)) = feature {
                    out.syntax.push(Evidence {
                        feature: (*feature).into(),
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
        out.warnings.push(Warning::MissingIntentComment.to_string());
    }
    if assertions && comments <= 1 {
        out.warnings
            .push(Warning::UnexplainedAssertions.to_string());
    }
    Ok(out)
}
