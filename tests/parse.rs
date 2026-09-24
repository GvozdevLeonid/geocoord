//! `FromStr` behaviour for every type.

use geocoord::*;

fn dd(s: &str) -> (f64, f64) {
    let v: DD = s.parse().unwrap_or_else(|e| panic!("{s:?}: {e}"));
    (v.latitude(), v.longitude())
}

fn close(a: (f64, f64), b: (f64, f64)) -> bool {
    (a.0 - b.0).abs() < 1e-9 && (a.1 - b.1).abs() < 1e-9
}

#[test]
fn angular_notations() {
    let p = (44.812345678, 20.461234567);
    for s in [
        "44.812345678, 20.461234567",
        "44.812345678 20.461234567",
        "44.812345678N 20.461234567E",
        "N44.812345678 E20.461234567",
        "20.461234567E 44.812345678N",
        "44.812345678° N, 20.461234567° E",
        "44°48.74074068'N 020°27.67407402'E",
        "44°48'44.4444408\"N 020°27'40.4444412\"E",
        "44 48 44.4444408 N 20 27 40.4444412 E",
        "44 48 44.4444408 20 27 40.4444412",
        "44:48:44.4444408N 20:27:40.4444412E",
        "44°48′44.4444408″N 020°27′40.4444412″E",
        "44°48'44.4444408''N 020°27'40.4444412''E",
    ] {
        assert!(close(dd(s), p), "{s} -> {:?}", dd(s));
    }
    assert!(close(dd("-44.5 -20.25"), (-44.5, -20.25)));
    assert!(close(dd("\u{2212}44.5, \u{2212}20.25"), (-44.5, -20.25)));
    assert!(close(dd("44.5S 20.25W"), (-44.5, -20.25)));
    assert!(close(dd("90 00.000 N 180 00.000 E"), (90.0, -180.0)));
    assert!(close(dd("45 59.99999999999999 N 20 0 E"), (46.0, 20.0)));
}

#[test]
fn angular_rejections() {
    for s in [
        "",
        "N",
        "44.8123",
        "44.8 20.4 30.1",
        "44,8123 20,4612",
        "-44.8123 S 20.4612 E",
        "44.8123 N 20.4612 N",
        "44 -48.7 N 20 27.6 E",
        "44.5°48'N 20°E",
        "44 70 N 20 27 E",
        "91 20",
        "45 181",
        "44.8123 lat 20.4612 lon",
        "45 60 N 20 0 E",
        "45 59.999999999999999 N 20 0 E",
    ] {
        assert!(s.parse::<DD>().is_err(), "{s:?}");
        assert!(s.parse::<DDM>().is_err(), "{s:?}");
        assert!(s.parse::<DMS>().is_err(), "{s:?}");
    }
}

#[test]
fn sexagesimal_targets() {
    let m: DDM = "44°48'44.4\"N 020°27'40.4\"E".parse().unwrap();
    assert_eq!(
        (m.latitude_degrees(), m.latitude_dir()),
        (44, LatitudeDir::North)
    );
    assert!((m.latitude_minutes() - (48.0 + 44.4 / 60.0)).abs() < 1e-12);
    let s: DMS = "44.5 20.25".parse().unwrap();
    assert_eq!(
        (
            s.latitude_degrees(),
            s.latitude_minutes(),
            s.latitude_seconds()
        ),
        (44, 30, 0.0)
    );
    let s: DMS = "44°48.5'S 20°E".parse().unwrap();
    assert_eq!(
        (s.latitude_dir(), s.latitude_minutes(), s.latitude_seconds()),
        (LatitudeDir::South, 48, 30.0)
    );
}

#[test]
fn utm_and_ups() {
    let u: UTM = "34T 458083 4960870".parse().unwrap();
    assert_eq!(
        (u.zone_number(), u.band(), u.easting(), u.northing()),
        (34, UTMBand::T, 458_083.0, 4_960_870.0)
    );
    assert_eq!("34 T 458083 4960870".parse::<UTM>().unwrap(), u);
    assert_eq!("34T458083 4960870".parse::<UTM>().unwrap(), u);
    assert_eq!("34n 458083 4960870".parse::<UTM>().unwrap(), u);
    assert_eq!(
        "34s 458083 4960870".parse::<UTM>().unwrap().band(),
        UTMBand::G
    );
    assert!(matches!(
        "34N 458083 4960870".parse::<UTM>(),
        Err(CoordError::InvalidFormat(_))
    ));
    for s in [
        "34t 458083 4960870",
        "34T 458083",
        "340T 458083 4960870",
        "34T 458083 4960870 7",
        "34I 458083 4960870",
        "34n 458083 9400000",
        "61T 458083 4960870",
        "Z 2000000 1333272",
    ] {
        assert!(s.parse::<UTM>().is_err(), "{s}");
    }
    let p: UPS = "Z 2000000 1333272".parse().unwrap();
    assert_eq!(
        (p.band(), p.easting(), p.northing()),
        (UPSBand::Z, 2_000_000.0, 1_333_272.0)
    );
    assert_eq!("n 2000000 1333272".parse::<UPS>().unwrap(), p);
    assert_eq!(
        "A 2000000 1444542".parse::<UPS>().unwrap().band(),
        UPSBand::B
    );
    assert!("34T 458083 4960870".parse::<UPS>().is_err());
    assert!("34T 458083 4960870"
        .parse::<UniversalCoord>()
        .unwrap()
        .is_utm());
    assert!("n 2000000 1333272"
        .parse::<UniversalCoord>()
        .unwrap()
        .is_ups());
}

