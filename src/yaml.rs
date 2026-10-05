// Copyright (c) 2025 Erick Bourgeois, firestoned
// SPDX-License-Identifier: Apache-2.0

//! Block-style YAML writer for [`Value`] trees (ADR-0006: replaces
//! `serde_yaml`). Layout and scalar quoting follow `serde_yaml` 0.9 (libyaml,
//! unlimited width), so output is byte-identical for forage's manifests.
//! Multi-line strings are written double-quoted rather than as literal
//! blocks; DNS data never contains newlines.

use std::collections::BTreeMap;
use std::fmt::Write as _;

use crate::json::Value;

const INDENT_STEP: usize = 2;
const HEX_RADIX: u32 = 16;
const OCTAL_RADIX: u32 = 8;
const BINARY_RADIX: u32 = 2;
/// Characters that may not start a plain scalar.
const LEADING_INDICATORS: &str = "#,[]{}&*!|>'\"%@`";

/// Serializes `value` as a YAML document body (no `---` marker).
pub fn to_yaml(value: &Value) -> String {
    let mut out = String::new();
    match value {
        Value::Object(map) if !map.is_empty() => write_map(&mut out, map, 0, false),
        Value::Array(items) if !items.is_empty() => write_seq(&mut out, items, 0, false),
        _ => {
            out.push_str(&inline(value));
            out.push('\n');
        }
    }
    out
}

/// A scalar or empty container, as written after `key: ` or `- `.
fn inline(value: &Value) -> String {
    match value {
        #[cfg(test)]
        Value::Null => "null".into(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => n.to_string(),
        Value::String(s) => scalar(s),
        Value::Array(_) => "[]".into(),
        Value::Object(_) => "{}".into(),
    }
}

/// Writes a non-empty mapping at `indent`. With `inline_first`, the first key
/// continues the current line (it follows a `- `).
fn write_map(out: &mut String, map: &BTreeMap<String, Value>, indent: usize, inline_first: bool) {
    for (i, (key, item)) in map.iter().enumerate() {
        if i > 0 || !inline_first {
            out.push_str(&" ".repeat(indent));
        }
        out.push_str(&scalar(key));
        out.push(':');
        match item {
            Value::Object(nested) if !nested.is_empty() => {
                out.push('\n');
                write_map(out, nested, indent + INDENT_STEP, false);
            }
            Value::Array(items) if !items.is_empty() => {
                out.push('\n');
                write_seq(out, items, indent, false);
            }
            _ => {
                let _ = writeln!(out, " {}", inline(item));
            }
        }
    }
}

/// Writes a non-empty sequence with its dashes at `indent`.
fn write_seq(out: &mut String, items: &[Value], indent: usize, inline_first: bool) {
    for (i, item) in items.iter().enumerate() {
        if i > 0 || !inline_first {
            out.push_str(&" ".repeat(indent));
        }
        out.push_str("- ");
        match item {
            Value::Object(nested) if !nested.is_empty() => {
                write_map(out, nested, indent + INDENT_STEP, true);
            }
            Value::Array(nested) if !nested.is_empty() => {
                write_seq(out, nested, indent + INDENT_STEP, true);
            }
            _ => {
                out.push_str(&inline(item));
                out.push('\n');
            }
        }
    }
}

/// A string scalar: plain when libyaml would allow it and it does not read as
/// another type; single-quoted when it only needs protecting; double-quoted
/// when it holds characters single quotes cannot carry.
pub fn scalar(s: &str) -> String {
    if s.chars()
        .any(|c| c == '\n' || c == '\r' || c == '\t' || c.is_control())
    {
        return double_quoted(s);
    }
    if resolves_to_non_string(s) || !plain_allowed(s) {
        return format!("'{}'", s.replace('\'', "''"));
    }
    s.to_string()
}

fn double_quoted(s: &str) -> String {
    let mut out = String::from("\"");
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c.is_control() => {
                let _ = write!(out, "\\x{:02X}", u32::from(c));
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// libyaml's block-context plain-scalar test (single-line strings).
fn plain_allowed(s: &str) -> bool {
    if s.starts_with(' ') || s.ends_with(' ') || s.starts_with("---") || s.starts_with("...") {
        return false;
    }
    let chars: Vec<char> = s.chars().collect();
    for (i, &c) in chars.iter().enumerate() {
        let followed_by_blank = chars.get(i + 1).map_or(true, |n| *n == ' ');
        let indicator = if i == 0 {
            LEADING_INDICATORS.contains(c) || (matches!(c, '?' | ':' | '-') && followed_by_blank)
        } else {
            (c == ':' && followed_by_blank) || (c == '#' && chars[i - 1] == ' ')
        };
        if indicator {
            return false;
        }
    }
    true
}

/// `serde_yaml` quotes strings a YAML 1.2 reader would take as null, bool,
/// int or float, and digit strings with a leading zero.
fn resolves_to_non_string(s: &str) -> bool {
    if matches!(
        s,
        "" | "~"
            | "null"
            | "Null"
            | "NULL"
            | "true"
            | "True"
            | "TRUE"
            | "false"
            | "False"
            | "FALSE"
    ) {
        return true;
    }
    let unsigned = s.strip_prefix(['-', '+']).unwrap_or(s);
    let leading_zero_digits = unsigned.len() > 1
        && unsigned.starts_with('0')
        && unsigned[1..].bytes().all(|b| b.is_ascii_digit());
    leading_zero_digits || is_int(s) || is_float(s)
}

fn is_int(s: &str) -> bool {
    let body = s.strip_prefix(['+', '-']).unwrap_or(s);
    if body.starts_with(['+', '-']) {
        return false;
    }
    for (prefix, radix) in [("0x", HEX_RADIX), ("0o", OCTAL_RADIX), ("0b", BINARY_RADIX)] {
        if let Some(digits) = body.strip_prefix(prefix) {
            return !digits.starts_with(['+', '-']) && u64::from_str_radix(digits, radix).is_ok();
        }
    }
    s.parse::<i128>().is_ok()
}

fn is_float(s: &str) -> bool {
    let unpositive = s.strip_prefix('+').unwrap_or(s);
    if s.starts_with('+') && unpositive.starts_with(['+', '-']) {
        return false;
    }
    matches!(unpositive, ".inf" | ".Inf" | ".INF")
        || matches!(s, "-.inf" | "-.Inf" | "-.INF" | ".nan" | ".NaN" | ".NAN")
        || unpositive.parse::<f64>().is_ok_and(f64::is_finite)
}

#[cfg(test)]
#[path = "yaml_tests.rs"]
mod yaml_tests;
