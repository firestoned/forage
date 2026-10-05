// Copyright (c) 2025 Erick Bourgeois, firestoned
// SPDX-License-Identifier: Apache-2.0

#[cfg(test)]
mod tests {
    use super::super::*;

    #[test]
    fn test_debug_flag_wins_over_rust_log() {
        assert_eq!(level_from(true, Some("error")), Level::Debug);
    }

    #[test]
    fn test_default_level_is_warn() {
        assert_eq!(level_from(false, None), Level::Warn);
        assert_eq!(level_from(false, Some("")), Level::Warn);
    }

    #[test]
    fn test_rust_log_bare_levels() {
        for (spec, level) in [
            ("error", Level::Error),
            ("WARN", Level::Warn),
            ("info", Level::Info),
            ("debug", Level::Debug),
            ("trace", Level::Trace),
            ("off", Level::Off),
        ] {
            assert_eq!(level_from(false, Some(spec)), level, "{spec}");
        }
    }

    #[test]
    fn test_rust_log_directives_for_forage_and_others() {
        assert_eq!(level_from(false, Some("forage=debug")), Level::Debug);
        assert_eq!(
            level_from(false, Some("warn,forage::mapper=info")),
            Level::Info
        );
        assert_eq!(level_from(false, Some("info,other=trace")), Level::Info);
        assert_eq!(level_from(false, Some("bogus")), Level::Warn);
    }

    #[test]
    fn test_utc_timestamp_formats_epoch_and_leap_day() {
        assert_eq!(utc_timestamp(0), "1970-01-01T00:00:00Z");
        assert_eq!(utc_timestamp(951_782_400), "2000-02-29T00:00:00Z");
        assert_eq!(utc_timestamp(1_791_196_398), "2026-10-05T10:33:18Z");
    }

    #[test]
    fn test_format_line_shape() {
        assert_eq!(
            format_line(0, Level::Warn, "forage::mapper", "zone skipped"),
            "1970-01-01T00:00:00Z  WARN forage::mapper: zone skipped"
        );
        assert_eq!(
            format_line(0, Level::Debug, "forage", "x"),
            "1970-01-01T00:00:00Z DEBUG forage: x"
        );
    }

    #[test]
    fn test_enabled_follows_the_set_level() {
        set_level(Level::Info);
        assert!(enabled(Level::Warn));
        assert!(enabled(Level::Info));
        assert!(!enabled(Level::Debug));
        set_level(Level::Off);
        assert!(!enabled(Level::Error));
    }

    #[test]
    fn test_level_labels() {
        let labels: Vec<&str> = [
            Level::Error,
            Level::Warn,
            Level::Info,
            Level::Debug,
            Level::Trace,
            Level::Off,
        ]
        .iter()
        .map(|l| l.label())
        .collect();
        assert_eq!(
            labels,
            vec!["ERROR", "WARN", "INFO", "DEBUG", "TRACE", "OFF"]
        );
    }
}
