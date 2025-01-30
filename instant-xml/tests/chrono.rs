#![cfg(feature = "chrono")]

use chrono::{DateTime, NaiveDateTime, TimeZone, Utc};
use similar_asserts::assert_eq;

use instant_xml::{from_str, to_string, FromXml, ToXml};

#[derive(Debug, Eq, PartialEq, FromXml, ToXml)]
struct Test<T> {
    dt: T,
}

type TestUtcDateTime = Test<DateTime<Utc>>;

#[test]
fn datetime() {
    let dt = Utc.with_ymd_and_hms(2022, 11, 21, 21, 17, 23).unwrap();
    let test = Test { dt };
    let xml = "<Test><dt>2022-11-21T21:17:23+00:00</dt></Test>";
    assert_eq!(to_string(&test).unwrap(), xml);
    assert_eq!(from_str::<TestUtcDateTime>(xml).unwrap(), test);

    let zulu = xml.replace("+00:00", "Z");
    assert_eq!(from_str::<TestUtcDateTime>(&zulu).unwrap(), test);
}

type TestNaiveDateTime = Test<NaiveDateTime>;

#[test]
fn naive_datetime() {
    let dt = NaiveDateTime::parse_from_str("2022-11-21T21:17:23", "%Y-%m-%dT%H:%M:%S").unwrap();
    let test = Test { dt };
    let xml = "<Test><dt>2022-11-21T21:17:23</dt></Test>";
    assert_eq!(to_string(&test).unwrap(), xml);
    assert_eq!(from_str::<TestNaiveDateTime>(xml).unwrap(), test);
}
