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
    pub const QUOT : &str = "&qout;";
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

pub fn escape(s: &str, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    Result::Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escape_size() {
        const NOESCAPE: &str = "abc$#@!%^*(){}[]=-+/.,;:FDSF3443";
        assert_eq!(escaped_size(NOESCAPE), NOESCAPE.len());
        assert_eq!(escaped_size("abc&def"), "abc&amp;def".len());
        assert_eq!(escaped_size("<>&'\""), "&lt;&gt;&amp;&apos;&quot;".len());
    }
}
