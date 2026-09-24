//! Transverse Mercator kernel: Krüger series in the third flattening to order n⁶
//! (Karney, "Transverse Mercator with an accuracy of a few nanometers", 2011, eq. 35/36).
//! Truncation error inside the UTM extent, 6°-wide zones included, is below 5 nm.

use crate::wgs84::*;

pub(crate) const TM_N: f64 = WGS84_FLATTENING / (2.0 - WGS84_FLATTENING);
pub(crate) const TM_N2: f64 = TM_N * TM_N;
pub(crate) const TM_N3: f64 = TM_N2 * TM_N;
pub(crate) const TM_N4: f64 = TM_N2 * TM_N2;
pub(crate) const TM_N5: f64 = TM_N4 * TM_N;
pub(crate) const TM_N6: f64 = TM_N4 * TM_N2;
pub(crate) const TM_A: f64 =
    WGS84_SEMI_MAJOR_AXIS / (1.0 + TM_N) * (1.0 + TM_N2 / 4.0 + TM_N4 / 64.0 + TM_N6 / 256.0);
pub(crate) const TM_ALPHA: [f64; 6] = [
    TM_N / 2.0 - 2.0 / 3.0 * TM_N2 + 5.0 / 16.0 * TM_N3 + 41.0 / 180.0 * TM_N4
        - 127.0 / 288.0 * TM_N5
        + 7891.0 / 37800.0 * TM_N6,
    13.0 / 48.0 * TM_N2 - 3.0 / 5.0 * TM_N3 + 557.0 / 1440.0 * TM_N4 + 281.0 / 630.0 * TM_N5
        - 1_983_433.0 / 1_935_360.0 * TM_N6,
    61.0 / 240.0 * TM_N3 - 103.0 / 140.0 * TM_N4
        + 15061.0 / 26880.0 * TM_N5
        + 167_603.0 / 181_440.0 * TM_N6,
    49561.0 / 161_280.0 * TM_N4 - 179.0 / 168.0 * TM_N5 + 6_601_661.0 / 7_257_600.0 * TM_N6,
    34729.0 / 80640.0 * TM_N5 - 3_418_889.0 / 1_995_840.0 * TM_N6,
    212_378_941.0 / 319_334_400.0 * TM_N6,
];
pub(crate) const TM_BETA: [f64; 6] = [
    TM_N / 2.0 - 2.0 / 3.0 * TM_N2 + 37.0 / 96.0 * TM_N3
        - 1.0 / 360.0 * TM_N4
        - 81.0 / 512.0 * TM_N5
        + 96199.0 / 604_800.0 * TM_N6,
    TM_N2 / 48.0 + TM_N3 / 15.0 - 437.0 / 1440.0 * TM_N4 + 46.0 / 105.0 * TM_N5
        - 1_118_711.0 / 3_870_720.0 * TM_N6,
    17.0 / 480.0 * TM_N3 - 37.0 / 840.0 * TM_N4 - 209.0 / 4480.0 * TM_N5 + 5569.0 / 90720.0 * TM_N6,
    4397.0 / 161_280.0 * TM_N4 - 11.0 / 504.0 * TM_N5 - 830_251.0 / 7_257_600.0 * TM_N6,
    4583.0 / 161_280.0 * TM_N5 - 108_847.0 / 3_991_680.0 * TM_N6,
    20_648_693.0 / 638_668_800.0 * TM_N6,
];

// tau' = tan(conformal latitude) as a function of tau = tan(phi).
pub(crate) fn taupf(tau: f64) -> f64 {
    let tau1 = tau.hypot(1.0);
    let sig = (WGS84_FIRST_ECCENTRICITY * (WGS84_FIRST_ECCENTRICITY * tau / tau1).atanh()).sinh();
    tau * sig.hypot(1.0) - sig * tau1
}

