#![doc = include_str!("../README.md")]
#![warn(missing_docs)]

mod angular;
mod grid;
mod parse;
mod polar;
mod projected;
#[cfg(feature = "serde")]
mod serde_impl;
mod tm;
mod wgs84;

pub use angular::{LatitudeDir, LongitudeDir, DD, DDM, DMS};
pub use grid::{GridLetter, MGRSZone, UPSBand, UTMBand};
pub use projected::{UniversalCoord, MGRS, UPS, UTM};

/// Every way a coordinate can be rejected.
#[derive(Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub enum CoordError {
    /// Latitude outside [-90, 90] (or not finite).
    InvalidLatitude(f64),
    /// Longitude outside [-180, 180] (or not finite).
    InvalidLongitude(f64),

    /// Whole degrees too large for the axis named in the message.
    InvalidDegrees(u8, &'static str),
    /// Minutes outside [0, 60).
    InvalidMinutes(f64),
    /// Seconds outside [0, 60).
    InvalidSeconds(f64),

    /// UTM zone outside 1..=60.
    InvalidUtmZone(u8),

    /// UPS easting/northing outside the polar grid extent of the band, or the 100 km square
    /// does not reach the UPS zone.
    InvalidUpsCoordinate(f64, f64),

    /// The requested projection does not cover this point.
    IncompatibleProjection(&'static str),
    /// Numeric range or band/zone admission check failed; the message says which.
    OutOfProjectionBounds(&'static str),

    /// Text could not be parsed; the message says what was expected.
    InvalidFormat(&'static str),
}
impl std::fmt::Display for CoordError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidLatitude(v) => write!(f, "Latitude {} is out of range [-90, 90]", v),
            Self::InvalidLongitude(v) => write!(f, "Longitude {} is out of range [-180, 180]", v),
            Self::InvalidDegrees(v, t) => write!(f, "Invalid {} degrees: {}", t, v),
            Self::InvalidMinutes(v) => write!(f, "Minutes {} must be in range [0, 60)", v),
            Self::InvalidSeconds(v) => write!(f, "Seconds {} must be in range [0, 60)", v),
            Self::InvalidUtmZone(v) => write!(f, "UTM Zone {} must be in range [1, 60]", v),
            Self::InvalidUpsCoordinate(e, n) => {
                write!(f, "UPS coordinates ({}, {}) out of grid", e, n)
            }
            Self::IncompatibleProjection(msg) => write!(f, "Incompatible projection: {}", msg),
            Self::OutOfProjectionBounds(msg) => write!(f, "Projection bounds exceeded: {}", msg),
            Self::InvalidFormat(msg) => write!(f, "Invalid format: {}", msg),
        }
    }
}

impl std::error::Error for CoordError {}
