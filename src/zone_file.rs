// Copyright (c) 2025 Erick Bourgeois, firestoned
// SPDX-License-Identifier: Apache-2.0

//! Zone-file reader (ADR-0006: replaces `hornet-bind9`'s zone-file parser).
//!
//! Follows RFC 1035 §5.1 master-file rules for the record types forage maps:
//! physical lines are joined while a `(` is open, `;` starts a comment and
//! quotes protect both, a line starting with whitespace inherits the previous
//! owner, and TTL and class may come in either order. A line that does not
//! parse is returned in [`ZoneFile::skipped`] with its line number, never
//! dropped silently.

use std::net::{Ipv4Addr, Ipv6Addr};
use std::path::Path;

use crate::error::{Context, Result};

const SECONDS_PER_MINUTE: u32 = 60;
const SECONDS_PER_HOUR: u32 = 3_600;
const SECONDS_PER_DAY: u32 = 86_400;
const SECONDS_PER_WEEK: u32 = 604_800;
const CLASSES: [&str; 4] = ["IN", "CH", "HS", "ANY"];
/// Types forage parses; a wrong field count for one of these is an error,
/// not an "other" type.
const KNOWN_TYPES: [&str; 9] = ["A", "AAAA", "NS", "CNAME", "MX", "TXT", "SRV", "CAA", "SOA"];

/// A parsed zone file.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ZoneFile {
    pub entries: Vec<Entry>,
    /// Logical lines that did not parse, with the line they started on.
    pub skipped: Vec<SkippedLine>,
}

impl ZoneFile {
    /// The resource records, in file order.
    pub fn records(&self) -> impl Iterator<Item = &ResourceRecord> {
        self.entries.iter().filter_map(|e| match e {
            Entry::Record(rr) => Some(rr),
            _ => None,
        })
    }
}

/// A line that could not be parsed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkippedLine {
    pub line: usize,
    pub text: String,
}

/// A directive or a record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Entry {
    Ttl(u32),
    Origin(String),
    Include(String),
    Record(ResourceRecord),
}

/// One resource record. `name` is the owner as written (inherited for a
/// blank owner); `None` only when the very first record has a blank owner.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResourceRecord {
    pub name: Option<Name>,
    pub ttl: Option<u32>,
    pub class: Option<String>,
    pub rdata: RData,
}

/// A domain name as written in the file (relative, absolute or `@`).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Name(String);

impl Name {
    /// Wraps a name.
    pub fn new(name: impl Into<String>) -> Self {
        Name(name.into())
    }

    /// Whether this is `@`, the zone apex.
    pub fn is_at(&self) -> bool {
        self.0 == "@"
    }

    /// The name as written.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Record data for the types forage maps; anything else (PTR, HINFO, …) is
/// `Other(type)`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RData {
    A(Ipv4Addr),
    Aaaa(Ipv6Addr),
    Ns(Name),
    Cname(Name),
    Mx(MxData),
    Txt(Vec<String>),
    Srv(SrvData),
    Caa(CaaData),
    Soa(SoaData),
    Other(String),
}

impl RData {
    /// The record type mnemonic.
    pub fn rtype(&self) -> &str {
        match self {
            RData::A(_) => "A",
            RData::Aaaa(_) => "AAAA",
            RData::Ns(_) => "NS",
            RData::Cname(_) => "CNAME",
            RData::Mx(_) => "MX",
            RData::Txt(_) => "TXT",
            RData::Srv(_) => "SRV",
            RData::Caa(_) => "CAA",
            RData::Soa(_) => "SOA",
            RData::Other(rtype) => rtype,
        }
    }
}

/// MX data.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MxData {
    pub preference: u16,
    pub exchange: Name,
}

/// SRV data.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SrvData {
    pub priority: u16,
    pub weight: u16,
    pub port: u16,
    pub target: Name,
}

/// CAA data.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaaData {
    pub flags: u8,
    pub tag: String,
    pub value: String,
}

