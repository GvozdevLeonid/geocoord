//! `FromStr` for every coordinate type. Angular notations share one tokenizer; grid coordinates
//! take an uppercase band letter (exact) or a lowercase n/s hemisphere from which the band is derived.

use crate::grid::*;
use crate::tm::utm_inverse;
use crate::wgs84::*;
use crate::{
    CoordError, LatitudeDir, LongitudeDir, MGRSZone, UPSBand, UniversalCoord, DD, DDM, DMS, MGRS,
    UPS, UTM,
};

const UTM_SYNTAX: &str = "expected `<zone><band> <easting> <northing>`, e.g. `34T 458083 4960870` (lowercase n/s = hemisphere instead of band)";
const UPS_SYNTAX: &str = "expected `<band> <easting> <northing>`, e.g. `Z 2000000 1333272` (lowercase n/s = hemisphere instead of band)";
const MGRS_SYNTAX: &str =
    "expected `<zone><band><square><digits>`, e.g. `34TDQ5808260869` or `Z AH 00000 00000`";

fn parse_metres(s: &str) -> Result<f64, CoordError> {
    match s.parse::<f64>() {
        Ok(v) if v.is_finite() => Ok(v),
        _ => Err(CoordError::InvalidFormat("malformed number")),
    }
}

// `<letters><digits>` head token: returns the leading letter and whatever follows it (usually empty or the easting).
fn split_letter_head(token: &str) -> Option<(char, &str)> {
    let mut chars = token.chars();
    let c = chars.next()?;
    Some((c, chars.as_str()))
}

// Grid-coordinate parsers accept an uppercase band letter (exact) or a lowercase n/s hemisphere,
// from which the band is derived by inverse projection: `34N` is band N (0–8°N), `34n` is "north".
fn parse_grid_tokens<'a>(
    s: &'a str,
    syntax: &'static str,
) -> Result<(u8, char, f64, f64), CoordError> {
    let mut parts = s.split_whitespace();
    let head = parts.next().ok_or(CoordError::InvalidFormat(syntax))?;
    let digits = head.chars().take_while(|c| c.is_ascii_digit()).count();
    if digits > 2 {
        return Err(CoordError::InvalidFormat(syntax));
    }
    let zone: u8 = if digits == 0 {
        0
    } else {
        head[..digits]
            .parse()
            .map_err(|_| CoordError::InvalidFormat(syntax))?
    };
    let letter_token = if digits == head.len() {
        parts.next().ok_or(CoordError::InvalidFormat(syntax))?
    } else {
        &head[digits..]
    };
    let (letter, glued) =
        split_letter_head(letter_token).ok_or(CoordError::InvalidFormat(syntax))?;
    let mut numbers: Vec<&str> = Vec::with_capacity(2);
    if !glued.is_empty() {
        numbers.push(glued);
    }
    numbers.extend(parts);
    if numbers.len() != 2 {
        return Err(CoordError::InvalidFormat(syntax));
    }
    Ok((
        zone,
        letter,
        parse_metres(numbers[0])?,
        parse_metres(numbers[1])?,
    ))
}

impl std::str::FromStr for UTM {
    type Err = CoordError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (zone, letter, easting, northing) = parse_grid_tokens(s, UTM_SYNTAX)?;
        if zone == 0 {
            return Err(CoordError::InvalidFormat(UTM_SYNTAX));
        }
        let band = match letter {
            'n' | 's' => {
                if !(1..=60).contains(&zone) {
                    return Err(CoordError::InvalidUtmZone(zone));
                }
                let (lat, _) = utm_inverse(zone, letter == 's', easting, northing);
                get_utm_band(lat).ok_or(CoordError::IncompatibleProjection("UTM northing is outside the 80°S–84°N latitude range"))?
            }
            c => utm_band_from_char(c).ok_or(CoordError::InvalidFormat(
                "band letter must be uppercase C..X without I and O, or lowercase n/s for the hemisphere"
            ))?,
        };
        match UTM::new(zone, band, easting, northing) {
            // The classic mistake: "34N" read as "north". Uppercase N and S are the 0–8°N and 32–40°N bands.
            Err(CoordError::OutOfProjectionBounds(_)) if matches!(letter, 'N' | 'S') => Err(CoordError::InvalidFormat(
                "uppercase N/S are latitude bands (0–8°N, 32–40°N) and do not match these coordinates; use lowercase n/s for the hemisphere"
            )),
            r => r,
        }
    }
}

