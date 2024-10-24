use std::borrow::Cow;

use bencher::{benchmark_group, benchmark_main, Bencher};
use instant_xml::{from_str, FromXml};

fn decode_short_ascii(bench: &mut Bencher) {
    let xml = "<Element><inner>foobar</inner></Element>";
    bench.iter(|| {
        from_str::<Element>(xml).unwrap();
    })
}

fn decode_longer_ascii(bench: &mut Bencher) {
    let mut xml = String::with_capacity(4096);
    xml.push_str("<Element><inner>");
    for _ in 0..64 {
        xml.push_str("abcdefghijklmnopqrstuvwxyz");
        xml.push_str("ABCDEFGHIJKLMNOPQRSTUVWXYZ");
        xml.push_str("0123456789");
    }
    xml.push_str("</inner></Element>");

    bench.iter(|| {
        from_str::<Element>(&xml).unwrap();
    })
}

fn decode_short_escaped(bench: &mut Bencher) {
    let xml = "<Element><inner>foo &amp; bar</inner></Element>";
    bench.iter(|| {
        from_str::<Element>(xml).unwrap();
    })
}

fn decode_longer_escaped(bench: &mut Bencher) {
    let mut xml = String::with_capacity(4096);
    xml.push_str("<Element><inner>");
    for _ in 0..64 {
        xml.push_str("abcdefghijklmnopqrstuvwxyz");
        xml.push_str("ABCDEFGHIJKLMNOPQRSTUVWXYZ");
        xml.push_str("0123456789");
        xml.push_str("&quot;");
    }
    xml.push_str("</inner></Element>");

    bench.iter(|| {
        from_str::<Element>(&xml).unwrap();
    })
}

#[derive(Debug, FromXml)]
struct Element<'a> {
    #[allow(dead_code)]
    inner: Cow<'a, str>,
}

benchmark_group!(
    benches,
    decode_short_ascii,
    decode_longer_ascii,
    decode_short_escaped,
    decode_longer_escaped,
);
benchmark_main!(benches);
