/*
** This file is a part of Iksemel (XML parser for Jabber/XMPP)
** Copyright (C) 2000-2025 Gurer Ozen
**
** Iksemel is free software: you can redistribute it and/or modify it
** under the terms of the GNU Lesser General Public License as
** published by the Free Software Foundation, either version 3 of
** the License, or (at your option) any later version.
*/

#[derive(Debug, Eq, PartialEq)]
pub enum ParserError {
    NoMemory,
    BadXml,
    HandlerError,
}

impl std::fmt::Display for ParserError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParserError::NoMemory => write!(f, "not enough memory"),
            ParserError::BadXml => write!(f, "invalid xml syntax"),
            ParserError::HandlerError => write!(f, "error from sax handler"),
        }
    }
}

impl std::error::Error for ParserError {}
