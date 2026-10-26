#![cfg(feature = "jiff02")]

use jiff02::{civil, tz::Offset, Timestamp};
use similar_asserts::assert_eq;

use instant_xml::{from_str, to_string, FromXml, ToXml};

#[derive(Debug, Eq, PartialEq, FromXml, ToXml)]
struct Test<T> {
    dt: T,
}

#[test]
fn timestamp() {
    let dt = civil::date(2022, 11, 21).at(21, 17, 23, 0);
    let dt = Offset::UTC.to_timestamp(dt).unwrap();
    let test = Test { dt };
    let xml = "<Test><dt>2022-11-21T21:17:23Z</dt></Test>";
    assert_eq!(to_string(&test).unwrap(), xml);
    assert_eq!(from_str::<Test<Timestamp>>(xml).unwrap(), test);

    let numeric = xml.replace("Z", "+00:00");
    assert_eq!(from_str::<Test<Timestamp>>(&numeric).unwrap(), test);

    let offset = "<Test><dt>2022-11-21T22:17:23+01:00</dt></Test>";
    assert_eq!(from_str::<Test<Timestamp>>(offset).unwrap(), test);

    let civil = "<Test><dt>2022-11-21T21:17:23</dt></Test>";
    assert!(from_str::<Test<Timestamp>>(civil).is_err());
}

#[test]
fn civil_datetime() {
    let dt = civil::date(2022, 11, 21).at(21, 17, 23, 0);
    let test = Test { dt };
    let xml = "<Test><dt>2022-11-21T21:17:23</dt></Test>";
    assert_eq!(to_string(&test).unwrap(), xml);
    assert_eq!(from_str::<Test<civil::DateTime>>(xml).unwrap(), test);

    let dt = civil::date(2022, 11, 21).at(21, 17, 23, 500_000_000);
    let test = Test { dt };
    let xml = "<Test><dt>2022-11-21T21:17:23.5</dt></Test>";
    assert_eq!(to_string(&test).unwrap(), xml);
    assert_eq!(from_str::<Test<civil::DateTime>>(xml).unwrap(), test);

    let offset = "<Test><dt>2022-11-21T21:17:23+01:00</dt></Test>";
    assert!(from_str::<Test<civil::DateTime>>(offset).is_err());
}

#[test]
fn civil_date() {
    let dt = civil::date(2022, 11, 21);
    let test = Test { dt };
    let xml = "<Test><dt>2022-11-21</dt></Test>";
    assert_eq!(to_string(&test).unwrap(), xml);
    assert_eq!(from_str::<Test<civil::Date>>(xml).unwrap(), test);

    let datetime = "<Test><dt>2022-11-21T21:17:23</dt></Test>";
    assert!(from_str::<Test<civil::Date>>(datetime).is_err());
}
