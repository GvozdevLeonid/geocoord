//! Reference behaviour: GEOTRANS vectors, zone exceptions, validation, canonical forms, precision.

use geocoord::*;

fn dd(lat: f64, lon: f64) -> DD {
    DD::new(lat, lon).unwrap()
}

fn mgrs(lat: f64, lon: f64) -> String {
    format!("{:.5}", MGRS::from(dd(lat, lon))).replace(' ', "")
}

#[test]
fn geotrans_vectors() {
    assert_eq!(mgrs(44.8, 20.47), "34TDQ5808260869");
    assert_eq!(mgrs(85.0, -100.0), "YSH5298196454");
    assert_eq!(mgrs(85.0, 100.0), "ZHH4701896454");
    assert_eq!(mgrs(-85.0, -100.0), "ASM5298103545");
    assert_eq!(mgrs(-85.0, 100.0), "BHM4701803545");
    assert_eq!(mgrs(90.0, 123.0), "ZAH0000000000");
    assert_eq!(mgrs(-90.0, 0.0), "BAN0000000000");
    assert_eq!(mgrs(84.0, 0.0), "31XDP6500529005");
    let utm = UTM::try_from(dd(44.8, 20.47)).unwrap();
    assert_eq!((utm.zone_number(), utm.band()), (34, UTMBand::T));
    assert!(
        (utm.easting() - 458_082.8218).abs() < 1e-3
            && (utm.northing() - 4_960_869.9371).abs() < 1e-3
    );
    let ups = UPS::try_from(dd(85.0, -100.0)).unwrap();
    assert_eq!(ups.band(), UPSBand::Y);
    assert!(
        (ups.easting() - 1_452_981.2545).abs() < 1e-3
            && (ups.northing() - 2_096_454.1638).abs() < 1e-3
    );
}

#[test]
fn antimeridian_and_polar_seam_have_one_encoding() {
    assert_eq!(dd(10.0, 180.0), dd(10.0, -180.0));
    assert_eq!(mgrs(10.0, 180.0), "01PAM7107106908");
    assert_eq!(mgrs(-80.0, 180.0), "01CDM4186716915");
    assert_eq!(mgrs(-85.0, 180.0), "BAG0000044542");
    assert_eq!(mgrs(-85.0, -180.0), "BAG0000044542");
    assert_eq!(mgrs(-80.0, -180.0), mgrs(-80.0, 180.0));
}

#[test]
fn zone_exceptions_and_polar_dispatch() {
    let zone = |lat: f64, lon: f64| UTM::try_from(dd(lat, lon)).unwrap().zone_number();
    assert_eq!(zone(56.0, 3.0), 32);
    assert_eq!(zone(56.0, 2.9999), 31);
    assert_eq!(zone(63.9999, 11.9999), 32);
    assert_eq!(zone(64.0, 11.9999), 32);
    assert_eq!(zone(72.0, 0.0), 31);
    assert_eq!(zone(71.9999, 0.0), 31);
    assert_eq!(zone(84.0, 8.9999), 31);
    assert_eq!(zone(84.0, 9.0), 33);
    assert_eq!(zone(80.0, 21.0), 35);
    assert_eq!(zone(80.0, 33.0), 37);
    assert_eq!(zone(84.0, 42.0), 38);
    assert!(UniversalCoord::from(dd(84.0, 0.0)).is_utm());
    assert!(UniversalCoord::from(dd(84.0000001, 0.0)).is_ups());
    assert!(UniversalCoord::from(dd(-80.0, 0.0)).is_utm());
    assert!(UniversalCoord::from(dd(-80.0000001, 0.0)).is_ups());
    assert!(UTM::try_from(dd(84.0000001, 0.0)).is_err());
    assert!(UPS::try_from(dd(83.4, 45.0)).is_err());
    assert!(UPS::try_from(dd(83.6, 45.0)).is_ok());
    assert!(UPS::try_from(dd(-79.6, 0.0)).is_ok());
    assert!(matches!(
        UPS::try_from(dd(83.6, 0.0)),
        Err(CoordError::OutOfProjectionBounds(_))
    ));
}

#[test]
fn impossible_references_are_rejected() {
    for s in [
        "01CAA0000000000",
        "32XMS1234512345",
        "34XDA1234512345",
        "11MLL1473598808",
        "YRA0000000000",
        "YQA0000000000",
        "34TIQ5808260869",
        "31TDQ",
        "01MAA6602100001",
    ] {
        assert!(s.parse::<MGRS>().is_err(), "{s}");
    }
    for s in [
        "31UDQ",
        "31UDQ0000000000",
        "01MAA6602100000",
        "33XVA0000000000",
        "ZAP0000066727",
        "YZA0000000000",
    ] {
        assert!(s.parse::<MGRS>().is_ok(), "{s}");
    }
    assert!(UTM::new(33, UTMBand::N, 500_000.0, 9_000_000.0).is_err());
    assert!(UTM::new(33, UTMBand::X, 500_000.0, 100.0).is_err());
    assert!(UTM::new(33, UTMBand::T, 500_000.0, 10_000_000.0).is_err());
    assert!(UTM::new(32, UTMBand::X, 500_000.0, 8_000_000.0).is_err());
    assert!(UTM::new(33, UTMBand::X, 500_000.0, 9_400_000.0).is_err());
    assert_eq!(
        DD::from(UTM::new(33, UTMBand::M, 500_000.0, 10_000_000.0).unwrap()),
        dd(0.0, 15.0)
    );
    assert!(UPS::new(UPSBand::Y, 1_300_000.0, 1_300_000.0).is_err());
    assert!(UPS::new(UPSBand::A, 800_000.0, 800_000.0).is_err());
    assert!(UPS::new(UPSBand::Z, 2_000_000.0, 2_000_000.0).is_ok());
}

