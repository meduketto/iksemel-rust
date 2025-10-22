/*
** This file is a part of Iksemel (XML parser for Jabber/XMPP)
** Copyright (C) 2000-2025 Gurer Ozen
**
** Iksemel is free software: you can redistribute it and/or modify it
** under the terms of the GNU Lesser General Public License as
** published by the Free Software Foundation, either version 3 of
** the License, or (at your option) any later version.
*/

#![deny(clippy::suspicious)]
#![deny(clippy::complexity)]
#![deny(clippy::perf)]
#![deny(clippy::style)]
#![deny(clippy::cargo)]
#![deny(clippy::items_after_statements)]
#![deny(clippy::missing_panics_doc)]
#![deny(clippy::uninlined_format_args)]
#![deny(clippy::unnecessary_semicolon)]
#![deny(clippy::unreadable_literal)]
#![deny(clippy::allow_attributes_without_reason)]
#![deny(clippy::panic)]
#![deny(clippy::partial_pub_fields)]
#![deny(clippy::redundant_test_prefix)]
//#![deny(clippy::undocumented_unsafe_blocks)]
//#![deny(missing_docs)]
#![allow(clippy::multiple_crate_versions, reason = "rpassword problem")]

//! # Introduction
//!
//! Iks is an XML parser library for [Jabber/XMPP](https://xmpp.org/)
//! and general XML processing applications. It aims to be easy to use,
//! fast, and usable in resource-constrained environments.
//!
//! Module architecture:
//! ```text
//!           +---------+
//!           |SaxParser|--\
//!           +---------+   \    +--------+    +------+    +--------+    +------+
//!                          \-->|Document|--->|Stream|--->|Xmpp    |--->|Xmpp  |
//! +-----+    +--------+        |Builder |    |Parser|    |Client  |    |Client|
//! |Arena|--->|Document|<------>|& Parser|    +------+    |Protocol|    +------+
//! +-----+    |& Cursor|        +--------+                +--------+
//!            +--------+
//!                 |             +-----+
//!                 v------------>|XPath|
//!                               +-----+
//! ```
//!
//! Arena: A compact and fast memory allocation arena for storing XML
//! element tree structs and character data. Not used directly by
//! the applications.
//!
//! Sax Parser: This fast and memory efficient parser is the core of
//! the Iksemel. Validates and processes byte streams and generates
//! XML elements.
//!
//! Document: Builds and queries XML element trees inside Arenas.
//!
//! Document Parser: Parses an XML byte stream into an XML element
//! tree structure.
//!
//! Stream Parser: Parses an XML byte stream into an XMPP Stream
//! with individual top level elements.
//!
//! XmppClientProtocol: A sans-io implementation of the XMPP client
//! stream protocol, including authentication and stanza handling.
//!
//! XmppClient: A complete client implementation using blocking-io
//! operations on top of the XmppClientProtocol.
//!

mod arena;
mod document;
mod entities;
mod parser;
mod xmpp;
mod xpath;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

pub use arena::Arena;
pub use arena::ArenaStats;
pub use arena::NoMemory;

pub use parser::Location;
pub use parser::ParseError;
pub use parser::SaxElement;
pub use parser::SaxElements;
pub use parser::SaxParser;

pub use document::Ancestor;
pub use document::Attributes;
pub use document::Children;
pub use document::Cursor;
pub use document::DescendantOrSelf;
pub use document::Document;
pub use document::DocumentBuilder;
pub use document::DocumentParser;
pub use document::FollowingSibling;
pub use document::PrecedingSibling;
pub use document::SyncCursor;

pub use xmpp::BadJid;
pub use xmpp::Jid;
pub use xmpp::StreamElement;
pub use xmpp::StreamError;
pub use xmpp::StreamParser;
pub use xmpp::XmppClient;
pub use xmpp::XmppClientError;
pub use xmpp::XmppClientProtocol;
pub use xmpp::constants::CLIENT_PORT as XMPP_CLIENT_PORT;
pub use xmpp::constants::SERVER_PORT as XMPP_SERVER_PORT;

pub use xpath::XPath;
