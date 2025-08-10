/*
** This file is a part of Iksemel (XML parser for Jabber/XMPP)
** Copyright (C) 2000-2025 Gurer Ozen
**
** Iksemel is free software: you can redistribute it and/or modify it
** under the terms of the GNU Lesser General Public License as
** published by the Free Software Foundation, either version 3 of
** the License, or (at your option) any later version.
*/

mod error;

pub use error::SaxError;

use error::XmlError;

#[derive(Debug, Eq, PartialEq)]
pub enum SaxElement<'a> {
    StartTag(&'a str),
    Attribute(&'a str, &'a str),
    EmptyElementTag,
    EndTag(&'a str),
    CData(&'a str),
}

pub trait SaxHandler {
    fn handle_element(&mut self, element: &SaxElement) -> Result<(), SaxError>;
}

pub struct SaxParser {
    state: State,
    error: Option<XmlError>,
    uni_len: u32,
    uni_left: u32,
    uni_char: u32,
    depth: usize,
    is_end_tag: bool,
    is_quot_value: bool,
    seen_content: bool,
    value_pos: usize,
    buffer: Vec<u8>,
    ref_buffer: Vec<u8>,
    char_ref_value: u32,
    is_value_ref: bool,
    nr_bytes: usize,
    nr_lines: usize,
    nr_column: usize,
}

#[derive(Eq, PartialEq)]
enum State {
    Prolog,
    TagStart,
    PI,
    PIEnd,
    Markup,
    CDataSectionC,
    CDataSectionCD,
    CDataSectionCDA,
    CDataSectionCDAT,
    CDataSectionCDATA,
    CDataSectionCDATAb,
    CDataSectionBody,
    CDataSectionMaybeEnd,
    CDataSectionMaybeEnd2,
    CommentStart,
    CommentBody,
    CommentMaybeEnd,
    CommentEnd,
    DoctypeDO,
    DoctypeDOC,
    DoctypeDOCT,
    DoctypeDOCTY,
    DoctypeDOCTYP,
    DoctypeDOCTYPE,
    DoctypeWhitespace,
    DoctypeSkip,
    DoctypeMarkupDecl,
    TagName,
    EndTagWhitespace,
    EmptyTagEnd,
    AttributeWhitespace,
    AttributeName,
    AttributeValueStart,
    AttributeValue,
    AttributeEq,
    CData,
    Reference,
    CharReference,
    CharReferenceBody,
    HexCharReference,
    Entity,
    Epilog,
}

const INITIAL_BUFFER_CAPACITY: usize = 128;

const REF_BUFFER_SIZE: usize = 8;

macro_rules! whitespace {
    () => {
        b' ' | b'\t' | b'\r' | b'\n'
    };
}

fn is_valid_xml_char(c: u32) -> bool {
    match c {
        0x09 | 0x0a | 0x0d | 0x20..0xd7ff | 0xe000..0xfffd | 0x10000..0x10ffff => true,
        _ => false,
    }
}

macro_rules! xml_error {
    ($a:ident, $b:ident) => {
        $a.error = Some(XmlError::$b);
        return Err(SaxError::BadXml);
    };
}

macro_rules! notsupp_error {
    ($a:ident, $b:ident) => {
        $a.error = Some(XmlError::$b);
        return Err(SaxError::NotSupported);
    };
}

impl SaxParser {
    pub fn new() -> SaxParser {
        SaxParser {
            state: State::Prolog,
            error: None,
            uni_len: 0,
            uni_left: 0,
            uni_char: 0,
            depth: 0,
            is_end_tag: false,
            is_quot_value: false,
            seen_content: false,
            value_pos: 0,
            buffer: Vec::<u8>::with_capacity(INITIAL_BUFFER_CAPACITY),
            ref_buffer: Vec::<u8>::with_capacity(REF_BUFFER_SIZE),
            char_ref_value: 0,
            is_value_ref: false,
            nr_bytes: 0,
            nr_lines: 0,
            nr_column: 0,
        }
    }

    fn send_u32_cdata(
        &mut self,
        handler: &mut impl SaxHandler,
        value: u32,
    ) -> Result<(), SaxError> {
        if !is_valid_xml_char(value) {
            xml_error!(self, CharInvalid);
        }

        let mut buf: [u8; 4] = [0; 4];
        let mut size = 1;
        const DATA_MASK: u32 = 0b00111111;
        const DATA_PREFIX: u8 = 0b10000000;
        match value {
            0..=0x7f => buf[0] = value as u8,
            0x80..=0x7ff => {
                buf[0] = 0b11000000 | ((value >> 6) as u8);
                buf[1] = DATA_PREFIX | ((value & DATA_MASK) as u8);
                size = 2;
            }
            0x800..=0xffff => {
                buf[0] = 0b11100000 | ((value >> 12) as u8);
                buf[1] = DATA_PREFIX | (((value >> 6) & DATA_MASK) as u8);
                buf[2] = DATA_PREFIX | ((value & DATA_MASK) as u8);
                size = 3;
            }
            0x10000..=0x10ffff => {
                buf[0] = 0b11110000 | ((value >> 18) as u8);
                buf[1] = DATA_PREFIX | (((value >> 12) & DATA_MASK) as u8);
                buf[2] = DATA_PREFIX | (((value >> 6) & DATA_MASK) as u8);
                buf[3] = DATA_PREFIX | ((value & DATA_MASK) as u8);
                size = 4;
            }
            _ => (),
        }

        if self.is_value_ref {
            self.buffer.extend(&buf[0..size]);
            Ok(())
        } else {
            let s = unsafe { std::str::from_utf8_unchecked(&buf[0..size]) };
            handler.handle_element(&SaxElement::CData(s))
        }
    }

    pub fn parse_finish(&mut self) -> Result<(), SaxError> {
        if self.error.is_some() {
            notsupp_error!(self, ParserReuseWithoutReset);
        }
        if !self.seen_content {
            xml_error!(self, DocNoContent);
        }
        if self.depth > 0 {
            xml_error!(self, DocOpenTags);
        }
        if self.state != State::Epilog {
            xml_error!(self, DocOpenMarkup);
        }
        Ok(())
    }

    pub fn parse_bytes_finish(
        &mut self,
        handler: &mut impl SaxHandler,
        bytes: &[u8],
    ) -> Result<(), SaxError> {
        self.parse_bytes(handler, bytes)?;
        self.parse_finish()
    }

    pub fn parse_bytes(
        &mut self,
        handler: &mut impl SaxHandler,
        bytes: &[u8],
    ) -> Result<(), SaxError> {
        if self.error.is_some() {
            notsupp_error!(self, ParserReuseWithoutReset);
        }

        let mut pos: usize = 0;
        let mut back: usize = 0;

        while pos < bytes.len() {
            let mut redo: bool = false;
            let c = bytes[pos];

            if self.uni_left > 0 {
                if c & 0xc0 != 0x80 {
                    xml_error!(self, Utf8InvalidContByte);
                }
                self.uni_char <<= 6;
                self.uni_char += c as u32 & 0x3f;
                self.uni_left -= 1;
                if self.uni_left == 0 {
                    // Sequences longer than the actual character codepoint
                    // size are security hazards.
                    if (self.uni_len == 2 && self.uni_char <= 0x7f)
                        || (self.uni_len == 3 && self.uni_char <= 0x7ff)
                        || (self.uni_len == 4 && self.uni_char <= 0xffff)
                    {
                        xml_error!(self, Utf8OverlongSequence);
                    }
                    if !is_valid_xml_char(self.uni_char) {
                        xml_error!(self, CharInvalid);
                    }
                }
            } else if c & 0x80 == 0x80 {
                if c & 0x60 == 0x40 {
                    self.uni_len = 2;
                    self.uni_left = 1;
                    self.uni_char = c as u32 & 0x1f;
                } else if c & 0x70 == 0x60 {
                    self.uni_len = 3;
                    self.uni_left = 2;
                    self.uni_char = c as u32 & 0x0f;
                } else if c & 0x78 == 0x70 {
                    self.uni_len = 4;
                    self.uni_left = 3;
                    self.uni_char = c as u32 & 0x07;
                } else {
                    xml_error!(self, Utf8InvalidPrefixByte);
                }
            } else if c < 0x20 && (c != 0x09 && c != 0x0a && c != 0x0d) {
                xml_error!(self, CharInvalid);
            }

            match self.state {
                State::Prolog => match c {
                    b'<' => self.state = State::TagStart,
                    whitespace!() => (),
                    _ => {
                        xml_error!(self, DocCdataWithoutParent);
                    }
                },

                State::TagStart => match c {
                    b'!' => {
                        self.state = State::Markup;
                    }
                    b'?' => self.state = State::PI,
                    b'/' => {
                        if self.depth == 0 {
                            xml_error!(self, TagCloseWithoutOpen);
                        }
                        back = pos + 1;
                        self.is_end_tag = true;
                        self.state = State::TagName;
                    }
                    whitespace!() | b'>' => {
                        xml_error!(self, TagWhitespaceStart);
                    }
                    _ => {
                        if self.depth == 0 && self.seen_content {
                            xml_error!(self, TagOutsideRoot);
                        }
                        self.depth += 1;
                        back = pos;
                        self.is_end_tag = false;
                        self.seen_content = true;
                        self.state = State::TagName;
                    }
                },

                State::Markup => match c {
                    b'-' => self.state = State::CommentStart,
                    b'[' => {
                        if self.depth == 0 {
                            xml_error!(self, MarkupCdataSectionOutsideRoot);
                        }
                        self.state = State::CDataSectionC;
                    }
                    b'D' => self.state = State::DoctypeDO,
                    _ => {
                        xml_error!(self, MarkupUnrecognized);
                    }
                },

                State::DoctypeDO => match c {
                    b'O' => self.state = State::DoctypeDOC,
                    _ => {
                        xml_error!(self, MarkupDoctypeBadStart);
                    }
                },

                State::DoctypeDOC => match c {
                    b'C' => self.state = State::DoctypeDOCT,
                    _ => {
                        xml_error!(self, MarkupDoctypeBadStart);
                    }
                },

                State::DoctypeDOCT => match c {
                    b'T' => self.state = State::DoctypeDOCTY,
                    _ => {
                        xml_error!(self, MarkupDoctypeBadStart);
                    }
                },

                State::DoctypeDOCTY => match c {
                    b'Y' => self.state = State::DoctypeDOCTYP,
                    _ => {
                        xml_error!(self, MarkupDoctypeBadStart);
                    }
                },

                State::DoctypeDOCTYP => match c {
                    b'P' => self.state = State::DoctypeDOCTYPE,
                    _ => {
                        xml_error!(self, MarkupDoctypeBadStart);
                    }
                },

                State::DoctypeDOCTYPE => match c {
                    b'E' => self.state = State::DoctypeWhitespace,
                    _ => {
                        xml_error!(self, MarkupDoctypeBadStart);
                    }
                },

                State::DoctypeWhitespace => match c {
                    whitespace!() => self.state = State::DoctypeSkip,
                    _ => {
                        xml_error!(self, MarkupDoctypeBadStart);
                    }
                },

                State::DoctypeSkip => match c {
                    b'<' => self.state = State::DoctypeMarkupDecl,
                    b'>' => self.state = State::Prolog,
                    _ => (),
                },

                State::DoctypeMarkupDecl => match c {
                    b'>' => self.state = State::DoctypeSkip,
                    _ => (),
                },

                State::CDataSectionC => {
                    if c != b'C' {
                        xml_error!(self, MarkupCdataSectionBadStart);
                    }
                    self.state = State::CDataSectionCD;
                }

                State::CDataSectionCD => {
                    if c != b'D' {
                        xml_error!(self, MarkupCdataSectionBadStart);
                    }
                    self.state = State::CDataSectionCDA;
                }

                State::CDataSectionCDA => {
                    if c != b'A' {
                        xml_error!(self, MarkupCdataSectionBadStart);
                    }
                    self.state = State::CDataSectionCDAT;
                }

                State::CDataSectionCDAT => {
                    if c != b'T' {
                        xml_error!(self, MarkupCdataSectionBadStart);
                    }
                    self.state = State::CDataSectionCDATA;
                }

                State::CDataSectionCDATA => {
                    if c != b'A' {
                        xml_error!(self, MarkupCdataSectionBadStart);
                    }
                    self.state = State::CDataSectionCDATAb;
                }

                State::CDataSectionCDATAb => {
                    if c != b'[' {
                        xml_error!(self, MarkupCdataSectionBadStart);
                    }
                    back = pos + 1;
                    self.state = State::CDataSectionBody;
                }

                State::CDataSectionBody => match c {
                    b']' => {
                        if back < pos {
                            let s = unsafe { std::str::from_utf8_unchecked(&bytes[back..pos]) };
                            handler.handle_element(&SaxElement::CData(s))?;
                        }
                        self.state = State::CDataSectionMaybeEnd;
                    }
                    _ => (),
                },

                State::CDataSectionMaybeEnd => match c {
                    b']' => self.state = State::CDataSectionMaybeEnd2,
                    _ => {
                        handler.handle_element(&SaxElement::CData("]"))?;
                        back = pos;
                        self.state = State::CDataSectionBody;
                    }
                },

                State::CDataSectionMaybeEnd2 => match c {
                    b'>' => {
                        back = pos + 1;
                        self.state = State::CData;
                    }
                    b']' => {
                        handler.handle_element(&SaxElement::CData("]"))?;
                    }
                    _ => {
                        handler.handle_element(&SaxElement::CData("]]"))?;
                        back = pos;
                        self.state = State::CDataSectionBody;
                    }
                },

                State::CommentStart => {
                    if c != b'-' {
                        xml_error!(self, CommentMissingDash);
                    }
                    self.state = State::CommentBody;
                }

                State::CommentBody => match c {
                    b'-' => self.state = State::CommentMaybeEnd,
                    _ => (),
                },

                State::CommentMaybeEnd => match c {
                    b'-' => self.state = State::CommentEnd,
                    _ => self.state = State::CommentBody,
                },

                State::CommentEnd => {
                    if c != b'>' {
                        xml_error!(self, CommentMissingEnd);
                    }
                    if self.depth > 0 {
                        back = pos + 1;
                        self.state = State::CData;
                    } else if self.seen_content {
                        self.state = State::Epilog;
                    } else {
                        self.state = State::Prolog;
                    }
                }

                State::PI => match c {
                    b'?' => self.state = State::PIEnd,
                    _ => (),
                },

                State::PIEnd => match c {
                    b'>' => {
                        if self.seen_content {
                            if self.depth > 0 {
                                back = pos + 1;
                                self.state = State::CData;
                            } else {
                                self.state = State::Epilog;
                            }
                        } else {
                            self.state = State::Prolog;
                        }
                    }
                    _ => {
                        xml_error!(self, PiMissingEnd);
                    }
                },

                State::TagName => match c {
                    b'/' | b'>' | whitespace!() => {
                        if back < pos {
                            self.buffer.extend_from_slice(&bytes[back..pos]);
                        }
                        {
                            if self.buffer.is_empty() {
                                xml_error!(self, TagEmptyName);
                            }
                            let s = unsafe { std::str::from_utf8_unchecked(&self.buffer) };
                            if self.is_end_tag {
                                if c == b'/' {
                                    xml_error!(self, TagDoubleEnd);
                                }
                                handler.handle_element(&SaxElement::EndTag(s))?;
                            } else {
                                handler.handle_element(&SaxElement::StartTag(s))?;
                            }
                        }
                        self.buffer.clear();
                        match c {
                            b'/' => {
                                handler.handle_element(&SaxElement::EmptyElementTag)?;
                                self.state = State::EmptyTagEnd;
                            }
                            b'>' => {
                                if self.is_end_tag {
                                    if self.depth == 0 {
                                        xml_error!(self, TagCloseWithoutOpen);
                                    }
                                    self.depth -= 1;
                                    if self.depth == 0 {
                                        self.state = State::Epilog;
                                    } else {
                                        back = pos + 1;
                                        self.state = State::CData;
                                    }
                                } else {
                                    back = pos + 1;
                                    self.state = State::CData;
                                }
                            }
                            whitespace!() => {
                                if self.is_end_tag {
                                    self.state = State::EndTagWhitespace;
                                } else {
                                    self.state = State::AttributeWhitespace;
                                }
                            }
                            _ => unreachable!(),
                        }
                    }
                    _ => (),
                },

                State::EmptyTagEnd => match c {
                    b'>' => {
                        if self.depth == 0 {
                            xml_error!(self, TagCloseWithoutOpen);
                        }
                        self.depth -= 1;
                        if self.depth == 0 {
                            self.state = State::Epilog;
                        } else {
                            back = pos + 1;
                            self.state = State::CData;
                        }
                    }
                    _ => {
                        xml_error!(self, TagEmptyTagMissingEnd);
                    }
                },

                State::EndTagWhitespace => match c {
                    b'>' => {
                        if self.depth == 0 {
                            xml_error!(self, TagCloseWithoutOpen);
                        }
                        self.depth -= 1;
                        if self.depth == 0 {
                            self.state = State::Epilog;
                        } else {
                            back = pos + 1;
                            self.state = State::CData;
                        }
                    }
                    whitespace!() => (),
                    _ => {
                        xml_error!(self, TagEndTagAttributes);
                    }
                },

                State::AttributeWhitespace => match c {
                    whitespace!() => (),
                    b'/' => {
                        if self.is_end_tag {
                            xml_error!(self, TagDoubleEnd);
                        }
                        handler.handle_element(&SaxElement::EmptyElementTag)?;
                        self.state = State::EmptyTagEnd;
                    }
                    b'>' => {
                        back = pos + 1;
                        self.state = State::CData;
                    }
                    _ => {
                        back = pos;
                        self.state = State::AttributeName;
                        redo = true;
                    }
                },

                State::AttributeName => match c {
                    b'=' | whitespace!() => {
                        if back < pos {
                            self.buffer.extend_from_slice(&bytes[back..pos]);
                        }
                        if c == b'=' {
                            self.state = State::AttributeValueStart;
                        } else {
                            self.state = State::AttributeEq;
                        }
                    }
                    b'/' | b'>' | b'<' => {
                        xml_error!(self, TagAttributeBadName);
                    }
                    _ => (),
                },

                State::AttributeEq => match c {
                    b'=' => self.state = State::AttributeValueStart,
                    whitespace!() => (),
                    _ => {
                        xml_error!(self, TagAttributeWithoutEqual);
                    }
                },

                State::AttributeValueStart => match c {
                    b'"' => {
                        self.is_quot_value = false;
                        self.value_pos = self.buffer.len();
                        back = pos + 1;
                        self.state = State::AttributeValue;
                    }
                    b'\'' => {
                        self.is_quot_value = true;
                        self.value_pos = self.buffer.len();
                        back = pos + 1;
                        self.state = State::AttributeValue;
                    }
                    whitespace!() => (),
                    _ => {
                        xml_error!(self, TagAttributeWithoutQuote);
                    }
                },

                State::AttributeValue => {
                    if (self.is_quot_value && c == b'\'') || (!self.is_quot_value && c == b'"') {
                        if back < pos {
                            self.buffer.extend_from_slice(&bytes[back..pos]);
                        }
                        let attr = unsafe {
                            std::str::from_utf8_unchecked(&self.buffer[0..self.value_pos])
                        };
                        let value = unsafe {
                            std::str::from_utf8_unchecked(&self.buffer[self.value_pos..])
                        };
                        handler.handle_element(&SaxElement::Attribute(attr, value))?;
                        self.buffer.clear();
                        self.state = State::AttributeWhitespace;
                    } else if c == b'&' {
                        if back < pos {
                            self.buffer.extend_from_slice(&bytes[back..pos]);
                        }
                        self.ref_buffer.clear();
                        self.is_value_ref = true;
                        self.state = State::Reference;
                    } else if c == b'<' {
                        xml_error!(self, TagAttributeBadValue);
                    }
                }

                State::CData => match c {
                    b'<' => {
                        if back < pos {
                            let s = unsafe { std::str::from_utf8_unchecked(&bytes[back..pos]) };
                            handler.handle_element(&SaxElement::CData(s))?;
                        }
                        back = pos + 1;
                        self.state = State::TagStart;
                    }
                    b'&' => {
                        if back < pos {
                            let s = unsafe { std::str::from_utf8_unchecked(&bytes[back..pos]) };
                            handler.handle_element(&SaxElement::CData(s))?;
                        }
                        self.ref_buffer.clear();
                        self.is_value_ref = false;
                        self.state = State::Reference;
                    }
                    _ => (),
                },

                State::Reference => match c {
                    b'#' => {
                        self.char_ref_value = 0;
                        self.state = State::CharReference;
                    }
                    _ => {
                        self.ref_buffer.push(c);
                        self.state = State::Entity;
                    }
                },

                State::Entity => match c {
                    b';' => {
                        let ent = match self.ref_buffer.as_slice() {
                            b"amp" => "&",
                            b"lt" => "<",
                            b"gt" => ">",
                            b"quot" => "\"",
                            b"apos" => "'",
                            _ => {
                                notsupp_error!(self, ReferenceCustomEntity);
                            }
                        };
                        if self.is_value_ref {
                            self.buffer.push(ent.as_bytes()[0]);
                            back = pos + 1;
                            self.state = State::AttributeValue;
                        } else {
                            back = pos + 1;
                            self.state = State::CData;
                            handler.handle_element(&SaxElement::CData(ent))?;
                        }
                    }
                    _ => {
                        if self.ref_buffer.len() >= REF_BUFFER_SIZE {
                            notsupp_error!(self, ReferenceCustomEntity);
                        }
                        self.ref_buffer.push(c);
                    }
                },

                State::CharReference => match c {
                    b'x' => self.state = State::HexCharReference,
                    _ => {
                        let digit: u32 = (c - b'0').into();
                        self.char_ref_value = digit;
                        self.state = State::CharReferenceBody;
                    }
                },

                State::CharReferenceBody => match c {
                    b';' => {
                        self.send_u32_cdata(handler, self.char_ref_value)?;
                        back = pos + 1;
                        if self.is_value_ref {
                            self.state = State::AttributeValue;
                        } else {
                            self.state = State::CData;
                        }
                    }
                    b'0'..=b'9' => {
                        let digit: u32 = (c - b'0').into();
                        self.char_ref_value = (self.char_ref_value * 10) + digit;
                    }
                    _ => {
                        xml_error!(self, ReferenceInvalidDecimal);
                    }
                },

                State::HexCharReference => match c {
                    b';' => {
                        self.send_u32_cdata(handler, self.char_ref_value)?;
                        back = pos + 1;
                        if self.is_value_ref {
                            self.state = State::AttributeValue;
                        } else {
                            self.state = State::CData;
                        }
                    }
                    b'0'..=b'9' => {
                        let digit: u32 = (c - b'0').into();
                        self.char_ref_value = (self.char_ref_value * 16) + digit;
                    }
                    b'a'..=b'f' => {
                        let digit: u32 = (c - b'a').into();
                        self.char_ref_value = (self.char_ref_value * 16) + digit + 10;
                    }
                    b'A'..=b'F' => {
                        let digit: u32 = (c - b'A').into();
                        self.char_ref_value = (self.char_ref_value * 16) + digit + 10;
                    }
                    _ => {
                        xml_error!(self, ReferenceInvalidHex);
                    }
                },

                State::Epilog => match c {
                    b'<' => self.state = State::TagStart,
                    whitespace!() => (),
                    _ => {
                        xml_error!(self, DocCdataWithoutParent);
                    }
                },
            }

            if !redo {
                pos += 1;
                self.nr_bytes += 1;
                self.nr_column += 1;
                if c == b'\n' {
                    self.nr_lines += 1;
                    self.nr_column = 0;
                }
            }
        }

        if back < pos {
            match self.state {
                State::TagName | State::AttributeName | State::AttributeValue => {
                    self.buffer.extend_from_slice(&bytes[back..pos])
                }
                State::CData | State::CDataSectionBody => {
                    let s = unsafe { std::str::from_utf8_unchecked(&bytes[back..pos]) };
                    handler.handle_element(&SaxElement::CData(s))?;
                }
                _ => (),
            }
        }

        Ok(())
    }

    pub fn nr_bytes(&self) -> usize {
        self.nr_bytes
    }

    pub fn nr_lines(&self) -> usize {
        self.nr_lines
    }

    pub fn nr_column(&self) -> usize {
        self.nr_column
    }

    pub fn error_description(&self) -> Option<&'static str> {
        match &self.error {
            None => None,
            Some(e) => Some(e.description()),
        }
    }
}

impl Default for SaxParser {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests;

// FIXME: parser reset
