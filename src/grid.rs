//! Zone and band selection, the MGRS 100 km square tables, and the geodetic admission checks
//! shared by the `UTM`, `UPS` and `MGRS` constructors.

use std::fmt;

use crate::polar::ups_inverse;
use crate::tm::utm_inverse;
use crate::wgs84::*;
use crate::CoordError;

pub(crate) const MGRS_E_ORIGINS: [i64; 6] = [0, 8, 16, 0, 8, 16];
pub(crate) const MGRS_N_ORIGINS: [i64; 6] = [0, 5, 0, 5, 0, 5];

pub(crate) const UPS_COL_LETTERS: [GridLetter; 18] = [
    GridLetter::A,
    GridLetter::B,
    GridLetter::C,
    GridLetter::F,
    GridLetter::G,
    GridLetter::H,
    GridLetter::J,
    GridLetter::K,
    GridLetter::L,
    GridLetter::P,
    GridLetter::Q,
    GridLetter::R,
    GridLetter::S,
    GridLetter::T,
    GridLetter::U,
    GridLetter::X,
    GridLetter::Y,
    GridLetter::Z,
];

// Transverse Mercator: Krüger series in the third flattening n to order n^6
// (Karney, "Transverse Mercator with an accuracy of a few nanometers", 2011,
// eq. 35/36). Truncation error inside the UTM extent (incl. 6°-wide zones) < 5 nm;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
/// UTM latitude band letter, C (80°S) to X (72°N–84°N); I and O are not used.
#[allow(missing_docs)]
pub enum UTMBand {
    C,
    D,
    E,
    F,
    G,
    H,
    J,
    K,
    L,
    M,
    N,
    P,
    Q,
    R,
    S,
    T,
    U,
    V,
    W,
    X,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
/// UPS zone letter: A/B south (west/east of 0°), Y/Z north.
#[allow(missing_docs)]
pub enum UPSBand {
    A,
    B,
    Y,
    Z,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
/// Grid zone of an MGRS reference: a UTM zone with its band, or a UPS zone.
pub enum MGRSZone {
    /// UTM zone number and latitude band.
    UTM(u8, UTMBand),
    /// UPS zone.
    UPS(UPSBand),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
/// 100 km square letter (column or row); I and O are not used.
#[allow(missing_docs)]
pub enum GridLetter {
    A,
    B,
    C,
    D,
    E,
    F,
    G,
    H,
    J,
    K,
    L,
    M,
    N,
    P,
    Q,
    R,
    S,
    T,
    U,
    V,
    W,
    X,
    Y,
    Z,
}

impl UPSBand {
    /// The zone letter.
    pub fn as_char(&self) -> char {
        b"ABYZ"[*self as usize] as char
    }
}

impl UTMBand {
    /// The band letter.
    pub fn as_char(&self) -> char {
        b"CDEFGHJKLMNPQRSTUVWX"[*self as usize] as char
    }

    /// Lower bound of the band's northing, the value MGRS row letters are resolved against
    /// (rows repeat every 2 000 km).
    pub fn get_min_northing(&self) -> f64 {
        match self {
            Self::C => 1100000.0,
            Self::D => 2000000.0,
            Self::E => 2800000.0,
            Self::F => 3700000.0,
            Self::G => 4600000.0,
            Self::H => 5500000.0,
            Self::J => 6400000.0,
            Self::K => 7300000.0,
            Self::L => 8200000.0,
            Self::M => 9100000.0,
            Self::N => 0.0,
            Self::P => 800000.0,
            Self::Q => 1700000.0,
            Self::R => 2600000.0,
            Self::S => 3500000.0,
            Self::T => 4400000.0,
            Self::U => 5300000.0,
            Self::V => 6200000.0,
            Self::W => 7000000.0,
            Self::X => 7900000.0,
        }
    }

    pub(crate) fn is_south(&self) -> bool {
        (*self as usize) < (Self::N as usize)
    }

    // [lower, upper] latitude of the band; X is 12° tall.
    pub(crate) fn lat_range(&self) -> (f64, f64) {
        let lo = UTM_LAT_MIN + 8.0 * (*self as usize) as f64;
        (
            lo,
            if *self == Self::X {
                UTM_LAT_MAX
            } else {
                lo + 8.0
            },
        )
    }
}

impl GridLetter {
    /// Letter at `idx` in the 24-letter alphabet without I and O; `idx` is taken modulo 24.
    pub fn from_index(idx: usize) -> Self {
        const VALS: [GridLetter; 24] = [
            GridLetter::A,
            GridLetter::B,
            GridLetter::C,
            GridLetter::D,
            GridLetter::E,
            GridLetter::F,
            GridLetter::G,
            GridLetter::H,
            GridLetter::J,
            GridLetter::K,
            GridLetter::L,
            GridLetter::M,
            GridLetter::N,
            GridLetter::P,
            GridLetter::Q,
            GridLetter::R,
            GridLetter::S,
            GridLetter::T,
            GridLetter::U,
            GridLetter::V,
            GridLetter::W,
            GridLetter::X,
            GridLetter::Y,
            GridLetter::Z,
        ];
        VALS[idx % 24]
    }
    /// Position in the 24-letter alphabet without I and O.
    pub fn as_index(&self) -> usize {
        *self as usize
    }
    /// The letter.
    pub fn as_char(&self) -> char {
        b"ABCDEFGHJKLMNPQRSTUVWXYZ"[*self as usize] as char
    }
}

impl fmt::Display for UTMBand {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_char())
    }
}
impl fmt::Display for UPSBand {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_char())
    }
}
impl fmt::Display for GridLetter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_char())
    }
}

