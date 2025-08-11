/*
** This file is a part of Iksemel (XML parser for Jabber/XMPP)
** Copyright (C) 2000-2025 Gurer Ozen
**
** Iksemel is free software: you can redistribute it and/or modify it
** under the terms of the GNU Lesser General Public License as
** published by the Free Software Foundation, either version 3 of
** the License, or (at your option) any later version.
*/

/// Type of the error which happened during the XML SAX parsing.
///
/// These categories are designed to be as few as possible and correspond to the distinct
/// actions the caller might take based on the problem. Further details are available via
/// other [SaxParser](super::SaxParser) methods.
///
/// Location of the error is available via [nr_bytes()](super::SaxParser::nr_bytes),
/// [nr_lines()](super::SaxParser::nr_lines), and
/// [nr_column()](super::SaxParser::nr_column) functions.
#[derive(Debug, Eq, PartialEq)]
pub enum SaxError {
    /// Parser could not allocate the memory needed for parsing buffers.
    NoMemory,

    /// A syntax error is encountered in the XML input.
    ///
    /// Typical action is telling error details to the user so they can fix the document.
    /// Description of the actual syntax issue can be retrieved via
    /// [error_description()](super::SaxParser::error_description) function.
    BadXml,

    /// A certain XML feature is not supported by the parser.
    ///
    /// Only unsupported feature at the moment is the custom entities defined in DTDs.
    ///
    /// Handling of this would be similar to BadXml except the wording should indicate
    /// that the issue is not caused by a problem in the document.
    NotSupported,

    /// Element handler function returned this error.
    ///
    /// This is intended for caller's handler to be able to abort the processing while
    /// signalling to the caller that the interruption is not caused by iksemel itself.
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
    DocCdataWithoutParent,
    TagCloseWithoutOpen,
    TagWhitespaceStart,
    TagOutsideRoot,
    TagEmptyName,
    TagDoubleEnd,
    TagEndTagAttributes,
    TagEmptyTagMissingEnd,
    TagAttributeWithoutEqual,
    TagAttributeWithoutQuote,
    TagAttributeBadName,
    TagAttributeBadValue,
    ReferenceInvalidDecimal,
    ReferenceInvalidHex,
    ReferenceCustomEntity,
    CommentMissingDash,
    CommentMissingEnd,
    MarkupCdataSectionBadStart,
    MarkupDoctypeBadStart,
    MarkupCdataSectionOutsideRoot,
    MarkupUnrecognized,
    PiMissingEnd,
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
            XmlError::DocCdataWithoutParent => "Character data not allowed outside of the root tag",
            XmlError::TagCloseWithoutOpen => "Close tag without open",
            XmlError::TagWhitespaceStart => "Tag cannot start with whitespace",
            XmlError::TagOutsideRoot => "Tags cannot be outside of the root tag",
            XmlError::TagEmptyName => "Tag has no name",
            XmlError::TagDoubleEnd => "End tag has standalone ending too",
            XmlError::TagEndTagAttributes => "End tag cannot have attributes",
            XmlError::TagEmptyTagMissingEnd => "Empty element tags must end after the '/'",
            XmlError::TagAttributeWithoutEqual => "Tag attributes must have '=' before the value",
            XmlError::TagAttributeWithoutQuote => {
                "Tag attribute value must be double or single quotes"
            }
            XmlError::TagAttributeBadName => "Tag attribute names cannot have '/', '<' or '>'",
            XmlError::TagAttributeBadValue => {
                "Tag value cannot have '<' character without a reference"
            }
            XmlError::ReferenceInvalidDecimal => "Non digit in decimal character refence",
            XmlError::ReferenceInvalidHex => "Non hex digit in hexadecimal character refence",
            XmlError::ReferenceCustomEntity => "Non-predefined entity references are not supported",
            XmlError::CommentMissingDash => "Comment tag should start with double dash",
            XmlError::CommentMissingEnd => "Comment tag should end after double dash",
            XmlError::MarkupCdataSectionBadStart => {
                "Character data sections must start with '[CDATA['"
            }
            XmlError::MarkupDoctypeBadStart => "Doctype must start with 'DOCTYPE '",
            XmlError::MarkupCdataSectionOutsideRoot => {
                "Character data sections cannot be outside of the root tag"
            }
            XmlError::MarkupUnrecognized => {
                "Markup is not a comment, character data section, or document type declaration"
            }
            XmlError::PiMissingEnd => "Processing instruction must end after closing the '?'",
        }
    }
}

// rustdoc
// no mem error?
// ci mutant check
