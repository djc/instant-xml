use proc_macro2::TokenStream;
use quote::{quote, ToTokens};
use syn::spanned::Spanned;

use crate::Namespace;

use super::{discard_lifetimes, ContainerMeta, FieldMeta};

pub fn to_xml(input: &syn::DeriveInput) -> proc_macro2::TokenStream {
    let meta = ContainerMeta::from_derive(input);
    match &input.data {
        syn::Data::Struct(ref data) => serialize_struct(input, data, meta),
        _ => todo!(),
    }
}

fn serialize_struct(
    input: &syn::DeriveInput,
    data: &syn::DataStruct,
    meta: ContainerMeta,
) -> proc_macro2::TokenStream {
    let mut body = TokenStream::new();
    let mut attributes = TokenStream::new();

    match data.fields {
        syn::Fields::Named(ref fields) => {
            fields.named.iter().for_each(|field| {
                process_named_field(field, &mut body, &mut attributes, &meta);
            });
        }
        syn::Fields::Unnamed(_) => todo!(),
        syn::Fields::Unit => {}
    };

    let default_namespace = match &meta.ns.uri {
        Some(ns) => quote!(#ns),
        None => quote!(""),
    };

    let cx_len = meta.ns.prefixes.len();
    let mut context = quote!(
        let mut new = ::instant_xml::ser::Context::<#cx_len>::default();
        new.default_ns = #default_namespace;
    );
    for (i, (prefix, ns)) in meta.ns.prefixes.iter().enumerate() {
        context.extend(quote!(
            new.prefixes[#i] = ::instant_xml::ser::Prefix { ns: #ns, prefix: #prefix };
        ));
    }

    let ident = &input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();
    let tag = match &meta.rename {
        Some(rename) => quote!(#rename),
        None => ident.to_string().into_token_stream(),
    };

    quote!(
        impl #impl_generics ToXml for #ident #ty_generics #where_clause {
            fn serialize<W: ::core::fmt::Write + ?::core::marker::Sized>(
                &self,
                serializer: &mut instant_xml::Serializer<W>,
            ) -> Result<(), instant_xml::Error> {
                // Start tag
                let prefix = serializer.write_start(#tag, #default_namespace, false)?;
                debug_assert_eq!(prefix, None);

                // Set up element context, this will also emit namespace declarations
                #context
                let old = serializer.push(new)?;

                // Finalize start element
                #attributes
                serializer.end_start()?;

                #body

                // Close tag
                serializer.write_close(prefix, #tag)?;
                serializer.pop(old);

                Ok(())
            }

            const KIND: ::instant_xml::Kind = ::instant_xml::Kind::Element(::instant_xml::Id {
                ns: #default_namespace,
                name: #tag,
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
    let field_name = field.ident.as_ref().unwrap();
    let field_meta = FieldMeta::from_field(field);
    let tag = match &field_meta.rename {
        Some(rename) => quote!(#rename),
        None => field_name.to_string().into_token_stream(),
    };

    let default_ns = &meta.ns.uri;
    if field_meta.attribute {
        let (ns, error) = match &field_meta.ns.uri {
            Some(Namespace::Path(path)) => match path.get_ident() {
                Some(prefix) => match &meta.ns.prefixes.get(&prefix.to_string()) {
                    Some(ns) => (quote!(#ns), quote!()),
                    None => (
                        quote!(""),
                        syn::Error::new(
                            field_meta.ns.uri.span(),
                            &format!("unknown prefix `{prefix}` (prefix must be defined on the field's type)"),
                        )
                        .into_compile_error(),
                    ),
                },
                None => (
                    quote!(""),
                    syn::Error::new(
                        field_meta.ns.uri.span(),
                        "attribute namespace must be a prefix identifier",
                    )
                    .into_compile_error(),
                ),
            },
            Some(Namespace::Literal(_)) => (
                quote!(""),
                syn::Error::new(
                    field_meta.ns.uri.span(),
                    "attribute namespace must be a prefix identifier",
                )
                .into_compile_error(),
            ),
            None => (match default_ns {
                Some(ns) => quote!(#ns),
                None => quote!(""),
            }, quote!()),
        };

        attributes.extend(quote!(
            #error
            serializer.write_attr(#tag, #ns, &self.#field_name)?;
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
                self.#field_name.serialize(serializer)?;
            }
            ::instant_xml::Kind::Scalar => {
                let prefix = serializer.write_start(#tag, #ns, true)?;
                serializer.end_start()?;
                self.#field_name.serialize(serializer)?;
                serializer.write_close(prefix, #tag)?;
            }
        }
    ));
}
