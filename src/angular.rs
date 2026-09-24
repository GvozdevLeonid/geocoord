//! Geographic coordinates in the three angular notations and the conversions between them.

use std::fmt;

use crate::CoordError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
/// Hemisphere of a latitude.
pub enum LatitudeDir {
    /// Northern hemisphere, sign +.
    North,
    /// Southern hemisphere, sign −.
    South,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
/// Hemisphere of a longitude.
pub enum LongitudeDir {
    /// East of Greenwich, sign +.
    East,
    /// West of Greenwich, sign −.
    West,
}

/// Geodetic WGS84 point in decimal degrees. Invariants: latitude in [-90, 90], longitude in
/// [-180, 180), no negative zero. +180 is stored as -180 so that every point has exactly one
/// encoding (and one UTM zone / UPS half).
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(try_from = "crate::serde_impl::DDFields"))]
pub struct DD {
    latitude: f64,
    longitude: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(try_from = "crate::serde_impl::DDMFields"))]
/// Degrees and decimal minutes. Invariants: degrees ≤ 90 / 180 (exactly 90 or 180 only with
/// zero minutes), minutes in [0, 60).
pub struct DDM {
    latitude_dir: LatitudeDir,
    latitude_degrees: u8,
    latitude_minutes: f64,

    longitude_dir: LongitudeDir,
    longitude_degrees: u8,
    longitude_minutes: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(try_from = "crate::serde_impl::DMSFields"))]
/// Degrees, minutes and decimal seconds. Invariants: degrees ≤ 90 / 180 (exactly 90 or 180 only
/// with zero minutes and seconds), minutes < 60, seconds in [0, 60).
pub struct DMS {
    latitude_dir: LatitudeDir,
    latitude_degrees: u8,
    latitude_minutes: u8,
    latitude_seconds: f64,

    longitude_dir: LongitudeDir,
    longitude_degrees: u8,
    longitude_minutes: u8,
    longitude_seconds: f64,
}

/// Invariants: zone 1..=60 (32/34/36 never with band X), easting in [100 km, 900 km),
/// northing in [0, 10 000 km], and the 100 km square holding the point reaches the
/// latitude band (see `utm_square_admits`). Hemisphere is taken from the band.

impl DD {
    /// Validates the ranges and canonicalizes the values (see the type docs).
    pub fn new(latitude: f64, longitude: f64) -> Result<Self, CoordError> {
        if !(-90.0..=90.0).contains(&latitude) {
            return Err(CoordError::InvalidLatitude(latitude));
        }
        if !(-180.0..=180.0).contains(&longitude) {
            return Err(CoordError::InvalidLongitude(longitude));
        }
        Ok(Self::canonical(latitude, longitude))
    }

    // `x + 0.0` turns -0.0 into 0.0 (IEEE round-to-nearest), the only sign-of-zero fix that survives the compiler.
    pub(crate) fn canonical(latitude: f64, longitude: f64) -> Self {
        Self {
            latitude: latitude + 0.0,
            longitude: if longitude == 180.0 {
                -180.0
            } else {
                longitude + 0.0
            },
        }
    }

    // Output of an inverse projection: bring it into the invariants without judging it.
    pub(crate) fn from_projection(latitude: f64, longitude: f64) -> Self {
        Self::canonical(
            latitude.clamp(-90.0, 90.0),
            (longitude + 180.0).rem_euclid(360.0) - 180.0,
        )
    }

    /// Latitude in degrees, [-90, 90].
    pub fn latitude(&self) -> f64 {
        self.latitude
    }
    /// Longitude in degrees, [-180, 180).
    pub fn longitude(&self) -> f64 {
        self.longitude
    }
}