pub(crate) fn utm_band_from_char(c: char) -> Option<UTMBand> {
    const BANDS: [UTMBand; 20] = [
        UTMBand::C,
        UTMBand::D,
        UTMBand::E,
        UTMBand::F,
        UTMBand::G,
        UTMBand::H,
        UTMBand::J,
        UTMBand::K,
        UTMBand::L,
        UTMBand::M,
        UTMBand::N,
        UTMBand::P,
        UTMBand::Q,
        UTMBand::R,
        UTMBand::S,
        UTMBand::T,
        UTMBand::U,
        UTMBand::V,
        UTMBand::W,
        UTMBand::X,
    ];
    "CDEFGHJKLMNPQRSTUVWX".find(c).map(|i| BANDS[i])
}

pub(crate) fn ups_band_from_char(c: char) -> Option<UPSBand> {
    match c {
        'A' => Some(UPSBand::A),
        'B' => Some(UPSBand::B),
        'Y' => Some(UPSBand::Y),
        'Z' => Some(UPSBand::Z),
        _ => None,
    }
}

pub(crate) fn grid_letter_from_char(c: char) -> Option<GridLetter> {
    "ABCDEFGHJKLMNPQRSTUVWXYZ"
        .find(c)
        .map(GridLetter::from_index)
}

pub(crate) fn single_letter(s: &str) -> Option<char> {
    let mut it = s.trim().chars().map(|c| c.to_ascii_uppercase());
    match (it.next(), it.next()) {
        (Some(c), None) => Some(c),
        _ => None,
    }
}

impl std::str::FromStr for UTMBand {
    type Err = CoordError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        single_letter(s)
            .and_then(utm_band_from_char)
            .ok_or(CoordError::InvalidFormat(
                "band letter must be one of C..X without I and O",
            ))
    }
}
impl std::str::FromStr for UPSBand {
    type Err = CoordError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        single_letter(s)
            .and_then(ups_band_from_char)
            .ok_or(CoordError::InvalidFormat(
                "UPS band must be one of A, B, Y, Z",
            ))
    }
}
impl std::str::FromStr for GridLetter {
    type Err = CoordError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        single_letter(s)
            .and_then(grid_letter_from_char)
            .ok_or(CoordError::InvalidFormat(
                "grid letter must be A..Z without I and O",
            ))
    }
}

pub(crate) fn utm_zone(lat: f64, lon: f64) -> u8 {
    let mut zone = (((lon + 180.0) / 6.0).floor() as u8 + 1).min(60);
    if between(lat, 56.0, 64.0) && between(lon, 3.0, 12.0) {
        zone = 32;
    } else if (72.0..=84.0).contains(&lat) {
        if between(lon, 0.0, 9.0) {
            zone = 31;
        } else if between(lon, 9.0, 21.0) {
            zone = 33;
        } else if between(lon, 21.0, 33.0) {
            zone = 35;
        } else if between(lon, 33.0, 42.0) {
            zone = 37;
        }
    }
    zone
}

pub(crate) fn ups_band(north: bool, easting: f64) -> UPSBand {
    match (north, easting >= UPS_FALSE_EASTING) {
        (true, true) => UPSBand::Z,
        (true, false) => UPSBand::Y,
        (false, true) => UPSBand::B,
        (false, false) => UPSBand::A,
    }
}

// Extent of the polar MGRS 100 km grid (both axes): the only region where the polar
// square letters exist, hence the domain of UPS in this library.
pub(crate) fn ups_grid_extent(north: bool) -> (f64, f64) {
    if north {
        (1_300_000.0, 2_700_000.0)
    } else {
        (800_000.0, 3_200_000.0)
    }
}

pub(crate) fn square_origin(v: f64) -> f64 {
    (v / MGRS_SQUARE).floor() * MGRS_SQUARE
}

