use std::collections::BTreeSet;

use proc_macro2::TokenStream;
use quote::{quote, ToTokens};
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
        (syn::Data::Struct(data), Some(Mode::Transparent)) => {
            serialize_inline_struct(input, data, meta)
        }
        (syn::Data::Enum(data), Some(Mode::Scalar)) => serialize_scalar_enum(input, data, meta),
        (syn::Data::Enum(data), Some(Mode::Forward)) => serialize_forward_enum(input, data, meta),
        (syn::Data::Struct(_), Some(mode)) => syn::Error::new(
            input.span(),
            format_args!("{mode:?} mode not allowed on struct type"),
        )
        .to_compile_error(),
        (syn::Data::Enum(_), Some(mode)) => syn::Error::new(
            input.span(),
            format_args!("{mode:?} mode not allowed on enum type"),
        )
        .to_compile_error(),
        (syn::Data::Enum(_), None) => {
            syn::Error::new(input.span(), "missing mode").to_compile_error()
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

    let default_namespace = meta.default_namespace();

    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();
    quote!(
        impl #impl_generics ToXml for #ident #ty_generics #where_clause {
            fn serialize<W: ::core::fmt::Write + ?::core::marker::Sized>(
                &self,
                field: Option<::instant_xml::Id<'_>>,
                serializer: &mut instant_xml::Serializer<W>,
            ) -> ::std::result::Result<(), instant_xml::Error> {
                let prefix = match field {
                    Some(id) => {
                        let prefix = serializer.write_start(id.name, #default_namespace)?;
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
        }
    )
}

fn serialize_forward_enum(
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
    quote!(
        impl #impl_generics ToXml for #ident #ty_generics #where_clause {
            fn serialize<W: ::core::fmt::Write + ?::core::marker::Sized>(
                &self,
                field: Option<::instant_xml::Id<'_>>,
                serializer: &mut instant_xml::Serializer<W>,
            ) -> ::std::result::Result<(), instant_xml::Error> {
                match self {
                    #variants
                }

                Ok(())
            }
        };
    )
}

fn serialize_struct(
    input: &syn::DeriveInput,
    data: &syn::DataStruct,
    meta: ContainerMeta,
) -> proc_macro2::TokenStream {
    let tag = meta.tag();
    let mut out = StructOutput::default();
    match &data.fields {
        syn::Fields::Named(fields) => {
            out.body.extend(quote!(serializer.end_start()?;));
            for field in &fields.named {
                if let Err(err) = out.named_field(field, &meta) {
                    return err.to_compile_error();
                }
            }
            out.body
                .extend(quote!(serializer.write_close(prefix, #tag)?;));
        }
        syn::Fields::Unnamed(fields) => {
            out.body.extend(quote!(serializer.end_start()?;));
            for (index, field) in fields.unnamed.iter().enumerate() {
                if let Err(err) = out.unnamed_field(field, index) {
                    return err.to_compile_error();
                }
            }
            out.body
                .extend(quote!(serializer.write_close(prefix, #tag)?;));
        }
        syn::Fields::Unit => out.body.extend(quote!(serializer.end_empty()?;)),
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
    let ident = &input.ident;
    quote!(
        impl #impl_generics ToXml for #ident #ty_generics #where_clause {
            fn serialize<W: ::core::fmt::Write + ?::core::marker::Sized>(
                &self,
                field: Option<::instant_xml::Id<'_>>,
                serializer: &mut instant_xml::Serializer<W>,
            ) -> ::std::result::Result<(), instant_xml::Error> {
                // Start tag
                let prefix = serializer.write_start(#tag, #default_namespace)?;

                // Set up element context, this will also emit namespace declarations
                #context
                let old = serializer.push(new)?;

                // Finalize start element
                #out

                serializer.pop(old);
                Ok(())
            }
        };
    )
}

fn serialize_inline_struct(
    input: &syn::DeriveInput,
    data: &syn::DataStruct,
    meta: ContainerMeta,
) -> proc_macro2::TokenStream {
    if !meta.ns.prefixes.is_empty() {
        return syn::Error::new(
            input.span(),
            "inline structs cannot have namespace declarations",
        )
        .to_compile_error();
    } else if let Some(ns) = meta.ns.uri {
        return syn::Error::new(
            ns.span(),
            "inline structs cannot have namespace declarations",
        )
        .to_compile_error();
    } else if let Some(rename) = meta.rename {
        return syn::Error::new(rename.span(), "inline structs cannot be renamed")
            .to_compile_error();
    }

    let mut out = StructOutput::default();
    match &data.fields {
        syn::Fields::Named(fields) => {
            for field in &fields.named {
                if let Err(err) = out.named_field(field, &meta) {
                    return err.to_compile_error();
                }

                if !out.attributes.is_empty() {
                    return syn::Error::new(
                        input.span(),
                        "no attributes allowed on inline structs",
                    )
                    .to_compile_error();
                }
            }
        }
        syn::Fields::Unnamed(fields) => {
            for (index, field) in fields.unnamed.iter().enumerate() {
                if let Err(err) = out.unnamed_field(field, index) {
                    return err.to_compile_error();
                }
            }
        }
        syn::Fields::Unit => out.body.extend(quote!(serializer.end_empty()?;)),
    }

    let mut generics = input.generics.clone();
    for param in generics.type_params_mut() {
        param
            .bounds
            .push(syn::parse_str("::instant_xml::ToXml").unwrap());
    }

    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
    let ident = &input.ident;
    quote!(
        impl #impl_generics ToXml for #ident #ty_generics #where_clause {
            fn serialize<W: ::core::fmt::Write + ?::core::marker::Sized>(
                &self,
                field: Option<::instant_xml::Id<'_>>,
                serializer: &mut instant_xml::Serializer<W>,
            ) -> ::std::result::Result<(), instant_xml::Error> {
                #out
                Ok(())
            }
        };
    )
}

#[derive(Default)]
struct StructOutput {
    body: TokenStream,
    attributes: TokenStream,
    borrowed: BTreeSet<syn::Lifetime>,
}

impl StructOutput {
    fn named_field(&mut self, field: &syn::Field, meta: &ContainerMeta) -> Result<(), syn::Error> {
        let field_name = field.ident.as_ref().unwrap();
        let field_meta = match FieldMeta::from_field(field, meta) {
            Ok(meta) => meta,
            Err(err) => {
                self.body.extend(err.into_compile_error());
                return Ok(());
            }
        };

        let tag = field_meta.tag;
        let default_ns = match &meta.ns.uri {
            Some(ns) => quote!(#ns),
            None => quote!(""),
        };

        if field_meta.attribute {
            if field_meta.direct {
                return Err(syn::Error::new(
                    field.span(),
                    "direct attribute is not supported on attributes",
                ));
            }

            let (ns, error) = match &field_meta.ns.uri {
                Some(Namespace::Path(path)) => match path.get_ident() {
                    Some(prefix) => match &meta.ns.prefixes.get(&prefix.to_string()) {
                        Some(ns) => (quote!(#ns), quote!()),
                        None => (
                            quote!(""),
                            syn::Error::new(
                                field_meta.ns.uri.span(),
                                format!("unknown prefix `{prefix}` (prefix must be defined on the field's type)"),
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

            self.attributes.extend(quote!(
                #error
                if self.#field_name.present() {
                    serializer.write_attr(#tag, #ns, &self.#field_name)?;
                }
            ));
            return Ok(());
        }

        let ns = match field_meta.ns.uri {
            Some(ref ns) => quote!(#ns),
            None => default_ns,
        };

        let mut no_lifetime_type = field.ty.clone();
        discard_lifetimes(&mut no_lifetime_type, &mut self.borrowed, false, true);
        if let Some(with) = field_meta.serialize_with {
            if field_meta.direct {
                return Err(syn::Error::new(
                    field.span(),
                    "direct serialization is not supported with `serialize_with`",
                ));
            }

            let path = with.to_string();
            let path = syn::parse_str::<syn::Path>(path.trim_matches('"')).map_err(|err| {
                syn::Error::new(
                    with.span(),
                    format!("failed to parse serialize_with as path: {err}"),
                )
            })?;

            self.body
                .extend(quote!(#path(&self.#field_name, serializer)?;));
            return Ok(());
        } else if field_meta.direct {
            self.body.extend(quote!(
                <#no_lifetime_type as ToXml>::serialize(
                    &self.#field_name, None, serializer
                )?;
            ));
        } else {
            self.body.extend(quote!(
                <#no_lifetime_type as ToXml>::serialize(
                    &self.#field_name,
                    Some(::instant_xml::Id { ns: #ns, name: #tag }),
                    serializer,
                )?;
            ));
        }

        Ok(())
    }

    fn unnamed_field(&mut self, field: &syn::Field, index: usize) -> Result<(), syn::Error> {
        if !field.attrs.is_empty() {
            return Err(syn::Error::new(
                field.span(),
                "unnamed fields cannot have attributes",
            ));
        }

        let mut no_lifetime_type = field.ty.clone();
        discard_lifetimes(&mut no_lifetime_type, &mut self.borrowed, false, true);
        let index = syn::Index::from(index);
        self.body.extend(quote!(
            self.#index.serialize(None, serializer)?;
        ));

        Ok(())
    }
}

impl ToTokens for StructOutput {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        self.attributes.to_tokens(tokens);
        self.body.to_tokens(tokens);
    }
}
