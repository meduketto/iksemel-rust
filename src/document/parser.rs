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
use crate::SaxParser;

use super::Document;
use super::DocumentBuilder;
use super::error::DocumentError;
use super::error::description;

pub struct DocumentParser {
    builder: DocumentBuilder,
    parser: SaxParser,
}

impl DocumentParser {
    pub fn new() -> DocumentParser {
        DocumentParser {
            builder: DocumentBuilder::new(),
            parser: SaxParser::new(),
        }
    }

    pub fn parse_bytes(&mut self, bytes: &[u8]) -> Result<(), DocumentError> {
        Ok(self.parser.parse_bytes(&mut self.builder, bytes)?)
    }

    pub fn into_document(mut self) -> Result<Document, DocumentError> {
        self.parser.parse_finish()?;
        let doc = self.builder.take();
        match doc {
            None => Err(DocumentError::BadXml(description::NO_DOCUMENT)),
            Some(doc) => Ok(doc),
        }
    }

    pub fn take_document(&mut self) -> Result<Document, DocumentError> {
        self.parser.parse_finish()?;
        let doc = self.builder.take();
        match doc {
            None => Err(DocumentError::BadXml(description::NO_DOCUMENT)),
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
