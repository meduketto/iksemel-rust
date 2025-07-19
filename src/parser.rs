/*
** This file is a part of Iksemel (XML parser for Jabber/XMPP)
** Copyright (C) 2000-2025 Gurer Ozen
**
** Iksemel is free software: you can redistribute it and/or modify it
** under the terms of the GNU Lesser General Public License as
** published by the Free Software Foundation, either version 3 of
** the License, or (at your option) any later version.
*/

enum State {
    Prolog,
    PrologTag,
    PI,
    TagName,
    AttributeName,
    Whitespace,
}

pub enum ParserError {
    NoMemory,
    BadXml,
    HandlerError,
}

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
    fn handle_element(&mut self, element: SaxElement) -> Result<(), ParserError> { Ok(()) }
}

pub struct Parser {
    state: State,
    nr_bytes: usize,
    nr_lines: usize,
    buffer: Vec<u8>,
}

macro_rules! whitespace {
    () => {
        b' ' | b'\t' | b'\r' | b'\n'
    };
}

const INITIAL_BUFFER_CAPACITY: usize = 128;

impl Parser {
    pub fn new() -> Parser {
        Parser {
            state: State::Prolog,
            nr_bytes: 0,
            nr_lines: 0,
            buffer: Vec::<u8>::with_capacity(INITIAL_BUFFER_CAPACITY),
        }
    }

    pub fn parse_bytes(&mut self, handler: &mut impl SaxHandler, bytes: &[u8]) -> Result<(), ParserError> {
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
                    },
                },

                State::TagName => match c {
                    b'/' | b'>' | whitespace!() => {
                        if back < pos {
                            self.buffer.extend_from_slice(&bytes[back..pos]);
                        }
                        handler.handle_element(SaxElement::StartTag("lala"))?;
                        self.buffer.clear();
                    },
                    _ => (),
                },

                _ => (),
            }

            if !redo {
                pos += 1;
                self.nr_bytes += 1;
                if c == b'\n' {
                    self.nr_lines += 1;
                }
            }
        }

        if back < pos  {
            match self.state {
                State::TagName => self.buffer.extend_from_slice(&bytes[back..pos]),
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
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestHandler<'a> {
        expected: &'a [SaxElement<'a>],
        current: usize,
    }

    impl<'a> TestHandler<'a> {
        fn new(expected: &'a [SaxElement]) -> TestHandler<'a> {
            TestHandler {
                expected,
                current: 0,
            }
        }

        fn check(&mut self, s: &str) {
            let mut parser = Parser::new();
            assert!(parser.parse_bytes(self, &s.as_bytes()).is_ok());
            assert_eq!(self.current, self.expected.len());
        }
    }

    impl<'a> SaxHandler for TestHandler<'a> {
        fn handle_element(&mut self, element: SaxElement) -> Result<(), ParserError> {
            if self.current >= self.expected.len() {
                return Err(ParserError::HandlerError);
            }
            match self.expected[self.current] {
                SaxElement::StartTag(s) => (), // Check tag name
                _ => return Err(ParserError::HandlerError),
            }
            self.current += 1;
            Ok(())
        }
    }

    #[test]
    fn it_works() {
        TestHandler::new(&[SaxElement::StartTag("a")]).check("<a>lala</a>");
    }
}

// FIXME: finish testing util

// FIXME: nr_column

// FIXME: intake type for parse() str? io?
// FIXME: return value for parse()
//        how to set lifetime on returned str
//        error types
// FIXME: dynamic buffer for tagname, entity, attribs
