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
    pub const LT: &str = "&lt;";
    pub const GT: &str = "&gt;";
    pub const AMP: &str = "&amp;";
    pub const APOS: &str = "&apos;";
    pub const QUOT: &str = "&quot;";
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

pub fn escape(s: &str, output: &mut String) {
    let bytes = s.as_bytes();
    let mut i: usize = 0;
    let mut back: usize = 0;

    while i < bytes.len() {
        while i < bytes.len() {
            match bytes[i] {
                b'<' | b'>' | b'&' | b'\'' | b'"' => break,
                _ => back += 1,
            }
            i += 1;
        }
        unsafe {
            output.push_str(std::str::from_utf8_unchecked(&bytes[i - back..i]));
        }
        if i < bytes.len() {
            match bytes[i] {
                b'<' => output.push_str(predefined::LT),
                b'>' => output.push_str(predefined::GT),
                b'&' => output.push_str(predefined::AMP),
                b'\'' => output.push_str(predefined::APOS),
                b'"' => output.push_str(predefined::QUOT),
                _ => unreachable!(),
            }
            i += 1;
        }
        back = 0;
    }
}

pub fn escape_fmt(s: &str, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    let bytes = s.as_bytes();
    let mut i: usize = 0;
    let mut back: usize = 0;

    while i < bytes.len() {
        while i < bytes.len() {
            match bytes[i] {
                b'<' | b'>' | b'&' | b'\'' | b'"' => break,
                _ => back += 1,
            }
            i += 1;
        }
        unsafe {
            f.write_str(std::str::from_utf8_unchecked(&bytes[i - back..i]))?;
        }
        if i < bytes.len() {
            match bytes[i] {
                b'<' => f.write_str(predefined::LT),
                b'>' => f.write_str(predefined::GT),
                b'&' => f.write_str(predefined::AMP),
                b'\'' => f.write_str(predefined::APOS),
                b'"' => f.write_str(predefined::QUOT),
                _ => unreachable!(),
            }?;
            i += 1;
        }
        back = 0;
    }

    Result::Ok(())
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

    fn check_escape(input: &str, expected: &str) {
        let mut s = String::new();
        escape(input, &mut s);
        assert_eq!(s, expected);
    }

    #[test]
    fn escape_correct() {
        check_escape(NOESCAPE, NOESCAPE);
        check_escape(MID_CHAR, MID_CHAR_ESC);
        check_escape(ALL, ALL_ESC);
    }
}

// FIXME: unescape
// FIXME: mutant tests