impl DDM {
    /// Validates degrees and minutes; the direction supplies the sign.
    pub fn new(
        lat_dir: LatitudeDir,
        lat_deg: u8,
        lat_min: f64,
        lon_dir: LongitudeDir,
        lon_deg: u8,
        lon_min: f64,
    ) -> Result<Self, CoordError> {
        if lat_deg > 90 || (lat_deg == 90 && lat_min > 0.0) {
            return Err(CoordError::InvalidDegrees(lat_deg, "latitude"));
        }
        if lon_deg > 180 || (lon_deg == 180 && lon_min > 0.0) {
            return Err(CoordError::InvalidDegrees(lon_deg, "longitude"));
        }

        if !(0.0..60.0).contains(&lat_min) {
            return Err(CoordError::InvalidMinutes(lat_min));
        }
        if !(0.0..60.0).contains(&lon_min) {
            return Err(CoordError::InvalidMinutes(lon_min));
        }
        Ok(Self {
            // `+ 0.0` keeps -0.0 out of the fields (it would print as `-0.000'`).
            latitude_dir: lat_dir,
            latitude_degrees: lat_deg,
            latitude_minutes: lat_min + 0.0,
            longitude_dir: lon_dir,
            longitude_degrees: lon_deg,
            longitude_minutes: lon_min + 0.0,
        })
    }

    /// Component accessor.
    pub fn latitude_dir(&self) -> LatitudeDir {
        self.latitude_dir
    }
    /// Component accessor.
    pub fn latitude_degrees(&self) -> u8 {
        self.latitude_degrees
    }
    /// Component accessor.
    pub fn latitude_minutes(&self) -> f64 {
        self.latitude_minutes
    }

    /// Component accessor.
    pub fn longitude_dir(&self) -> LongitudeDir {
        self.longitude_dir
    }
    /// Component accessor.
    pub fn longitude_degrees(&self) -> u8 {
        self.longitude_degrees
    }
    /// Component accessor.
    pub fn longitude_minutes(&self) -> f64 {
        self.longitude_minutes
    }
}

impl DMS {
    /// Validates degrees, minutes and seconds; the direction supplies the sign.
    pub fn new(
        lat_dir: LatitudeDir,
        lat_deg: u8,
        lat_min: u8,
        lat_sec: f64,
        lon_dir: LongitudeDir,
        lon_deg: u8,
        lon_min: u8,
        lon_sec: f64,
    ) -> Result<Self, CoordError> {
        if lat_deg > 90 || (lat_deg == 90 && (lat_min > 0 || lat_sec > 0.0)) {
            return Err(CoordError::InvalidDegrees(lat_deg, "latitude"));
        }
        if lon_deg > 180 || (lon_deg == 180 && (lon_min > 0 || lon_sec > 0.0)) {
            return Err(CoordError::InvalidDegrees(lon_deg, "longitude"));
        }
        if lat_min >= 60 {
            return Err(CoordError::InvalidMinutes(lat_min as f64));
        }
        if lon_min >= 60 {
            return Err(CoordError::InvalidMinutes(lon_min as f64));
        }
        if !(0.0..60.0).contains(&lat_sec) {
            return Err(CoordError::InvalidSeconds(lat_sec));
        }
        if !(0.0..60.0).contains(&lon_sec) {
            return Err(CoordError::InvalidSeconds(lon_sec));
        }
        Ok(Self {
            latitude_dir: lat_dir,
            latitude_degrees: lat_deg,
            latitude_minutes: lat_min,
            latitude_seconds: lat_sec + 0.0,
            longitude_dir: lon_dir,
            longitude_degrees: lon_deg,
            longitude_minutes: lon_min,
            longitude_seconds: lon_sec + 0.0,
        })
    }

    /// Component accessor.
    pub fn latitude_dir(&self) -> LatitudeDir {
        self.latitude_dir
    }
    /// Component accessor.
    pub fn latitude_degrees(&self) -> u8 {
        self.latitude_degrees
    }
    /// Component accessor.
    pub fn latitude_minutes(&self) -> u8 {
        self.latitude_minutes
    }
    /// Component accessor.
    pub fn latitude_seconds(&self) -> f64 {
        self.latitude_seconds
    }

    /// Component accessor.
    pub fn longitude_dir(&self) -> LongitudeDir {
        self.longitude_dir
    }
    /// Component accessor.
    pub fn longitude_degrees(&self) -> u8 {
        self.longitude_degrees
    }
    /// Component accessor.
    pub fn longitude_minutes(&self) -> u8 {
        self.longitude_minutes
    }
    /// Component accessor.
    pub fn longitude_seconds(&self) -> f64 {
        self.longitude_seconds
    }
}