// Inverse of taupf by Newton; quadratic convergence, the break tolerance leaves an O(1e-18) residual.
pub(crate) fn tauf(taup: f64) -> f64 {
    let e2m = 1.0 - WGS84_FIRST_ECCENTRICITY_SQUARED;
    let mut tau = taup / e2m;
    let stol = 0.1 * f64::EPSILON.sqrt() * taup.abs().max(1.0);
    for _ in 0..5 {
        let taupa = taupf(tau);
        let dtau =
            (taup - taupa) * (1.0 + e2m * tau * tau) / (e2m * tau.hypot(1.0) * taupa.hypot(1.0));
        tau += dtau;
        if dtau.abs() < stol {
            break;
        }
    }
    tau
}

// (sum c_j sin(2j xi) cosh(2j eta), sum c_j cos(2j xi) sinh(2j eta))
pub(crate) fn tm_series(coef: &[f64; 6], xi: f64, eta: f64) -> (f64, f64) {
    let mut re = 0.0;
    let mut im = 0.0;
    for (j, c) in coef.iter().enumerate() {
        let k = 2.0 * (j + 1) as f64;
        re += c * (k * xi).sin() * (k * eta).cosh();
        im += c * (k * xi).cos() * (k * eta).sinh();
    }
    (re, im)
}

// Degrees in; (x east of the central meridian, y north of the equator), k0-scaled, no false offsets.
pub(crate) fn tm_forward(lat: f64, dlon: f64) -> (f64, f64) {
    let taup = taupf(lat.to_radians().tan());
    let (sl, cl) = dlon.to_radians().sin_cos();
    let xip = taup.atan2(cl);
    let etap = (sl / taup.hypot(cl)).asinh();
    let (dxi, deta) = tm_series(&TM_ALPHA, xip, etap);
    let k = UTM_CENTRAL_SCALE_FACTOR * TM_A;
    (k * (etap + deta), k * (xip + dxi))
}

// Inverse of tm_forward; returns (lat, dlon) in degrees.
pub(crate) fn tm_inverse(x: f64, y: f64) -> (f64, f64) {
    let k = UTM_CENTRAL_SCALE_FACTOR * TM_A;
    let (xi, eta) = (y / k, x / k);
    let (dxi, deta) = tm_series(&TM_BETA, xi, eta);
    let (xip, etap) = (xi - dxi, eta - deta);
    let (sx, cx) = xip.sin_cos();
    let sh = etap.sinh();
    let taup = sx / sh.hypot(cx);
    (tauf(taup).atan().to_degrees(), sh.atan2(cx).to_degrees())
}

pub(crate) fn utm_central_meridian(zone: u8) -> f64 {
    zone as f64 * 6.0 - 183.0
}

pub(crate) fn utm_inverse(zone: u8, south: bool, easting: f64, northing: f64) -> (f64, f64) {
    let y = northing - if south { UTM_FALSE_NORTHING } else { 0.0 };
    let (lat, dlon) = tm_inverse(easting - UTM_FALSE_EASTING, y);
    (lat, utm_central_meridian(zone) + dlon)
}

// Longitude is canonical [-180, 180), so the raw zone is always 1..=60.

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rectifying_radius_matches_reference() {
        assert!((TM_A - 6_367_449.145_823_415).abs() < 1e-6);
    }

    #[test]
    fn forward_inverse_round_trip_is_sub_micron() {
        for &(lat, dlon) in &[
            (0.0, 0.0),
            (44.8, 1.47),
            (-33.9, -2.9),
            (63.99, -5.99),
            (83.99, 5.99),
            (-79.99, 2.99),
        ] {
            let (x, y) = tm_forward(lat, dlon);
            let (lat2, dlon2) = tm_inverse(x, y);
            assert!(
                (lat2 - lat).abs() < 1e-11 && (dlon2 - dlon).abs() < 1e-11,
                "{lat} {dlon}"
            );
        }
    }

    #[test]
    fn equator_and_central_meridian_map_to_zero() {
        assert_eq!(tm_forward(0.0, 0.0), (0.0, 0.0));
        assert_eq!(tm_inverse(0.0, 0.0), (0.0, 0.0));
    }
}
