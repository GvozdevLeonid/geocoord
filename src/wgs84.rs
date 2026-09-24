//! WGS84 ellipsoid and NGA projection parameters shared by the projection kernels and the grid logic.

pub(crate) const WGS84_SEMI_MAJOR_AXIS: f64 = 6378137.0;
pub(crate) const WGS84_FLATTENING: f64 = 1.0 / 298.257223563;
pub(crate) const WGS84_FIRST_ECCENTRICITY: f64 = 0.0818191908426215;
pub(crate) const WGS84_FIRST_ECCENTRICITY_SQUARED: f64 =
    WGS84_FLATTENING * (2.0 - WGS84_FLATTENING);

pub(crate) const UTM_CENTRAL_SCALE_FACTOR: f64 = 0.9996;
pub(crate) const UTM_FALSE_EASTING: f64 = 500_000.0;
pub(crate) const UTM_FALSE_NORTHING: f64 = 10_000_000.0;
pub(crate) const UTM_LAT_MIN: f64 = -80.0;
pub(crate) const UTM_LAT_MAX: f64 = 84.0;

pub(crate) const UPS_CENTRAL_SCALE_FACTOR: f64 = 0.994;
pub(crate) const UPS_FALSE_EASTING: f64 = 2_000_000.0;
pub(crate) const UPS_FALSE_NORTHING: f64 = 2_000_000.0;
// NGA: UPS zones reach 83.5°N / 79.5°S, overlapping UTM by 0.5°.
pub(crate) const UPS_NORTH_LAT_MIN: f64 = 83.5;
pub(crate) const UPS_SOUTH_LAT_MAX: f64 = -79.5;

// Slack for latitude-domain checks. Absorbs projection noise of other producers
// (GEOTRANS inverse TM: ~5e-7°); never large enough to admit a wrong band.
pub(crate) const LAT_TOLERANCE_DEG: f64 = 1e-6;

pub(crate) const MGRS_SQUARE: f64 = 100_000.0;
// UTM row letters repeat every 2 000 km of northing.
pub(crate) const MGRS_ROW_PERIOD: f64 = 2_000_000.0;
