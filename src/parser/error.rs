/*
** This file is a part of Iksemel (XML parser for Jabber/XMPP)
** Copyright (C) 2000-2025 Gurer Ozen
**
** Iksemel is free software: you can redistribute it and/or modify it
** under the terms of the GNU Lesser General Public License as
** published by the Free Software Foundation, either version 3 of
** the License, or (at your option) any later version.
*/

use std::error::Error;
use std::fmt::Display;

/// The error type for the SAX handler.
#[derive(Debug, Eq, PartialEq, Copy, Clone)]
pub enum SaxHandlerError {
    /// Handler wants to abort parsing.
    ///
    /// This should be used when the reason for aborting is
    /// application specific and unrelated to the syntax or
    /// well-formedness of the XML document.
    ///
    /// [parse_bytes()](super::SaxParser::parse_bytes) will stop
    /// processing and return [SaxError::HandlerError](super::SaxError::HandlerAbort)
    /// to the caller.
    Abort,

    /// Handler detected an error in the XML document.
    ///
    /// Certain errors, such as mismatched names of the start and end tags,
    /// or duplicate attribute names in the same tag, are not checked by the
    /// [SaxParser](super::SaxParser), since that requires a potentially large
    /// state to be maintained.
    ///
    /// Users of the [SaxParser](super::SaxParser) can implement these
    /// checks if they want to. The [DocumentParser](crate::DocumentParser)
    /// for example, already builds an in-memory representation of the XML
    /// tree structure, and applies all these checks without additional cost.
    ///
    /// The static string argument should provide a short description of
    /// the error detected by the handler.
    ///
    /// This error will be returned as [SaxError::BadXml] to the caller
    /// from [parse_bytes()](super::SaxParser::parse_bytes) method.
    BadXml(&'static str),
}

impl Display for SaxHandlerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SaxHandlerError::Abort => write!(f, "SAX handler aborts"),
            SaxHandlerError::BadXml(msg) => write!(f, "SAX handler error: {}", msg),
        }
    }
}

impl Error for SaxHandlerError {}

/// The error type for the SAX parsing operations.
///
/// These categories are designed to be as few as possible and correspond to the distinct
/// actions you might take based on the nature of the problem.
///
/// Location of the error is available via [nr_bytes()](super::SaxParser::nr_bytes),
/// [nr_lines()](super::SaxParser::nr_lines), and
/// [nr_column()](super::SaxParser::nr_column) functions.
#[derive(Debug, Eq, PartialEq, Copy, Clone)]
pub enum SaxError {
    /// Parser could not allocate the memory needed for parsing buffers.
    ///
    /// A character buffer is used to collect the tag names and the attribute key
    /// value pairs before passing them to the application as a continous string.
    /// This is important when the data is coming through a network connection
    /// or not loaded entirely into the memory. This also helps with the reference
    /// substitution in the attribute values.
    ///
    /// Initial buffer (128 bytes) is more than enough for normal documents.
    /// Seeing this error is highly unlikely in practice, but since the XML standard
    /// does not specify a maximum tag length, it is possible to hit this condition
    /// with either a degenarate document with gigabytes long tag name, or
    /// while running under severely memory constrained platforms.
    NoMemory,

    /// A syntax error is encountered in the XML input.
    ///
    /// The static string describes the actual syntax issue.
    BadXml(&'static str),

    /// Element handler method wants to abort.
    ///
    /// This is intended for your handler to be able to abort the parsing while
    /// signalling that the interruption is not caused by iksemel itself.
    HandlerAbort,
}

impl From<SaxHandlerError> for SaxError {
    fn from(err: SaxHandlerError) -> Self {
        match err {
            SaxHandlerError::Abort => SaxError::HandlerAbort,
            SaxHandlerError::BadXml(msg) => SaxError::BadXml(msg),
        }
    }
}

impl Display for SaxError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SaxError::NoMemory => write!(f, "not enough memory"),
            SaxError::BadXml(msg) => write!(f, "invalid xml syntax: {}", msg),
            SaxError::HandlerAbort => write!(f, "abort from sax handler"),
        }
    }
}

impl Error for SaxError {}

pub(super) mod description {
    pub(in super::super) const UTF8_INVALID_CONT_BYTE: &str = "Invalid UTF8 continuation byte";
    pub(in super::super) const UTF8_OVERLONG_SEQUENCE: &str = "Overlong UTF8 sequence";
    pub(in super::super) const UTF8_INVALID_PREFIX_BYTE: &str = "Invalid UTF8 prefix byte";
    pub(in super::super) const CHAR_INVALID: &str = "Invalid XML character";
    pub(in super::super) const DOC_NO_CONTENT: &str = "Document has no root tag";
    pub(in super::super) const DOC_OPEN_TAGS: &str = "Document has unclosed tags";
    pub(in super::super) const DOC_OPEN_MARKUP: &str =
        "Document epilog has unclosed PI or comment tag";
    pub(in super::super) const DOC_CDATA_WITHOUT_PARENT: &str =
        "Character data not allowed outside of the root tag";
    pub(in super::super) const TAG_CLOSE_WITHOUT_OPEN: &str = "Close tag without open";
    pub(in super::super) const TAG_WHITESPACE_START: &str = "Tag cannot start with whitespace";
    pub(in super::super) const TAG_OUTSIDE_ROOT: &str = "Tag cannot be outside of the root tag";
    pub(in super::super) const TAG_EMPTY_NAME: &str = "Tag has no name";
    pub(in super::super) const TAG_DOUBLE_END: &str = "End tag has standalone ending too";
    pub(in super::super) const TAG_END_TAG_ATTRIBUTES: &str = "End tag cannot have attributes";
    pub(in super::super) const TAG_EMPTY_TAG_MISSING_END: &str =
        "Empty element tags must end after the '/'";
    pub(in super::super) const TAG_ATTRIBUTE_WITHOUT_EQUAL: &str =
        "Tag attributes must have '=' before the value";
    pub(in super::super) const TAG_ATTRIBUTE_WITHOUT_QUOTE: &str =
        "Tag attribute value must be double or single quotes";
    pub(in super::super) const TAG_ATTRIBUTE_BAD_NAME: &str =
        "Tag attribute names cannot have '/', '<' or '>'";
    pub(in super::super) const TAG_ATTRIBUTE_BAD_VALUE: &str =
        "Tag attribute value cannot have '<' character without a reference";
    pub(in super::super) const REFERENCE_INVALID_DECIMAL: &str =
        "Non digit in decimal character reference";
    pub(in super::super) const REFERENCE_INVALID_HEX: &str =
        "Non hex digit in hexadecimal character reference";
    pub(in super::super) const REFERENCE_CUSTOM_ENTITY: &str =
        "Non-predefined entity references are not supported";
    pub(in super::super) const COMMENT_MISSING_DASH: &str =
        "Comment tag should start with double dash";
    pub(in super::super) const COMMENT_MISSING_END: &str =
        "Comment tag should end after double dash";
    pub(in super::super) const MARKUP_CDATA_SECTION_BAD_START: &str =
        "Character data sections must start with '[CDATA['";
    pub(in super::super) const MARKUP_DOCTYPE_BAD_START: &str =
        "Doctype must start with 'DOCTYPE '";
    pub(in super::super) const MARKUP_CDATA_SECTION_OUTSIDE_ROOT: &str =
        "Character data sections cannot be outside of the root tag";
    pub(in super::super) const MARKUP_UNRECOGNIZED: &str =
        "Markup is not a comment, character data section, or document type declaration";
    pub(in super::super) const PI_MISSING_END: &str =
        "Processing instruction must end after closing the '?'";
}
