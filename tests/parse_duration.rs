//! Tests for `fadetop::ser::parse_duration`.
//!
//! These tests pin down the *current* behavior of the helper, including the
//! cases the implementation handles poorly. Tests marked `#[should_panic]`
//! document fragility surfaced by the code review and should be flipped to
//! ordinary error assertions if/when the function is hardened.

use fadetop::ser::parse_duration;
use serde::de::IntoDeserializer;
use std::time::Duration;

type De = serde::de::value::StringDeserializer<serde::de::value::Error>;

fn parse(s: &str) -> Result<Duration, serde::de::value::Error> {
    let de: De = s.to_owned().into_deserializer();
    parse_duration(de)
}

#[test]
fn parses_seconds() {
    assert_eq!(parse("10s").unwrap(), Duration::from_secs(10));
    assert_eq!(parse("0s").unwrap(), Duration::from_secs(0));
}

#[test]
fn parses_minutes() {
    assert_eq!(parse("2m").unwrap(), Duration::from_secs(120));
}

#[test]
fn parses_hours() {
    assert_eq!(parse("1h").unwrap(), Duration::from_secs(3600));
    assert_eq!(parse("24h").unwrap(), Duration::from_secs(86_400));
}

#[test]
fn rejects_unknown_unit_letter() {
    let err = parse("100x").unwrap_err();
    assert!(format!("{}", err).contains("invalid duration unit"));
}

#[test]
fn rejects_non_numeric_value() {
    let err = parse("abcs").unwrap_err();
    assert!(format!("{}", err).contains("invalid unsigned integer"));
}

/// "100ms" is a natural input but not supported: only the trailing 's' is
/// taken as the unit, so the value string becomes "100m" and fails to parse.
/// The error message is misleading ("invalid unsigned integer") rather than
/// pointing at the unit. Locked-in here so the regression is visible.
#[test]
fn millisecond_suffix_is_silently_unsupported() {
    let err = parse("100ms").unwrap_err();
    let msg = format!("{}", err);
    assert!(
        msg.contains("invalid unsigned integer"),
        "unexpected error message: {msg}"
    );
}

/// Empty input panics due to `s.len() - 1` underflow on `usize`.
#[test]
#[should_panic]
fn empty_input_panics() {
    let _ = parse("");
}

/// Non-ASCII trailing byte panics: `&s[s.len()-1..]` is not a char boundary.
#[test]
#[should_panic]
fn non_ascii_trailing_char_panics() {
    let _ = parse("10ñ");
}
