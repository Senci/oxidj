//! Integration tests for the oxidj-core public API.
//!
//! These exercise domain rules through the crate's *public* surface — the same
//! surface the CLI and later the GUI/adapters will consume. See
//! `docs/testing.md` for the full test strategy (fixtures, persistence,
//! semantic round-trips, fuzz/perf).

use oxidj_core::{TrackId, TrackIdError};
use std::str::FromStr;

#[test]
fn rejects_the_reserved_zero_identity() {
    assert_eq!(TrackId::new(0), Err(TrackIdError::Zero));
}

#[test]
fn the_minimal_identity_is_track_id_min() {
    assert_eq!(TrackId::new(TrackId::MIN), Ok(TrackId::new(1).unwrap()));
}

#[test]
fn zero_rejected_when_parsed_from_string() {
    assert_eq!("0".parse::<TrackId>().unwrap_err(), TrackIdError::Zero);
}

#[test]
fn malformed_and_overflow_inputs_are_not_zero_errors() {
    for input in ["not-a-number", "18446744073709551616"] {
        assert_ne!(input.parse::<TrackId>().unwrap_err(), TrackIdError::Zero);
    }
}

#[test]
fn display_then_parse_is_identity_roundtrip() {
    let id = TrackId::new(9_999_999).unwrap();
    let rendered = id.to_string();
    let reparsed = TrackId::from_str(&rendered).unwrap();
    assert_eq!(reparsed, id);
}

#[test]
fn ordering_is_total_and_unsigned() {
    let lo = TrackId::new(1).unwrap();
    let hi = TrackId::new(2).unwrap();
    assert!(lo < hi);
}
