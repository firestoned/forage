// Copyright (c) 2025 Erick Bourgeois, firestoned
// SPDX-License-Identifier: Apache-2.0

//! JSON value tree and pretty writer (ADR-0006: replaces `serde` and
//! `serde_json`). Object keys are sorted, and the pretty layout is the one
//! `serde_json::to_string_pretty` produced, so output is byte-identical.

use std::collections::BTreeMap;
use std::fmt::Write as _;
#[cfg(test)]
use std::ops::Index;

const INDENT: &str = "  ";

/// A JSON value. Objects keep keys sorted.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    /// Tests only: what indexing a missing key yields. forage never writes
    /// `null` (absent options are omitted).
    #[cfg(test)]
    Null,
    Bool(bool),
    Number(i64),
    String(String),
    Array(Vec<Value>),
    Object(BTreeMap<String, Value>),
}

#[cfg(test)]
static NULL: Value = Value::Null;

impl Value {
    /// An empty object.
    pub fn object() -> Self {
        Value::Object(BTreeMap::new())
    }

    /// Sets `key` on an object; ignored on any other value.
    pub fn insert(&mut self, key: &str, value: Value) {
        if let Value::Object(map) = self {
            map.insert(key.to_string(), value);
        }
    }

    /// Builder form of [`Value::insert`].
    #[must_use]
    pub fn with(mut self, key: &str, value: impl Into<Value>) -> Self {
        self.insert(key, value.into());
        self
    }

    /// Sets `key` only when `value` is `Some` (serde's `skip_serializing_if = "Option::is_none"`).
    #[must_use]
    pub fn with_opt<T: Into<Value>>(self, key: &str, value: Option<T>) -> Self {
        match value {
            Some(v) => self.with(key, v),
            None => self,
        }
    }

    /// Sets `key` only when `value` is not an empty array or object.
    #[must_use]
    pub fn with_nonempty(self, key: &str, value: Value) -> Self {
        let empty = match &value {
            Value::Array(items) => items.is_empty(),
            Value::Object(map) => map.is_empty(),
            _ => false,
        };
        if empty {
            return self;
        }
        self.with(key, value)
    }

    /// Pretty JSON with two-space indentation.
    pub fn to_json_pretty(&self) -> String {
        let mut out = String::new();
        write_pretty(&mut out, self, 0);
        out
    }
}

/// Inspection API used by tests only: production builds values and prints
/// them, it never reads them back.
#[cfg(test)]
impl Value {
    /// The member `key` of an object.
    pub fn get(&self, key: &str) -> Option<&Value> {
        self.as_object().and_then(|map| map.get(key))
    }

    /// The string, if this is one.
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Value::String(s) => Some(s),
            _ => None,
        }
    }

    /// The number, if this is one.
    pub fn as_i64(&self) -> Option<i64> {
        match self {
            Value::Number(n) => Some(*n),
            _ => None,
        }
    }

    /// The items, if this is an array.
    pub fn as_array(&self) -> Option<&Vec<Value>> {
        match self {
            Value::Array(items) => Some(items),
            _ => None,
        }
    }

    /// The members, if this is an object.
    pub fn as_object(&self) -> Option<&BTreeMap<String, Value>> {
        match self {
            Value::Object(map) => Some(map),
            _ => None,
        }
    }
}

fn write_pretty(out: &mut String, value: &Value, depth: usize) {
    match value {
        #[cfg(test)]
        Value::Null => out.push_str("null"),
        Value::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
        Value::Number(n) => out.push_str(&n.to_string()),
        Value::String(s) => write_string(out, s),
        Value::Array(items) if items.is_empty() => out.push_str("[]"),
        Value::Object(map) if map.is_empty() => out.push_str("{}"),
        Value::Array(items) => {
            out.push('[');
            for (i, item) in items.iter().enumerate() {
                out.push_str(if i == 0 { "\n" } else { ",\n" });
                out.push_str(&INDENT.repeat(depth + 1));
                write_pretty(out, item, depth + 1);
            }
            out.push('\n');
            out.push_str(&INDENT.repeat(depth));
            out.push(']');
        }
        Value::Object(map) => {
            out.push('{');
            for (i, (key, item)) in map.iter().enumerate() {
                out.push_str(if i == 0 { "\n" } else { ",\n" });
                out.push_str(&INDENT.repeat(depth + 1));
                write_string(out, key);
                out.push_str(": ");
                write_pretty(out, item, depth + 1);
            }
            out.push('\n');
            out.push_str(&INDENT.repeat(depth));
            out.push('}');
        }
    }
}

/// Writes a JSON string literal with `serde_json`'s escaping.
fn write_string(out: &mut String, s: &str) {
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{8}' => out.push_str("\\b"),
            '\u{c}' => out.push_str("\\f"),
            c if u32::from(c) < 0x20 => {
                let _ = write!(out, "\\u{:04x}", u32::from(c));
            }
            c => out.push(c),
        }
    }
    out.push('"');
}

#[cfg(test)]
impl Index<&str> for Value {
    type Output = Value;
    fn index(&self, key: &str) -> &Value {
        self.get(key).unwrap_or(&NULL)
    }
}

#[cfg(test)]
impl Index<usize> for Value {
    type Output = Value;
    fn index(&self, i: usize) -> &Value {
        self.as_array()
            .and_then(|items| items.get(i))
            .unwrap_or(&NULL)
    }
}

impl From<&str> for Value {
    fn from(s: &str) -> Self {
        Value::String(s.to_string())
    }
}

impl From<String> for Value {
    fn from(s: String) -> Self {
        Value::String(s)
    }
}

impl From<bool> for Value {
    fn from(b: bool) -> Self {
        Value::Bool(b)
    }
}

impl From<i64> for Value {
    fn from(n: i64) -> Self {
        Value::Number(n)
    }
}

impl From<i32> for Value {
    fn from(n: i32) -> Self {
        Value::Number(n.into())
    }
}

impl From<u32> for Value {
    fn from(n: u32) -> Self {
        Value::Number(n.into())
    }
}

impl From<u16> for Value {
    fn from(n: u16) -> Self {
        Value::Number(n.into())
    }
}

impl From<u8> for Value {
    fn from(n: u8) -> Self {
        Value::Number(n.into())
    }
}

impl<T: Into<Value>> From<Vec<T>> for Value {
    fn from(items: Vec<T>) -> Self {
        Value::Array(items.into_iter().map(Into::into).collect())
    }
}

impl From<BTreeMap<String, String>> for Value {
    fn from(map: BTreeMap<String, String>) -> Self {
        Value::Object(
            map.into_iter()
                .map(|(k, v)| (k, Value::String(v)))
                .collect(),
        )
    }
}

#[cfg(test)]
impl PartialEq<&str> for Value {
    fn eq(&self, other: &&str) -> bool {
        self.as_str() == Some(*other)
    }
}

#[cfg(test)]
impl PartialEq<i64> for Value {
    fn eq(&self, other: &i64) -> bool {
        self.as_i64() == Some(*other)
    }
}

#[cfg(test)]
impl PartialEq<i32> for Value {
    fn eq(&self, other: &i32) -> bool {
        self.as_i64() == Some(i64::from(*other))
    }
}

#[cfg(test)]
#[path = "json_tests.rs"]
mod json_tests;
