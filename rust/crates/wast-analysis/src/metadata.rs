//! File-level annotations, using WAST tokens to preserve strings and source offsets.

use anyhow::{Context, Result, ensure};
use cm_catalog::WastMetadata;
use std::collections::BTreeSet;
use wast::{
    lexer::{Lexer, Token, TokenKind},
    parser::{self, ParseBuffer},
};

/// Remove annotation tokens from syntax evidence and extract the optional header.
pub(crate) fn extract(text: &str) -> Result<(Option<WastMetadata>, Vec<Token>)> {
    let lexer = Lexer::new(text);
    let tokens = lexer.iter(0).collect::<Result<Vec<_>, _>>()?;
    let mut code = Vec::new();
    let mut metadata = None;
    let mut seen_code = false;
    let mut i = 0;
    while i < tokens.len() {
        let token = tokens[i];
        if token.kind == TokenKind::LParen
            && let Some(annotation) = lexer.annotation(token.offset + 1)?
        {
            let end = group_end(&tokens, i)?;
            if annotation.annotation(text)? == "cm-testsuite" {
                ensure!(
                    metadata.is_none(),
                    "duplicate @cm-testsuite header at byte {}",
                    token.offset
                );
                ensure!(
                    !seen_code,
                    "@cm-testsuite must precede all WAST directives at byte {}",
                    token.offset
                );
                metadata = Some(
                    parse_header(text, &tokens[i + 2..end])
                        .with_context(|| format!("@cm-testsuite at byte {}", token.offset))?,
                );
            }
            i = end + 1;
            continue;
        }
        if !trivia(token) {
            seen_code = true;
        }
        code.push(token);
        i += 1;
    }
    ensure!(seen_code, "empty WAST script");
    Ok((metadata, code))
}

fn trivia(token: Token) -> bool {
    matches!(
        token.kind,
        TokenKind::Whitespace | TokenKind::LineComment | TokenKind::BlockComment
    )
}

fn group_end(tokens: &[Token], start: usize) -> Result<usize> {
    let mut depth = 0;
    for (i, token) in tokens.iter().enumerate().skip(start) {
        match token.kind {
            TokenKind::LParen => depth += 1,
            TokenKind::RParen => {
                depth -= 1;
                if depth == 0 {
                    return Ok(i);
                }
            }
            _ => {}
        }
    }
    anyhow::bail!("unclosed annotation at byte {}", tokens[start].offset)
}

fn parse_header(text: &str, tokens: &[Token]) -> Result<WastMetadata> {
    let tokens: Vec<_> = tokens.iter().copied().filter(|t| !trivia(*t)).collect();
    let mut fields = BTreeSet::new();
    let mut schema = None;
    let mut intent = None;
    let mut requires = None;
    let mut covers = None;
    let mut i = 0;
    while i < tokens.len() {
        ensure!(
            tokens[i].kind == TokenKind::LParen,
            "expected metadata field at byte {}",
            tokens[i].offset
        );
        let end = group_end(&tokens, i)?;
        ensure!(
            end > i + 1 && tokens[i + 1].kind == TokenKind::Keyword,
            "expected metadata field name at byte {}",
            tokens[i].offset
        );
        let name = tokens[i + 1].src(text);
        ensure!(fields.insert(name), "duplicate metadata field {name}");
        let values = &tokens[i + 2..end];
        match name {
            "schema" => {
                ensure!(values.len() == 1, "schema requires one integer");
                let buffer = ParseBuffer::new(values[0].src(text))?;
                let value = parser::parse::<u32>(&buffer)?;
                ensure!(value == 1, "unsupported @cm-testsuite schema {value}");
                schema = Some(value);
            }
            "intent" => {
                ensure!(values.len() == 1, "intent requires one string");
                intent = Some(string(text, values[0])?);
            }
            "requires" => requires = Some(labels(text, values)?),
            "covers" => covers = Some(labels(text, values)?),
            "extensions" => {
                // Extension payloads remain in the original WAST; their semantics are
                // owned by other tools. Require namespaced annotations as containers.
                let mut j = 0;
                while j < values.len() {
                    ensure!(
                        values[j].kind == TokenKind::LParen
                            && Lexer::new(text).annotation(values[j].offset + 1)?.is_some(),
                        "extensions must contain annotations"
                    );
                    j = group_end(values, j)? + 1;
                }
            }
            _ => anyhow::bail!(
                "unknown metadata field {name}; use extensions for custom annotations"
            ),
        }
        i = end + 1;
    }
    Ok(WastMetadata {
        schema: schema.context("missing metadata schema")?,
        intent: intent.context("missing metadata intent")?,
        requires,
        covers,
    })
}

fn string(text: &str, token: Token) -> Result<String> {
    ensure!(
        token.kind == TokenKind::String,
        "expected metadata string at byte {}",
        token.offset
    );
    let value = String::from_utf8(token.string(text).into_owned())
        .context("metadata strings must be UTF-8")?;
    ensure!(
        !value.trim().is_empty(),
        "metadata strings must not be blank"
    );
    Ok(value)
}

fn labels(text: &str, tokens: &[Token]) -> Result<BTreeSet<String>> {
    tokens.iter().map(|t| string(text, *t)).collect()
}
