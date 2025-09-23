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

//! This library is made up from layered modules which build upon each
//! other to give applications a lot of flexibility and control while
//! providing interfaces at every level.
//!
//! # Sax Parser
//!
//! This fast and memory efficient parser is the core of the Iksemel.
//! It validates and processes byte streams and generates XML elements.
//!
//! See:
//! [ParseError],
//! [SaxParser],
//! [SaxElement],
//! [SaxElements],
//! [Location]
//!
//! # Arena
//!
//! This module provides a compact and fast memory allocation arena
//! for storing XML element tree structs and character data. It is
//! generally not used directly by applications.
//!
//! See:
//! [Arena],
//! [ArenaStats],
//! [NoMemory]
//!
//! # Document
//!
//! This module builds upon the Arena module to create and query
//! XML element trees.
//!
//! See:
//! [Document],
//! [Cursor],
//! [Ancestor],
//! [Children],
//! [Attributes],
//! [DescendantOrSelf],
//! [FollowingSibling],
//! [PrecedingSibling]
//!
//! # Document Parser
//!
//! This module builds upon the Document and Sax Parser modules to
//! parse an XML byte stream into an XML element tree structure.
//!
//! See:
//! [DocumentBuilder],
//! [DocumentParser]
//!
//! # Stream Parser
//!
//! This module builds upon the Document and Sax Parser modules to
//! parse an XML byte stream into an XMPP Stream.
//!
//! See:
//! [StreamError],
//! [StreamParser]
//!
//! # Client Stream
//!
//! This module builds upon the Stream Parser module to handle XMPP
//! client stream protocol, including authentication and stanza handling.
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

pub use xmpp::BadJid;
pub use xmpp::Jid;
pub use xmpp::StreamElement;
pub use xmpp::StreamError;
pub use xmpp::StreamParser;

pub use xpath::XPath;
