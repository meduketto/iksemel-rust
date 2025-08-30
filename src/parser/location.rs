/*
** This file is a part of Iksemel (XML parser for Jabber/XMPP)
** Copyright (C) 2000-2025 Gurer Ozen
**
** Iksemel is free software: you can redistribute it and/or modify it
** under the terms of the GNU Lesser General Public License as
** published by the Free Software Foundation, either version 3 of
** the License, or (at your option) any later version.
*/

use std::fmt::Display;

/// A position in the parser input byte stream.
///
/// This struct is returned from the
/// [location()](crate::SaxParser::location) method and gives you
/// the position of the last parsed byte in the input stream.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Location {
    pub bytes: usize,
    pub lines: usize,
    pub column: usize,
}

impl Location {
    /// Creates a new location at the beginning of the input stream.
    ///
    /// This constructor is made public in case if you store the
    /// location in somewhere and need a default.
    pub fn new() -> Self {
        Location {
            bytes: 0,
            lines: 0,
            column: 0,
        }
    }

    pub(super) fn advance(&mut self, c: u8) {
        self.bytes += 1;
        if c == b'\n' {
            self.lines += 1;
            self.column = 0;
        } else {
            self.column += 1;
        }
    }
}

impl Default for Location {
    fn default() -> Self {
        Location::new()
    }
}

impl Display for Location {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "byte: {}, line: {}, column: {}",
            self.bytes, self.lines, self.column
        )
    }
}