#[test]
fn mgrs_forms() {
    let m: MGRS = "34TDQ5808260869".parse().unwrap();
    for s in [
        "34T DQ 58082 60869",
        "34tdq5808260869",
        " 34T DQ 5808260869 ",
    ] {
        assert_eq!(s.parse::<MGRS>().unwrap(), m, "{s}");
    }
    assert_eq!((m.easting(), m.northing()), (58_082.0, 60_869.0));
    assert_eq!(
        "34T DQ 5808 6086".parse::<MGRS>().unwrap().easting(),
        58_080.0
    );
    assert_eq!("34T DQ".parse::<MGRS>().unwrap().easting(), 0.0);
    let fine: MGRS = "34TDQ580828217608699370".parse().unwrap();
    assert!(
        (fine.easting() - 58_082.8217).abs() < 1e-9 && (fine.northing() - 60_869.9370).abs() < 1e-9
    );
    assert_eq!(format!("{:.9}", fine), "34T DQ 580828217 608699370");
    assert!("Z AH 00000 00000".parse::<MGRS>().unwrap().zone() == MGRSZone::UPS(UPSBand::Z));
    assert!("BAN".parse::<MGRS>().is_ok());
    for s in [
        "34T",
        "34TDQ580826086",
        "34TDQ58082608699",
        "340TDQ5808260869",
        "34TDQ 58082 60869 x",
        "",
    ] {
        assert!(s.parse::<MGRS>().is_err(), "{s}");
    }
    assert_eq!(
        "34T".parse::<MGRSZone>().unwrap(),
        MGRSZone::UTM(34, UTMBand::T)
    );
    assert_eq!("z".parse::<MGRSZone>().unwrap(), MGRSZone::UPS(UPSBand::Z));
    assert_eq!("t".parse::<UTMBand>().unwrap(), UTMBand::T);
    assert_eq!("d".parse::<GridLetter>().unwrap(), GridLetter::D);
    assert!("i".parse::<GridLetter>().is_err());
    assert_eq!("south".parse::<LatitudeDir>().unwrap(), LatitudeDir::South);
    assert_eq!("W".parse::<LongitudeDir>().unwrap(), LongitudeDir::West);
}

#[test]
fn display_output_parses_back() {
    let mut x = 0x2545_F491_4F6C_DD1Du64;
    let mut rnd = move || {
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        (x >> 11) as f64 / (1u64 << 53) as f64
    };
    for _ in 0..5_000 {
        let p = DD::new(rnd() * 180.0 - 90.0, rnd() * 360.0 - 180.0).unwrap();
        let check = |s: String, kind: &str| {
            let q: DD = s.parse().unwrap_or_else(|e| panic!("{kind} {s:?}: {e}"));
            let mut dlon = (q.longitude() - p.longitude()).abs();
            if dlon > 180.0 {
                dlon = 360.0 - dlon;
            }
            assert!(
                (q.latitude() - p.latitude()).abs() < 1e-9 && dlon < 1e-9,
                "{kind} {s}"
            );
        };
        check(format!("{:.12}", p), "DD");
        check(format!("{:.10}", DDM::from(p)), "DDM");
        check(format!("{:.8}", DMS::from(p)), "DMS");
        let uc: UniversalCoord = p.into();
        let s = format!("{:.6}", uc);
        assert_eq!(format!("{:.6}", s.parse::<UniversalCoord>().unwrap()), s);
        let m: MGRS = p.into();
        for prec in [0usize, 3, 5, 9] {
            let s = format!("{:.p$}", m, p = prec);
            assert_eq!(format!("{:.p$}", s.parse::<MGRS>().unwrap(), p = prec), s);
        }
    }
}

#[test]
fn garbage_never_panics() {
    let alphabet: Vec<char> = "0123456789.,;/ +-°'\"′″NSEWnsewabcTtXxYZ:\u{2212}\t"
        .chars()
        .collect();
    let mut x = 0x0123_4567_89AB_CDEFu64;
    for _ in 0..20_000 {
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        let len = (x % 20) as usize;
        let s: String = (0..len)
            .map(|i| alphabet[((x >> (i % 50)) as usize + i * 7) % alphabet.len()])
            .collect();
        let _ = s.parse::<DD>();
        let _ = s.parse::<DDM>();
        let _ = s.parse::<DMS>();
        let _ = s.parse::<UTM>();
        let _ = s.parse::<UPS>();
        let _ = s.parse::<MGRS>();
        let _ = s.parse::<UniversalCoord>();
        let _ = s.parse::<MGRSZone>();
    }
}