impl std::str::FromStr for UPS {
    type Err = CoordError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (zone, letter, easting, northing) = parse_grid_tokens(s, UPS_SYNTAX)?;
        if zone != 0 {
            return Err(CoordError::InvalidFormat(UPS_SYNTAX));
        }
        let band = match letter {
            'n' | 's' => ups_band(letter == 'n', easting),
            c => ups_band_from_char(c).ok_or(CoordError::InvalidFormat(
                "band letter must be one of A, B, Y, Z, or lowercase n/s for the hemisphere",
            ))?,
        };
        // Easting exactly 2 000 000 is the A|B / Y|Z seam and belongs to the east half; a west
        // letter with it is what `Display` prints for a point a few decimetres west of the seam.
        let band = match band {
            UPSBand::A if easting == UPS_FALSE_EASTING => UPSBand::B,
            UPSBand::Y if easting == UPS_FALSE_EASTING => UPSBand::Z,
            b => b,
        };
        UPS::new(band, easting, northing)
    }
}

impl std::str::FromStr for UniversalCoord {
    type Err = CoordError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().chars().next() {
            Some(c) if c.is_ascii_digit() => s.parse().map(UniversalCoord::UTM),
            Some(_) => s.parse().map(UniversalCoord::UPS),
            None => Err(CoordError::InvalidFormat(UTM_SYNTAX)),
        }
    }
}

impl std::str::FromStr for MGRSZone {
    type Err = CoordError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let t: Vec<char> = s.trim().chars().map(|c| c.to_ascii_uppercase()).collect();
        let digits = t.iter().take_while(|c| c.is_ascii_digit()).count();
        if digits == 0 {
            return match t.as_slice() {
                [c] => ups_band_from_char(*c)
                    .map(MGRSZone::UPS)
                    .ok_or(CoordError::InvalidFormat(
                        "UPS zone must be one of A, B, Y, Z",
                    )),
                _ => Err(CoordError::InvalidFormat(
                    "expected `<zone><band>` such as `34T`, or a UPS zone letter",
                )),
            };
        }
        if digits > 2 || t.len() != digits + 1 {
            return Err(CoordError::InvalidFormat(
                "expected `<zone><band>` such as `34T`, or a UPS zone letter",
            ));
        }
        let zone: u8 = t[..digits]
            .iter()
            .collect::<String>()
            .parse()
            .map_err(|_| CoordError::InvalidFormat("malformed zone number"))?;
        if !(1..=60).contains(&zone) {
            return Err(CoordError::InvalidUtmZone(zone));
        }
        let band = utm_band_from_char(t[digits]).ok_or(CoordError::InvalidFormat(
            "band letter must be one of C..X without I and O",
        ))?;
        Ok(MGRSZone::UTM(zone, band))
    }
}

