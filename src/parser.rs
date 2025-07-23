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

#[derive(Debug, Eq, PartialEq)]
pub enum SaxElement<'a> {
    StartTag(&'a str),
    Attribute(&'a str, &'a str),
    EmptyElementTag,
    EndTag(&'a str),
    CData(&'a str),
    Comment(&'a str),
    PI(&'a str),
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
    nr_bytes: usize,
    nr_lines: usize,
    nr_column: usize,
}

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
    TagName,
    EndTagWhitespace,
    EmptyTagEnd,
    AttributeWhitespace,
    AttributeName,
    AttributeValueStart,
    AttributeValue,
    AttributeEq,
    CData,
    Epilog,
}

const INITIAL_BUFFER_CAPACITY: usize = 128;

macro_rules! whitespace {
    () => {
        b' ' | b'\t' | b'\r' | b'\n'
    };
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
            nr_bytes: 0,
            nr_lines: 0,
            nr_column: 0,
        }
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
                    whitespace!() => return Err(ParserError::BadXml),
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
                    // FIXME: doctype
                    _ => return Err(ParserError::BadXml),
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
                    b'&' => (),
                    _ => (),
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
                parser.parse_bytes(self, &s.as_bytes()),
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
        //        BadTester::new(2).check("<a a='1' b='></a>");
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
    }
}

// FIXME: parse entitites in cdata and attrib values

// FIXME: consolidate tag end code

// FIXME: parse doctype
// FIXME: check utf8

// FIXME: returned error details
