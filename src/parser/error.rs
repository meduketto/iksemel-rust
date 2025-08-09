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
    DocNoContent,
    DocOpenTags,
    DocOpenMarkup,
    PrologCdata,
    TagCloseWithoutOpen,
    TagWhitespaceStart,
    TagOutsideRoot,
}

impl XmlError {
    pub(super) fn description(&self) -> &'static str {
        match self {
            XmlError::ParserReuseWithoutReset => "cannot continue after an error without a reset",
            XmlError::Utf8InvalidContByte => "Invalid UTF8 continuation byte",
            XmlError::Utf8OverlongSequence => "Overlong UTF8 sequence",
            XmlError::Utf8InvalidPrefixByte => "Invalid UTF8 prefix byte",
            XmlError::CharInvalid => "Invalid XML character",
            XmlError::DocNoContent => "Document has no root tag",
            XmlError::DocOpenTags => "Document has unclosed tags",
            XmlError::DocOpenMarkup => "Document epilog has unclosed PI or comment tag",
            XmlError::PrologCdata => "Character data not allowed outside of root tag",
            XmlError::TagCloseWithoutOpen => "Close tag without open",
            XmlError::TagWhitespaceStart => "Tag cannot start with whitespace",
            XmlError::TagOutsideRoot => "Tags cannot be outside of the root tag",
        }
    }
}