impl std::str::FromStr for MGRS {
    type Err = CoordError;
    // Whitespace is ignored anywhere, letters are case-insensitive: `34TDQ5808260869`, `34T DQ 58082 60869`,
    // `z ah` all parse. Digit pairs give the precision (0..=9 pairs); values are the square offsets.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let t: Vec<char> = s
            .chars()
            .filter(|c| !c.is_whitespace())
            .map(|c| c.to_ascii_uppercase())
            .collect();
        let digits = t.iter().take_while(|c| c.is_ascii_digit()).count();
        if digits > 2 || t.len() < digits + 3 {
            return Err(CoordError::InvalidFormat(MGRS_SYNTAX));
        }
        let zone_end = if digits == 0 { 1 } else { digits + 1 };
        let zone: MGRSZone = t[..zone_end].iter().collect::<String>().parse()?;
        let col = grid_letter_from_char(t[zone_end]).ok_or(CoordError::InvalidFormat(
            "square letters must be A..Z without I and O",
        ))?;
        let row = grid_letter_from_char(t[zone_end + 1]).ok_or(CoordError::InvalidFormat(
            "square letters must be A..Z without I and O",
        ))?;
        let rest = &t[zone_end + 2..];
        if rest.iter().any(|c| !c.is_ascii_digit()) {
            return Err(CoordError::InvalidFormat(MGRS_SYNTAX));
        }
        if rest.len() % 2 != 0 || rest.len() > 18 {
            return Err(CoordError::InvalidFormat(
                "MGRS digits must come in 0 to 9 pairs (easting then northing)",
            ));
        }
        let p = rest.len() / 2;
        let offset = |d: &[char]| -> Result<f64, CoordError> {
            if p == 0 {
                return Ok(0.0);
            }
            let v: u64 = d
                .iter()
                .collect::<String>()
                .parse()
                .map_err(|_| CoordError::InvalidFormat(MGRS_SYNTAX))?;
            Ok(if p >= 5 {
                v as f64 / 10f64.powi(p as i32 - 5)
            } else {
                v as f64 * 10f64.powi(5 - p as i32)
            })
        };
        MGRS::new(zone, (col, row), offset(&rest[..p])?, offset(&rest[p..])?)
    }
}

// ---- angular notations (DD / DDM / DMS) --------------------------------------------------------

#[derive(Clone, Copy, PartialEq)]
enum AngleUnit {
    Deg,
    Min,
    Sec,
}

#[derive(Clone, Copy)]
struct AngleNum {
    value: f64,
    signed: bool,
    negative: bool,
    unit: Option<AngleUnit>,
}

#[derive(Clone, Copy)]
enum AngleTok {
    Num(AngleNum),
    Hemi(char),
    Split,
}

const ANGLE_AMBIGUOUS: &str =
    "expected latitude and longitude (1 to 3 numbers each); separate them with a comma";

// Numbers with an optional sign, unit marks (° º ˚, ' ′ ’, " ″ ” ''), ':' between components,
// hemisphere letters N S E W (any case, before or after the numbers) and one ',' ';' or '/'
// between the two angles. Anything else is an error; the decimal separator is '.'.
fn tokenize_angles(s: &str) -> Result<Vec<AngleTok>, CoordError> {
    let chars: Vec<char> = s.chars().collect();
    let mut toks = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c.is_whitespace() {
            i += 1;
            continue;
        }
        if matches!(c, ',' | ';' | '/') {
            toks.push(AngleTok::Split);
            i += 1;
            continue;
        }
        if matches!(c, '+' | '-' | '\u{2212}') || c.is_ascii_digit() || c == '.' {
            let signed = !(c.is_ascii_digit() || c == '.');
            let negative = matches!(c, '-' | '\u{2212}');
            if signed {
                i += 1;
            }
            let start = i;
            while i < chars.len() && (chars[i].is_ascii_digit() || chars[i] == '.') {
                i += 1;
            }
            let text: String = chars[start..i].iter().collect();
            let value = match text.parse::<f64>() {
                Ok(v) if v.is_finite() && text.chars().any(|c| c.is_ascii_digit()) => v,
                _ => return Err(CoordError::InvalidFormat("malformed number")),
            };
            let mut j = i;
            while j < chars.len() && chars[j].is_whitespace() {
                j += 1;
            }
            let mut unit = None;
            if let Some(&m) = chars.get(j) {
                match m {
                    '\u{00B0}' | '\u{00BA}' | '\u{02DA}' => {
                        unit = Some(AngleUnit::Deg);
                        i = j + 1;
                    }
                    '\'' | '\u{2032}' | '\u{2019}' | '\u{2018}' => {
                        unit = Some(AngleUnit::Min);
                        i = j + 1;
                        if chars.get(i) == Some(&'\'') {
                            unit = Some(AngleUnit::Sec);
                            i += 1;
                        }
                    }
                    '"' | '\u{2033}' | '\u{201D}' | '\u{201C}' => {
                        unit = Some(AngleUnit::Sec);
                        i = j + 1;
                    }
                    ':' => {
                        i = j + 1;
                    }
                    _ => {}
                }
            }
            toks.push(AngleTok::Num(AngleNum {
                value,
                signed,
                negative,
                unit,
            }));
            continue;
        }
        if c.is_alphabetic() {
            let h = c.to_ascii_uppercase();
            if !matches!(h, 'N' | 'S' | 'E' | 'W') {
                return Err(CoordError::InvalidFormat(
                    "only the hemisphere letters N, S, E, W are allowed",
                ));
            }
            toks.push(AngleTok::Hemi(h));
            i += 1;
            continue;
        }
        return Err(CoordError::InvalidFormat("unexpected character"));
    }
    Ok(toks)
}

