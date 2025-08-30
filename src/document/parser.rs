/*
** This file is a part of Iksemel (XML parser for Jabber/XMPP)
** Copyright (C) 2000-2025 Gurer Ozen
**
** Iksemel is free software: you can redistribute it and/or modify it
** under the terms of the GNU Lesser General Public License as
** published by the Free Software Foundation, either version 3 of
** the License, or (at your option) any later version.
*/

use std::ptr::null_mut;

use crate::Location;
use crate::SaxElement;
use crate::SaxError;
use crate::SaxHandler;
use crate::SaxParser;

use super::Cursor;
use super::Document;
use super::Node;
use super::error::DocumentError;
use super::error::description;

struct DocumentBuilder {
    doc: Option<Document>,
    node: *mut Node,
}

impl DocumentBuilder {
    fn new() -> DocumentBuilder {
        DocumentBuilder {
            doc: None,
            node: null_mut(),
        }
    }
}

impl SaxHandler for DocumentBuilder {
    fn handle_element(&mut self, element: &SaxElement) -> Result<(), SaxError> {
        match &self.doc {
            None => match element {
                SaxElement::StartTag(name) => {
                    let doc = Document::new(name);
                    self.node = doc.root().get_node_ptr();
                    self.doc = Some(doc);
                }
                _ => return Err(SaxError::HandlerAbort),
            },
            Some(doc) => match element {
                SaxElement::StartTag(name) => {
                    let new_tag = Cursor::new(self.node).insert_tag(doc, name);
                    self.node = new_tag.get_node_ptr();
                }
                SaxElement::Attribute(name, value) => {
                    Cursor::new(self.node).set_attribute(doc, name, value);
                }
                SaxElement::EmptyElementTag => {
                    self.node = Cursor::new(self.node).parent().get_node_ptr();
                }
                SaxElement::CData(cdata) => {
                    Cursor::new(self.node).insert_cdata(doc, cdata);
                }
                SaxElement::EndTag(name) => {
                    if name != &Cursor::new(self.node).name() {
                        return Err(SaxError::BadXml(description::TAG_MISMATCH));
                    }
                    self.node = Cursor::new(self.node).parent().get_node_ptr();
                }
            },
        }

        Ok(())
    }
}

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
        match self.builder.doc {
            None => Err(DocumentError::BadXml(description::NO_DOCUMENT)),
            Some(doc) => Ok(doc),
        }
    }

    pub fn take_document(&mut self) -> Result<Document, DocumentError> {
        self.parser.parse_finish()?;
        let doc = self.builder.doc.take();
        match doc {
            None => Err(DocumentError::BadXml(description::NO_DOCUMENT)),
            Some(doc) => Ok(doc),
        }
    }

    pub fn reuse_document_memory(&mut self, doc: Document) {
        let _old_doc = self.builder.doc.replace(doc);
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
