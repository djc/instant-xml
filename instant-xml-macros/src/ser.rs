use proc_macro2::TokenStream;
use quote::quote;

use crate::{ContainerMeta, FieldMeta};

pub fn to_xml(input: &syn::DeriveInput) -> proc_macro2::TokenStream {
    let ident = &input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    let root_name = ident.to_string();
    let mut serializer = Serializer::new(&input);

    let mut header = TokenStream::new();
    serializer.add_header(&mut header);

    let mut body = TokenStream::new();
    let mut attributes = TokenStream::new();
    match &input.data {
        syn::Data::Struct(ref data) => {
            match data.fields {
                syn::Fields::Named(ref fields) => {
                    fields.named.iter().for_each(|field| {
                        serializer.process_named_field(field, &mut body, &mut attributes);
                    });
                }
                syn::Fields::Unnamed(_) => todo!(),
                syn::Fields::Unit => {}
            };
        }
        _ => todo!(),
    };

    let mut footer = TokenStream::new();
    serializer.add_footer(&root_name, &mut footer);

    let current_namespaces = serializer.namespaces_token();

    quote!(
        impl #impl_generics ToXml for #ident #ty_generics #where_clause {
            fn serialize<W: ::core::fmt::Write + ?::core::marker::Sized>(
                &self,
                serializer: &mut instant_xml::Serializer<W>,
            ) -> Result<(), instant_xml::Error> {
                let _ = serializer.consume_field_context();
                let mut field_context = instant_xml::ser::FieldContext {
                    name: #root_name,
                    attribute: None,
                };

                #attributes

                #header
                #current_namespaces
                #body
                #footer

                // Removing current namespaces
                for it in to_remove {
                    serializer.parent_namespaces.remove(it);
                }

                Ok(())
            }
        };
    )
}

struct Serializer {
    meta: ContainerMeta,
}

impl<'a> Serializer {
    fn new(input: &syn::DeriveInput) -> Self {
        Self {
            meta: ContainerMeta::from_derive(input),
        }
    }

    fn add_header(&mut self, output: &'a mut TokenStream) {
        output.extend(quote!(
            serializer.output.write_char('<')?;

        ));

        let default_namespace = match &self.meta.ns.uri {
            Some(ns) => quote!(#ns),
            None => quote!(""),
        };

        output.extend(quote!(
            // Check if parent default namespace equals
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
        ));

        for (key, val) in &self.meta.ns.prefixes {
            output.extend(quote!(
                if serializer.parent_namespaces.get(#val).is_none() {
                    serializer.output.write_str(" xmlns:")?;
                    serializer.output.write_str(#key)?;
                    serializer.output.write_str("=\"")?;
                    serializer.output.write_str(#val)?;
                    serializer.output.write_char('\"')?;
                }
            ));
        }

        // Attributes
        output.extend(quote!(
            serializer.consume_current_attributes()?;
        ));

        output.extend(quote!(
            serializer.output.write_char('>')?;
        ));
    }

    fn add_footer(&mut self, root_name: &str, output: &'a mut TokenStream) {
        output.extend(quote!(
            serializer.output.write_str("</")?;
            serializer.output.write_str(#root_name)?;
            serializer.output.write_char('>')?;
            serializer.retrieve_parent_default_namespace();
        ));
    }

    fn process_named_field(
        &mut self,
        field: &syn::Field,
        body: &mut TokenStream,
        attributes: &mut TokenStream,
    ) {
        let name = field.ident.as_ref().unwrap().to_string();
        let field_value = field.ident.as_ref().unwrap();

        let declaration = quote!(
            let mut field = instant_xml::ser::FieldContext {
                name: #name,
                attribute: None,
            };
        );

        let field_meta = FieldMeta::from_field(field);
        if field_meta.attribute {
            attributes.extend(quote!(
                #declaration

                serializer.add_attribute_key(&#name)?;
                field.attribute = Some(instant_xml::FieldAttribute::Attribute);
                serializer.set_field_context(field)?;
                self.#field_value.serialize(serializer)?;
            ));
            return;
        }

        let ns = match field_meta.ns.uri {
            Some(ns) => quote!(#ns),
            None => match &self.meta.ns.uri {
                Some(ns) => quote!(#ns),
                None => quote!(""),
            },
        };

        body.extend(quote!(
            #declaration
            match serializer.parent_namespaces.get(#ns) {
                Some(prefix) => field.attribute = Some(::instant_xml::FieldAttribute::Prefix(prefix)),
                None => field.attribute = Some(::instant_xml::FieldAttribute::Namespace(#ns)),
            }
            serializer.set_field_context(field)?;
            self.#field_value.serialize(serializer)?;
        ));
    }

    fn namespaces_token(&self) -> TokenStream {
        let mut namespaces = quote!(
            let mut to_remove: Vec<&str> = Vec::new();
        );
        for (k, v) in self.meta.ns.prefixes.iter() {
            namespaces.extend(quote!(
                // Only adding to HashMap if namespace do not exist, if it exist it will use the parent defined prefix
                if let std::collections::hash_map::Entry::Vacant(v) = serializer.parent_namespaces.entry(#v) {
                    v.insert(#k);
                    // Will remove added namespaces when going "up"
                    to_remove.push(#v);
                };
            ))
        }
        namespaces
    }
}
