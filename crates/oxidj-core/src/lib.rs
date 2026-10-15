//! OxiDJ core: the neutral domain model and rules.
//!
//! M0 scope (ADR-004): domain identities and rules only. Storage, audio,
//! vendor formats and UI live in higher crates; core depends on none of them,
//! which is enforced by the `cargo xtask layers` gate.
//!
//! The types here are minimal stand-ins so the workspace + gate can be proven
//! without committing M1's full schema. Each carries a real invariant and real
//! tests (unit, integration, and doctest). Serialization lives in storage /
//! adapter crates, not here.

use core::fmt;
use core::str::FromStr;

/// A domain-unique identifier for a track.
///
/// `0` is reserved (it denotes "no track"); a `TrackId` can never hold it. The
/// invariant is therefore enforced at construction, so an invalid identity
/// cannot exist at rest.
///
/// ```
/// use oxidj_core::TrackId;
/// let id = TrackId::new(1).unwrap();
/// assert_eq!(id.to_string(), "1");
/// assert!(TrackId::new(0).is_err());
/// ```
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct TrackId(u64);

impl TrackId {
    /// The smallest valid identifier; `0` is reserved and rejected by [`new`].
    pub const MIN: u64 = 1;

    /// Construct a `TrackId`, rejecting the reserved value `0`.
    pub const fn new(id: u64) -> Result<Self, TrackIdError> {
        if id == 0 {
            Err(TrackIdError::Zero)
        } else {
            Ok(Self(id))
        }
    }

    /// The raw underlying value — an explicit escape hatch, intentionally named.
    pub const fn as_u64(self) -> u64 {
        self.0
    }
}

/// Errors returned when constructing a [`TrackId`].
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum TrackIdError {
    /// The supplied value was the reserved identifier `0`.
    Zero,
}

impl fmt::Display for TrackId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl fmt::Display for TrackIdError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Zero => write!(f, "track id must be >= 1; got 0"),
        }
    }
}

impl FromStr for TrackId {
    type Err = TrackIdError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parsed: u64 = s.parse().map_err(|_| TrackIdError::Zero)?;
        Self::new(parsed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_is_rejected() {
        assert_eq!(TrackId::new(0), Err(TrackIdError::Zero));
    }

    #[test]
    fn one_is_valid() {
        assert_eq!(TrackId::new(1), Ok(TrackId(1)));
    }

    #[test]
    fn max_u64_is_valid() {
        let id = TrackId::new(u64::MAX).unwrap();
        assert_eq!(id.as_u64(), u64::MAX);
    }

    #[test]
    fn identity_roundtrips_through_string() {
        let id = TrackId::new(20_241_015).unwrap();
        let rendered = id.to_string();
        let reparsed: TrackId = rendered.parse().unwrap();
        assert_eq!(reparsed, id);
        assert_eq!(reparsed.as_u64(), 20_241_015);
    }

    #[test]
    fn zero_string_roundtrips_to_error() {
        assert_eq!("0".parse::<TrackId>().unwrap_err(), TrackIdError::Zero);
    }
}
