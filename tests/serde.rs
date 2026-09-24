//! Serialization round trips and constructor-backed deserialization.
#![cfg(feature = "serde")]

use geocoord::*;

#[test]
fn round_trips() {
    let p = DD::new(44.8, 20.47).unwrap();
    let ddm: DDM = p.into();
    let dms: DMS = p.into();
    let uc: UniversalCoord = p.into();
    let m: MGRS = p.into();
    assert_eq!(
        serde_json::from_str::<DD>(&serde_json::to_string(&p).unwrap()).unwrap(),
        p
    );
    assert_eq!(
        serde_json::from_str::<DDM>(&serde_json::to_string(&ddm).unwrap()).unwrap(),
        ddm
    );
    assert_eq!(
        serde_json::from_str::<DMS>(&serde_json::to_string(&dms).unwrap()).unwrap(),
        dms
    );
    assert_eq!(
        serde_json::from_str::<UniversalCoord>(&serde_json::to_string(&uc).unwrap()).unwrap(),
        uc
    );
    assert_eq!(
        serde_json::from_str::<MGRS>(&serde_json::to_string(&m).unwrap()).unwrap(),
        m
    );
    let polar: UPS = DD::new(85.0, -100.0).unwrap().try_into().unwrap();
    assert_eq!(
        serde_json::from_str::<UPS>(&serde_json::to_string(&polar).unwrap()).unwrap(),
        polar
    );
    assert_eq!(
        serde_json::to_string(&p).unwrap(),
        r#"{"latitude":44.8,"longitude":20.47}"#
    );
}

#[test]
fn deserialization_runs_the_constructors() {
    assert!(serde_json::from_str::<DD>(r#"{"latitude":91,"longitude":0}"#).is_err());
    assert!(serde_json::from_str::<DD>(r#"{"latitude":45,"longitude":-181}"#).is_err());
    assert!(serde_json::from_str::<UTM>(
        r#"{"zone_number":0,"band":"T","easting":458082.8,"northing":4960869.9}"#
    )
    .is_err());
    assert!(serde_json::from_str::<UTM>(
        r#"{"zone_number":33,"band":"N","easting":500000,"northing":9000000}"#
    )
    .is_err());
    assert!(serde_json::from_str::<MGRS>(
        r#"{"zone":{"UTM":[34,"T"]},"square":["Z","Q"],"easting":58082,"northing":60869}"#
    )
    .is_err());
    assert!(serde_json::from_str::<MGRS>(
        r#"{"zone":{"UPS":"Y"},"square":["R","A"],"easting":0,"northing":0}"#
    )
    .is_err());
    assert!(serde_json::from_str::<DDM>(r#"{"latitude_dir":"North","latitude_degrees":45,"latitude_minutes":60,"longitude_dir":"East","longitude_degrees":20,"longitude_minutes":0}"#).is_err());
    let canonical: DD =
        serde_json::from_str(r#"{"latitude":-0.0,"longitude":180.0,"altitude":123.4}"#).unwrap();
    assert!(canonical.latitude().is_sign_positive() && canonical.longitude() == -180.0);
    let ok: MGRS = serde_json::from_str(
        r#"{"zone":{"UPS":"Y"},"square":["S","H"],"easting":52981,"northing":96454}"#,
    )
    .unwrap();
    assert_eq!(format!("{:.5}", ok), "Y SH 52981 96454");
}