// Band checks work at 100 km-square resolution: the square holding the point must reach
// the band's latitude interval. This is what keeps coarse references ("31U DQ") and points
// whose square straddles a band boundary valid, while rejecting any northing that belongs
// to another band or hemisphere. Latitude is monotonic in northing and in |easting - 500 km|
// (with opposite sign per hemisphere) and 500 km is always a square edge, so the extremes
// sit on the four corners.
pub(crate) fn utm_square_admits(zone: u8, band: UTMBand, easting: f64, northing: f64) -> bool {
    // The MGRS row letter is resolved into [min, min + 2 000 km); a northing outside that
    // window (only reachable > 10° off the central meridian) has no round-trippable grid square.
    let min_northing = band.get_min_northing();
    if !(min_northing..min_northing + MGRS_ROW_PERIOD).contains(&northing) {
        return false;
    }
    let e0 = square_origin(easting);
    let n0 = square_origin(northing);
    let south = band.is_south();
    let (mut lat_min, mut lat_max) = (f64::INFINITY, f64::NEG_INFINITY);
    for (e, n) in [
        (e0, n0),
        (e0 + MGRS_SQUARE, n0),
        (e0, n0 + MGRS_SQUARE),
        (e0 + MGRS_SQUARE, n0 + MGRS_SQUARE),
    ] {
        let (lat, _) = utm_inverse(zone, south, e, n);
        lat_min = lat_min.min(lat);
        lat_max = lat_max.max(lat);
    }
    let (lo, hi) = band.lat_range();
    lat_max >= lo - LAT_TOLERANCE_DEG && lat_min <= hi + LAT_TOLERANCE_DEG
}

// Same principle for UPS: the point of the square nearest the pole must lie inside the zone.
pub(crate) fn ups_square_admits(north: bool, easting: f64, northing: f64) -> bool {
    let e0 = square_origin(easting);
    let n0 = square_origin(northing);
    let (lat, _) = ups_inverse(
        north,
        UPS_FALSE_EASTING.clamp(e0, e0 + MGRS_SQUARE),
        UPS_FALSE_NORTHING.clamp(n0, n0 + MGRS_SQUARE),
    );
    if north {
        lat >= UPS_NORTH_LAT_MIN - LAT_TOLERANCE_DEG
    } else {
        lat <= UPS_SOUTH_LAT_MAX + LAT_TOLERANCE_DEG
    }
}

pub(crate) fn utm_zone_exists(zone: u8, band: UTMBand) -> bool {
    !(band == UTMBand::X && matches!(zone, 32 | 34 | 36))
}

pub(crate) fn mgrs_set(zone: u8) -> usize {
    ((zone - 1) % 6) as usize
}

pub(crate) fn ups_col_pos(g: GridLetter) -> Option<usize> {
    UPS_COL_LETTERS.iter().position(|&c| c == g)
}

// South-west corner (easting, northing) of a UTM 100 km square, northing already resolved
// against the band's minimum (row letters repeat every 2 000 km).
pub(crate) fn utm_square_corner(
    zone: u8,
    band: UTMBand,
    square: (GridLetter, GridLetter),
) -> Result<(f64, f64), CoordError> {
    let set = mgrs_set(zone);
    let col_off = (square.0.as_index() as i64 - MGRS_E_ORIGINS[set]).rem_euclid(24);
    if col_off >= 8 {
        return Err(CoordError::OutOfProjectionBounds(
            "MGRS column letter is not valid for this UTM zone set",
        ));
    }
    if square.1.as_index() >= 20 {
        return Err(CoordError::OutOfProjectionBounds(
            "MGRS row letter must be in A..V for UTM zones",
        ));
    }
    let n100k = (square.1.as_index() as i64 - MGRS_N_ORIGINS[set]).rem_euclid(20);
    let easting = (col_off + 1) as f64 * MGRS_SQUARE;
    let mut northing = n100k as f64 * MGRS_SQUARE;
    let min_northing = band.get_min_northing();
    if northing < min_northing {
        northing += ((min_northing - northing) / MGRS_ROW_PERIOD).ceil() * MGRS_ROW_PERIOD;
    }
    Ok((easting, northing))
}

// South-west corner of a polar 100 km square; the column/row ranges are the grid extent per zone.
pub(crate) fn ups_square_corner(
    band: UPSBand,
    square: (GridLetter, GridLetter),
) -> Result<(f64, f64), CoordError> {
    let north = matches!(band, UPSBand::Y | UPSBand::Z);
    let east = matches!(band, UPSBand::Z | UPSBand::B);
    let pos = ups_col_pos(square.0).ok_or(CoordError::OutOfProjectionBounds(
        "MGRS polar column letter must exclude D, E, I, M, N, O, V, W",
    ))?;
    let e100k = pos as i64 + if east { 20 } else { 2 };
    let (e_lo, e_hi) = match band {
        UPSBand::Y => (13, 19),
        UPSBand::Z => (20, 26),
        UPSBand::A => (8, 19),
        UPSBand::B => (20, 31),
    };
    if !(e_lo..=e_hi).contains(&e100k) {
        return Err(CoordError::OutOfProjectionBounds(
            "MGRS polar column letter is not valid for this UPS band",
        ));
    }
    let row_max = if north { 13 } else { 23 };
    if square.1.as_index() > row_max {
        return Err(CoordError::OutOfProjectionBounds(
            "MGRS polar row letter is out of range for this hemisphere",
        ));
    }
    let n100k = square.1.as_index() as i64 + if north { 13 } else { 8 };
    Ok((e100k as f64 * MGRS_SQUARE, n100k as f64 * MGRS_SQUARE))
}

