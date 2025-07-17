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

pub struct Parser {
    state: State,
    old_state: State,
    nr_bytes: usize,
    nr_lines: usize,
}

pub enum ParserError {
    NoMemory,
    BadXml,
}

pub enum Element {
    NotYet,
    StartTag,
    Attribute,
    EndTag,
    CData,
    Comment,
    PI,
}

macro_rules! whitespace {
    () => {
        b' ' | b'\t' | b'\r' | b'\n'
    };
}

impl Parser {
    pub fn new() -> Parser {
        Parser {
            state: State::Prolog,
            old_state: State::Prolog,
            nr_bytes: 0,
            nr_lines: 0,
        }
    }

    pub fn parse_bytes(&mut self, bytes: &[u8]) -> Result<Element, ParserError> {
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
                    _ => self.state = State::TagName,
                },

                State::TagName => match c {
                    b'/' => (),
                    b'>' => (),
                    whitespace!() => (),
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

        Ok(Element::NotYet)
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

    #[test]
    fn it_works() {
        let mut parser = Parser();
    }
}

// FIXME: intake type for parse() str? io?
// FIXME: return value for parse()
//        how to set lifetime on returned str
//        error types
// FIXME: dynamic buffer for tagname, entity, attribs
