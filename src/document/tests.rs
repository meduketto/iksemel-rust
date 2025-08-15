/*
** This file is a part of Iksemel (XML parser for Jabber/XMPP)
** Copyright (C) 2000-2025 Gurer Ozen
**
** Iksemel is free software: you can redistribute it and/or modify it
** under the terms of the GNU Lesser General Public License as
** published by the Free Software Foundation, either version 3 of
** the License, or (at your option) any later version.
*/

use super::error::description::*;
use super::*;

fn check_doc_xml(doc: &Document, expected: &str) {
    let xml = doc.to_string();
    assert_eq!(xml, expected);
    // Verify that the capacity is measured correctly
    assert_eq!(xml.len(), xml.capacity());
    // Verify that the Display and to_string are same
    let xml2 = format!("{}", doc);
    assert_eq!(xml2, expected);
}

#[test]
fn it_works() {
    let doc = Document::new("html");
    let blink = doc
        .insert_tag("p")
        .insert_tag(&doc, "b")
        .insert_tag(&doc, "blink")
        .insert_cdata(&doc, "lala");
    assert!(!blink.is_null());

    let p2 = doc
        .root()
        .first_child()
        .append_cdata(&doc, "foo&")
        .append_tag(&doc, "p2");

    p2.prepend_cdata(&doc, "bar").prepend_tag(&doc, "p3");

    check_doc_xml(
        &doc,
        "<html><p><b><blink>lala</blink></b></p>foo&amp;<p3/>bar<p2/></html>",
    );
}

#[test]
fn attributes() {
    let doc = Document::new("doc");
    let _ = doc
        .insert_tag("a")
        .set_attribute(&doc, "i", "1")
        .set_attribute(&doc, "j", "2");
    let _ = doc
        .insert_tag("b")
        .set_attribute(&doc, "i", "1")
        .set_attribute(&doc, "i", "2");
    check_doc_xml(&doc, "<doc><a i=\"1\" j=\"2\"/><b i=\"2\"/></doc>");

    let _ = doc.root().first_child().set_attribute(&doc, "k", "3");
    check_doc_xml(&doc, "<doc><a i=\"1\" j=\"2\" k=\"3\"/><b i=\"2\"/></doc>");
}

#[test]
fn doc_parser() {
    let doc = Document::from_str("<a><b>123<c/>456</b><d x='1' y='2'>lala</d></a>");
    println!("{}", doc.unwrap());
}

#[test]
fn bad_doc_parser() {
    assert_eq!(
        Document::from_str("<a>lala</b>").err(),
        Some(SaxError::BadXml(TAG_MISMATCH))
    );
    assert_eq!(
        Document::from_str("<a><b><c/></d></a>").err(),
        Some(SaxError::BadXml(TAG_MISMATCH))
    );
    assert_eq!(
        Document::from_str("<a><b><c/></b><d></d><e></e2></a>").err(),
        Some(SaxError::BadXml(TAG_MISMATCH))
    );
}