struct AngleGroup {
    nums: Vec<AngleNum>,
    hemi: Option<char>,
}

// `[hemi] number{1,3} [hemi]` — one side of an explicitly separated pair.
fn group_from_slice(toks: &[AngleTok]) -> Result<AngleGroup, CoordError> {
    let mut g = AngleGroup {
        nums: Vec::new(),
        hemi: None,
    };
    for t in toks {
        match t {
            AngleTok::Num(n) => g.nums.push(*n),
            AngleTok::Hemi(h) => {
                if g.hemi.is_some() {
                    return Err(CoordError::InvalidFormat(
                        "two hemisphere letters for one angle",
                    ));
                }
                g.hemi = Some(*h);
            }
            AngleTok::Split => return Err(CoordError::InvalidFormat(ANGLE_AMBIGUOUS)),
        }
    }
    Ok(g)
}

fn halves(nums: &[AngleNum]) -> Result<(Vec<AngleNum>, Vec<AngleNum>), CoordError> {
    if matches!(nums.len(), 2 | 4 | 6) {
        let (a, b) = nums.split_at(nums.len() / 2);
        Ok((a.to_vec(), b.to_vec()))
    } else {
        Err(CoordError::InvalidFormat(ANGLE_AMBIGUOUS))
    }
}

fn nums_in(toks: &[AngleTok]) -> Vec<AngleNum> {
    toks.iter()
        .filter_map(|t| {
            if let AngleTok::Num(n) = t {
                Some(*n)
            } else {
                None
            }
        })
        .collect()
}

