//! Projected coordinates — UTM, UPS and the MGRS grid reference — and every conversion that
//! crosses a projection.

use std::fmt;

use crate::angular::{DD, DDM, DMS};
use crate::grid::*;
use crate::polar::{ups_forward, ups_inverse};
use crate::tm::{tm_forward, utm_central_meridian, utm_inverse};
use crate::wgs84::*;
use crate::CoordError;

/// Universal Transverse Mercator coordinate. Invariants: zone 1..=60 (32/34/36 never with band
/// X), easting in [100 km, 900 km), northing in [0, 10 000 km], and the 100 km square holding
/// the point reaches the latitude band. The hemisphere is taken from the band.
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(try_from = "crate::serde_impl::UTMFields"))]
pub struct UTM {
    zone_number: u8,
    band: UTMBand,
    easting: f64,
    northing: f64,
}

/// Invariants: inside the polar MGRS grid extent of its band and the 100 km square holding
/// the point reaches the UPS zone (>= 83.5°N or <= 79.5°S, NGA overlap included).
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(try_from = "crate::serde_impl::UPSFields"))]
pub struct UPS {
    band: UPSBand,
    easting: f64,
    northing: f64,
}

/// `easting`/`northing` are the metre offsets inside the 100 km square, [0, 100 000).
/// Reverse conversion yields the south-west corner plus the offsets (GEOTRANS semantics).
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(try_from = "crate::serde_impl::MGRSFields"))]
pub struct MGRS {
    zone: MGRSZone,
    square: (GridLetter, GridLetter),
    easting: f64,
    northing: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
/// Either projection, chosen by latitude: UTM within [80°S, 84°N], UPS beyond.
pub enum UniversalCoord {
    /// A UTM coordinate.
    UTM(UTM),
    /// A UPS coordinate.
    UPS(UPS),
}

impl UTM {
    /// Validates the numeric ranges, the zone/band combination and the band admission
    /// (see the type docs).
    pub fn new(
        zone_number: u8,
        band: UTMBand,
        easting: f64,
        northing: f64,
    ) -> Result<Self, CoordError> {
        if !(1..=60).contains(&zone_number) {
            return Err(CoordError::InvalidUtmZone(zone_number));
        }
        if !(100_000.0..900_000.0).contains(&easting)
            || !(0.0..=UTM_FALSE_NORTHING).contains(&northing)
        {
            return Err(CoordError::OutOfProjectionBounds(
                "UTM coordinates out of range",
            ));
        }
        if !utm_zone_exists(zone_number, band) {
            return Err(CoordError::OutOfProjectionBounds(
                "UTM zones 32, 34 and 36 do not exist in latitude band X",
            ));
        }
        if !utm_square_admits(zone_number, band, easting, northing) {
            return Err(CoordError::OutOfProjectionBounds(
                "UTM northing does not belong to the latitude band",
            ));
        }
        Ok(Self {
            zone_number,
            band,
            easting,
            northing: northing + 0.0,
        })
    }

    /// Zone number, 1..=60.
    pub fn zone_number(&self) -> u8 {
        self.zone_number
    }
    /// Latitude band, the hemisphere is derived from it.
    pub fn band(&self) -> UTMBand {
        self.band
    }
    /// Easting in metres, false easting included.
    pub fn easting(&self) -> f64 {
        self.easting
    }
    /// Northing in metres, false northing (southern bands) included.
    pub fn northing(&self) -> f64 {
        self.northing
    }
}

impl UPS {
    /// Validates the grid extent of the band and the zone admission (see the type docs).
    pub fn new(band: UPSBand, easting: f64, northing: f64) -> Result<Self, CoordError> {
        let north = matches!(band, UPSBand::Y | UPSBand::Z);
        let (n_lo, n_hi) = ups_grid_extent(north);
        let (e_lo, e_hi) = match band {
            UPSBand::Y => (1_300_000.0, 2_000_000.0),
            UPSBand::Z => (2_000_000.0, 2_700_000.0),
            UPSBand::A => (800_000.0, 2_000_000.0),
            UPSBand::B => (2_000_000.0, 3_200_000.0),
        };
        if !(e_lo..e_hi).contains(&easting) || !(n_lo..n_hi).contains(&northing) {
            return Err(CoordError::InvalidUpsCoordinate(easting, northing));
        }
        if !ups_square_admits(north, easting, northing) {
            return Err(CoordError::InvalidUpsCoordinate(easting, northing));
        }
        Ok(Self {
            band,
            easting,
            northing,
        })
    }

