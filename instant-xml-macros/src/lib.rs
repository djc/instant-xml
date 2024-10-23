extern crate proc_macro;

use std::collections::BTreeSet;
use std::mem;

use proc_macro2::Span;
use syn::parse_macro_input;

mod case;
mod de;
mod meta;
use meta::{meta_items, Namespace};
mod ser;

#[proc_macro_derive(ToXml, attributes(xml))]
pub fn to_xml(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let ast = parse_macro_input!(input as syn::DeriveInput);
    ser::to_xml(&ast).into()
}

#[proc_macro_derive(FromXml, attributes(xml))]
pub fn from_xml(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let ast = parse_macro_input!(input as syn::DeriveInput);
    de::from_xml(&ast).into()
}

fn discard_lifetimes(
    ty: &mut syn::Type,
    borrowed: &mut BTreeSet<syn::Lifetime>,
    borrow: bool,
    top: bool,
) {
    match ty {
        syn::Type::Path(ty) => discard_path_lifetimes(ty, borrowed, borrow),
        syn::Type::Reference(ty) => {
            if top {
                // If at the top level, we'll want to borrow from `&'a str` and `&'a [u8]`.
                match &*ty.elem {
                    syn::Type::Path(inner) if top && inner.path.is_ident("str") => {
                        if let Some(lt) = ty.lifetime.take() {
                            borrowed.insert(lt);
                        }
                    }
                    syn::Type::Slice(inner) if top => match &*inner.elem {
                        syn::Type::Path(inner) if inner.path.is_ident("u8") => {
                            borrowed.extend(ty.lifetime.take());
                        }
                        _ => {}
                    },
                    _ => {}
                }
            } else if borrow {
                // Otherwise, only borrow if the user has requested it.
                borrowed.extend(ty.lifetime.take());
            } else {
                ty.lifetime = None;
            }

            discard_lifetimes(&mut ty.elem, borrowed, borrow, false);
        }
        _ => {}
    }
}

// IMPROVEMENT: nest in discard_lifetimes function?
fn discard_path_lifetimes(
    path: &mut syn::TypePath,
    borrowed: &mut BTreeSet<syn::Lifetime>,
    borrow: bool,
) {
    if let Some(q) = &mut path.qself {
        discard_lifetimes(&mut q.ty, borrowed, borrow, false);
    }

    for segment in &mut path.path.segments {
        match &mut segment.arguments {
            syn::PathArguments::None => {}
            syn::PathArguments::AngleBracketed(args) => {
                args.args.iter_mut().for_each(|arg| match arg {
                    syn::GenericArgument::Lifetime(lt) => {
                        let lt = mem::replace(lt, syn::Lifetime::new("'_", Span::call_site()));
                        if borrow {
                            borrowed.insert(lt);
                        }
                    }
                    syn::GenericArgument::Type(ty) => {
                        discard_lifetimes(ty, borrowed, borrow, false)
                    }
                    _ => {}
                })
            }
            syn::PathArguments::Parenthesized(args) => args
                .inputs
                .iter_mut()
                .for_each(|ty| discard_lifetimes(ty, borrowed, borrow, false)),
        }
    }
}

#[cfg(test)]
mod tests {
    use syn::parse_quote;

    #[test]
    fn non_unit_enum_variant_unsupported() {
        dbg!(super::ser::to_xml(&parse_quote! {
            #[xml(scalar)]
            pub enum TestEnum {
                Foo(String),
                Bar,
                Baz
            }
        })
        .to_string())
        .find("compile_error ! { \"only unit enum variants are permitted!\" }")
        .unwrap();
    }

    #[test]
    fn non_scalar_enums_unsupported() {
        dbg!(super::ser::to_xml(&parse_quote! {
            #[xml()]
            pub enum TestEnum {
                Foo,
                Bar,
                Baz
            }
        })
        .to_string())
        .find("compile_error ! { \"missing mode\" }")
        .unwrap();
    }

    #[test]
    fn scalar_variant_attribute_not_permitted() {
        dbg!(super::ser::to_xml(&parse_quote! {
            #[xml(scalar)]
            pub enum TestEnum {
                Foo,
                Bar,
                #[xml(scalar)]
                Baz
            }
        })
        .to_string())
        .find("compile_error ! { \"only 'rename' attribute is permitted on enum variants\" }")
        .unwrap();
    }

    #[test]
    fn scalar_discrimintant_must_be_literal() {
        assert_eq!(
            None,
            dbg!(super::ser::to_xml(&parse_quote! {
                #[xml(scalar)]
                pub enum TestEnum {
                    Foo = 1,
                    Bar,
                    Baz
                }
            })
            .to_string())
            .find("compile_error ! { \"invalid field discriminant value!\" }")
        );

        dbg!(super::ser::to_xml(&parse_quote! {
            #[xml(scalar)]
            pub enum TestEnum {
                Foo = 1+1,
                Bar,
                Baz
            }
        })
        .to_string())
        .find("compile_error ! { \"invalid field discriminant value!\" }")
        .unwrap();
    }

    #[test]
    fn rename_all_attribute_not_permitted() {
        dbg!(super::ser::to_xml(&parse_quote! {
            pub struct TestStruct {
                #[xml(rename_all = "UPPERCASE")]
                field_1: String,
                field_2: u8,
            }
        })
        .to_string())
        .find("compile_error ! { \"attribute 'rename_all' invalid in field xml attribute\" }")
        .unwrap();

        dbg!(super::ser::to_xml(&parse_quote! {
            #[xml(scalar)]
            pub enum TestEnum {
                Foo = 1,
                Bar,
                #[xml(rename_all = "UPPERCASE")]
                Baz
            }
        })
        .to_string())
        .find("compile_error ! { \"only 'rename' attribute is permitted on enum variants\" }")
        .unwrap();
    }

    #[test]
    fn bogus_rename_all_not_permitted() {
        dbg!(super::ser::to_xml(&parse_quote! {
            #[xml(rename_all = "forgetaboutit")]
            pub struct TestStruct {
                field_1: String,
                field_2: u8,
            }
        })
        .to_string())
        .find("compile_error ! {")
        .unwrap();
    }
}
