/*
** This file is a part of Iksemel (XML parser for Jabber/XMPP)
** Copyright (C) 2000-2025 Gurer Ozen
**
** Iksemel is free software: you can redistribute it and/or modify it
** under the terms of the GNU Lesser General Public License as
** published by the Free Software Foundation, either version 3 of
** the License, or (at your option) any later version.
*/

use crate::Document;
use crate::DocumentBuilder;
use crate::SaxElement;
use crate::SaxError;
use crate::SaxHandler;
use crate::parser::SaxParser;

use super::StreamError;

struct StreamBuilder<'a> {
    builder: DocumentBuilder,
    level: usize,
    handler: &'a mut dyn StreamHandler,
}

impl<'a> StreamBuilder<'a> {
    fn new(handler: &'a mut impl StreamHandler) -> Self {
        Self {
            builder: DocumentBuilder::new(),
            level: 0,
            handler,
        }
    }
}

impl<'a> SaxHandler for StreamBuilder<'a> {
    fn handle_element(&mut self, element: &SaxElement) -> Result<(), SaxError> {
        match element {
            SaxElement::StartTag(_) => {
                self.level += 1;
            }
            SaxElement::EmptyElementTag | SaxElement::EndTag(_) => {
                self.level -= 1;
            }
            _ => {}
        }
        self.builder.handle_element(element)?;
        match element {
            SaxElement::EmptyElementTag | SaxElement::EndTag(_) => {
                if self.level == 0 {
                    self.handler
                        .handle_stream_element(self.builder.take().unwrap());
                }
            }
            _ => {}
        }
        Ok(())
    }
}

pub trait StreamHandler {
    fn handle_stream_element(&mut self, element: Document);
    fn handle_stream_end(&mut self);
}

pub struct StreamParser<'a> {
    parser: SaxParser,
    builder: StreamBuilder<'a>,
}

impl<'a> StreamParser<'a> {
    pub fn new(handler: &'a mut impl StreamHandler) -> Self {
        Self {
            parser: SaxParser::new(),
            builder: StreamBuilder::new(handler),
        }
    }

    pub fn parse_bytes(&mut self, bytes: &[u8]) -> Result<(), StreamError> {
        Ok(self.parser.parse_bytes(&mut self.builder, bytes)?)
    }
}
