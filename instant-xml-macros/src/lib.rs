extern crate proc_macro;

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput, Lit, Meta, NestedMeta};

fn retrieve_default_namespace(input: &DeriveInput) -> Option<String> {
    for attr in &input.attrs {
        if !attr.path.is_ident(XML) {
            continue;
        }

        let nested = match attr.parse_meta() {
            Ok(Meta::List(meta)) => meta.nested,
            Ok(_) => todo!(),
            _ => todo!(),
        };

        let list = match nested.first() {
            Some(NestedMeta::Meta(Meta::List(list))) => list,
            _ => todo!(),
        };

        if list.path.get_ident()? == "namespace" {
            if let NestedMeta::Lit(Lit::Str(v)) = list.nested.first()? {
                return Some(v.value());
            }
        }
    }

    None
}

const XML: &str = "xml";

#[proc_macro_derive(ToXml, attributes(xml))]
pub fn to_xml(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as syn::DeriveInput);

    let ident = &ast.ident;
    let root_name = ident.to_string();
    let header = match retrieve_default_namespace(&ast) {
        Some(ns) => format!("{} xmlns=\"{}\"", root_name, ns),
        None => root_name.clone(),
    };

    let mut output: proc_macro2::TokenStream =
        TokenStream::from(quote!("<".to_owned() + #header + ">")).into();

    match &ast.data {
        syn::Data::Struct(ref data) => {
            match data.fields {
                syn::Fields::Named(ref fields) => {
                    fields
                    .named
                    .iter()
                    .for_each(|field| {
                        let field_name = field.ident.as_ref().unwrap().to_string();
                        let field_value = field.ident.as_ref().unwrap();
                        output.extend(quote!(+ "<" + #field_name + ">" + self.#field_value.to_string().as_str() + "</" + #field_name + ">"));
                    });
                }
                syn::Fields::Unnamed(_) => todo!(),
                syn::Fields::Unit => {}
            };
        }
        _ => todo!(),
    };

    output.extend(quote!(+ "</" + #root_name + ">"));

    TokenStream::from(quote!(
        impl ToXml for #ident {
            fn write_xml<W: ::std::fmt::Write>(&self, write: &mut W) -> Result<(), instant_xml::Error> {
                write.write_str(&(#output))?;
                Ok(())
            }
        }
    ))
}

#[proc_macro_derive(FromXml, attributes(xml))]
pub fn from_xml(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as syn::ItemStruct);
    let ident = &ast.ident;
    let name = ident.to_string();
    TokenStream::from(quote!(
        impl<'xml> FromXml<'xml> for #ident {
            fn from_xml(input: &str) -> Result<Self, ::instant_xml::Error> {
                use ::instant_xml::parse::Parse;
                let mut iter = ::instant_xml::xmlparser::Tokenizer::from(input);
                iter.next().element_start(None, #name)?;
                iter.next().element_end(None, #name)?;
                Ok(Self)
            }
        }
    ))
}
