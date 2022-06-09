extern crate proc_macro;

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput, Lit, Meta, NestedMeta};

fn retrieve_default_namespace(input: &DeriveInput) -> Option<String> {
    if let NestedMeta::Meta(Meta::List(list)) = get_meta_items(&input.attrs).first()? {
        if list.path.get_ident()? == "namespace" {
            if let NestedMeta::Lit(Lit::Str(v)) = list.nested.first()? {
                return Some(v.value());
            }
        }
    }
    None
}

const XML: &str = "xml";

pub(crate) fn get_meta_items(attrs: &[syn::Attribute]) -> Vec<NestedMeta> {
    let mut out = Vec::new();
    for attr in attrs {
        if !attr.path.is_ident(XML) {
            continue;
        }

        match attr.parse_meta() {
            Ok(Meta::List(meta)) => out.extend(meta.nested.into_iter()),
            Ok(_) => todo!(),
            _ => todo!(),
        }
    }
    out
}

#[proc_macro_derive(ToXml, attributes(xml))]
pub fn to_xml(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as syn::DeriveInput);

    let ident = &ast.ident;
    let root_name = ident.to_string();
    let mut header: String = root_name.to_string();
    if let Some(v) = retrieve_default_namespace(&ast) {
        header += format!(" xmlns=\"{}\"", v).as_str();
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
