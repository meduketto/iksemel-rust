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
#[derive(Debug, Eq, PartialEq)]
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
/// actions you might take based on the nature of theproblem.
///
/// Location of the error is available via [nr_bytes()](super::SaxParser::nr_bytes),
/// [nr_lines()](super::SaxParser::nr_lines), and
/// [nr_column()](super::SaxParser::nr_column) functions.
#[derive(Debug, Eq, PartialEq)]
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
