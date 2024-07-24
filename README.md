![Cover logo](./cover.svg)

# instant-xml: more rigorously mapping XML to Rust types

[![Documentation](https://docs.rs/instant-xml/badge.svg)](https://docs.rs/instant-xml)
[![Crates.io](https://img.shields.io/crates/v/instant-xml.svg)](https://crates.io/crates/instant-xml)
[![Build status](https://github.com/InstantDomain/instant-xml/workflows/CI/badge.svg)](https://github.com/InstantDomain/instant-xml/actions?query=workflow%3ACI)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE-MIT)
[![License: Apache 2.0](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](LICENSE-APACHE)

instant-xml is a serde-like library providing traits and procedural macros to help map XML to Rust
types. While serde is great for formats like JSON, the underlying structure it provides is not a
great fit for XML, limiting serde-based tools like quick-xml. instant-xml more rigorously maps the
XML data model (including namespaces) to Rust types while providing a serde-like interface.

This library is used in production at [Instant Domains](https://instantdomains.com/).

## Features

* Familiar serde-like interface
* Full support for XML namespaces
* Avoids copying deserialized data where possible
* Minimum supported Rust version is 1.58

## Limitations

instant-xml is still early in its lifecycle. While it works well for our use cases, it might not
work well for you, and several more semver-incompatible releases should be expected to flesh out
the core trait APIs as we throw more test cases at it. There's also currently not that much
documentation.

We'd love to hear your feedback!

## Thanks

Thanks to [@rsdy](https://github.com/rsdy) and [@choinskib](https://github.com/choinskib) for
their work on this library, and thanks (of course) to [@dtolnay](https://github.com/dtolnay/) for
creating serde.
