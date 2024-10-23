mod container;
pub(crate) use container::{ContainerMeta, Mode};

mod field;
pub(crate) use field::FieldMeta;

mod variant;
pub(crate) use variant::VariantMeta;

mod namespace;
pub(crate) use namespace::*;

use proc_macro2::{Delimiter, Literal, Span, TokenTree};

pub(crate) fn meta_items(attrs: &[syn::Attribute]) -> Vec<(MetaItem, Span)> {
    let list = match attrs.iter().find(|attr| attr.path().is_ident("xml")) {
        Some(attr) => match &attr.meta {
            syn::Meta::List(list) => list,
            _ => panic!("expected list in xml attribute"),
        },
        None => return Vec::new(),
    };

    let mut items = Vec::new();
    let mut state = MetaState::Start;
    for tree in list.tokens.clone() {
        let span = tree.span();
        state = match (state, tree) {
            (MetaState::Start, TokenTree::Ident(id)) => {
                if id == "attribute" {
                    items.push((MetaItem::Attribute, span));
                    MetaState::Comma
                } else if id == "borrow" {
                    items.push((MetaItem::Borrow, span));
                    MetaState::Comma
                } else if id == "direct" {
                    items.push((MetaItem::Direct, span));
                    MetaState::Comma
                } else if id == "transparent" {
                    items.push((MetaItem::Mode(Mode::Transparent), span));
                    MetaState::Comma
                } else if id == "ns" {
                    MetaState::Ns
                } else if id == "rename" {
                    MetaState::Rename
                } else if id == "rename_all" {
                    MetaState::RenameAll
                } else if id == "forward" {
                    items.push((MetaItem::Mode(Mode::Forward), span));
                    MetaState::Comma
                } else if id == "scalar" {
                    items.push((MetaItem::Mode(Mode::Scalar), span));
                    MetaState::Comma
                } else if id == "serialize_with" {
                    MetaState::SerializeWith
                } else if id == "deserialize_with" {
                    MetaState::DeserializeWith
                } else {
                    panic!("unexpected key in xml attribute");
                }
            }
            (MetaState::Comma, TokenTree::Punct(punct)) if punct.as_char() == ',' => {
                MetaState::Start
            }
            (MetaState::Ns, TokenTree::Group(group))
                if group.delimiter() == Delimiter::Parenthesis =>
            {
                items.push((MetaItem::Ns(NamespaceMeta::from_tokens(group)), span));
                MetaState::Comma
            }
            (MetaState::Rename, TokenTree::Punct(punct)) if punct.as_char() == '=' => {
                MetaState::RenameValue
            }
            (MetaState::RenameValue, TokenTree::Literal(lit)) => {
                items.push((MetaItem::Rename(lit), span));
                MetaState::Comma
            }
            (MetaState::RenameAll, TokenTree::Punct(punct)) if punct.as_char() == '=' => {
                MetaState::RenameAllValue
            }
            (MetaState::RenameAllValue, TokenTree::Literal(lit)) => {
                items.push((MetaItem::RenameAll(lit), span));
                MetaState::Comma
            }
            (MetaState::SerializeWith, TokenTree::Punct(punct)) if punct.as_char() == '=' => {
                MetaState::SerializeWithValue
            }
            (MetaState::SerializeWithValue, TokenTree::Literal(lit)) => {
                items.push((MetaItem::SerializeWith(lit), span));
                MetaState::Comma
            }
            (MetaState::DeserializeWith, TokenTree::Punct(punct)) if punct.as_char() == '=' => {
                MetaState::DeserializeWithValue
            }
            (MetaState::DeserializeWithValue, TokenTree::Literal(lit)) => {
                items.push((MetaItem::DeserializeWith(lit), span));
                MetaState::Comma
            }
            (state, tree) => {
                panic!(
                    "invalid state transition while parsing xml attribute ({}, {tree})",
                    state.name()
                )
            }
        };
    }

    items
}

#[derive(Debug)]
enum MetaState {
    Start,
    Comma,
    Ns,
    Rename,
    RenameValue,
    RenameAll,
    RenameAllValue,
    SerializeWith,
    SerializeWithValue,
    DeserializeWith,
    DeserializeWithValue,
}

impl MetaState {
    fn name(&self) -> &'static str {
        match self {
            MetaState::Start => "Start",
            MetaState::Comma => "Comma",
            MetaState::Ns => "Ns",
            MetaState::Rename => "Rename",
            MetaState::RenameValue => "RenameValue",
            MetaState::RenameAll => "RenameAll",
            MetaState::RenameAllValue => "RenameAllValue",
            MetaState::SerializeWith => "SerializeWith",
            MetaState::SerializeWithValue => "SerializeWithValue",
            MetaState::DeserializeWith => "DeserializeWith",
            MetaState::DeserializeWithValue => "DeserializeWithValue",
        }
    }
}

#[derive(Debug)]
pub(crate) enum MetaItem {
    Attribute,
    Borrow,
    Direct,
    Ns(NamespaceMeta),
    Rename(Literal),
    Mode(Mode),
    RenameAll(Literal),
    SerializeWith(Literal),
    DeserializeWith(Literal),
}