    /// Zone letter.
    pub fn band(&self) -> UPSBand {
        self.band
    }
    /// Easting in metres, false easting included.
    pub fn easting(&self) -> f64 {
        self.easting
    }
    /// Northing in metres, false northing included.
    pub fn northing(&self) -> f64 {
        self.northing
    }
}

impl MGRS {
    /// Validates the offsets, the square letters against the zone and the square against the
    /// zone's latitude range (see the type docs).
    pub fn new(
        zone: MGRSZone,
        square: (GridLetter, GridLetter),
        easting: f64,
        northing: f64,
    ) -> Result<Self, CoordError> {
        if !(0.0..MGRS_SQUARE).contains(&easting) || !(0.0..MGRS_SQUARE).contains(&northing) {
            return Err(CoordError::OutOfProjectionBounds(
                "MGRS precision coordinates must be in range [0, 100,000) meters",
            ));
        }

        match zone {
            MGRSZone::UTM(zone_number, band) => {
                if !(1..=60).contains(&zone_number) {
                    return Err(CoordError::InvalidUtmZone(zone_number));
                }
                if !utm_zone_exists(zone_number, band) {
                    return Err(CoordError::OutOfProjectionBounds(
                        "UTM zones 32, 34 and 36 do not exist in latitude band X",
                    ));
                }
                let (e0, n0) = utm_square_corner(zone_number, band, square)?;
                if !utm_square_admits(zone_number, band, e0, n0) {
                    return Err(CoordError::OutOfProjectionBounds(
                        "MGRS 100 km square does not belong to the latitude band",
                    ));
                }
                // Only reachable for a southern band with the square's lower edge on the equator.
                if n0 + northing > UTM_FALSE_NORTHING {
                    return Err(CoordError::OutOfProjectionBounds(
                        "MGRS northing crosses the equator in a southern band",
                    ));
                }
            }
            MGRSZone::UPS(band) => {
                let north = matches!(band, UPSBand::Y | UPSBand::Z);
                let (e0, n0) = ups_square_corner(band, square)?;
                if !ups_square_admits(north, e0, n0) {
                    return Err(CoordError::OutOfProjectionBounds(
                        "MGRS polar square lies outside the UPS zone",
                    ));
                }
            }
        }

        Ok(Self {
            zone,
            square,
            easting: easting + 0.0,
            northing: northing + 0.0,
        })
    }

    /// Grid zone.
    pub fn zone(&self) -> MGRSZone {
        self.zone
    }
    /// 100 km square letters, (column, row).
    pub fn square(&self) -> (GridLetter, GridLetter) {
        self.square
    }
    /// Easting offset inside the square, metres in [0, 100 000).
    pub fn easting(&self) -> f64 {
        self.easting
    }
    /// Northing offset inside the square, metres in [0, 100 000).
    pub fn northing(&self) -> f64 {
        self.northing
    }
}

impl UniversalCoord {
    /// `true` for the UTM variant.
    pub fn is_utm(&self) -> bool {
        matches!(self, Self::UTM(_))
    }

    /// `true` for the UPS variant.
    pub fn is_ups(&self) -> bool {
        matches!(self, Self::UPS(_))
    }
}

impl fmt::Display for UTM {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let p = f.precision().unwrap_or(0).clamp(0, 10);
        let ew = if p > 0 { p + 7 } else { 6 };
        let nw = if p > 0 { p + 8 } else { 7 };

        write!(
            f,
            "{:02}{} {:0ew$.p$} {:0nw$.p$}",
            self.zone_number,
            self.band,
            self.easting,
            self.northing,
            ew = ew,
            nw = nw,
            p = p
        )
    }
}

impl fmt::Display for UPS {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let p = f.precision().unwrap_or(0).clamp(0, 10);
        let w = if p > 0 { p + 8 } else { 7 };

