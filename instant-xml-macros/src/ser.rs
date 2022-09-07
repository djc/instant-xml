use proc_macro2::TokenStream;
use quote::quote;

use super::{discard_lifetimes, ContainerMeta, FieldMeta};

pub fn to_xml(input: &syn::DeriveInput) -> proc_macro2::TokenStream {
    let mut body = TokenStream::new();
    let mut attributes = TokenStream::new();
    let meta = ContainerMeta::from_derive(input);
    match &input.data {
        syn::Data::Struct(ref data) => {
            match data.fields {
                syn::Fields::Named(ref fields) => {
                    fields.named.iter().for_each(|field| {
                        process_named_field(field, &mut body, &mut attributes, &meta);
                    });
                }
                syn::Fields::Unnamed(_) => todo!(),
                syn::Fields::Unit => {}
            };
        }
        _ => todo!(),
    };

    let mut prefixes = TokenStream::new();
    for (key, val) in &meta.ns.prefixes {
        prefixes.extend(quote!(
            if serializer.parent_namespaces.get(#val).is_none() {
                serializer.write_prefix(#key, #val)?;
            }

            if let ::std::collections::hash_map::Entry::Vacant(v) = serializer.parent_namespaces.entry(#val) {
                v.insert(#key);
                // Will remove added namespaces when going "up"
                to_remove.push(#val);
            };
        ));
    }

    let ident = &input.ident;
    let root_name = ident.to_string();
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();
    let default_namespace = match &meta.ns.uri {
        Some(ns) => quote!(#ns),
        None => quote!(""),
    };

    quote!(
        impl #impl_generics ToXml for #ident #ty_generics #where_clause {
            fn serialize<W: ::core::fmt::Write + ?::core::marker::Sized>(
                &self,
                serializer: &mut instant_xml::Serializer<W>,
            ) -> Result<(), instant_xml::Error> {
                // Start tag
                match serializer.parent_default_namespace() == #default_namespace {
                    true => serializer.write_start(None, #root_name, None)?,
                    false => serializer.write_start(None, #root_name, Some(#default_namespace))?,
                }

                serializer.update_parent_default_namespace(#default_namespace);
                let mut to_remove: Vec<&str> = Vec::new();
                #prefixes
                #attributes
                serializer.end_start()?;

                #body

                // Close tag
                serializer.write_close(None, #root_name)?;
                serializer.retrieve_parent_default_namespace();

                // Removing current namespaces
                for it in to_remove {
                    serializer.parent_namespaces.remove(it);
                }

                Ok(())
            }

            const KIND: ::instant_xml::Kind = ::instant_xml::Kind::Element(::instant_xml::Id {
                ns: #default_namespace,
                name: #root_name,
            });
        };
    )
}

fn process_named_field(
    field: &syn::Field,
    body: &mut TokenStream,
    attributes: &mut TokenStream,
    meta: &ContainerMeta,
) {
    let name = field.ident.as_ref().unwrap().to_string();
    let field_value = field.ident.as_ref().unwrap();

    let field_meta = FieldMeta::from_field(field);
    if field_meta.attribute {
        attributes.extend(quote!(
            serializer.write_attr(#name, &self.#field_value)?;
        ));
        return;
    }

    let ns = match field_meta.ns.uri {
        Some(ns) => quote!(#ns),
        None => match &meta.ns.uri {
            Some(ns) => quote!(#ns),
            None => quote!(""),
        },
    };

    let mut no_lifetime_type = field.ty.clone();
    discard_lifetimes(&mut no_lifetime_type);
    body.extend(quote!(
        match <#no_lifetime_type as ToXml>::KIND {
            ::instant_xml::Kind::Element(_) => {
                self.#field_value.serialize(serializer)?;
            }
            ::instant_xml::Kind::Scalar => {
                let (prefix, ns) = match serializer.parent_default_namespace() == #ns {
                    true => (None, None),
                    false => match serializer.parent_namespaces.get(#ns) {
                        Some(&prefix) => (Some(prefix), None),
                        None => (None, Some(#ns)),
                    },
                };

                serializer.write_start(prefix, #name, ns)?;
                serializer.end_start()?;
                self.#field_value.serialize(serializer)?;
                serializer.write_close(prefix, #name)?;
            }
        }
    ));
}
