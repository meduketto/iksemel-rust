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
use crate::ParseError;
use crate::SaxElement;
use crate::SaxParser;

use super::StreamError;
use super::constants::*;

pub enum StreamElement {
    Element(Document),
    End,
}

pub struct StreamElements<'a> {
    parser: &'a mut StreamParser,
    bytes: &'a [u8],
    bytes_parsed: usize,
}

impl<'a> StreamElements<'a> {
    pub fn new(parser: &'a mut StreamParser, bytes: &'a [u8]) -> Self {
        Self {
            parser,
            bytes,
            bytes_parsed: 0,
        }
    }

    pub fn next(&mut self) -> Option<Result<StreamElement, StreamError>> {
        if self.bytes_parsed >= self.bytes.len() {
            return None;
        }
        match self.parser.parse_bytes(&self.bytes[self.bytes_parsed..]) {
            Ok(Some((element, bytes))) => {
                self.bytes_parsed += bytes;
                Some(Ok(element))
            }
            Ok(None) => {
                self.bytes_parsed = self.bytes.len();
                None
            }
            Err(err) => Some(Err(err.into())),
        }
    }
}

pub struct StreamParser {
    sax_parser: SaxParser,
    builder: DocumentBuilder,
    level: usize,
}

impl StreamParser {
    pub fn new() -> Self {
        Self {
            sax_parser: SaxParser::new(),
            builder: DocumentBuilder::new(),
            level: 0,
        }
    }

    pub fn reset(&mut self) {
        self.sax_parser.reset();
        self.builder.take();
        self.level = 0;
    }

    pub fn elements<'a>(&'a mut self, bytes: &'a [u8]) -> StreamElements<'a> {
        StreamElements::new(self, bytes)
    }

    pub fn parse_bytes(
        &mut self,
        bytes: &[u8],
    ) -> Result<Option<(StreamElement, usize)>, ParseError> {
        let mut bytes_parsed = 0;
        while bytes_parsed < bytes.len() {
            let sax_element = match self.sax_parser.parse_bytes(&bytes[bytes_parsed..]) {
                Ok(Some((element, bytes))) => {
                    bytes_parsed += bytes;
                    element
                }
                Ok(None) => {
                    return Ok(None);
                }
                Err(err) => return Err(err),
            };
            match sax_element {
                SaxElement::StartTag(_) => {
                    self.level += 1;
                }
                SaxElement::StartTagEmpty => {
                    self.level -= 1;
                }
                SaxElement::EndTag(name) => {
                    if self.level == 0 && name == STREAM_TAG {
                        return Ok(Some((StreamElement::End, bytes_parsed)));
                    }
                    self.level -= 1;
                }
                _ => {}
            }
            self.builder.append_element(&sax_element)?;
            match sax_element {
                SaxElement::StartTagContent => {
                    if self.level == 1
                        && self
                            .builder
                            .peek()
                            .is_some_and(|doc| doc.root().name() == STREAM_TAG)
                        && let Some(doc) = self.builder.take()
                    {
                        self.level = 0;
                        return Ok(Some((StreamElement::Element(doc), bytes_parsed)));
                    }
                }
                SaxElement::EndTag(_) | SaxElement::StartTagEmpty => {
                    if self.level == 0
                        && let Some(doc) = self.builder.take()
                    {
                        return Ok(Some((StreamElement::Element(doc), bytes_parsed)));
                    }
                }
                _ => {}
            }
        }
        Ok(None)
    }
}

impl Default for StreamParser {
    fn default() -> Self {
        Self::new()
    }
}
