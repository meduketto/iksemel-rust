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
use crate::DocumentError;
use crate::SaxElement;
use crate::SaxError;
use crate::SaxHandler;
use crate::parser::SaxParser;

struct StreamBuilder {}

impl SaxHandler for StreamBuilder {
    fn handle_element(&mut self, _element: &SaxElement) -> Result<(), SaxError> {
        Ok(())
    }
}

pub trait StreamHandler {
    fn handle_stream_element(&mut self, element: Document);
    fn handle_stream_end(&mut self);
}

pub struct StreamParser {
    parser: SaxParser,
    builder: StreamBuilder,
}

impl StreamParser {
    pub fn new() -> Self {
        Self {
            parser: SaxParser::new(),
            builder: StreamBuilder {},
        }
    }

    pub fn parse_bytes(&mut self, bytes: &[u8]) -> Result<(), DocumentError> {
        Ok(self.parser.parse_bytes(&mut self.builder, bytes)?)
    }
}

impl Default for StreamParser {
    fn default() -> Self {
        Self::new()
    }
}
