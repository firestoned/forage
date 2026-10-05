// Copyright (c) 2025 Erick Bourgeois, firestoned
// SPDX-License-Identifier: Apache-2.0

//! Helpers for the integration tests (ADR-0006: replace the `tempfile`,
//! `serde_json` and `serde_yaml` dev-dependencies). A minimal JSON reader is
//! enough: forage's JSON output is a stream of objects, strings, integers,
//! booleans, nulls and arrays.

#![allow(dead_code)]

use std::collections::BTreeMap;
use std::ops::Index;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT_DIR: AtomicUsize = AtomicUsize::new(0);

/// A temp directory removed on drop.
pub struct TempDir(PathBuf);

impl TempDir {
    pub fn new() -> Self {
        let n = NEXT_DIR.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!("forage-it-{}-{n}", std::process::id()));
        std::fs::create_dir_all(&path).expect("create temp dir");
        Self(path)
    }

    pub fn path(&self) -> &Path {
        &self.0
    }

    pub fn write(&self, name: &str, content: &str) -> PathBuf {
        let path = self.0.join(name);
        std::fs::write(&path, content).expect("write file");
        path
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// A parsed JSON value.
#[derive(Debug, Clone, PartialEq)]
pub enum Json {
    Null,
    Bool(bool),
    Number(i64),
    String(String),
    Array(Vec<Json>),
    Object(BTreeMap<String, Json>),
}

static NULL: Json = Json::Null;

impl Json {
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Json::String(s) => Some(s),
            _ => None,
        }
    }

    pub fn get(&self, key: &str) -> Option<&Json> {
        match self {
            Json::Object(map) => map.get(key),
            _ => None,
        }
    }
}

impl Index<&str> for Json {
    type Output = Json;
    fn index(&self, key: &str) -> &Json {
        self.get(key).unwrap_or(&NULL)
    }
}

impl Index<usize> for Json {
    type Output = Json;
    fn index(&self, i: usize) -> &Json {
        match self {
            Json::Array(items) => items.get(i).unwrap_or(&NULL),
            _ => &NULL,
        }
    }
}

impl PartialEq<&str> for Json {
    fn eq(&self, other: &&str) -> bool {
        self.as_str() == Some(*other)
    }
}

impl PartialEq<i64> for Json {
    fn eq(&self, other: &i64) -> bool {
        matches!(self, Json::Number(n) if n == other)
    }
}

/// Builds a string array, for comparisons.
pub fn strings(items: &[&str]) -> Json {
    Json::Array(
        items
            .iter()
            .map(|s| Json::String((*s).to_string()))
            .collect(),
    )
}

/// Parses a stream of concatenated JSON values.
pub fn parse_stream(input: &str) -> Vec<Json> {
    let mut chars: Vec<char> = input.chars().collect();
    chars.push('\0');
    let mut pos = 0;
    let mut values = Vec::new();
    skip_ws(&chars, &mut pos);
    while chars[pos] != '\0' {
        values.push(value(&chars, &mut pos));
        skip_ws(&chars, &mut pos);
    }
    values
}

fn skip_ws(c: &[char], pos: &mut usize) {
    while c[*pos].is_whitespace() {
        *pos += 1;
    }
}

fn expect(c: &[char], pos: &mut usize, ch: char) {
    assert_eq!(c[*pos], ch, "invalid JSON at {pos}");
    *pos += 1;
}

fn value(c: &[char], pos: &mut usize) -> Json {
    skip_ws(c, pos);
    match c[*pos] {
        '{' => {
            *pos += 1;
            let mut map = BTreeMap::new();
            skip_ws(c, pos);
            while c[*pos] != '}' {
                skip_ws(c, pos);
                let Json::String(key) = value(c, pos) else {
                    panic!("object key must be a string")
                };
                skip_ws(c, pos);
                expect(c, pos, ':');
                map.insert(key, value(c, pos));
                skip_ws(c, pos);
                if c[*pos] == ',' {
                    *pos += 1;
                }
            }
            *pos += 1;
            Json::Object(map)
        }
        '[' => {
            *pos += 1;
            let mut items = Vec::new();
            skip_ws(c, pos);
            while c[*pos] != ']' {
                items.push(value(c, pos));
                skip_ws(c, pos);
                if c[*pos] == ',' {
                    *pos += 1;
                }
                skip_ws(c, pos);
            }
            *pos += 1;
            Json::Array(items)
        }
        '"' => {
            *pos += 1;
            let mut s = String::new();
            while c[*pos] != '"' {
                if c[*pos] == '\\' {
                    *pos += 1;
                    s.push(match c[*pos] {
                        'n' => '\n',
                        't' => '\t',
                        'r' => '\r',
                        other => other,
                    });
                } else {
                    s.push(c[*pos]);
                }
                *pos += 1;
            }
            *pos += 1;
            Json::String(s)
        }
        't' | 'f' | 'n' => {
            let word: String = c[*pos..]
                .iter()
                .take_while(|ch| ch.is_ascii_alphabetic())
                .collect();
            *pos += word.len();
            match word.as_str() {
                "true" => Json::Bool(true),
                "false" => Json::Bool(false),
                _ => Json::Null,
            }
        }
        _ => {
            let num: String = c[*pos..]
                .iter()
                .take_while(|ch| ch.is_ascii_digit() || **ch == '-')
                .collect();
            *pos += num.len();
            Json::Number(num.parse().expect("integer"))
        }
    }
}
