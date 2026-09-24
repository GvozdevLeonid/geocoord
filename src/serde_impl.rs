//! Serde support. Deserialization goes through the public constructors: each type is first read
//! into a plain shadow struct with the same fields, then converted with `TryFrom`, so serialized
//! data cannot bypass the invariants (`#[serde(try_from)]` on the public types).

use crate::{
    CoordError, GridLetter, LatitudeDir, LongitudeDir, MGRSZone, UPSBand, UTMBand, DD, DDM, DMS,
    MGRS, UPS, UTM,
};

#[derive(serde::Deserialize)]
pub(crate) struct DDFields {
    pub(crate) latitude: f64,
    pub(crate) longitude: f64,
}

#[derive(serde::Deserialize)]
pub(crate) struct DDMFields {
    pub(crate) latitude_dir: LatitudeDir,
    pub(crate) latitude_degrees: u8,
    pub(crate) latitude_minutes: f64,
    pub(crate) longitude_dir: LongitudeDir,
    pub(crate) longitude_degrees: u8,
    pub(crate) longitude_minutes: f64,
}

#[derive(serde::Deserialize)]
pub(crate) struct DMSFields {
    pub(crate) latitude_dir: LatitudeDir,
    pub(crate) latitude_degrees: u8,
    pub(crate) latitude_minutes: u8,
    pub(crate) latitude_seconds: f64,
    pub(crate) longitude_dir: LongitudeDir,
    pub(crate) longitude_degrees: u8,
    pub(crate) longitude_minutes: u8,
    pub(crate) longitude_seconds: f64,
}

#[derive(serde::Deserialize)]
pub(crate) struct UTMFields {
    pub(crate) zone_number: u8,
    pub(crate) band: UTMBand,
    pub(crate) easting: f64,
    pub(crate) northing: f64,
}

#[derive(serde::Deserialize)]
pub(crate) struct UPSFields {
    pub(crate) band: UPSBand,
    pub(crate) easting: f64,
    pub(crate) northing: f64,
}

#[derive(serde::Deserialize)]
pub(crate) struct MGRSFields {
    pub(crate) zone: MGRSZone,
    pub(crate) square: (GridLetter, GridLetter),
    pub(crate) easting: f64,
    pub(crate) northing: f64,
}

impl TryFrom<DDFields> for DD {
    type Error = CoordError;
    fn try_from(v: DDFields) -> Result<Self, Self::Error> {
        Self::new(v.latitude, v.longitude)
    }
}
impl TryFrom<DDMFields> for DDM {
    type Error = CoordError;
    fn try_from(v: DDMFields) -> Result<Self, Self::Error> {
        Self::new(
            v.latitude_dir,
            v.latitude_degrees,
            v.latitude_minutes,
            v.longitude_dir,
            v.longitude_degrees,
            v.longitude_minutes,
        )
    }
}
impl TryFrom<DMSFields> for DMS {
    type Error = CoordError;
    fn try_from(v: DMSFields) -> Result<Self, Self::Error> {
        Self::new(
            v.latitude_dir,
            v.latitude_degrees,
            v.latitude_minutes,
            v.latitude_seconds,
            v.longitude_dir,
            v.longitude_degrees,
            v.longitude_minutes,
            v.longitude_seconds,
        )
    }
}
impl TryFrom<UTMFields> for UTM {
    type Error = CoordError;
    fn try_from(v: UTMFields) -> Result<Self, Self::Error> {
        Self::new(v.zone_number, v.band, v.easting, v.northing)
    }
}
impl TryFrom<UPSFields> for UPS {
    type Error = CoordError;
    fn try_from(v: UPSFields) -> Result<Self, Self::Error> {
        Self::new(v.band, v.easting, v.northing)
    }
}
impl TryFrom<MGRSFields> for MGRS {
    type Error = CoordError;
    fn try_from(v: MGRSFields) -> Result<Self, Self::Error> {
        Self::new(v.zone, v.square, v.easting, v.northing)
    }
}