// Splits the token stream into the two angles. With a separator the split is explicit; otherwise
// the hemisphere letters delimit the angles, and with none the numbers are halved (2, 4 or 6).
fn group_angles(toks: &[AngleTok]) -> Result<(AngleGroup, AngleGroup), CoordError> {
    let splits: Vec<usize> = toks
        .iter()
        .enumerate()
        .filter(|(_, t)| matches!(t, AngleTok::Split))
        .map(|(i, _)| i)
        .collect();
    if !splits.is_empty() {
        if splits.len() != 1 {
            return Err(CoordError::InvalidFormat(
                "expected one comma between latitude and longitude; the decimal separator is '.'",
            ));
        }
        return Ok((
            group_from_slice(&toks[..splits[0]])?,
            group_from_slice(&toks[splits[0] + 1..])?,
        ));
    }
    if !toks.iter().any(|t| matches!(t, AngleTok::Num(_))) {
        return Err(CoordError::InvalidFormat("no numbers given"));
    }
    let hemis: Vec<(usize, char)> = toks
        .iter()
        .enumerate()
        .filter_map(|(i, t)| {
            if let AngleTok::Hemi(h) = t {
                Some((i, *h))
            } else {
                None
            }
        })
        .collect();
    let group = |nums: Vec<AngleNum>, hemi: Option<char>| AngleGroup { nums, hemi };
    match hemis.as_slice() {
        [] => {
            let (a, b) = halves(&nums_in(toks))?;
            Ok((group(a, None), group(b, None)))
        }
        [(i, h)] => {
            let before = nums_in(&toks[..*i]);
            let after = nums_in(&toks[*i + 1..]);
            match (before.is_empty(), after.is_empty()) {
                (false, false) => Ok((group(before, Some(*h)), group(after, None))),
                (true, false) => {
                    let (a, b) = halves(&after)?;
                    Ok((group(a, Some(*h)), group(b, None)))
                }
                (false, true) => {
                    let (a, b) = halves(&before)?;
                    Ok((group(a, None), group(b, Some(*h))))
                }
                (true, true) => Err(CoordError::InvalidFormat("no numbers given")),
            }
        }
        [(i, h1), (j, h2)] => {
            let before = nums_in(&toks[..*i]);
            let between = nums_in(&toks[*i + 1..*j]);
            let after = nums_in(&toks[*j + 1..]);
            match (before.is_empty(), between.is_empty(), after.is_empty()) {
                (false, false, true) => Ok((group(before, Some(*h1)), group(between, Some(*h2)))),
                (true, false, false) => Ok((group(between, Some(*h1)), group(after, Some(*h2)))),
                (false, true, false) => Ok((group(before, Some(*h1)), group(after, Some(*h2)))),
                (true, false, true) => {
                    let (a, b) = halves(&between)?;
                    Ok((group(a, Some(*h1)), group(b, Some(*h2))))
                }
                _ => Err(CoordError::InvalidFormat(ANGLE_AMBIGUOUS)),
            }
        }
        _ => Err(CoordError::InvalidFormat(
            "more than two hemisphere letters",
        )),
    }
}

struct Angle {
    negative: bool,
    hemi: Option<char>,
    deg: f64,
    min: f64,
    sec: f64,
    components: usize,
}

fn angle_from_group(g: AngleGroup) -> Result<Angle, CoordError> {
    if g.nums.is_empty() || g.nums.len() > 3 {
        return Err(CoordError::InvalidFormat(
            "each angle needs 1 to 3 numbers (degrees, minutes, seconds)",
        ));
    }
    for (k, n) in g.nums.iter().enumerate() {
        if let Some(u) = n.unit {
            if u != [AngleUnit::Deg, AngleUnit::Min, AngleUnit::Sec][k] {
                return Err(CoordError::InvalidFormat(
                    "unit marks are out of order (degrees, minutes, seconds)",
                ));
            }
        }
        if k > 0 && n.signed {
            return Err(CoordError::InvalidFormat(
                "only the degrees may carry a sign",
            ));
        }
    }
    if g.nums[0].signed && g.hemi.is_some() {
        return Err(CoordError::InvalidFormat(
            "give either a sign or a hemisphere letter, not both",
        ));
    }
    let deg = g.nums[0].value;
    let min = g.nums.get(1).map_or(0.0, |n| n.value);
    let sec = g.nums.get(2).map_or(0.0, |n| n.value);
    if g.nums.len() >= 2 {
        if deg.fract() != 0.0 {
            return Err(CoordError::InvalidFormat(
                "degrees must be a whole number when minutes are given",
            ));
        }
        if min >= 60.0 {
            return Err(CoordError::InvalidMinutes(min));
        }
    }
    if g.nums.len() == 3 {
        if min.fract() != 0.0 {
            return Err(CoordError::InvalidFormat(
                "minutes must be a whole number when seconds are given",
            ));
        }
        if sec >= 60.0 {
            return Err(CoordError::InvalidSeconds(sec));
        }
    }
    Ok(Angle {
        negative: g.nums[0].negative || matches!(g.hemi, Some('S') | Some('W')),
        hemi: g.hemi,
        deg,
        min,
        sec,
        components: g.nums.len(),
    })
}