impl LatitudeDir {
    /// +1.0 for north, −1.0 for south.
    pub fn sign(&self) -> f64 {
        match self {
            Self::North => 1.0,
            Self::South => -1.0,
        }
    }
}

impl LongitudeDir {
    /// +1.0 for east, −1.0 for west.
    pub fn sign(&self) -> f64 {
        match self {
            Self::East => 1.0,
            Self::West => -1.0,
        }
    }
}

impl fmt::Display for LatitudeDir {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::North => 'N',
                Self::South => 'S',
            }
        )
    }
}
impl fmt::Display for LongitudeDir {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::East => 'E',
                Self::West => 'W',
            }
        )
    }
}

impl fmt::Display for DD {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let p = f.precision().unwrap_or(6).clamp(0, 15);
        write!(
            f,
            "{:lat_w$.p$}, {:lon_w$.p$}",
            self.latitude,
            self.longitude,
            lat_w = p + 4,
            lon_w = p + 5,
            p = p
        )
    }
}

fn round_carry_min(deg: u8, min: f64, p: usize) -> (u8, f64) {
    let k = 10f64.powi(p as i32);
    let m = (min * k).round() / k;
    if m >= 60.0 {
        (deg + 1, 0.0)
    } else {
        (deg, m)
    }
}
fn round_carry_sec(deg: u8, min: u8, sec: f64, p: usize) -> (u8, u8, f64) {
    let k = 10f64.powi(p as i32);
    let s = (sec * k).round() / k;
    if s >= 60.0 {
        if min == 59 {
            (deg + 1, 0, 0.0)
        } else {
            (deg, min + 1, 0.0)
        }
    } else {
        (deg, min, s)
    }
}

impl fmt::Display for DDM {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let p = f.precision().unwrap_or(3).clamp(0, 12);
        let w = if p > 0 { p + 3 } else { 2 };
        let (lat_d, lat_m) = round_carry_min(self.latitude_degrees, self.latitude_minutes, p);
        let (lon_d, lon_m) = round_carry_min(self.longitude_degrees, self.longitude_minutes, p);
        write!(
            f,
            "{:02}°{:0w$.p$}'{} {:03}°{:0w$.p$}'{}",
            lat_d,
            lat_m,
            self.latitude_dir,
            lon_d,
            lon_m,
            self.longitude_dir,
            w = w,
            p = p
        )
    }
}
impl fmt::Display for DMS {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let p = f.precision().unwrap_or(2).clamp(0, 10);
        let w = if p > 0 { p + 3 } else { 2 };
        let (lat_d, lat_m, lat_s) = round_carry_sec(
            self.latitude_degrees,
            self.latitude_minutes,
            self.latitude_seconds,
            p,
        );
        let (lon_d, lon_m, lon_s) = round_carry_sec(
            self.longitude_degrees,
            self.longitude_minutes,
            self.longitude_seconds,
            p,
        );
        write!(
            f,
            "{:02}°{:02}'{:0w$.p$}\"{} {:03}°{:02}'{:0w$.p$}\"{}",
            lat_d,
            lat_m,
            lat_s,
            self.latitude_dir,
            lon_d,
            lon_m,
            lon_s,
            self.longitude_dir,
            w = w,
            p = p
        )
    }
}
impl From<DD> for DDM {
    fn from(dd: DD) -> Self {
        let lat_abs = dd.latitude.abs();
        let lon_abs = dd.longitude.abs();

        DDM {
            latitude_dir: if dd.latitude < 0.0 {
                LatitudeDir::South
            } else {
                LatitudeDir::North
            },
            latitude_degrees: lat_abs.trunc() as u8,
            latitude_minutes: lat_abs.fract() * 60.0,
            longitude_dir: if dd.longitude < 0.0 {
                LongitudeDir::West
            } else {
                LongitudeDir::East
            },
            longitude_degrees: lon_abs.trunc() as u8,
            longitude_minutes: lon_abs.fract() * 60.0,
        }
    }
}
impl From<DD> for DMS {
    fn from(dd: DD) -> Self {
        let lat_abs = dd.latitude.abs();
        let lon_abs = dd.longitude.abs();

        let lat_minutes_full = lat_abs.fract() * 60.0;
        let lon_minutes_full = lon_abs.fract() * 60.0;

        DMS {
            latitude_dir: if dd.latitude < 0.0 {
                LatitudeDir::South
            } else {
                LatitudeDir::North
            },
            latitude_degrees: lat_abs.trunc() as u8,
            latitude_minutes: lat_minutes_full.trunc() as u8,
            latitude_seconds: lat_minutes_full.fract() * 60.0,
            longitude_dir: if dd.longitude < 0.0 {
                LongitudeDir::West
            } else {
                LongitudeDir::East
            },
            longitude_degrees: lon_abs.trunc() as u8,
            longitude_minutes: lon_minutes_full.trunc() as u8,
            longitude_seconds: lon_minutes_full.fract() * 60.0,
        }
    }
}

