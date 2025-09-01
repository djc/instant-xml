use instant_xml::{from_str, FromXml};
use similar_asserts::assert_eq;

#[derive(FromXml, PartialEq, Debug)]
struct Number {
    pub i_8: i8,
    pub i_16: i16,
    pub i_32: i32,
    pub i_64: i64,
    pub i_size: isize,
    pub u_8: u8,
    pub u_16: u16,
    pub u_32: u32,
    pub u_64: u64,
    pub u_size: usize,
    pub f_32: f32,
    pub f_64: f64,
}

#[test]
fn deserialize_spaced_numbers_fields() {
    let v = Number {
        i_8: -1,
        i_16: -32456_i16,
        i_32: -6034568_i32,
        i_64: -1245789630056_i64,
        i_size: -125698389,
        u_8: 9,
        u_16: 64469_u16,
        u_32: 6034568_u32,
        u_64: 99245789630056_u64,
        u_size: 125698389,
        f_32: -12.5683_f32,
        f_64: 104568.568932_f64,
    };
    let xml = r#"<Number><i_8>  -1 </i_8><i_16>-32456 </i_16><i_32>-6034568 </i_32><i_64>-1245789630056 </i_64><i_size>-125698389 </i_size><u_8>9 </u_8><u_16>64469   </u_16><u_32>6034568 </u_32><u_64> 99245789630056 </u_64><u_size>125698389 </u_size><f_32>    -12.5683   </f_32><f_64>  104568.568932 </f_64></Number>"#;
    assert_eq!(v, from_str(xml).unwrap());
}
