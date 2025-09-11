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

use crate::SaxElement;
use crate::SaxError;
use crate::SaxHandler;

use super::Cursor;
use super::Document;
use super::Node;
use super::error::description;

pub struct DocumentBuilder {
    doc: Option<Document>,
    node: *mut Node,
}

impl DocumentBuilder {
    pub fn new() -> Self {
        DocumentBuilder {
            doc: None,
            node: null_mut(),
        }
    }

    pub fn take(&mut self) -> Option<Document> {
        self.doc.take()
    }

    pub fn replace(&mut self, doc: Document) -> Option<Document> {
        self.doc.replace(doc)
    }
}

impl Default for DocumentBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl SaxHandler for DocumentBuilder {
    fn handle_element(&mut self, element: &SaxElement) -> Result<(), SaxError> {
        match &self.doc {
            None => match element {
                SaxElement::StartTag(name) => {
                    let doc = Document::new(name)?;
                    self.node = doc.root().get_node_ptr();
                    self.doc = Some(doc);
                }
                _ => return Err(SaxError::HandlerAbort),
            },
            Some(doc) => match element {
                SaxElement::StartTag(name) => {
                    let new_tag = Cursor::new(self.node, &doc.arena).insert_tag(name)?;
                    self.node = new_tag.get_node_ptr();
                }
                SaxElement::Attribute(name, value) => {
                    Cursor::new(self.node, &doc.arena).insert_attribute(name, value)?;
                }
                SaxElement::EmptyElementTag => {
                    self.node = Cursor::new(self.node, &doc.arena).parent().get_node_ptr();
                }
                SaxElement::CData(cdata) => {
                    Cursor::new(self.node, &doc.arena).insert_cdata(cdata)?;
                }
                SaxElement::EndTag(name) => {
                    if name != &Cursor::new(self.node, &doc.arena).name() {
                        return Err(SaxError::BadXml(description::TAG_MISMATCH));
                    }
                    self.node = Cursor::new(self.node, &doc.arena).parent().get_node_ptr();
                }
            },
        }
        Ok(())
    }
}
