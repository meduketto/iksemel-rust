/*
** This file is a part of Iksemel (XML parser for Jabber/XMPP)
** Copyright (C) 2000-2025 Gurer Ozen
**
** Iksemel is free software: you can redistribute it and/or modify it
** under the terms of the GNU Lesser General Public License as
** published by the Free Software Foundation, either version 3 of
** the License, or (at your option) any later version.
*/

pub mod predefined {
    pub const LT : &str = "&lt;";
    pub const GT : &str = "&gt;";
    pub const AMP : &str = "&amp;";
    pub const APOS : &str = "&apos;";
    pub const QUOT : &str = "&quot;";
}

pub fn escaped_size(s: &str) -> usize {
    let mut size = 0;
    for c in s.chars() {
        match c {
            '<' => size += predefined::LT.len(),
            '>' => size += predefined::GT.len(),
            '&' => size += predefined::AMP.len(),
            '\'' => size += predefined::APOS.len(),
            '"' => size += predefined::QUOT.len(),
            _ => size += 1,
        }
    }

    size
}

pub struct Encoder<'a> {
    bytes: &'a [u8],
    i: usize,
    back: usize,
}

impl<'a> Encoder<'a> {
    pub fn new(s: &'a str) -> Encoder<'a> {
        Encoder {
            bytes: s.as_bytes(),
            i: 0,
            back: 0,
        }
    }

    pub fn escape(&mut self, out: &mut String, max_bytes: usize) -> () {
        while self.i < self.bytes.len() {
            self.back = 0;
            while self.i < self.bytes.len() {
                match self.bytes[self.i] {
                    b'<' | b'>' | b'&' | b'\'' | b'"' => break,
                    _ => self.back += 1,
                };
                self.i += 1;
            }
            unsafe {
                out.push_str(std::str::from_utf8_unchecked(&self.bytes[self.i-self.back..self.i]));
            }
            if self.i < self.bytes.len() {
                match self.bytes[self.i] {
                    b'<' => out.push_str(predefined::LT),
                    b'>' => out.push_str(predefined::GT),
                    b'&' => out.push_str(predefined::AMP),
                    b'\'' => out.push_str(predefined::APOS),
                    b'"' => out.push_str(predefined::QUOT),
                    _ => unreachable!(),
                };
                self.i += 1;
            }
        }
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    const NOESCAPE: &str = "abc$#@!%^*(){}[]=-+/.,;:FDSF3443";
    const MID_CHAR: &str = "abc&def";
    const MID_CHAR_ESC: &str = "abc&amp;def";
    const ALL: &str = "<>&'\"";
    const ALL_ESC: &str = "&lt;&gt;&amp;&apos;&quot;";

    #[test]
    fn escaped_size_correct() {
        assert_eq!(escaped_size(NOESCAPE), NOESCAPE.len());
        assert_eq!(escaped_size(MID_CHAR), MID_CHAR_ESC.len());
        assert_eq!(escaped_size(ALL), ALL_ESC.len());
    }

    #[test]
    fn escape_correct() {
        let mut s = String::new();
        Encoder::new(NOESCAPE).escape(&mut s, 0);
        assert_eq!(s, NOESCAPE);
        s.clear();
        Encoder::new(MID_CHAR).escape(&mut s, 0);
        assert_eq!(s, MID_CHAR_ESC);
        s.clear();
        Encoder::new(ALL).escape(&mut s, 0);
        assert_eq!(s, ALL_ESC);
    }

}


// FIXME: escaped_size to method?
// FIXME: unescape
// FIXME: mutant tests
// FIXME: max_bytes impl