        write!(
            f,
            "{} {:0w$.p$} {:0w$.p$}",
            self.band,
            self.easting,
            self.northing,
            w = w,
            p = p
        )
    }
}
impl fmt::Display for MGRS {
    // Grid references truncate (NGA), never round. The 1 µm pre-rounding only removes
    // arithmetic noise (~1e-10 m from the false-easting sums), 100x below the finest digit.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let p = f.precision().unwrap_or(5).min(9);
        let to_um = |v: f64| ((v * 1e6).round() as u64).min(99_999_999_999);
        let div = 10u64.pow(11 - p as u32);
        let e = to_um(self.easting) / div;
        let n = to_um(self.northing) / div;
        match &self.zone {
            MGRSZone::UTM(num, band) => {
                write!(f, "{:02}{} {}{}", num, band, self.square.0, self.square.1)?;
            }
            MGRSZone::UPS(band) => {
                write!(f, "{} {}{}", band, self.square.0, self.square.1)?;
            }
        }
        if p > 0 {
            write!(f, " {:0p$} {:0p$}", e, n, p = p)?;
        }
        Ok(())
    }
}
impl fmt::Display for UniversalCoord {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            UniversalCoord::UTM(utm) => utm.fmt(f),
            UniversalCoord::UPS(ups) => ups.fmt(f),
        }
    }
}

pub(crate) fn utm_project(dd: DD, band: UTMBand) -> UTM {
    let zone = utm_zone(dd.latitude(), dd.longitude());
    let (x, y) = tm_forward(dd.latitude(), dd.longitude() - utm_central_meridian(zone));
    UTM {
        zone_number: zone,
        band,
        easting: x + UTM_FALSE_EASTING,
        northing: y + if dd.latitude() < 0.0 {
            UTM_FALSE_NORTHING
        } else {
            0.0
        },
    }
}

impl From<DD> for UniversalCoord {
    fn from(dd: DD) -> Self {
        match get_utm_band(dd.latitude()) {
            Some(band) => UniversalCoord::UTM(utm_project(dd, band)),
            // Beyond 84°N / 80°S rho <= 667 km / 1 113 km, always inside the grid extent.
            None => {
                let north = dd.latitude() > 0.0;
                let (easting, northing) = ups_forward(north, dd.latitude(), dd.longitude());
                UniversalCoord::UPS(UPS {
                    band: ups_band(north, easting),
                    easting,
                    northing,
                })
            }
        }
    }
}
impl From<DD> for MGRS {
    fn from(dd: DD) -> Self {
        let uc: UniversalCoord = dd.into();
        uc.into()
    }
}

impl From<DDM> for UniversalCoord {
    fn from(ddm: DDM) -> Self {
        let dd: DD = ddm.into();
        dd.into()
    }
}
impl From<DDM> for MGRS {
    fn from(ddm: DDM) -> Self {
        let dd: DD = ddm.into();
        let uc: UniversalCoord = dd.into();
        uc.into()
    }
}

impl From<DMS> for UniversalCoord {
    fn from(dms: DMS) -> Self {
        let dd: DD = dms.into();
        dd.into()
    }
}
impl From<DMS> for MGRS {
    fn from(dms: DMS) -> Self {
        let dd: DD = dms.into();
        let uc: UniversalCoord = dd.into();
        uc.into()
    }
}

impl From<UTM> for DD {
    fn from(utm: UTM) -> Self {
        let (lat, lon) = utm_inverse(
            utm.zone_number,
            utm.band.is_south(),
            utm.easting,
            utm.northing,
        );
        DD::from_projection(lat, lon)
    }
}
impl From<UTM> for DDM {
    fn from(utm: UTM) -> Self {
        let dd: DD = utm.into();
        dd.into()
    }
}
impl From<UTM> for DMS {
    fn from(utm: UTM) -> Self {
        let dd: DD = utm.into();
        dd.into()
    }
}
impl From<UTM> for MGRS {
    fn from(utm: UTM) -> Self {
        let e100k = (utm.easting / MGRS_SQUARE) as i64;
        let n100k = ((utm.northing / MGRS_SQUARE) as i64) % 20;

        let set = mgrs_set(utm.zone_number);
        let e_idx = (MGRS_E_ORIGINS[set] + e100k - 1).rem_euclid(24) as usize;
        let n_idx = (MGRS_N_ORIGINS[set] + n100k).rem_euclid(20) as usize;

        MGRS {
            zone: MGRSZone::UTM(utm.zone_number, utm.band),
            square: (GridLetter::from_index(e_idx), GridLetter::from_index(n_idx)),
            easting: utm.easting % MGRS_SQUARE,
            northing: utm.northing % MGRS_SQUARE,
        }
    }
}
impl From<UTM> for UniversalCoord {
    fn from(utm: UTM) -> Self {
        Self::UTM(utm)
    }
}