fn parse_angles(s: &str) -> Result<(Angle, Angle), CoordError> {
    let toks = tokenize_angles(s)?;
    let (a, b) = group_angles(&toks)?;
    let (a, b) = (angle_from_group(a)?, angle_from_group(b)?);
    let is_lat = |h: Option<char>| matches!(h, Some('N') | Some('S'));
    let is_lon = |h: Option<char>| matches!(h, Some('E') | Some('W'));
    match (a.hemi, b.hemi) {
        (Some(_), Some(_)) if is_lat(a.hemi) && is_lon(b.hemi) => Ok((a, b)),
        (Some(_), Some(_)) if is_lon(a.hemi) && is_lat(b.hemi) => Ok((b, a)),
        (Some(_), Some(_)) => Err(CoordError::InvalidFormat(
            "hemisphere letters must be one of N/S and one of E/W",
        )),
        (Some(_), None) => {
            if is_lat(a.hemi) {
                Ok((a, b))
            } else {
                Ok((b, a))
            }
        }
        (None, Some(_)) => {
            if is_lon(b.hemi) {
                Ok((a, b))
            } else {
                Ok((b, a))
            }
        }
        (None, None) => Ok((a, b)),
    }
}

impl Angle {
    fn decimal(&self) -> f64 {
        let v = self.deg + self.min / 60.0 + self.sec / 3600.0;
        if self.negative {
            -v
        } else {
            v
        }
    }

    // Whole degrees as u8 for the sexagesimal constructors; their own range checks follow.
    fn whole_degrees(&self) -> Result<u8, CoordError> {
        if self.deg > 180.0 {
            return Err(CoordError::InvalidFormat("degrees out of range"));
        }
        Ok(self.deg.trunc() as u8)
    }

    fn ddm(&self) -> Result<(u8, f64), CoordError> {
        let deg = self.whole_degrees()?;
        Ok(match self.components {
            1 => (deg, self.deg.fract() * 60.0),
            2 => (deg, self.min),
            _ => {
                let m = self.min + self.sec / 60.0;
                if m >= 60.0 {
                    (deg + 1, 0.0)
                } else {
                    (deg, m)
                }
            }
        })
    }

    fn dms(&self) -> Result<(u8, u8, f64), CoordError> {
        let deg = self.whole_degrees()?;
        Ok(match self.components {
            1 => {
                let minutes = self.deg.fract() * 60.0;
                (deg, minutes.trunc() as u8, minutes.fract() * 60.0)
            }
            2 => (deg, self.min.trunc() as u8, self.min.fract() * 60.0),
            _ => (deg, self.min as u8, self.sec),
        })
    }

    fn lat_dir(&self) -> LatitudeDir {
        if self.negative {
            LatitudeDir::South
        } else {
            LatitudeDir::North
        }
    }
    fn lon_dir(&self) -> LongitudeDir {
        if self.negative {
            LongitudeDir::West
        } else {
            LongitudeDir::East
        }
    }
}

impl std::str::FromStr for DD {
    type Err = CoordError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (lat, lon) = parse_angles(s)?;
        DD::new(lat.decimal(), lon.decimal())
    }
}

impl std::str::FromStr for DDM {
    type Err = CoordError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (lat, lon) = parse_angles(s)?;
        let (lat_deg, lat_min) = lat.ddm()?;
        let (lon_deg, lon_min) = lon.ddm()?;
        DDM::new(
            lat.lat_dir(),
            lat_deg,
            lat_min,
            lon.lon_dir(),
            lon_deg,
            lon_min,
        )
    }
}

impl std::str::FromStr for DMS {
    type Err = CoordError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (lat, lon) = parse_angles(s)?;
        let (lat_deg, lat_min, lat_sec) = lat.dms()?;
        let (lon_deg, lon_min, lon_sec) = lon.dms()?;
        DMS::new(
            lat.lat_dir(),
            lat_deg,
            lat_min,
            lat_sec,
            lon.lon_dir(),
            lon_deg,
            lon_min,
            lon_sec,
        )
    }
}
