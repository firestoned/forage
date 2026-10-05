// Copyright (c) 2025 Erick Bourgeois, firestoned
// SPDX-License-Identifier: Apache-2.0

//! Leveled logging to stderr (ADR-0006: replaces `tracing` and
//! `tracing-subscriber`). Lines look like
//! `2026-10-05T10:33:18Z  WARN forage::mapper: message`.

use std::sync::atomic::{AtomicU8, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

const SECONDS_PER_MINUTE: u64 = 60;
const SECONDS_PER_HOUR: u64 = 3_600;
const SECONDS_PER_DAY: u64 = 86_400;
// Civil-from-days constants (Howard Hinnant's algorithm, proleptic Gregorian).
const DAYS_TO_0000_03_01: i64 = 719_468;
const DAYS_PER_ERA: i64 = 146_097;
const YEARS_PER_ERA: i64 = 400;
const DAYS_PER_4_YEARS: i64 = 1_460;
const DAYS_PER_100_YEARS: i64 = 36_524;
const DAYS_PER_YEAR: i64 = 365;
const MONTH_DAYS_NUMERATOR: i64 = 153;
const MONTH_OFFSET: i64 = 2;
const MONTH_DAYS_DENOMINATOR: i64 = 5;
const MARCH_BASED_SHIFT: i64 = 3;
const MONTHS_PER_YEAR: i64 = 12;
const LAST_MONTH_BEFORE_JANUARY: i64 = 10;

/// Log levels, most severe first. `Off` disables everything.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Level {
    Off,
    Error,
    Warn,
    Info,
    Debug,
    Trace,
}

impl Level {
    /// The level's name as printed in log lines.
    pub fn label(self) -> &'static str {
        match self {
            Level::Off => "OFF",
            Level::Error => "ERROR",
            Level::Warn => "WARN",
            Level::Info => "INFO",
            Level::Debug => "DEBUG",
            Level::Trace => "TRACE",
        }
    }

    fn parse(name: &str) -> Option<Self> {
        [
            Level::Off,
            Level::Error,
            Level::Warn,
            Level::Info,
            Level::Debug,
            Level::Trace,
        ]
        .into_iter()
        .find(|l| l.label().eq_ignore_ascii_case(name.trim()))
    }
}

static MAX_LEVEL: AtomicU8 = AtomicU8::new(Level::Off as u8);

/// The effective level: `--debug` wins; otherwise `RUST_LOG`, where a bare
/// level or a `forage…=level` directive applies (last one wins); else `Warn`.
pub fn level_from(debug: bool, rust_log: Option<&str>) -> Level {
    if debug {
        return Level::Debug;
    }
    let mut level = Level::Warn;
    for directive in rust_log.unwrap_or_default().split(',') {
        let parsed = match directive.split_once('=') {
            Some((target, name)) if target.trim().starts_with("forage") => Level::parse(name),
            Some(_) => None,
            None => Level::parse(directive),
        };
        level = parsed.unwrap_or(level);
    }
    level
}

/// Sets the maximum level that is printed.
pub fn set_level(level: Level) {
    MAX_LEVEL.store(level as u8, Ordering::Relaxed);
}

/// Whether a message at `level` would be printed.
pub fn enabled(level: Level) -> bool {
    level != Level::Off && level as u8 <= MAX_LEVEL.load(Ordering::Relaxed)
}

/// Prints one log line to stderr, stamped with the current UTC time.
pub fn write(level: Level, target: &str, message: std::fmt::Arguments<'_>) {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());
    eprintln!("{}", format_line(now, level, target, &message.to_string()));
}

/// Formats one log line (the testable part of [`write`]).
pub fn format_line(epoch_secs: u64, level: Level, target: &str, message: &str) -> String {
    format!(
        "{} {:>5} {target}: {message}",
        utc_timestamp(epoch_secs),
        level.label()
    )
}

/// `YYYY-MM-DDTHH:MM:SSZ` for a Unix timestamp.
pub fn utc_timestamp(epoch_secs: u64) -> String {
    let secs_of_day = epoch_secs % SECONDS_PER_DAY;
    let days = i64::try_from(epoch_secs / SECONDS_PER_DAY).unwrap_or(0) + DAYS_TO_0000_03_01;
    let era = days.div_euclid(DAYS_PER_ERA);
    let day_of_era = days - era * DAYS_PER_ERA;
    let year_of_era = (day_of_era - day_of_era / DAYS_PER_4_YEARS
        + day_of_era / DAYS_PER_100_YEARS
        - day_of_era / (DAYS_PER_ERA - 1))
        / DAYS_PER_YEAR;
    let day_of_year =
        day_of_era - (DAYS_PER_YEAR * year_of_era + year_of_era / 4 - year_of_era / 100);
    let mp = (MONTH_DAYS_DENOMINATOR * day_of_year + MONTH_OFFSET) / MONTH_DAYS_NUMERATOR;
    let day = day_of_year - (MONTH_DAYS_NUMERATOR * mp + MONTH_OFFSET) / MONTH_DAYS_DENOMINATOR + 1;
    let month = if mp < LAST_MONTH_BEFORE_JANUARY {
        mp + MARCH_BASED_SHIFT
    } else {
        mp + MARCH_BASED_SHIFT - MONTHS_PER_YEAR
    };
    let year = year_of_era + era * YEARS_PER_ERA + i64::from(month <= 2);
    format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}Z",
        secs_of_day / SECONDS_PER_HOUR,
        secs_of_day % SECONDS_PER_HOUR / SECONDS_PER_MINUTE,
        secs_of_day % SECONDS_PER_MINUTE
    )
}

/// Logs at `Warn` from the calling module.
macro_rules! warn {
    ($($arg:tt)*) => { $crate::log::emit($crate::log::Level::Warn, module_path!(), format_args!($($arg)*)) };
}

/// Logs at `Info` from the calling module.
macro_rules! info {
    ($($arg:tt)*) => { $crate::log::emit($crate::log::Level::Info, module_path!(), format_args!($($arg)*)) };
}

/// Logs at `Debug` from the calling module.
macro_rules! debug {
    ($($arg:tt)*) => { $crate::log::emit($crate::log::Level::Debug, module_path!(), format_args!($($arg)*)) };
}

/// Prints `message` when `level` is enabled. The macros call this so the
/// level check is one function, not repeated at every call site.
pub fn emit(level: Level, target: &str, message: std::fmt::Arguments<'_>) {
    if enabled(level) {
        write(level, target, message);
    }
}

#[cfg(test)]
#[path = "log_tests.rs"]
mod log_tests;
