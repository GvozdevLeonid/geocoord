# geocoord

| Type | Content | Text form (`Display` / `FromStr`) |
| --- | --- | --- |
| `DD` | decimal degrees | `44.812346, 20.461235` |
| `DDM` | degrees, decimal minutes | `44°48.741'N 020°27.674'E` |
| `DMS` | degrees, minutes, decimal seconds | `44°48'44.44"N 020°27'40.44"E` |
| `UTM` | zone, latitude band, easting, northing | `34T 458083 4960870` |
| `UPS` | polar zone, easting, northing | `Z 2000000 1333272` |
| `MGRS` | grid zone, 100 km square, offsets | `34T DQ 58082 60869` |

`UniversalCoord` is `UTM` or `UPS`, selected by latitude (UTM within 80°S–84°N, UPS beyond).
Every conversion is a `From`/`TryFrom` impl; `DD → UTM` and `DD → UPS` are `TryFrom` because
the projection may not cover the point, everything else is infallible once a value exists.

## Usage

```rust
use geocoord::{DD, MGRS, UniversalCoord, UTM};

let dd = DD::new(44.8, 20.47)?;
let grid: MGRS = dd.into();
assert_eq!(format!("{}", grid), "34T DQ 58082 60869");

let utm: UTM = "34T 458083 4960870".parse()?;
let back: DD = utm.into();

let any: UniversalCoord = "Z 2000000 1333272".parse()?;
assert!(any.is_ups());
# Ok::<(), geocoord::CoordError>(())
```

`Display` precision is the number of decimals (`{:.3}` on `UTM`), or the number of digit pairs
for `MGRS` (`{:.0}` → `34T DQ`, `{:.9}` → 0.1 mm).

## Features

* `serde` (default) — `Serialize`/`Deserialize` for every type. Deserialization runs through the
  constructors (`#[serde(try_from)]`), so invalid data is an error, not a value. Disable with
  `default-features = false` for a crate with no dependencies at all.
