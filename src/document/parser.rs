/*
** This file is a part of Iksemel (XML parser for Jabber/XMPP)
** Copyright (C) 2000-2025 Gurer Ozen
**
** Iksemel is free software: you can redistribute it and/or modify it
** under the terms of the GNU Lesser General Public License as
** published by the Free Software Foundation, either version 3 of
** the License, or (at your option) any later version.
*/

use crate::Location;
use crate::ParseError;
use crate::SaxElements;
use crate::SaxParser;

use super::Document;
use super::DocumentBuilder;
use super::error::description;

/// A DOM (Document Object Model) parser.
///
/// This parser uses a SAX parser to parse XML input text,
/// and builds a document tree. Such a tree can be used to
/// run queries or make manipulations on the content.
///
/// # Examples
///
/// ```
/// use iks::{DocumentParser, ParseError};
/// # fn main() -> Result<(), ParseError> {
///
/// let xml_text = "<a><b>abc</b></a>";
///
/// let mut parser = DocumentParser::new();
/// parser.parse_bytes(xml_text.as_bytes())?;
/// let document = parser.into_document()?;
///
/// assert_eq!(document.find_tag("b").first_child().cdata(), "abc");
/// # Ok(())
/// # }
/// ```
///
/// You can also use the `From` trait if you want to parse
/// a complete XML text inside a slice.
/// ```
/// # use iks::ParseError;
/// use std::str::FromStr;
/// use iks::Document;
/// # fn main() -> Result<(), ParseError> {
/// let document = Document::from_str("<a><b>abc</b></a>")?;
/// # Ok(())
/// # }
/// ```
///
pub struct DocumentParser {
    builder: DocumentBuilder,
    parser: SaxParser,
}

impl DocumentParser {
    /// Creates a new `DocumentParser`.
    pub fn new() -> DocumentParser {
        DocumentParser {
            builder: DocumentBuilder::new(),
            parser: SaxParser::new(),
        }
    }

    /// Creates a new `DocumentParser` optimized for the given XML text size.
    ///
    /// The size hint is the approximate number of bytes in the
    /// XML text which is going to be parsed. This number is used
    /// to finetune the backing memory arena for the document
    /// structures, so a fewer allocations are made.
    pub fn with_size_hint(size_hint: usize) -> DocumentParser {
        DocumentParser {
            builder: DocumentBuilder::with_size_hint(size_hint),
            parser: SaxParser::new(),
        }
    }

    pub fn parse_bytes(&mut self, bytes: &[u8]) -> Result<(), ParseError> {
        let mut elements = SaxElements::new(&mut self.parser, bytes);
        loop {
            match elements.next() {
                Some(Ok(element)) => {
                    self.builder.append_element(&element)?;
                }
                Some(Err(err)) => return Err(err),
                None => {
                    break;
                }
            }
        }
        Ok(())
    }

    pub fn into_document(mut self) -> Result<Document, ParseError> {
        self.parser.parse_finish()?;
        let doc = self.builder.take();
        match doc {
            None => Err(ParseError::BadXml(description::NO_DOCUMENT)),
            Some(doc) => Ok(doc),
        }
    }

    pub fn take_document(&mut self) -> Result<Document, ParseError> {
        self.parser.parse_finish()?;
        let doc = self.builder.take();
        match doc {
            None => Err(ParseError::BadXml(description::NO_DOCUMENT)),
            Some(doc) => Ok(doc),
        }
    }

    pub fn reuse_document_memory(&mut self, doc: Document) {
        let _old_doc = self.builder.replace(doc);
    }

    pub fn location(&self) -> Location {
        self.parser.location()
    }
}

impl Default for DocumentParser {
    fn default() -> Self {
        Self::new()
    }
}
