//! Universal Polar Stereographic kernel (Snyder 21-33 / 21-30..31 forward, 7-9 inverse).

use crate::wgs84::*;

pub(crate) fn ups_c() -> f64 {
    let e = WGS84_FIRST_ECCENTRICITY;
    ((1.0 + e).powf(1.0 + e) * (1.0 - e).powf(1.0 - e)).sqrt()
}

// Polar stereographic (Snyder 21-33 / 21-30..31); returns (easting, northing).
pub(crate) fn ups_forward(north: bool, lat: f64, lon: f64) -> (f64, f64) {
    let phi = lat.abs().to_radians();
    let esin = WGS84_FIRST_ECCENTRICITY * phi.sin();
    let t = (std::f64::consts::FRAC_PI_4 - phi / 2.0).tan()
        * ((1.0 + esin) / (1.0 - esin)).powf(WGS84_FIRST_ECCENTRICITY / 2.0);
    let rho = 2.0 * WGS84_SEMI_MAJOR_AXIS * UPS_CENTRAL_SCALE_FACTOR * t / ups_c();
    // sin(±180°) is ±1.2e-16, not 0: the antimeridian would land on the Y/Z (A/B) seam by rounding luck.
    let (s, c) = if lon.abs() == 180.0 {
        (0.0, -1.0)
    } else {
        lon.to_radians().sin_cos()
    };
    let northing = if north {
        UPS_FALSE_NORTHING - rho * c
    } else {
        UPS_FALSE_NORTHING + rho * c
    };
    (UPS_FALSE_EASTING + rho * s, northing)
}

// Inverse of ups_forward; returns (lat, lon) in degrees, lon = 0 at the pole itself.
pub(crate) fn ups_inverse(north: bool, easting: f64, northing: f64) -> (f64, f64) {
    let dx = easting - UPS_FALSE_EASTING;
    let dy = northing - UPS_FALSE_NORTHING;
    let rho = dx.hypot(dy);
    let t = rho * ups_c() / (2.0 * WGS84_SEMI_MAJOR_AXIS * UPS_CENTRAL_SCALE_FACTOR);
    // Snyder 7-9 fixed point; contraction ratio e^2 per step, so 1e-15 rad is reached in <= 8 steps.
    let mut phi = std::f64::consts::FRAC_PI_2 - 2.0 * t.atan();
    for _ in 0..12 {
        let esin = WGS84_FIRST_ECCENTRICITY * phi.sin();
        let next = std::f64::consts::FRAC_PI_2
            - 2.0 * (t * ((1.0 - esin) / (1.0 + esin)).powf(WGS84_FIRST_ECCENTRICITY / 2.0)).atan();
        let converged = (next - phi).abs() < 1e-15;
        phi = next;
        if converged {
            break;
        }
    }
    let lat = if north { phi } else { -phi }.to_degrees();
    let lon = if rho == 0.0 {
        0.0
    } else if north {
        dx.atan2(-dy)
    } else {
        dx.atan2(dy)
    };
    (lat, lon.to_degrees())
}