/// SOA data (timers in seconds).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SoaData {
    pub mname: Name,
    pub rname: Name,
    pub serial: u32,
    pub refresh: u32,
    pub retry: u32,
    pub expire: u32,
    pub minimum: u32,
}

/// Reads and parses a zone file.
///
/// # Errors
/// I/O failures, prefixed with the path. Syntax problems are not errors:
/// they are reported in [`ZoneFile::skipped`].
pub fn parse_zone_file_from_path(path: &Path) -> Result<ZoneFile> {
    let input = std::fs::read_to_string(path).with_context(|| path.display().to_string())?;
    Ok(parse_zone_file(&input))
}

/// Parses zone-file text.
pub fn parse_zone_file(input: &str) -> ZoneFile {
    let mut zone = ZoneFile::default();
    let mut owner: Option<Name> = None;
    for (line, tokens, complete) in logical_lines(input) {
        match complete.then(|| parse_entry(&tokens, &mut owner)).flatten() {
            Some(entry) => zone.entries.push(entry),
            None => zone.skipped.push(SkippedLine {
                line,
                text: display(&tokens),
            }),
        }
    }
    zone
}

/// A token and whether it was quoted (quoted tokens are never owners, TTLs,
/// classes or types).
type Token = (String, bool);

/// A logical line's tokens as text, for reporting a line that did not parse.
fn display(tokens: &[Token]) -> String {
    let words: Vec<String> = tokens
        .iter()
        .filter(|(t, quoted)| *quoted || !t.is_empty())
        .map(|(t, quoted)| {
            if *quoted {
                format!("\"{t}\"")
            } else {
                t.clone()
            }
        })
        .collect();
    words.join(" ")
}

/// Splits input into logical lines: (start line, tokens, complete). A leading
/// empty unquoted token marks a blank owner; `complete` is false when a quote
/// or parenthesis is never closed.
fn logical_lines(input: &str) -> Vec<(usize, Vec<Token>, bool)> {
    let mut lines = Vec::new();
    let mut chars = input.chars().peekable();
    let mut line_no = 1;
    while chars.peek().is_some() {
        let (start, mut depth, mut complete) = (line_no, 0_usize, true);
        let mut tokens: Vec<Token> = Vec::new();
        let mut current: Option<Token> = None;
        if chars.peek().is_some_and(|c| *c == ' ' || *c == '\t') {
            tokens.push((String::new(), false));
        }
        while let Some(c) = chars.next() {
            if c == '\n' {
                line_no += 1;
                if depth == 0 {
                    break;
                }
            }
            match c {
                ';' => while chars.next_if(|n| *n != '\n').is_some() {},
                '"' => {
                    let mut word = String::new();
                    loop {
                        match chars.next() {
                            Some('"') => break,
                            Some('\\') => word.extend(chars.next()),
                            Some('\n') | None => {
                                (complete, line_no) = (false, line_no + 1);
                                break;
                            }
                            Some(ch) => word.push(ch),
                        }
                    }
                    tokens.extend(current.take());
                    tokens.push((word, true));
                }
                c if c.is_whitespace() || c == '(' || c == ')' => {
                    depth = match c {
                        '(' => depth + 1,
                        ')' => depth.saturating_sub(1),
                        _ => depth,
                    };
                    tokens.extend(current.take());
                }
                c => current.get_or_insert_with(Default::default).0.push(c),
            }
            if !complete {
                break;
            }
        }
        tokens.extend(current.take());
        complete &= depth == 0;
        if !complete || tokens.iter().any(|(t, quoted)| *quoted || !t.is_empty()) {
            lines.push((start, tokens, complete));
        }
    }
    lines
}

