// Copyright (c) 2025 Erick Bourgeois, firestoned
// SPDX-License-Identifier: Apache-2.0

//! named.conf reader (ADR-0006: replaces `hornet-bind9`'s named.conf parser).
//!
//! The file is read as a generic statement tree (words, optional `{ … }`
//! block, `;`), which every named.conf statement fits. forage then takes
//! only what it maps: `zone` (name, `type`, `file`), `options { directory }`,
//! `include`, and the names of `view`s. Everything else is kept as
//! [`Statement::Other`] and ignored.

use std::path::Path;

use crate::error::{Context, Error, Result};

/// A parsed named.conf, top-level statements in file order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NamedConf {
    pub statements: Vec<Statement>,
}

/// The top-level statements forage distinguishes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Statement {
    Zone(ZoneStmt),
    Options(OptionsStmt),
    Include(String),
    View(String),
    /// Any other statement, by keyword.
    Other(String),
}

/// A `zone` statement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ZoneStmt {
    pub name: String,
    pub options: ZoneOptions,
}

/// The zone options forage reads.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ZoneOptions {
    /// The `type`, with `master`/`slave` normalized to `primary`/`secondary`.
    pub zone_type: Option<String>,
    pub file: Option<String>,
}

/// The `options` values forage reads.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct OptionsStmt {
    pub directory: Option<String>,
}

/// Reads and parses a named.conf file.
///
/// # Errors
/// I/O failures and syntax errors, prefixed with the path.
pub fn parse_named_conf_file(path: &Path) -> Result<NamedConf> {
    let input = std::fs::read_to_string(path).with_context(|| path.display().to_string())?;
    parse_named_conf(&input).with_context(|| path.display().to_string())
}

/// Parses named.conf text.
///
/// # Errors
/// Unbalanced braces, missing `;`, unterminated strings or comments, and
/// statements that cannot carry the fields forage needs, with a line number.
pub fn parse_named_conf(input: &str) -> Result<NamedConf> {
    let tokens = tokenize(input)?;
    let mut pos = 0;
    let tree = parse_block(&tokens, &mut pos, false)?;
    let statements = tree.iter().map(classify).collect::<Result<_>>()?;
    Ok(NamedConf { statements })
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Tok {
    Word(String),
    Open,
    Close,
    Semi,
}

/// A generic statement: leading words, an optional block, the line it began on.
struct Node {
    words: Vec<String>,
    block: Option<Vec<Node>>,
    line: usize,
}

fn tokenize(input: &str) -> Result<Vec<(Tok, usize)>> {
    let mut tokens = Vec::new();
    let mut chars = input.chars().peekable();
    let mut line = 1;
    while let Some(c) = chars.next() {
        let start = line;
        match c {
            '\n' => line += 1,
            c if c.is_whitespace() => {}
            '#' => while chars.next_if(|n| *n != '\n').is_some() {},
            '/' if chars.peek() == Some(&'/') => while chars.next_if(|n| *n != '\n').is_some() {},
            '/' if chars.peek() == Some(&'*') => {
                chars.next();
                read_until(&mut chars, &mut line, "*/")
                    .ok_or_else(|| Error::new(format!("line {start}: unterminated comment")))?;
            }
            '{' => tokens.push((Tok::Open, start)),
            '}' => tokens.push((Tok::Close, start)),
            ';' => tokens.push((Tok::Semi, start)),
            '"' => {
                let word = read_until(&mut chars, &mut line, "\"")
                    .ok_or_else(|| Error::new(format!("line {start}: unterminated string")))?;
                tokens.push((Tok::Word(word), start));
            }
            c => {
                let mut word = c.to_string();
                while let Some(n) = chars.next_if(|n| !n.is_whitespace() && !"{};\"".contains(*n)) {
                    word.push(n);
                }
                tokens.push((Tok::Word(word), start));
            }
        }
    }
    Ok(tokens)
}

/// Reads up to and including `end` (which is dropped), counting newlines;
/// `None` if the input ends first.
fn read_until(
    chars: &mut std::iter::Peekable<std::str::Chars<'_>>,
    line: &mut usize,
    end: &str,
) -> Option<String> {
    let mut text = String::new();
    while !text.ends_with(end) {
        let ch = chars.next()?;
        *line += usize::from(ch == '\n');
        text.push(ch);
    }
    text.truncate(text.len() - end.len());
    Some(text)
}

/// Parses statements until `}` (when `nested`) or the end of input.
fn parse_block(tokens: &[(Tok, usize)], pos: &mut usize, nested: bool) -> Result<Vec<Node>> {
    let mut nodes = Vec::new();
    let open_line = pos.checked_sub(1).map_or(1, |p| tokens[p].1);
    loop {
        let Some((tok, line)) = tokens.get(*pos) else {
            if nested {
                return Err(Error::new(format!("line {open_line}: unclosed '{{'")));
            }
            return Ok(nodes);
        };
        let line = *line;
        if *tok == Tok::Close {
            if !nested {
                return Err(Error::new(format!("line {line}: unexpected '}}'")));
            }
            *pos += 1;
            return Ok(nodes);
        }
        let mut node = Node {
            words: Vec::new(),
            block: None,
            line,
        };
        while let Some((Tok::Word(w), _)) = tokens.get(*pos) {
            node.words.push(w.clone());
            *pos += 1;
        }
        if let Some((Tok::Open, _)) = tokens.get(*pos) {
            if node.words.is_empty() {
                return Err(Error::new(format!(
                    "line {line}: block without a statement keyword"
                )));
            }
            *pos += 1;
            node.block = Some(parse_block(tokens, pos, true)?);
        }
        match tokens.get(*pos) {
            Some((Tok::Semi, _)) => *pos += 1,
            _ => return Err(Error::new(format!("line {line}: missing ';'"))),
        }
        if !node.words.is_empty() {
            nodes.push(node);
        }
    }
}

/// The second word of the first sub-statement whose keyword is `key`.
fn sub_value(block: Option<&Vec<Node>>, key: &str) -> Option<String> {
    block?
        .iter()
        .find(|n| n.words[0] == key)
        .and_then(|n| n.words.get(1).cloned())
}

fn classify(node: &Node) -> Result<Statement> {
    let block = node.block.as_ref();
    Ok(match (node.words[0].as_str(), node.words.get(1).cloned()) {
        ("zone", Some(name)) => Statement::Zone(ZoneStmt {
            name,
            options: ZoneOptions {
                zone_type: sub_value(block, "type").map(|t| match t.as_str() {
                    "master" => "primary".to_string(),
                    "slave" => "secondary".to_string(),
                    _ => t,
                }),
                file: sub_value(block, "file"),
            },
        }),
        ("zone", None) => {
            return Err(Error::new(format!(
                "line {}: zone without a name",
                node.line
            )))
        }
        ("options", _) => Statement::Options(OptionsStmt {
            directory: sub_value(block, "directory"),
        }),
        ("include", Some(path)) => Statement::Include(path),
        ("view", Some(name)) => Statement::View(name),
        (keyword, _) => Statement::Other(keyword.to_string()),
    })
}

#[cfg(test)]
#[path = "named_conf_tests.rs"]
mod named_conf_tests;
