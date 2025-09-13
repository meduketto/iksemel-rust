/*
** This file is a part of Iksemel (XML parser for Jabber/XMPP)
** Copyright (C) 2000-2025 Gurer Ozen
**
** Iksemel is free software: you can redistribute it and/or modify it
** under the terms of the GNU Lesser General Public License as
** published by the Free Software Foundation, either version 3 of
** the License, or (at your option) any later version.
*/

use crate::xmpp::parser::StreamElement;

use super::*;

fn check_stream(stream_text: &str, expected: &[&str]) {
    let mut parser = StreamParser::new();
    let mut current: usize = 0;
    let mut ended: bool = false;

    let mut elements = parser.elements(&stream_text.as_bytes());
    while let Some(element) = elements.next() {
        assert!(!ended);
        match element.unwrap() {
            StreamElement::Element(document) => {
                assert_eq!(document.to_string(), expected[current]);
                current += 1;
            }
            StreamElement::End => {
                assert!(!ended);
                ended = true;
                assert_eq!(current, expected.len());
            }
        }
    }
    assert!(ended);
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
