extern crate proc_macro;

use proc_macro::TokenStream;
use quote::quote;
use syn::parse_macro_input;

#[proc_macro_derive(ToXml)]
pub fn to_xml(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as syn::ItemStruct);
    let ident = &ast.ident;
    let name = ident.to_string();
    TokenStream::from(quote!(
        impl ToXml for #ident {
            fn write_xml<W: ::std::fmt::Write>(&self, write: &mut W) -> Result<(), instant_xml::Error> {
                write.write_str("<")?;
                write.write_fmt(format_args!("{}", #name))?;
                write.write_str("/>")?;
                Ok(())
            }
        }
    ))
}

#[proc_macro_derive(FromXml)]
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
