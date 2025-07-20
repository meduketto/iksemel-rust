/*
** This file is a part of Iksemel (XML parser for Jabber/XMPP)
** Copyright (C) 2000-2025 Gurer Ozen
**
** Iksemel is free software: you can redistribute it and/or modify it
** under the terms of the GNU Lesser General Public License as
** published by the Free Software Foundation, either version 3 of
** the License, or (at your option) any later version.
*/

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
    nr_bytes: usize,
    nr_lines: usize,
    nr_column: usize,
    buffer: Vec<u8>,
}

enum State {
    Prolog,
    PrologTag,
    PI,
    TagName,
    EmptyTagEnd,
    TagEnd,
    AttributeWhitespace,
    AttributeName,
    CData,
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
            nr_bytes: 0,
            nr_lines: 0,
            nr_column: 0,
            buffer: Vec::<u8>::with_capacity(INITIAL_BUFFER_CAPACITY),
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
                    b'<' => self.state = State::PrologTag,
                    whitespace!() => (),
                    _ => return Err(ParserError::BadXml),
                },

                State::PrologTag => match c {
                    b'!' => (),
                    b'?' => self.state = State::PI,
                    _ => {
                        back = pos;
                        self.state = State::TagName;
                    }
                },

                State::TagName => match c {
                    b'/' | b'>' | whitespace!() => {
                        if back < pos {
                            self.buffer.extend_from_slice(&bytes[back..pos]);
                        }
                        {
                            let s = unsafe { std::str::from_utf8_unchecked(&self.buffer) };
                            handler.handle_element(&SaxElement::StartTag(&s))?;
                        }
                        self.buffer.clear();
                        match c {
                            b'/' => {
                                handler.handle_element(&SaxElement::EmptyElementTag)?;
                                self.state = State::EmptyTagEnd;
                            }
                            b'>' => {
                                back = pos + 1;
                                self.state = State::CData;
                            }
                            whitespace!() => self.state = State::AttributeWhitespace,
                            _ => unreachable!(),
                        }
                    }
                    _ => (),
                },

                State::AttributeWhitespace => match c {
                    whitespace!() => (),
                    _ => {
                        self.state = State::AttributeName;
                        redo = true;
                    }
                },

                State::AttributeName => match c {
                    b'/' => {
                        handler.handle_element(&SaxElement::EmptyElementTag)?;
                        self.state = State::TagEnd;
                    }
                    b'>' => {
                        back = pos + 1;
                        self.state = State::CData;
                    }
                    whitespace!() => (),
                    _ => (),
                },

                State::EmptyTagEnd => match c {
                    b'>' => {
                        // FIXME: epilog
                        back = pos + 1;
                        self.state = State::CData;
                    },
                    whitespace!() => (),
                    _ => (),
                },

                State::TagEnd => match c {
                    b'>' => {
                        // FIXME: epilog
                        let s = unsafe { std::str::from_utf8_unchecked(&self.buffer) };
                        handler.handle_element(&SaxElement::EndTag(&s))?;
                        back = pos + 1;
                        self.state = State::CData;
                    },
                    whitespace!() => (),
                    _ => (),
                },

                State::CData => match c {
                    b'<' => {
                        if back < pos {
                            let s = unsafe { std::str::from_utf8_unchecked(&bytes[back..pos]) };
                            handler.handle_element(&SaxElement::CData(&s))?;
                        }
                        back = pos + 1;
                        self.state = State::TagName;
                    }
                    b'&' => (),
                    _ => (),
                },

                _ => (),
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
                State::TagName => self.buffer.extend_from_slice(&bytes[back..pos]),
                State::CData => {
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
    }

    impl<'a> Tester<'a> {
        fn new(expected: &'a [SaxElement]) -> Tester<'a> {
            Tester {
                expected,
                current: 0,
            }
        }

        fn compare(&mut self, s: &str) {
            let mut parser = Parser::new();
            assert!(parser.parse_bytes(self, &s.as_bytes()).is_ok());
            assert_eq!(self.current, self.expected.len());
        }
    }

    impl<'a> SaxHandler for Tester<'a> {
        fn handle_element(&mut self, element: &SaxElement) -> Result<(), ParserError> {
            println!("{:?}", element);
            assert!(self.current < self.expected.len());
            assert_eq!(element, &self.expected[self.current]);
            self.current += 1;
            Ok(())
        }
    }

    #[test]
    fn tags() {
        Tester::new(&[SaxElement::StartTag("lonely"), SaxElement::EmptyElementTag])
            .compare("<lonely/>");

        Tester::new(&[
            SaxElement::StartTag("parent"),
            SaxElement::StartTag("child"),
            SaxElement::EmptyElementTag,
            SaxElement::StartTag("child"),
            SaxElement::EmptyElementTag,
            SaxElement::CData("child"),
            SaxElement::EndTag("parent"),
        ])
        .compare("<parent><child/><child/>child</parent>");
    }
}

// FIXME: Handle CData partials

// FIXME: intake type for parse() str? io?
// FIXME: return value for parse()
//        how to set lifetime on returned str
//        error types
// FIXME: dynamic buffer for tagname, entity, attribs
