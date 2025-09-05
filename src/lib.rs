/*
** This file is a part of Iksemel (XML parser for Jabber/XMPP)
** Copyright (C) 2000-2025 Gurer Ozen
**
** Iksemel is free software: you can redistribute it and/or modify it
** under the terms of the GNU Lesser General Public License as
** published by the Free Software Foundation, either version 3 of
** the License, or (at your option) any later version.
*/

mod arena;
mod document;
mod entities;
mod parser;
mod xpath;

pub use arena::Arena;
pub use arena::ArenaStats;
pub use arena::NoMemory;

pub use parser::Location;
pub use parser::SaxElement;
pub use parser::SaxError;
pub use parser::SaxHandler;
pub use parser::SaxParser;

pub use document::Attributes;
pub use document::Cursor;
pub use document::DescendantOrSelf;
pub use document::Document;
pub use document::DocumentError;
pub use document::DocumentParser;

pub use xpath::XPath;
