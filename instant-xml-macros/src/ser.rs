use proc_macro2::TokenStream;
use quote::quote;
use syn::spanned::Spanned;

use super::{discard_lifetimes, meta_items, ContainerMeta, FieldMeta, Mode, VariantMeta};
use crate::{case::RenameRule, Namespace};

pub fn to_xml(input: &syn::DeriveInput) -> proc_macro2::TokenStream {
    let meta = match ContainerMeta::from_derive(input) {
        Ok(meta) => meta,
        Err(e) => return e.to_compile_error(),
    };

    match (&input.data, meta.mode) {
        (syn::Data::Struct(data), None) => serialize_struct(input, data, meta),
        (syn::Data::Enum(data), Some(Mode::Scalar)) => serialize_scalar_enum(input, data, meta),
        (syn::Data::Enum(data), Some(Mode::Wrapped)) => serialize_wrapped_enum(input, data, meta),
        (syn::Data::Struct(_), _) => {
            syn::Error::new(input.span(), "enum mode not allowed on struct type").to_compile_error()
        }
        (syn::Data::Enum(_), _) => {
            syn::Error::new(input.span(), "missing enum mode").to_compile_error()
        }
        _ => todo!(),
    }
}

fn serialize_scalar_enum(
    input: &syn::DeriveInput,
    data: &syn::DataEnum,
    meta: ContainerMeta,
) -> TokenStream {
    let ident = &input.ident;
    let mut variants = TokenStream::new();

    for variant in data.variants.iter() {
        let meta = match VariantMeta::from_variant(variant, &meta) {
            Ok(meta) => meta,
            Err(err) => return err.to_compile_error(),
        };

        let v_ident = &variant.ident;
        let serialize_as = meta.serialize_as;
        variants.extend(quote!(#ident::#v_ident => #serialize_as,));
    }

    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();
    quote!(
        impl #impl_generics ToXml for #ident #ty_generics #where_clause {
            fn serialize<W: ::core::fmt::Write + ?::core::marker::Sized>(
                &self,
                field: Option<::instant_xml::Id<'_>>,
                serializer: &mut instant_xml::Serializer<W>,
            ) -> Result<(), instant_xml::Error> {
                let prefix = match field {
                    Some(id) => {
                        let prefix = serializer.write_start(id.name, id.ns, true)?;
                        serializer.end_start()?;
                        Some((prefix, id.name))
                    }
                    None => None,
                };

                serializer.write_str(match self { #variants })?;
                if let Some((prefix, name)) = prefix {
                    serializer.write_close(prefix, name)?;
                }

                Ok(())
            }

            const KIND: ::instant_xml::Kind<'static> = ::instant_xml::Kind::Scalar;
        }
    )
}

fn serialize_wrapped_enum(
    input: &syn::DeriveInput,
    data: &syn::DataEnum,
    meta: ContainerMeta,
) -> TokenStream {
    if meta.rename_all != RenameRule::None {
        return syn::Error::new(
            input.span(),
            "rename_all is not allowed on wrapped enum type",
        )
        .to_compile_error();
    }

    let ident = &input.ident;
    let mut variants = TokenStream::new();
    for variant in data.variants.iter() {
        match &variant.fields {
            syn::Fields::Unnamed(fields) if fields.unnamed.len() == 1 => {}
            _ => {
                return syn::Error::new(
                    input.span(),
                    "wrapped enum variants must have 1 unnamed field",
                )
                .to_compile_error()
            }
        }

        if !meta_items(&variant.attrs).is_empty() {
            return syn::Error::new(
                input.span(),
                "attributes not allowed on wrapped enum variants",
            )
            .to_compile_error();
        }

        let v_ident = &variant.ident;
        variants.extend(quote!(#ident::#v_ident(inner) => inner.serialize(None, serializer)?,));
    }

    let default_namespace = meta.default_namespace();
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

    let mut generics = input.generics.clone();
    for param in generics.type_params_mut() {
        param
            .bounds
            .push(syn::parse_str("::instant_xml::ToXml").unwrap());
    }

    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
    let tag = meta.tag();
    quote!(
        impl #impl_generics ToXml for #ident #ty_generics #where_clause {
            fn serialize<W: ::core::fmt::Write + ?::core::marker::Sized>(
                &self,
                field: Option<::instant_xml::Id<'_>>,
                serializer: &mut instant_xml::Serializer<W>,
            ) -> Result<(), instant_xml::Error> {
                // Start tag
                let prefix = serializer.write_start(#tag, #default_namespace, false)?;
                debug_assert_eq!(prefix, None);

                // Set up element context, this will also emit namespace declarations
                #context
                let old = serializer.push(new)?;

                // Finalize start element
                serializer.end_start()?;

                match self {
                    #variants
                }

                // Close tag
                serializer.write_close(prefix, #tag)?;
                serializer.pop(old);

                Ok(())
            }

            const KIND: ::instant_xml::Kind<'static> = ::instant_xml::Kind::Element(::instant_xml::Id {
                ns: #default_namespace,
                name: #tag,
            });
        };
    )
}

fn serialize_struct(
    input: &syn::DeriveInput,
    data: &syn::DataStruct,
    meta: ContainerMeta,
) -> proc_macro2::TokenStream {
    let mut body = TokenStream::new();
    let mut attributes = TokenStream::new();

    match &data.fields {
        syn::Fields::Named(fields) => {
            for field in &fields.named {
                if let Err(err) = named_field(field, &mut body, &mut attributes, &meta) {
                    return err.to_compile_error();
                }
            }
        }
        syn::Fields::Unnamed(_) => todo!(),
        syn::Fields::Unit => {}
    };

    let default_namespace = meta.default_namespace();
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

    let mut generics = input.generics.clone();
    for param in generics.type_params_mut() {
        param
            .bounds
            .push(syn::parse_str("::instant_xml::ToXml").unwrap());
    }

    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
    let tag = meta.tag();
    let ident = &input.ident;

    quote!(
        impl #impl_generics ToXml for #ident #ty_generics #where_clause {
            fn serialize<W: ::core::fmt::Write + ?::core::marker::Sized>(
                &self,
                field: Option<::instant_xml::Id<'_>>,
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

            const KIND: ::instant_xml::Kind<'static> = ::instant_xml::Kind::Element(::instant_xml::Id {
                ns: #default_namespace,
                name: #tag,
            });
        };
    )
}

fn named_field(
    field: &syn::Field,
    body: &mut TokenStream,
    attributes: &mut TokenStream,
    meta: &ContainerMeta,
) -> Result<(), syn::Error> {
    let field_name = field.ident.as_ref().unwrap();
    let field_meta = match FieldMeta::from_field(field, meta) {
        Ok(meta) => meta,
        Err(err) => {
            body.extend(err.into_compile_error());
            return Ok(());
        }
    };

    if let Some(with) = field_meta.serialize_with {
        let path = with.to_string();
        let path = syn::parse_str::<syn::Path>(path.trim_matches('"')).map_err(|err| {
            syn::Error::new(
                with.span(),
                format!("failed to parse serialize_with as path: {err}"),
            )
        })?;

        body.extend(quote!(#path(&self.#field_name, serializer)?;));
        return Ok(());
    }

    let tag = field_meta.tag;
    let default_ns = match &meta.ns.uri {
        Some(ns) => quote!(#ns),
        None => quote!(""),
    };

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
            None => (default_ns, quote!()),
        };

        attributes.extend(quote!(
            #error
            serializer.write_attr(#tag, #ns, &self.#field_name)?;
        ));
        return Ok(());
    }

    let ns = match field_meta.ns.uri {
        Some(ref ns) => quote!(#ns),
        None => default_ns,
    };

    let mut no_lifetime_type = field.ty.clone();
    discard_lifetimes(&mut no_lifetime_type);
    body.extend(quote!(
        self.#field_name.serialize(Some(::instant_xml::Id { ns: #ns, name: #tag }), serializer)?;
    ));

    Ok(())
}
