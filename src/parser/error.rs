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
pub enum SaxError {
    NoMemory,
    BadXml,
    NotSupported,
    HandlerError,
}

impl std::fmt::Display for SaxError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SaxError::NoMemory => write!(f, "not enough memory"),
            SaxError::BadXml => write!(f, "invalid xml syntax"),
            SaxError::NotSupported => write!(f, "xml construct not supported"),
            SaxError::HandlerError => write!(f, "error from sax handler"),
        }
    }
}

impl std::error::Error for SaxError {}

pub(super) enum XmlError {
    ParserReuseWithoutReset,
    Utf8InvalidContByte,
    Utf8OverlongSequence,
    Utf8InvalidPrefixByte,
    CharInvalid,
}

impl XmlError {
    pub(super) fn description(&self) -> &'static str {
        match self {
            XmlError::ParserReuseWithoutReset => "blah",
            XmlError::Utf8InvalidContByte => "blah",
            XmlError::Utf8OverlongSequence => "blah",
            XmlError::Utf8InvalidPrefixByte => "blah",
            XmlError::CharInvalid => "blah",
        }
    }
}