impl From<UPS> for DD {
    fn from(ups: UPS) -> Self {
        let north = matches!(ups.band, UPSBand::Y | UPSBand::Z);
        let (lat, lon) = ups_inverse(north, ups.easting, ups.northing);
        DD::from_projection(lat, lon)
    }
}
impl From<UPS> for DDM {
    fn from(ups: UPS) -> Self {
        let dd: DD = ups.into();
        dd.into()
    }
}
impl From<UPS> for DMS {
    fn from(ups: UPS) -> Self {
        let dd: DD = ups.into();
        dd.into()
    }
}
impl From<UPS> for MGRS {
    fn from(ups: UPS) -> Self {
        let north = matches!(ups.band, UPSBand::Y | UPSBand::Z);
        let east = matches!(ups.band, UPSBand::Z | UPSBand::B);

        let e100k = (ups.easting / MGRS_SQUARE) as i64;
        let n100k = (ups.northing / MGRS_SQUARE) as i64;

        let col = UPS_COL_LETTERS[(e100k - if east { 20 } else { 2 }) as usize];
        let row = GridLetter::from_index((n100k - if north { 13 } else { 8 }) as usize);

        MGRS {
            zone: MGRSZone::UPS(ups.band),
            square: (col, row),
            easting: ups.easting % MGRS_SQUARE,
            northing: ups.northing % MGRS_SQUARE,
        }
    }
}
impl From<UPS> for UniversalCoord {
    fn from(ups: UPS) -> Self {
        Self::UPS(ups)
    }
}

impl From<MGRS> for DD {
    fn from(mgrs: MGRS) -> Self {
        let uc: UniversalCoord = mgrs.into();
        uc.into()
    }
}
impl From<MGRS> for DDM {
    fn from(mgrs: MGRS) -> Self {
        let uc: UniversalCoord = mgrs.into();
        uc.into()
    }
}
impl From<MGRS> for DMS {
    fn from(mgrs: MGRS) -> Self {
        let uc: UniversalCoord = mgrs.into();
        uc.into()
    }
}
impl From<MGRS> for UniversalCoord {
    fn from(mgrs: MGRS) -> Self {
        match mgrs.zone {
            MGRSZone::UTM(zone_number, band) => {
                let (e0, n0) = utm_square_corner(zone_number, band, mgrs.square)
                    .expect("square letters validated by MGRS::new or produced by From<UTM>");
                UniversalCoord::UTM(UTM {
                    zone_number,
                    band,
                    easting: e0 + mgrs.easting,
                    northing: n0 + mgrs.northing,
                })
            }
            MGRSZone::UPS(band) => {
                let (e0, n0) = ups_square_corner(band, mgrs.square)
                    .expect("square letters validated by MGRS::new or produced by From<UPS>");
                UniversalCoord::UPS(UPS {
                    band,
                    easting: e0 + mgrs.easting,
                    northing: n0 + mgrs.northing,
                })
            }
        }
    }
}

impl From<UniversalCoord> for DD {
    fn from(uc: UniversalCoord) -> Self {
        match uc {
            UniversalCoord::UPS(ups) => ups.into(),
            UniversalCoord::UTM(utm) => utm.into(),
        }
    }
}
impl From<UniversalCoord> for DDM {
    fn from(uc: UniversalCoord) -> Self {
        match uc {
            UniversalCoord::UPS(ups) => ups.into(),
            UniversalCoord::UTM(utm) => utm.into(),
        }
    }
}
impl From<UniversalCoord> for DMS {
    fn from(uc: UniversalCoord) -> Self {
        match uc {
            UniversalCoord::UPS(ups) => ups.into(),
            UniversalCoord::UTM(utm) => utm.into(),
        }
    }
}
impl From<UniversalCoord> for MGRS {
    fn from(uc: UniversalCoord) -> Self {
        match uc {
            UniversalCoord::UPS(ups) => ups.into(),
            UniversalCoord::UTM(utm) => utm.into(),
        }
    }
}

