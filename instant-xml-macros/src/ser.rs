use proc_macro2::TokenStream;
use quote::quote;

use crate::{ContainerMeta, FieldMeta};

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
                serializer.output.write_str(" xmlns:")?;
                serializer.output.write_str(#key)?;
                serializer.output.write_str("=\"")?;
                serializer.output.write_str(#val)?;
                serializer.output.write_char('\"')?;
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
                use ::instant_xml::ser::{FieldAttribute, FieldContext};

                let _ = serializer.consume_field_context();
                let mut field_context = FieldContext {
                    name: #root_name,
                    attribute: None,
                };

                // Start tag
                serializer.output.write_char('<')?;
                if serializer.parent_default_namespace() != #default_namespace {
                    if let Some(prefix) = serializer.parent_namespaces.get(#default_namespace) {
                        serializer.output.write_str(prefix)?;
                        serializer.output.write_char(':')?;
                        serializer.output.write_str(field_context.name)?;
                    } else {
                        serializer.output.write_str(field_context.name)?;
                        serializer.output.write_str(" xmlns=\"")?;
                        serializer.output.write_str(#default_namespace)?;
                        serializer.output.write_char('\"')?;
                    }
                } else {
                    serializer.output.write_str(field_context.name)?;
                }

                serializer.update_parent_default_namespace(#default_namespace);
                let mut to_remove: Vec<&str> = Vec::new();
                #prefixes
                #attributes
                serializer.consume_current_attributes()?;
                serializer.output.write_char('>')?;

                #body

                // Close tag
                serializer.output.write_str("</")?;
                serializer.output.write_str(#root_name)?;
                serializer.output.write_char('>')?;
                serializer.retrieve_parent_default_namespace();

                // Removing current namespaces
                for it in to_remove {
                    serializer.parent_namespaces.remove(it);
                }

                Ok(())
            }
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

    let declaration = quote!(
        let mut field = FieldContext {
            name: #name,
            attribute: None,
        };
    );

    let field_meta = FieldMeta::from_field(field);
    if field_meta.attribute {
        attributes.extend(quote!(
            #declaration

            serializer.add_attribute_key(&#name)?;
            field.attribute = Some(FieldAttribute::Attribute);
            serializer.set_field_context(field)?;
            self.#field_value.serialize(serializer)?;
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

    body.extend(quote!(
        #declaration
        match serializer.parent_namespaces.get(#ns) {
            Some(prefix) => field.attribute = Some(FieldAttribute::Prefix(prefix)),
            None => field.attribute = Some(FieldAttribute::Namespace(#ns)),
        }
        serializer.set_field_context(field)?;
        self.#field_value.serialize(serializer)?;
    ));
}
