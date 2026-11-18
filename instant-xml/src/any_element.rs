use std::{borrow::Cow, fmt};

use crate::de::Node;
use crate::ser::Context;
use crate::{Deserializer, Error, FromXml, Id, Kind, Serializer, ToXml};

/// A dynamically captured XML element.
///
/// The [`AnyElement`] type captures an arbitrary XML element tree at runtime,
/// preserving its namespace, name, attributes, text
/// content, and nested children. This is useful when the XML schema allows
/// arbitrary content (e.g. `<xs:any namespace="##any" processContents="skip" />`)
/// or when the element structure is not known at compile time.
///
/// String values are borrowed from the input XML where possible via [`Cow`].
///
/// # Example
///
/// As root:
/// ```
/// use instant_xml::{from_str, AnyElement};
///
/// let xml = r#"<item xmlns="http://example.com" key="val">hello</item>"#;
/// let elem: AnyElement<'_> = from_str(xml).unwrap();
///
/// assert_eq!(elem.name, "item");
/// assert_eq!(elem.ns, "http://example.com");
/// assert_eq!(elem.text.as_deref(), Some("hello"));
/// assert_eq!(elem.attributes.len(), 1);
/// ```
///
/// As child (borrowing from the input):
/// ```
/// use instant_xml::{from_str, FromXml, AnyElement};
///
/// #[derive(Debug, FromXml, PartialEq)]
/// #[xml(ns("http://example.com"))]
/// struct Wrapper<'a> {
///     #[xml(borrow)]
///     inner: AnyElement<'a>,
/// }
///
/// let xml = r#"<Wrapper xmlns="http://example.com"><item>text</item></Wrapper>"#;
/// let parsed: Wrapper<'_> = from_str(xml).unwrap();
///
/// assert_eq!(parsed.inner.name, "item");
/// assert_eq!(parsed.inner.ns, "http://example.com");
/// assert_eq!(parsed.inner.text.as_deref(), Some("text"));
/// ```
///
/// **Note:** When using `AnyElement` as a field in a derived struct, add
/// `#[xml(borrow)]` so the derive macro generates the correct lifetime bounds.
/// Use [`into_owned()`](Self::into_owned) to convert to `AnyElement<'static>`
/// when you need to decouple from the input lifetime.
///
/// # Matching behavior
///
/// `AnyElement` matches any XML element regardless of namespace or name. When
/// used as a field in a derived struct, it will capture whichever element appears
/// in that position. For capturing multiple arbitrary children, use
/// `Vec<AnyElement>`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AnyElement<'xml> {
    /// XML namespace URI of this element.
    pub ns: Cow<'xml, str>,
    /// Local element name.
    pub name: Cow<'xml, str>,
    /// Attributes on this element.
    pub attributes: Vec<AnyAttribute<'xml>>,
    /// Text content of this element, if any.
    pub text: Option<Cow<'xml, str>>,
    /// Nested child elements.
    pub children: Vec<Self>,
}

impl<'a> AnyElement<'a> {
    fn deserialize<'xml: 'a>(
        deserializer: &mut Deserializer<'_, 'xml>,
        id: Id<'xml>,
    ) -> Result<Self, Error> {
        let mut elem = Self {
            ns: Cow::Borrowed(id.ns),
            name: Cow::Borrowed(id.name),
            attributes: Vec::new(),
            text: None,
            children: Vec::new(),
        };

        loop {
            match deserializer.next() {
                Some(Ok(Node::Attribute(attr))) => {
                    // Namespace declarations (`xmlns:prefix="uri"`) are consumed by the
                    // deserializer to resolve prefixes, so only regular attributes arrive
                    // here. Resolve the prefix to a namespace URI immediately.
                    let id = deserializer.attribute_id(&attr)?;

                    elem.attributes.push(AnyAttribute {
                        ns: Cow::Borrowed(id.ns),
                        name: Cow::Borrowed(id.name),
                        value: attr.value,
                    });
                }
                Some(Ok(Node::Open(element))) => {
                    let child_id = deserializer.element_id(&element)?;
                    let mut nested = deserializer.nested(element);
                    elem.children
                        .push(Self::deserialize(&mut nested, child_id)?);
                }
                Some(Ok(Node::Text(text))) => elem.text = Some(text),
                Some(Ok(Node::Close { .. })) => break,
                Some(Ok(_)) => continue,
                Some(Err(e)) => return Err(e),
                None => break,
            }
        }

        Ok(elem)
    }

