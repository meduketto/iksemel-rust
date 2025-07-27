/*
** This file is a part of Iksemel (XML parser for Jabber/XMPP)
** Copyright (C) 2000-2025 Gurer Ozen
**
** Iksemel is free software: you can redistribute it and/or modify it
** under the terms of the GNU Lesser General Public License as
** published by the Free Software Foundation, either version 3 of
** the License, or (at your option) any later version.
*/

#[derive(Debug, Eq, PartialEq)]
pub enum ParserError {
    NoMemory,
    BadXml,
    HandlerError,
}

impl std::fmt::Display for ParserError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParserError::NoMemory => write!(f, "not enough memory"),
            ParserError::BadXml => write!(f, "invalid xml syntax"),
            ParserError::HandlerError => write!(f, "error from sax handler"),
        }
    }
}

impl std::error::Error for ParserError {}

#[derive(Debug, Eq, PartialEq)]
pub enum SaxElement<'a> {
    StartTag(&'a str),
    Attribute(&'a str, &'a str),
    EmptyElementTag,
    EndTag(&'a str),
    CData(&'a str),
}

pub trait SaxHandler {
    fn handle_element(&mut self, element: &SaxElement) -> Result<(), ParserError>;
}

pub struct Parser {
    state: State,
    depth: usize,
    is_end_tag: bool,
    is_quot_value: bool,
    seen_content: bool,
    value_pos: usize,
    buffer: Vec<u8>,
    ref_buffer: Vec<u8>,
    char_ref_value: u32,
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

impl Parser {
    pub fn new() -> Parser {
        Parser {
            state: State::Prolog,
            depth: 0,
            is_end_tag: false,
            is_quot_value: false,
            seen_content: false,
            value_pos: 0,
            buffer: Vec::<u8>::with_capacity(INITIAL_BUFFER_CAPACITY),
            ref_buffer: Vec::<u8>::with_capacity(REF_BUFFER_SIZE),
            char_ref_value: 0,
            nr_bytes: 0,
            nr_lines: 0,
            nr_column: 0,
        }
    }

    fn send_u32_cdata(
        &mut self,
        handler: &mut impl SaxHandler,
        value: u32,
    ) -> Result<(), ParserError> {
        if !is_valid_xml_char(value) {
            return Err(ParserError::BadXml);
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

        let s = unsafe { std::str::from_utf8_unchecked(&buf[0..size]) };
        handler.handle_element(&SaxElement::CData(&s))
    }

    pub fn parse_finish(&self) -> Result<(), ParserError> {
        if !self.seen_content {
            return Err(ParserError::BadXml);
        }
        if self.depth > 0 {
            return Err(ParserError::BadXml);
        }
        if self.state != State::Epilog {
            return Err(ParserError::BadXml);
        }
        Ok(())
    }

    pub fn parse_bytes_finish(
        &mut self,
        handler: &mut impl SaxHandler,
        bytes: &[u8],
    ) -> Result<(), ParserError> {
        self.parse_bytes(handler, bytes)?;
        self.parse_finish()
    }

    pub fn parse_bytes(
        &mut self,
        handler: &mut impl SaxHandler,
        bytes: &[u8],
    ) -> Result<(), ParserError> {
        let mut pos: usize = 0;
        let mut back: usize = 0;

        while pos < bytes.len() {
            let mut redo: bool = false;
            let c = bytes[pos];

            match self.state {
                State::Prolog => match c {
                    b'<' => self.state = State::TagStart,
                    whitespace!() => (),
                    _ => return Err(ParserError::BadXml),
                },

                State::TagStart => match c {
                    b'!' => {
                        self.state = State::Markup;
                    }
                    b'?' => self.state = State::PI,
                    b'/' => {
                        if self.depth == 0 {
                            return Err(ParserError::BadXml);
                        }
                        back = pos + 1;
                        self.is_end_tag = true;
                        self.state = State::TagName;
                    }
                    whitespace!() | b'>' => return Err(ParserError::BadXml),
                    _ => {
                        if self.depth == 0 && self.seen_content {
                            return Err(ParserError::BadXml);
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
                            return Err(ParserError::BadXml);
                        }
                        self.state = State::CDataSectionC;
                    }
                    b'D' => self.state = State::DoctypeDO,
                    _ => return Err(ParserError::BadXml),
                },

                State::DoctypeDO => match c {
                    b'O' => self.state = State::DoctypeDOC,
                    _ => return Err(ParserError::BadXml),
                },

                State::DoctypeDOC => match c {
                    b'C' => self.state = State::DoctypeDOCT,
                    _ => return Err(ParserError::BadXml),
                },

                State::DoctypeDOCT => match c {
                    b'T' => self.state = State::DoctypeDOCTY,
                    _ => return Err(ParserError::BadXml),
                },

                State::DoctypeDOCTY => match c {
                    b'Y' => self.state = State::DoctypeDOCTYP,
                    _ => return Err(ParserError::BadXml),
                },

                State::DoctypeDOCTYP => match c {
                    b'P' => self.state = State::DoctypeDOCTYPE,
                    _ => return Err(ParserError::BadXml),
                },

                State::DoctypeDOCTYPE => match c {
                    b'E' => self.state = State::DoctypeWhitespace,
                    _ => return Err(ParserError::BadXml),
                },

                State::DoctypeWhitespace => match c {
                    whitespace!() => self.state = State::DoctypeSkip,
                    _ => return Err(ParserError::BadXml),
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
                        return Err(ParserError::BadXml);
                    }
                    self.state = State::CDataSectionCD;
                }

                State::CDataSectionCD => {
                    if c != b'D' {
                        return Err(ParserError::BadXml);
                    }
                    self.state = State::CDataSectionCDA;
                }

                State::CDataSectionCDA => {
                    if c != b'A' {
                        return Err(ParserError::BadXml);
                    }
                    self.state = State::CDataSectionCDAT;
                }

                State::CDataSectionCDAT => {
                    if c != b'T' {
                        return Err(ParserError::BadXml);
                    }
                    self.state = State::CDataSectionCDATA;
                }

                State::CDataSectionCDATA => {
                    if c != b'A' {
                        return Err(ParserError::BadXml);
                    }
                    self.state = State::CDataSectionCDATAb;
                }

                State::CDataSectionCDATAb => {
                    if c != b'[' {
                        return Err(ParserError::BadXml);
                    }
                    back = pos + 1;
                    self.state = State::CDataSectionBody;
                }

                State::CDataSectionBody => match c {
                    b']' => {
                        if back < pos {
                            let s = unsafe { std::str::from_utf8_unchecked(&bytes[back..pos]) };
                            handler.handle_element(&SaxElement::CData(&s))?;
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
                        return Err(ParserError::BadXml);
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
                        return Err(ParserError::BadXml);
                    }
                    if self.depth > 0 {
                        back = pos + 1;
                        self.state = State::CData;
                    } else {
                        if self.seen_content {
                            self.state = State::Epilog;
                        } else {
                            self.state = State::Prolog;
                        }
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
                    _ => return Err(ParserError::BadXml),
                },

                State::TagName => match c {
                    b'/' | b'>' | whitespace!() => {
                        if back < pos {
                            self.buffer.extend_from_slice(&bytes[back..pos]);
                        }
                        {
                            if self.buffer.len() == 0 {
                                return Err(ParserError::BadXml);
                            }
                            let s = unsafe { std::str::from_utf8_unchecked(&self.buffer) };
                            if self.is_end_tag {
                                if c == b'/' {
                                    return Err(ParserError::BadXml);
                                }
                                handler.handle_element(&SaxElement::EndTag(&s))?;
                            } else {
                                handler.handle_element(&SaxElement::StartTag(&s))?;
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
                                        return Err(ParserError::BadXml);
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
                            return Err(ParserError::BadXml);
                        }
                        self.depth -= 1;
                        if self.depth == 0 {
                            self.state = State::Epilog;
                        } else {
                            back = pos + 1;
                            self.state = State::CData;
                        }
                    }
                    _ => return Err(ParserError::BadXml),
                },

                State::EndTagWhitespace => match c {
                    b'>' => {
                        if self.depth == 0 {
                            return Err(ParserError::BadXml);
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
                    _ => return Err(ParserError::BadXml),
                },

                State::AttributeWhitespace => match c {
                    whitespace!() => (),
                    b'/' => {
                        if self.is_end_tag {
                            return Err(ParserError::BadXml);
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
                    b'/' | b'>' | b'<' => return Err(ParserError::BadXml),
                    _ => (),
                },

                State::AttributeEq => match c {
                    b'=' => self.state = State::AttributeValueStart,
                    whitespace!() => (),
                    _ => return Err(ParserError::BadXml),
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
                    _ => return Err(ParserError::BadXml),
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
                        handler.handle_element(&SaxElement::Attribute(&attr, &value))?;
                        self.buffer.clear();
                        self.state = State::AttributeWhitespace;
                    } else if c == b'<' {
                        return Err(ParserError::BadXml);
                    }
                }

                State::CData => match c {
                    b'<' => {
                        if back < pos {
                            let s = unsafe { std::str::from_utf8_unchecked(&bytes[back..pos]) };
                            handler.handle_element(&SaxElement::CData(&s))?;
                        }
                        back = pos + 1;
                        self.state = State::TagStart;
                    }
                    b'&' => {
                        if back < pos {
                            let s = unsafe { std::str::from_utf8_unchecked(&bytes[back..pos]) };
                            handler.handle_element(&SaxElement::CData(&s))?;
                        }
                        self.ref_buffer.clear();
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
                        match self.ref_buffer.as_slice() {
                            b"amp" => handler.handle_element(&SaxElement::CData("&"))?,
                            b"lt" => handler.handle_element(&SaxElement::CData("<"))?,
                            b"gt" => handler.handle_element(&SaxElement::CData(">"))?,
                            b"quot" => handler.handle_element(&SaxElement::CData("\""))?,
                            b"apos" => handler.handle_element(&SaxElement::CData("'"))?,
                            _ => return Err(ParserError::BadXml),
                        };
                        back = pos + 1;
                        self.state = State::CData;
                    }
                    _ => {
                        if self.ref_buffer.len() >= REF_BUFFER_SIZE {
                            return Err(ParserError::BadXml);
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
                        self.state = State::CData;
                    }
                    b'0'..=b'9' => {
                        let digit: u32 = (c - b'0').into();
                        self.char_ref_value = (self.char_ref_value * 10) + digit;
                    }
                    _ => return Err(ParserError::BadXml),
                },

                State::HexCharReference => match c {
                    b';' => {
                        self.send_u32_cdata(handler, self.char_ref_value)?;
                        back = pos + 1;
                        self.state = State::CData;
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
                    _ => return Err(ParserError::BadXml),
                },

                State::Epilog => match c {
                    b'<' => self.state = State::TagStart,
                    whitespace!() => (),
                    _ => return Err(ParserError::BadXml),
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
                    handler.handle_element(&SaxElement::CData(&s))?;
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
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Tester<'a> {
        expected: &'a [SaxElement<'a>],
        current: usize,
        cdata_buf: String,
    }

    impl<'a> Tester<'a> {
        fn new(expected: &'a [SaxElement]) -> Tester<'a> {
            Tester {
                expected,
                current: 0,
                cdata_buf: String::new(),
            }
        }

        fn check(&mut self, s: &str) {
            let mut parser = Parser::new();
            assert!(parser.parse_bytes(self, &s.as_bytes()).is_ok());
            assert_eq!(self.current, self.expected.len());
        }
    }

    impl<'a> SaxHandler for Tester<'a> {
        fn handle_element(&mut self, element: &SaxElement) -> Result<(), ParserError> {
            assert!(self.current < self.expected.len());
            if let SaxElement::CData(cdata) = element {
                if let SaxElement::CData(cdata2) = self.expected[self.current] {
                    self.cdata_buf.push_str(cdata);
                    if self.cdata_buf.len() >= cdata2.len() {
                        assert_eq!(self.cdata_buf, cdata2);
                        self.current += 1;
                        self.cdata_buf.clear();
                    }
                } else {
                    assert_eq!(element, &self.expected[self.current]);
                }
            } else {
                assert_eq!(element, &self.expected[self.current]);
                self.current += 1;
            }
            Ok(())
        }
    }

    struct BadTester {
        bad_byte: usize,
    }

    impl BadTester {
        fn new(bad_byte: usize) -> BadTester {
            BadTester { bad_byte }
        }

        fn check(&mut self, s: &str) {
            let mut parser = Parser::new();
            assert_eq!(
                parser.parse_bytes_finish(self, &s.as_bytes()),
                Err(ParserError::BadXml)
            );
            assert_eq!(parser.nr_bytes(), self.bad_byte);
        }
    }

    impl SaxHandler for BadTester {
        fn handle_element(&mut self, _element: &SaxElement) -> Result<(), ParserError> {
            Ok(())
        }
    }

    #[test]
    fn tags() {
        Tester::new(&[SaxElement::StartTag("lonely"), SaxElement::EmptyElementTag])
            .check("<lonely/>");

        Tester::new(&[SaxElement::StartTag("lonely"), SaxElement::EmptyElementTag])
            .check("   <lonely/>    ");

        Tester::new(&[
            SaxElement::StartTag("parent"),
            SaxElement::StartTag("child"),
            SaxElement::EmptyElementTag,
            SaxElement::StartTag("child"),
            SaxElement::EmptyElementTag,
            SaxElement::CData("child"),
            SaxElement::EndTag("parent"),
        ])
        .check("<?xml version='1.0'?><parent><child/><child/>child</parent>");

        Tester::new(&[
            SaxElement::StartTag("parent"),
            SaxElement::StartTag("empty"),
            SaxElement::EmptyElementTag,
            SaxElement::StartTag("b"),
            SaxElement::CData("lala"),
            SaxElement::EndTag("b"),
            SaxElement::EndTag("parent"),
        ])
        .check("<parent  ><empty \t /><b>lala</b \n></parent>");

        Tester::new(&[
            SaxElement::StartTag("mytag"),
            SaxElement::Attribute("abc", "123"),
            SaxElement::Attribute("id", "XC72"),
            SaxElement::EndTag("mytag"),
        ])
        .check("<mytag abc='123' id=\"XC72\"></mytag>");

        Tester::new(&[
            SaxElement::StartTag("a"),
            SaxElement::StartTag("b"),
            SaxElement::Attribute("x1", "lala"),
            SaxElement::EmptyElementTag,
            SaxElement::StartTag("c"),
            SaxElement::Attribute("x2", "bibi"),
            SaxElement::EmptyElementTag,
            SaxElement::EndTag("a"),
        ])
        .check("<a><b x1 ='lala'/><c x2\t= \t'bibi'/></a>");

        Tester::new(&[
            SaxElement::StartTag("tag"),
            SaxElement::Attribute("a", "1"),
            SaxElement::Attribute("b", "2"),
            SaxElement::Attribute("c", "3"),
            SaxElement::Attribute("d", "4"),
            SaxElement::Attribute("e", "5"),
            SaxElement::Attribute("f", "6"),
            SaxElement::Attribute("g", "7"),
            SaxElement::Attribute("id", "xyz9"),
            SaxElement::StartTag("sub"),
            SaxElement::EndTag("sub"),
            SaxElement::EndTag("tag"),
        ])
        .check(
            "<tag a  =  '1' b  ='2' c=  '3' d='4'   e='5' f='6' g='7' id='xyz9'><sub></sub></tag>",
        );

        Tester::new(&[
            SaxElement::StartTag("tag"),
            SaxElement::Attribute("a", "12\"34"),
            SaxElement::Attribute("b", "123'456"),
            SaxElement::EmptyElementTag,
        ])
        .check("<tag a='12\"34' b=\"123'456\" />");
    }

    #[test]
    fn comments() {
        Tester::new(&[
            SaxElement::StartTag("item"),
            SaxElement::Attribute("url", "http://jabber.org"),
            SaxElement::CData("Jabber Site"),
            SaxElement::EndTag("item"),
        ])
        .check("<item url='http://jabber.org'><!-- little comment -->Jabber Site</item>");

        Tester::new(&[
            SaxElement::StartTag("index"),
            SaxElement::StartTag("item"),
            SaxElement::Attribute("name", "lala"),
            SaxElement::Attribute("page", "42"),
            SaxElement::EmptyElementTag,
            SaxElement::EndTag("index"),
        ])
        .check("<index><!-- <item> - tag has no childs --><item name='lala' page='42'/></index>");

        Tester::new(&[SaxElement::StartTag("empty"), SaxElement::EmptyElementTag])
            .check("<!-- comment --> <empty/> <!-- lala -->");
    }

    #[test]
    fn cdatas() {
        Tester::new(&[
            SaxElement::StartTag("ka"),
            SaxElement::CData("1234 <ka> lala ] ]] ]]] 4321"),
            SaxElement::EndTag("ka"),
        ])
        .check("<ka>1234<![CDATA[ <ka> lala ] ]] ]]] ]]>4321</ka>");

        Tester::new(&[
            SaxElement::StartTag("data"),
            SaxElement::CData("[TEST]"),
            SaxElement::EndTag("data"),
        ])
        .check("<data><![CDATA[[TEST]]]></data>");

        Tester::new(&[
            SaxElement::StartTag("data"),
            SaxElement::CData("[TEST]]"),
            SaxElement::EndTag("data"),
        ])
        .check("<data><![CDATA[[TEST]]]]></data>");
    }

    #[test]
    fn dtds() {
        Tester::new(&[
            SaxElement::StartTag("x"),
            SaxElement::CData("foo"),
            SaxElement::EndTag("x"),
        ])
        .check(" <!DOCTYPE greeting [ <!ELEMENT greeting (#PCDATA)> ]> <x>foo</x>");
    }

    #[test]
    fn entities() {
        Tester::new(&[
            SaxElement::StartTag("body"),
            SaxElement::CData("I'm fixing parser&tester for \"<\" and \">\" chars."),
            SaxElement::EndTag("body"),
        ])
        .check("<body>I&apos;m fixing parser&amp;tester for &quot;&lt;&quot; and &quot;&gt;&quot; chars.</body>");

        Tester::new(&[
            SaxElement::StartTag("test"),
            SaxElement::StartTag("standalone"),
            SaxElement::Attribute("be", "happy"),
            SaxElement::EmptyElementTag,
            SaxElement::CData("abcd"),
            SaxElement::StartTag("br"),
            SaxElement::EmptyElementTag,
            SaxElement::CData("<escape>"),
            SaxElement::EndTag("test"),
        ])
        .check("<test><standalone be='happy'/>abcd<br/>&lt;escape&gt;</test>");

        Tester::new(&[
            SaxElement::StartTag("a"),
            SaxElement::CData(";AB;"),
            SaxElement::EndTag("a"),
        ])
        .check("<a>&#x3B;&#65;&#x42;&#x3b;</a>");

        Tester::new(&[
            SaxElement::StartTag("a"),
            SaxElement::CData(" \u{90} \u{900} \u{10abc} "),
            SaxElement::EndTag("a"),
        ])
        .check("<a> &#x90; &#x900; &#x10abc; </a>");
    }

    #[test]
    fn bad_tags() {
        BadTester::new(4).check("<a>< b/></a>");
        BadTester::new(6).check("<a><b/ ></a>");
        BadTester::new(8).check("<a></ccc/></a>");
        BadTester::new(13).check("<a><b/><c></c/></a>");
        BadTester::new(1).check("</a>");
        BadTester::new(9).check("<a> </a  b>");
        BadTester::new(8).check("<a></a><b/>");
        BadTester::new(2).check("  lala <a></a>");
        BadTester::new(10).check("  <a></a> lala");
        BadTester::new(10).check("<a a='1' b></a>");
        BadTester::new(11).check("<a a='1' b=></a>");
        BadTester::new(12).check("<a a='12' b '2'></a>");
        BadTester::new(13).check("<a a='123' b c='5'></a>");
        BadTester::new(14).check("<a a='12'></a b='1'>");
        BadTester::new(17).check("<g><test a='123'/ b='lala'></g>");
        BadTester::new(13).check("<a a='1' b='></a>");
        BadTester::new(13).check("<a a='1' b=\"></a>");
        BadTester::new(5).check("<a> <> </a>");
        BadTester::new(6).check("<a> </> </a>");
    }

    #[test]
    fn bad_comments() {
        BadTester::new(10).check("<e><!-- -- --></e>");
        BadTester::new(22).check("<ha><!-- <lala> --><!- comment -></ha>");
        BadTester::new(12).check("<!-- c1 --> lala <ha/>");
        BadTester::new(31).check("<!-- c1 --> <ha/> <!-- pika -->c");
        BadTester::new(9).check("<!-- c ---> <ha/>");
    }

    #[test]
    fn bad_cdatas() {
        BadTester::new(2).check("<![CDATA[lala]> <a/>");
        BadTester::new(8).check(" <a/> <![CDATA[lala]>");
        BadTester::new(7).check("<a> <![DATA[lala]> </a>");
        BadTester::new(9).check("<a> <![CDaTA[lala]> </a>");
        BadTester::new(12).check("<a> <![CDATAlala]> </a>");
    }

    #[test]
    fn bad_entities() {
        BadTester::new(8).check("<a>&lala;<a/>");
        BadTester::new(12).check("<a>&lala           <a/>");
        BadTester::new(6).check("<a>&#1a;<a/>");
        BadTester::new(6).check("<a>&#Xaa;<a/>");
        BadTester::new(8).check("<a>&#xa5g;<a/>");
        BadTester::new(6).check("<a>&#8;<a/>");
        BadTester::new(7).check("<a>&#11;<a/>");
        BadTester::new(7).check("<a>&#15;<a/>");
        BadTester::new(10).check("<a>&#xD800;<a/>");
        BadTester::new(10).check("<a>&#xDfFf;<a/>");
        BadTester::new(10).check("<a>&#xfFfE;<a/>");
        BadTester::new(10).check("<a>&#xFFff;<a/>");
        BadTester::new(12).check("<a>&#x110000;<a/>");
    }

    #[test]
    fn bad_unfinished() {
        BadTester::new(5).check(" <a> ");
        BadTester::new(20).check("  <!-- lala -->     ");
        BadTester::new(27).check(" <a></a> <!-- open comment ");
        BadTester::new(23).check(" <a></a> <?app open pi ");
    }
}

// FIXME: parse references in attrib values

// FIXME: consolidate tag end code
// FIXME: consolidate [CDATA[ states
// FIXME: check utf8
// FIXME: check char ranges

// FIXME: returned error details
// not supported error? for entity refs