#[test]
fn every_conversion_output_revalidates() {
    let mut x = 0x9E37_79B9_7F4A_7C15u64;
    let mut rnd = move || {
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        (x >> 11) as f64 / (1u64 << 53) as f64
    };
    for _ in 0..20_000 {
        let p = dd(rnd() * 180.0 - 90.0, rnd() * 360.0 - 180.0);
        let uc: UniversalCoord = p.into();
        match uc {
            UniversalCoord::UTM(u) => {
                assert!(UTM::new(u.zone_number(), u.band(), u.easting(), u.northing()).is_ok())
            }
            UniversalCoord::UPS(u) => {
                assert!(UPS::new(u.band(), u.easting(), u.northing()).is_ok())
            }
        }
        let m: MGRS = uc.into();
        assert!(MGRS::new(m.zone(), m.square(), m.easting(), m.northing()).is_ok());
        let back: DD = m.into();
        let mut dlon = (back.longitude() - p.longitude()).abs();
        if dlon > 180.0 {
            dlon = 360.0 - dlon;
        }
        if p.latitude().abs() > 89.9999 {
            dlon = 0.0;
        }
        assert!(
            (back.latitude() - p.latitude()).abs() < 1e-9 && dlon < 1e-9,
            "{p}"
        );
    }
}

#[test]
fn cell_corners_survive_a_round_trip() {
    for &(lat, lon) in &[
        (56.01, 3.01),
        (63.99, 11.99),
        (83.9, 41.9),
        (-79.9, 179.9),
        (0.0, 15.0),
        (45.0, -122.0),
    ] {
        let s = format!("{:.5}", MGRS::from(dd(lat, lon)));
        let m: MGRS = s.parse().unwrap();
        let again: MGRS = DD::from(m).into();
        assert_eq!(format!("{:.5}", again), s);
    }
}

#[test]
fn canonical_forms() {
    let z = DD::new(-0.0, 180.0).unwrap();
    assert!(z.latitude().is_sign_positive() && z.longitude() == -180.0);
    assert_eq!(format!("{:.3}", z), "  0.000, -180.000");
    assert_eq!(
        format!(
            "{}",
            DDM::new(LatitudeDir::North, 45, -0.0, LongitudeDir::East, 20, 0.0).unwrap()
        ),
        "45°00.000'N 020°00.000'E"
    );
    assert_eq!(
        format!(
            "{}",
            DMS::new(
                LatitudeDir::North,
                45,
                0,
                -0.0,
                LongitudeDir::East,
                20,
                0,
                0.0
            )
            .unwrap()
        ),
        "45°00'00.00\"N 020°00'00.00\"E"
    );
    assert_eq!(
        format!("{}", UTM::new(33, UTMBand::N, 500_000.0, -0.0).unwrap()),
        "33N 500000 0000000"
    );
    let ddm: DDM = DDM::new(LatitudeDir::South, 0, 0.0, LongitudeDir::West, 0, 0.0).unwrap();
    assert!(DD::from(ddm).latitude().is_sign_positive());
}

#[test]
fn sexagesimal_carry() {
    let dms = DMS::new(
        LatitudeDir::North,
        45,
        59,
        59.999_999_999_999_99,
        LongitudeDir::East,
        20,
        0,
        0.0,
    )
    .unwrap();
    let ddm: DDM = dms.into();
    assert_eq!((ddm.latitude_degrees(), ddm.latitude_minutes()), (46, 0.0));
    assert_eq!(
        format!("{:.2}", DDM::from(dd(89.9999999, 179.9999999))),
        "90°00.00'N 180°00.00'E"
    );
    assert_eq!(
        format!("{:.0}", DMS::from(dd(59.999999, 9.999999))),
        "60°00'00\"N 010°00'00\"E"
    );
}

#[test]
fn display_formats() {
    let p = dd(44.812345678, 20.461234567);
    assert_eq!(format!("{}", p), " 44.812346,   20.461235");
    assert_eq!(format!("{}", DDM::from(p)), "44°48.741'N 020°27.674'E");
    assert_eq!(
        format!("{}", DMS::from(p)),
        "44°48'44.44\"N 020°27'40.44\"E"
    );
    assert_eq!(format!("{}", UniversalCoord::from(p)), "34T 457399 4962246");
    assert_eq!(
        format!("{:.3}", UTM::try_from(p).unwrap()),
        "34T 457398.660 4962245.895"
    );
    let m = MGRS::from(p);
    assert_eq!(format!("{}", m), "34T DQ 57398 62245");
    assert_eq!(format!("{:.0}", m), "34T DQ");
    assert_eq!(format!("{:.2}", m), "34T DQ 57 62");
    assert_eq!(format!("{:.9}", m), "34T DQ 573986600 622458950");
    assert_eq!(
        format!("{}", UniversalCoord::from(dd(90.0, 0.0))),
        "Z 2000000 2000000"
    );
}