    /// Converts this element into an owned version with `'static` lifetime.
    ///
    /// This recursively converts all borrowed strings into owned copies,
    /// decoupling the result from the original XML input.
    pub fn into_owned(self) -> AnyElement<'static> {
        AnyElement {
            ns: Cow::Owned(self.ns.into_owned()),
            name: Cow::Owned(self.name.into_owned()),
            attributes: self
                .attributes
                .into_iter()
                .map(|a| a.into_owned())
                .collect(),
            text: self.text.map(|t| Cow::Owned(t.into_owned())),
            children: self.children.into_iter().map(|c| c.into_owned()).collect(),
        }
    }
}

impl<'xml, 'a> FromXml<'xml> for AnyElement<'a>
where
    'xml: 'a,
{
    /// Matches any element regardless of namespace or name.
    fn matches(_id: Id<'_>, _field: Option<Id<'_>>) -> bool {
        true
    }

    fn deserialize<'cx>(
        into: &mut Self::Accumulator,
        _field: &'static str,
        deserializer: &mut Deserializer<'cx, 'xml>,
    ) -> Result<(), Error> {
        let id = deserializer.parent();
        *into = Some(Self::deserialize(deserializer, id)?);
        Ok(())
    }

    type Accumulator = Option<Self>;
    const KIND: Kind = Kind::Element;
}

impl<'xml, 'a> FromXml<'xml> for AnyAttribute<'a>
where
    'xml: 'a,
{
    /// Matches any attribute.
    fn matches(_id: Id<'_>, field: Option<Id<'_>>) -> bool {
        field.is_some()
    }

    fn deserialize<'cx>(
        into: &mut Self::Accumulator,
        field: &'static str,
        deserializer: &mut Deserializer<'cx, 'xml>,
    ) -> Result<(), Error> {
        if into.is_some() {
            return Err(Error::DuplicateValue(field));
        }

        let Some(value) = deserializer.take_str()? else {
            return Ok(());
        };

        *into = Some(Self {
            ns: Cow::Borrowed(""),
            name: Cow::Borrowed(""),
            value,
        });
        Ok(())
    }

    fn deserialize_attribute<'cx>(
        into: &mut Self::Accumulator,
        field: &'static str,
        id: Id<'xml>,
        value: Cow<'xml, str>,
        _deserializer: &mut Deserializer<'cx, 'xml>,
    ) -> Result<(), Error> {
        if into.is_some() {
            return Err(Error::DuplicateValue(field));
        }

        *into = Some(Self {
            ns: Cow::Borrowed(id.ns),
            name: Cow::Borrowed(id.name),
            value,
        });
        Ok(())
    }

    type Accumulator = Option<Self>;
    const KIND: Kind = Kind::Scalar;
}

/// An XML attribute with a resolved namespace URI.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AnyAttribute<'xml> {
    /// Namespace URI of this attribute (empty string for unprefixed attributes).
    pub ns: Cow<'xml, str>,
    /// Local attribute name.
    pub name: Cow<'xml, str>,
    /// Attribute value.
    pub value: Cow<'xml, str>,
}

impl<'a> AnyAttribute<'a> {
    /// Converts this attribute into an owned version with `'static` lifetime.
    pub fn into_owned(self) -> AnyAttribute<'static> {
        AnyAttribute {
            ns: Cow::Owned(self.ns.into_owned()),
            name: Cow::Owned(self.name.into_owned()),
            value: Cow::Owned(self.value.into_owned()),
        }
    }
}

impl ToXml for AnyElement<'_> {
    fn serialize<W: fmt::Write + ?Sized>(
        &self,
        field: Option<Id<'_>>,
        serializer: &mut Serializer<'_, W>,
    ) -> Result<(), Error> {
        let id = field.unwrap_or(Id {
            name: &self.name,
            ns: &self.ns,
        });
        let element = serializer.write_start(id.name, id.ns, None::<Context<0>>)?;

        for attr in &self.attributes {
            serializer.write_attr(&attr.name, &attr.ns, &attr.value)?;
        }

        let has_content = self.text.is_some() || !self.children.is_empty();
        if !has_content {
            serializer.end_empty()?;
            return Ok(());
        }

        serializer.end_start()?;

        if let Some(text) = &self.text {
            text.serialize(None, serializer)?;
        }

        for child in &self.children {
            child.serialize(None, serializer)?;
        }

        serializer.write_close(element)?;
        Ok(())
    }
}

impl ToXml for AnyAttribute<'_> {
    fn serialize<W: fmt::Write + ?Sized>(
        &self,
        field: Option<Id<'_>>,
        serializer: &mut Serializer<'_, W>,
    ) -> Result<(), Error> {
        self.value.serialize(field, serializer)
    }

    fn serialize_attribute<W: fmt::Write + ?Sized>(
        &self,
        _field: Option<Id<'_>>,
        serializer: &mut Serializer<'_, W>,
    ) -> Result<(), Error> {
        serializer.write_attr(&self.name, &self.ns, &self.value)
    }
}
