use instant_xml::{from_str, to_string, FromXml, ToXml};
use similar_asserts::assert_eq;

#[derive(Debug, Eq, FromXml, PartialEq, ToXml)]
#[xml(scalar)]
enum Foo {
    A,
    B,
}

#[derive(Debug, Eq, FromXml, PartialEq, ToXml)]
struct Container {
    foo: Foo,
}

#[test]
fn scalar_enum() {
    let v = Container { foo: Foo::A };
    let xml = r#"<Container><foo>A</foo></Container>"#;
    assert_eq!(xml, to_string(&v).unwrap());
    assert_eq!(v, from_str(xml).unwrap());
}

#[derive(Debug, Eq, FromXml, PartialEq, ToXml)]
#[xml(scalar, ns("URI", x = "URI"))]
enum Bar {
    A,
    B,
}

#[derive(Debug, Eq, FromXml, PartialEq, ToXml)]
#[xml(ns("OTHER", x = "URI"))]
struct NsContainer {
    bar: Bar,
}

#[test]
fn scalar_enum_ns() {
    let v = NsContainer { bar: Bar::A };
    let xml = r#"<NsContainer xmlns="OTHER" xmlns:x="URI"><x:bar>A</x:bar></NsContainer>"#;
    assert_eq!(xml, to_string(&v).unwrap());
    assert_eq!(v, from_str(xml).unwrap());
}

const DIDL: &str = "DIDL";
const UPNP: &str = "UPNP";
const DC: &str = "DC";

#[derive(Debug, FromXml, PartialEq, ToXml)]
#[xml(rename = "DIDL-Lite", ns(DIDL, dc = DC, upnp = UPNP))]
struct DidlLite {
    item: Vec<UpnpItem>,
}

#[derive(Debug, FromXml, PartialEq, ToXml)]
#[xml(rename = "item", ns(DIDL))]
struct UpnpItem {
    class: Option<ObjectClass>,
}

#[derive(Debug, Clone, PartialEq, FromXml, ToXml)]
#[xml(rename = "class", scalar, ns(UPNP, upnp = UPNP))]
enum ObjectClass {
    #[xml(rename = "object.item.audioItem.musicTrack")]
    MusicTrack,
    #[xml(rename = "object.item.audioItem.audioBroadcast")]
    AudioBroadcast,
    #[xml(rename = "object.container.playlistContainer")]
    PlayList,
}

#[test]
fn scalar_enum_ns_match() {
    let v = DidlLite {
        item: vec![UpnpItem {
            class: Some(ObjectClass::AudioBroadcast),
        }],
    };

    // Keep the `upnp::mimeType` element after `upnp::class` to ensure that
    // we tickle a `DuplicateValue` error if we don't match correctly.
    let xml = r#"<DIDL-Lite xmlns="DIDL" xmlns:upnp="UPNP" xmlns:dc="DC" xmlns:dlna="DLNA">
        <item>
            <upnp:class>object.item.audioItem.audioBroadcast</upnp:class>
            <upnp:mimeType>audio/flac</upnp:mimeType>
        </item>
    </DIDL-Lite>"#;
    assert_eq!(v, from_str(xml).unwrap());
}
