// Copyright (c) 2025 Erick Bourgeois, firestoned
// SPDX-License-Identifier: Apache-2.0

#[cfg(test)]
mod tests {
    use super::super::*;

    #[test]
    fn test_error_displays_its_message() {
        assert_eq!(Error::new("boom").to_string(), "boom");
    }

    #[test]
    fn test_io_error_converts_with_its_message() {
        let io = std::io::Error::new(std::io::ErrorKind::NotFound, "gone");
        assert_eq!(Error::from(io).to_string(), "gone");
    }

    #[test]
    fn test_with_context_prefixes_the_cause() {
        let failed: std::result::Result<(), Error> = Err(Error::new("root cause"));
        let err = failed
            .with_context(|| "reading named.conf".to_string())
            .unwrap_err();
        assert_eq!(err.to_string(), "reading named.conf: root cause");
    }

    #[test]
    fn test_with_context_leaves_ok_untouched() {
        let ok: std::result::Result<u8, Error> = Ok(7);
        assert_eq!(ok.with_context(|| unreachable!()).unwrap(), 7);
    }

    #[test]
    fn test_error_is_a_std_error() {
        let err: Box<dyn std::error::Error> = Box::new(Error::new("x"));
        assert_eq!(err.to_string(), "x");
    }
}
