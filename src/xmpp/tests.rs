/*
** This file is a part of Iksemel (XML parser for Jabber/XMPP)
** Copyright (C) 2000-2025 Gurer Ozen
**
** Iksemel is free software: you can redistribute it and/or modify it
** under the terms of the GNU Lesser General Public License as
** published by the Free Software Foundation, either version 3 of
** the License, or (at your option) any later version.
*/

use super::*;

struct Tester<'a> {
    expected: &'a [&'a str],
    current: usize,
    ended: bool,
}

impl<'a> Tester<'a> {
    fn new(expected: &'a [&'a str]) -> Tester<'a> {
        Tester {
            expected,
            current: 0,
            ended: false,
        }
    }
}

impl<'a> StreamHandler for Tester<'a> {
    fn handle_stream_element(&mut self, element: Document) {
        assert!(!self.ended);
        assert!(self.current < self.expected.len());
        assert_eq!(element.to_string(), self.expected[self.current]);
        self.current += 1;
    }

    fn handle_stream_end(&mut self) {
        assert!(!self.ended);
        self.ended = true;
        assert_eq!(self.current, self.expected.len());
    }
}

fn check_stream(stream_text: &str, expected: &[&str]) {
    let mut tester = Tester::new(expected);
    let mut parser = StreamParser::new(&mut tester);

    assert_eq!(parser.parse_bytes(&stream_text.as_bytes()), Ok(()));
    assert!(tester.ended);
}

#[test]
fn stream_elements() {
    check_stream(
        "<stream:stream xmlns:stream='http://etherx.jabber.org/streams'\
                        version='1.0'\
                        from='example.com'\
                        to='user@example.com'>\
        <message to='user@example.com'>\
            <body>Hello!</body>\
        </message>\
        </stream:stream>",
        &[
            "<stream:stream xmlns:stream=\"http://etherx.jabber.org/streams\" version=\"1.0\" from=\"example.com\" to=\"user@example.com\"/>",
            "<message to=\"user@example.com\"><body>Hello!</body></message>",
        ],
    );
}
