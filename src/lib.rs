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

pub use arena::Arena;
pub use arena::NoMemory;

pub use document::Cursor;
pub use document::Document;
pub use document::DocumentParser;

pub use parser::SaxElement;
pub use parser::SaxError;
pub use parser::SaxHandler;
pub use parser::SaxHandlerError;
pub use parser::SaxParser;
