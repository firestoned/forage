// Copyright (c) 2025 Erick Bourgeois, firestoned
// SPDX-License-Identifier: Apache-2.0

//! Error type with context chaining (ADR-0006: replaces `anyhow`).

use std::fmt;

/// An error carrying a human-readable message; context is prepended as
/// `context: cause`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Error {
    message: String,
}

impl Error {
    /// Creates an error from a message.
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for Error {}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Self::new(err.to_string())
    }
}

/// Result type used throughout forage.
pub type Result<T> = std::result::Result<T, Error>;

/// Adds context to an error, as `anyhow::Context::with_context` did.
pub trait Context<T> {
    /// Wraps the error as `"{context}: {cause}"`; `context` runs only on error.
    ///
    /// # Errors
    /// Returns the original error with the context prepended.
    fn with_context<F: FnOnce() -> String>(self, context: F) -> Result<T>;
}

impl<T, E: fmt::Display> Context<T> for std::result::Result<T, E> {
    fn with_context<F: FnOnce() -> String>(self, context: F) -> Result<T> {
        self.map_err(|err| Error::new(format!("{}: {err}", context())))
    }
}

#[cfg(test)]
#[path = "error_tests.rs"]
mod error_tests;
