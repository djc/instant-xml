extern crate proc_macro;

use proc_macro::TokenStream;
use quote::quote;
use std::collections::HashMap;
use syn::{parse_macro_input, Lit, Meta, NestedMeta};

const XML: &str = "xml";

struct Serializer {
    default_namespace: Option<String>,
    other_namespaces: HashMap<String, String>,
}

impl<'a> Serializer {
    pub fn init(attributes: &'a Vec<syn::Attribute>) -> Serializer {
        let mut default_namespace: Option<String> = None;
        let mut other_namespaces = HashMap::<String, String>::default();

        for attr in attributes {
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

            if let Some(ident) = list.path.get_ident() {
                if ident == "namespace" {
                    let mut iter = list.nested.iter();
                    if let Some(NestedMeta::Lit(Lit::Str(v))) = iter.next() {
                        default_namespace = Some(v.value());
                    }

                    for item in iter {
                        match item {
                            NestedMeta::Meta(Meta::NameValue(key)) => {
                                if let Lit::Str(value) = &key.lit {
                                    other_namespaces.insert(
                                        key.path.get_ident().unwrap().to_string(),
                                        value.value(),
                                    );
                                }
                            }
                            _ => todo!(),
                        }
                    }
                }
            }
        }

        Serializer {
            default_namespace,
            other_namespaces,
        }
    }

    fn add_header(&mut self, root_name: String, output: &'a mut proc_macro2::TokenStream) {
        output.extend(quote!(+ "<" + #root_name));

        if let Some(default_namespace) = self.default_namespace.as_ref() {
            output.extend(quote!(+ " xmlns=\"" + #default_namespace + "\""));
        }

        output.extend(quote!(+ ">"));
    }

    fn add_footer(&mut self, root_name: String, output: &'a mut proc_macro2::TokenStream) {
        output.extend(quote!(+ "</" + #root_name + ">"));
    }

    fn process_named_field(
        &mut self,
        field: &syn::Field,
        output: &'a mut proc_macro2::TokenStream,
    ) {
        let field_name = field.ident.as_ref().unwrap().to_string();
        let field_value = field.ident.as_ref().unwrap();
        if !self.other_namespaces.is_empty() {
            if let Some(namespace_key) = Serializer::init(&field.attrs).default_namespace {
                if let Some(namespace_value) = self.other_namespaces.get(&namespace_key) {
                    output.extend(
                        quote!(+ "<" + #field_name + " xmlns=\"" + #namespace_value + "\""),
                    );
                } else if let Some(default) = &self.default_namespace {
                    // Not exist in the map, adding default one if exist
                    output.extend(quote!(+ "<" + #field_name + " xmlns=\"" + #default + "\""));
                } else {
                    // Without the namespace
                    output.extend(quote!(+ "<" + #field_name +));
                }
            }
        } else {
            // Without the namespace
            output.extend(quote!(+ "<" + #field_name +));
        }
        output.extend(
            quote!(+ ">" + self.#field_value.to_string().as_str() + "</" + #field_name + ">"),
        );
    }
}

#[proc_macro_derive(ToXml, attributes(xml))]
pub fn to_xml(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as syn::DeriveInput);

    let ident = &ast.ident;
    let root_name = ident.to_string();

    let header: String = root_name.to_string();
    let mut output: proc_macro2::TokenStream = TokenStream::from(quote!("".to_owned())).into();

    let mut serializer = Serializer::init(&ast.attrs);
    serializer.add_header(header, &mut output);

    match &ast.data {
        syn::Data::Struct(ref data) => {
            match data.fields {
                syn::Fields::Named(ref fields) => {
                    fields.named.iter().for_each(|field| {
                        serializer.process_named_field(field, &mut output);
                    });
                }
                syn::Fields::Unnamed(_) => todo!(),
                syn::Fields::Unit => {}
            };
        }
        _ => todo!(),
    };

    serializer.add_footer(root_name, &mut output);

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