/// Parses a TTL: seconds, or units `w d h m s` (any case, combinable).
pub fn parse_ttl(token: &str) -> Option<u32> {
    if token.is_empty() {
        return None;
    }
    if let Ok(secs) = token.parse::<u32>() {
        return Some(secs);
    }
    let (mut total, mut digits) = (0_u32, String::new());
    for c in token.chars() {
        if c.is_ascii_digit() {
            digits.push(c);
            continue;
        }
        let unit = match c.to_ascii_lowercase() {
            'w' => SECONDS_PER_WEEK,
            'd' => SECONDS_PER_DAY,
            'h' => SECONDS_PER_HOUR,
            'm' => SECONDS_PER_MINUTE,
            's' => 1,
            _ => return None,
        };
        total = total.checked_add(digits.parse::<u32>().ok()?.checked_mul(unit)?)?;
        digits.clear();
    }
    digits.is_empty().then_some(total)
}

fn parse_entry(tokens: &[Token], owner: &mut Option<Name>) -> Option<Entry> {
    let (first, _) = tokens.first()?;
    if let Some(directive) = first.strip_prefix('$') {
        let arg = &tokens.get(1)?.0;
        return match (directive.to_ascii_uppercase().as_str(), tokens.len()) {
            ("TTL", 2) => parse_ttl(arg).map(Entry::Ttl),
            ("ORIGIN", 2) => Some(Entry::Origin(arg.clone())),
            ("INCLUDE", _) => Some(Entry::Include(arg.clone())),
            _ => None,
        };
    }
    if !first.is_empty() {
        *owner = Some(Name::new(first.clone()));
    }
    let mut rest = &tokens[1..];
    let (mut ttl, mut class) = (None, None);
    while let Some(((word, false), tail)) = rest.split_first() {
        if ttl.is_none() && parse_ttl(word).is_some() {
            ttl = parse_ttl(word);
        } else if class.is_none() && CLASSES.contains(&word.to_ascii_uppercase().as_str()) {
            class = Some(word.to_ascii_uppercase());
        } else {
            break;
        }
        rest = tail;
    }
    let ((rtype, false), data) = rest.split_first()? else {
        return None;
    };
    let rdata = parse_rdata(&rtype.to_ascii_uppercase(), data)?;
    Some(Entry::Record(ResourceRecord {
        name: owner.clone(),
        ttl,
        class,
        rdata,
    }))
}

fn parse_rdata(rtype: &str, data: &[Token]) -> Option<RData> {
    let words: Vec<&str> = data.iter().map(|(t, _)| t.as_str()).collect();
    let name = |i: usize| words.get(i).map(|w| Name::new(*w));
    let num = |i: usize| words.get(i)?.parse::<u16>().ok();
    Some(match (rtype, words.len()) {
        ("A", 1) => RData::A(words[0].parse().ok()?),
        ("AAAA", 1) => RData::Aaaa(words[0].parse().ok()?),
        ("NS", 1) => RData::Ns(name(0)?),
        ("CNAME", 1) => RData::Cname(name(0)?),
        ("MX", 2) => RData::Mx(MxData {
            preference: num(0)?,
            exchange: name(1)?,
        }),
        ("TXT", n) if n > 0 => RData::Txt(words.iter().map(|w| (*w).to_string()).collect()),
        ("SRV", 4) => RData::Srv(SrvData {
            priority: num(0)?,
            weight: num(1)?,
            port: num(2)?,
            target: name(3)?,
        }),
        ("CAA", 3) => RData::Caa(CaaData {
            flags: words[0].parse().ok()?,
            tag: words[1].to_string(),
            value: words[2].to_string(),
        }),
        ("SOA", 7) => {
            let mut timers = words[2..].iter().map(|w| parse_ttl(w));
            let mut timer = || timers.next().flatten();
            RData::Soa(SoaData {
                mname: name(0)?,
                rname: name(1)?,
                serial: timer()?,
                refresh: timer()?,
                retry: timer()?,
                expire: timer()?,
                minimum: timer()?,
            })
        }
        (other, _)
            if !KNOWN_TYPES.contains(&other)
                && other.starts_with(|c: char| c.is_ascii_alphabetic())
                && other.chars().all(|c| c.is_ascii_alphanumeric()) =>
        {
            RData::Other(other.to_string())
        }
        _ => return None,
    })
}

#[cfg(test)]
#[path = "zone_file_tests.rs"]
mod zone_file_tests;
