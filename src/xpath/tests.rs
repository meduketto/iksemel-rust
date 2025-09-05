/*
** This file is a part of Iksemel (XML parser for Jabber/XMPP)
** Copyright (C) 2000-2025 Gurer Ozen
**
** Iksemel is free software: you can redistribute it and/or modify it
** under the terms of the GNU Lesser General Public License as
** published by the Free Software Foundation, either version 3 of
** the License, or (at your option) any later version.
*/

use crate::Document;

use super::*;

fn check_path(document: &Document, expression: &str, expected: &[&str]) {
    let p1 = XPath::new(expression).unwrap();
    let sequence = p1.apply(document).unwrap();
    assert_eq!(sequence.items.len(), expected.len());
    for (i, node) in sequence.items.iter().enumerate() {
        let result = match node {
            XPathValue::Node(cursor) => cursor.to_string(),
        };
        assert_eq!(result, expected[i]);
    }
}

#[test]
fn simple_steps() {
    let doc = Document::from_str("<a><b><c/></b><d><e>123</e><f>456<b/>789</f><b>abc</b></d></a>")
        .unwrap();

    check_path(
        &doc,
        "/*",
        &["<a><b><c/></b><d><e>123</e><f>456<b/>789</f><b>abc</b></d></a>"],
    );

    check_path(
        &doc,
        "/a",
        &["<a><b><c/></b><d><e>123</e><f>456<b/>789</f><b>abc</b></d></a>"],
    );

    check_path(&doc, "/a/b", &["<b><c/></b>"]);

    check_path(
        &doc,
        "/a/d/*",
        &["<e>123</e>", "<f>456<b/>789</f>", "<b>abc</b>"],
    );

    check_path(&doc, "//b", &["<e>123</e>", "<f>456</f>"]);
}