pub(crate) fn between(x: f64, a: f64, b: f64) -> bool {
    x >= a && x < b
}

pub(crate) fn get_utm_band(latitude: f64) -> Option<UTMBand> {
    if latitude < UTM_LAT_MIN || latitude > UTM_LAT_MAX {
        return None;
    }
    const BANDS: [UTMBand; 20] = [
        UTMBand::C,
        UTMBand::D,
        UTMBand::E,
        UTMBand::F,
        UTMBand::G,
        UTMBand::H,
        UTMBand::J,
        UTMBand::K,
        UTMBand::L,
        UTMBand::M,
        UTMBand::N,
        UTMBand::P,
        UTMBand::Q,
        UTMBand::R,
        UTMBand::S,
        UTMBand::T,
        UTMBand::U,
        UTMBand::V,
        UTMBand::W,
        UTMBand::X,
    ];
    let idx = (((latitude - UTM_LAT_MIN) / 8.0).trunc() as usize).min(19);
    Some(BANDS[idx])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn square_letters_follow_the_aa_scheme() {
        // zone 34 (set 4): columns start at A, rows at F
        assert_eq!(mgrs_set(34), 3);
        assert_eq!(
            utm_square_corner(34, UTMBand::T, (GridLetter::D, GridLetter::Q)).unwrap(),
            (400_000.0, 4_900_000.0)
        );
        // zone 1 (set 1), band C: row A resolves above the band minimum, not to 0
        assert_eq!(
            utm_square_corner(1, UTMBand::C, (GridLetter::A, GridLetter::A)).unwrap(),
            (100_000.0, 2_000_000.0)
        );
        assert!(utm_square_corner(34, UTMBand::T, (GridLetter::Z, GridLetter::Q)).is_err());
        assert!(utm_square_corner(34, UTMBand::T, (GridLetter::D, GridLetter::W)).is_err());
    }

    #[test]
    fn polar_square_letters() {
        assert_eq!(
            ups_square_corner(UPSBand::Y, (GridLetter::R, GridLetter::A)).unwrap(),
            (1_300_000.0, 1_300_000.0)
        );
        assert_eq!(
            ups_square_corner(UPSBand::Z, (GridLetter::J, GridLetter::P)).unwrap(),
            (2_600_000.0, 2_600_000.0)
        );
        assert_eq!(
            ups_square_corner(UPSBand::A, (GridLetter::J, GridLetter::A)).unwrap(),
            (800_000.0, 800_000.0)
        );
        assert_eq!(
            ups_square_corner(UPSBand::B, (GridLetter::R, GridLetter::Z)).unwrap(),
            (3_100_000.0, 3_100_000.0)
        );
        assert!(ups_square_corner(UPSBand::Y, (GridLetter::Q, GridLetter::A)).is_err());
        assert!(ups_square_corner(UPSBand::Z, (GridLetter::K, GridLetter::P)).is_err());
    }

    #[test]
    fn band_of_latitude() {
        assert_eq!(get_utm_band(-80.0), Some(UTMBand::C));
        assert_eq!(get_utm_band(-0.0), Some(UTMBand::N));
        assert_eq!(get_utm_band(72.0), Some(UTMBand::X));
        assert_eq!(get_utm_band(84.0), Some(UTMBand::X));
        assert_eq!(get_utm_band(84.000001), None);
        assert_eq!(get_utm_band(-80.000001), None);
    }

    #[test]
    fn zone_exceptions() {
        assert_eq!(utm_zone(56.0, 3.0), 32);
        assert_eq!(utm_zone(56.0, 2.999), 31);
        assert_eq!(utm_zone(64.0, 11.999), 32);
        assert_eq!(utm_zone(72.0, 0.0), 31);
        assert_eq!(utm_zone(84.0, 8.999), 31);
        assert_eq!(utm_zone(84.0, 9.0), 33);
        assert_eq!(utm_zone(80.0, 21.0), 35);
        assert_eq!(utm_zone(80.0, 33.0), 37);
        assert_eq!(utm_zone(80.0, 42.0), 38);
        assert_eq!(utm_zone(0.0, -180.0), 1);
        assert_eq!(utm_zone(0.0, 179.999), 60);
    }
}