impl TryFrom<DD> for UTM {
    type Error = CoordError;

    fn try_from(dd: DD) -> Result<Self, Self::Error> {
        let band = get_utm_band(dd.latitude()).ok_or(CoordError::IncompatibleProjection(
            "Latitude is in polar regions (use UPS instead)",
        ))?;
        Ok(utm_project(dd, band))
    }
}
impl TryFrom<DD> for UPS {
    type Error = CoordError;

    fn try_from(dd: DD) -> Result<Self, Self::Error> {
        if dd.latitude() > UPS_SOUTH_LAT_MAX && dd.latitude() < UPS_NORTH_LAT_MIN {
            return Err(CoordError::IncompatibleProjection(
                "Latitude is outside the UPS zones (83.5°N / 79.5°S); use UTM instead.",
            ));
        }
        let north = dd.latitude() > 0.0;
        let (easting, northing) = ups_forward(north, dd.latitude(), dd.longitude());
        // In the 83.5°–83.7°N overlap rho reaches 722 km while the grid stops at 700 km on the axes.
        let (lo, hi) = ups_grid_extent(north);
        if !(lo..hi).contains(&easting) || !(lo..hi).contains(&northing) {
            return Err(CoordError::OutOfProjectionBounds(
                "UPS point lies outside the polar MGRS grid extent",
            ));
        }
        Ok(UPS {
            band: ups_band(north, easting),
            easting,
            northing,
        })
    }
}

impl TryFrom<DDM> for UTM {
    type Error = CoordError;

    fn try_from(ddm: DDM) -> Result<Self, Self::Error> {
        let dd: DD = ddm.into();
        dd.try_into()
    }
}
impl TryFrom<DDM> for UPS {
    type Error = CoordError;

    fn try_from(ddm: DDM) -> Result<Self, Self::Error> {
        let dd: DD = ddm.into();
        dd.try_into()
    }
}

impl TryFrom<DMS> for UTM {
    type Error = CoordError;

    fn try_from(dms: DMS) -> Result<Self, Self::Error> {
        let dd: DD = dms.into();
        dd.try_into()
    }
}
impl TryFrom<DMS> for UPS {
    type Error = CoordError;

    fn try_from(dms: DMS) -> Result<Self, Self::Error> {
        let dd: DD = dms.into();
        dd.try_into()
    }
}

impl TryFrom<MGRS> for UTM {
    type Error = CoordError;

    fn try_from(mgrs: MGRS) -> Result<Self, Self::Error> {
        let uc: UniversalCoord = mgrs.into();
        match uc {
            UniversalCoord::UPS(_) => Err(CoordError::IncompatibleProjection(
                "Coordinate is in polar regions (UPS). Cannot convert to UTM.",
            )),
            UniversalCoord::UTM(utm) => Ok(utm),
        }
    }
}
impl TryFrom<MGRS> for UPS {
    type Error = CoordError;

    fn try_from(mgrs: MGRS) -> Result<Self, Self::Error> {
        let uc: UniversalCoord = mgrs.into();
        match uc {
            UniversalCoord::UPS(ups) => Ok(ups),
            UniversalCoord::UTM(_) => Err(CoordError::IncompatibleProjection(
                "Coordinate is outside polar regions (UTM). Cannot convert to UPS.",
            )),
        }
    }
}

impl TryFrom<UniversalCoord> for UTM {
    type Error = CoordError;

    fn try_from(uc: UniversalCoord) -> Result<Self, Self::Error> {
        match uc {
            UniversalCoord::UTM(utm) => Ok(utm),
            UniversalCoord::UPS(_) => Err(CoordError::IncompatibleProjection(
                "Cannot convert UniversalCoord(UPS) to UTM",
            )),
        }
    }
}
impl TryFrom<UniversalCoord> for UPS {
    type Error = CoordError;

    fn try_from(uc: UniversalCoord) -> Result<Self, Self::Error> {
        match uc {
            UniversalCoord::UPS(ups) => Ok(ups),
            UniversalCoord::UTM(_) => Err(CoordError::IncompatibleProjection(
                "Cannot convert UniversalCoord(UTM) to UPS",
            )),
        }
    }
}