impl From<DDM> for DD {
    fn from(ddm: DDM) -> Self {
        DD::canonical(
            (ddm.latitude_degrees as f64 + ddm.latitude_minutes / 60.0) * ddm.latitude_dir.sign(),
            (ddm.longitude_degrees as f64 + ddm.longitude_minutes / 60.0)
                * ddm.longitude_dir.sign(),
        )
    }
}
impl From<DDM> for DMS {
    fn from(ddm: DDM) -> Self {
        DMS {
            latitude_dir: ddm.latitude_dir,
            latitude_degrees: ddm.latitude_degrees,
            latitude_minutes: ddm.latitude_minutes.trunc() as u8,
            latitude_seconds: ddm.latitude_minutes.fract() * 60.0,
            longitude_dir: ddm.longitude_dir,
            longitude_degrees: ddm.longitude_degrees,
            longitude_minutes: ddm.longitude_minutes.trunc() as u8,
            longitude_seconds: ddm.longitude_minutes.fract() * 60.0,
        }
    }
}

impl From<DMS> for DD {
    fn from(dms: DMS) -> Self {
        DD::canonical(
            (dms.latitude_degrees as f64
                + dms.latitude_minutes as f64 / 60.0
                + dms.latitude_seconds / 3600.0)
                * dms.latitude_dir.sign(),
            (dms.longitude_degrees as f64
                + dms.longitude_minutes as f64 / 60.0
                + dms.longitude_seconds / 3600.0)
                * dms.longitude_dir.sign(),
        )
    }
}
impl From<DMS> for DDM {
    fn from(dms: DMS) -> Self {
        // 59 + 59.99999999999999/60 rounds to exactly 60.0 in f64: carry, or the DDM invariant breaks.
        fn carry(deg: u8, min: u8, sec: f64) -> (u8, f64) {
            let m = min as f64 + sec / 60.0;
            if m >= 60.0 {
                (deg + 1, 0.0)
            } else {
                (deg, m)
            }
        }
        let (latitude_degrees, latitude_minutes) = carry(
            dms.latitude_degrees,
            dms.latitude_minutes,
            dms.latitude_seconds,
        );
        let (longitude_degrees, longitude_minutes) = carry(
            dms.longitude_degrees,
            dms.longitude_minutes,
            dms.longitude_seconds,
        );
        DDM {
            latitude_dir: dms.latitude_dir,
            latitude_degrees,
            latitude_minutes,
            longitude_dir: dms.longitude_dir,
            longitude_degrees,
            longitude_minutes,
        }
    }
}

impl std::str::FromStr for LatitudeDir {
    type Err = CoordError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_ascii_uppercase().as_str() {
            "N" | "NORTH" => Ok(LatitudeDir::North),
            "S" | "SOUTH" => Ok(LatitudeDir::South),
            _ => Err(CoordError::InvalidFormat(
                "latitude direction must be N or S",
            )),
        }
    }
}
impl std::str::FromStr for LongitudeDir {
    type Err = CoordError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_ascii_uppercase().as_str() {
            "E" | "EAST" => Ok(LongitudeDir::East),
            "W" | "WEST" => Ok(LongitudeDir::West),
            _ => Err(CoordError::InvalidFormat(
                "longitude direction must be E or W",
            )),
        }
    }
}
