extern crate proc_macro;

mod de;
mod se;

use proc_macro::TokenStream;
use quote::quote;
use std::collections::BTreeSet;
use syn::parse_macro_input;

#[proc_macro_derive(ToXml, attributes(xml))]
pub fn to_xml(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as syn::DeriveInput);
    let ident = &ast.ident;
    let root_name = ident.to_string();
    let mut output: proc_macro2::TokenStream = TokenStream::from(quote!("".to_owned())).into();
    let mut missing_prefixes = BTreeSet::new();

    let mut serializer = se::Serializer::new(&ast.attrs);
    serializer.add_header(&root_name, &mut output);

    match &ast.data {
        syn::Data::Struct(ref data) => {
            match data.fields {
                syn::Fields::Named(ref fields) => {
                    fields.named.iter().for_each(|field| {
                        serializer.process_named_field(field, &mut output, &mut missing_prefixes);
                    });
                }
                syn::Fields::Unnamed(_) => todo!(),
                syn::Fields::Unit => {}
            };
        }
        _ => todo!(),
    };

    serializer.add_footer(&root_name, &mut output);

    let current_prefixes: BTreeSet<&str> = serializer.get_keys_set();
    TokenStream::from(quote!(
        impl ToXml for #ident {
            fn write_xml<W: ::std::fmt::Write>(&self, write: &mut W, parent_prefixes: Option<&mut std::collections::BTreeSet<&str>>) -> Result<(), instant_xml::Error> {
                match parent_prefixes {
                    Some(child_prefixes) => {
                        let mut to_remove: Vec<&str> = Vec::new();
                        #(if child_prefixes.insert(#current_prefixes) {
                            to_remove.push(#current_prefixes);
                        };)*;
                        write.write_str(&(#output))?;

                        for it in to_remove {
                            child_prefixes.remove(it);
                        }
                    },
                    None => {
                        let mut set = std::collections::BTreeSet::<&str>::new();
                        let child_prefixes = &mut set;
                        #(child_prefixes.insert(#current_prefixes);)*;
                        write.write_str(&(#output))?;
                    }
                }
                Ok(())
            }

            fn to_xml(&self, parent_prefixes: Option<&mut std::collections::BTreeSet<&str>>) -> Result<String, instant_xml::Error> {
                //#(println!("{}", #missing_prefixes);)*;
                if let Some(parent_prefixes) = parent_prefixes.as_ref() {
                    #(
                        if parent_prefixes.get(#missing_prefixes).is_none() {
                            panic!("wrong prefix");
                        }
                    )*;
                }

                let mut out = String::new();
                self.write_xml(&mut out, parent_prefixes)?;
                Ok(out)
            }
        };
    ))
}

#[proc_macro_derive(FromXml, attributes(xml))]
pub fn from_xml(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as syn::DeriveInput);
    let ident = &ast.ident;

    let deserializer = de::Deserializer::new(&ast);
    let fn_deserialize = deserializer.fn_deserialize;

    TokenStream::from(quote!(
        impl<'xml> FromXml<'xml> for #ident {
            #fn_deserialize
        }
    ))
}
