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
    let a = doc.insert_tag("a");
    assert!(a.insert_attribute(&doc, "i", "1").unwrap().is_tag());
    assert_eq!(
        a.insert_attribute(&doc, "i", "1").unwrap_err(),
        DocumentError::BadXml(description::DUPLICATE_ATTRIBUTE)
    );
    assert!(a.insert_attribute(&doc, "j", "2").unwrap().is_tag());
    assert_eq!(
        a.insert_attribute(&doc, "i", "1").unwrap_err(),
        DocumentError::BadXml(description::DUPLICATE_ATTRIBUTE)
    );
    assert_eq!(
        a.insert_attribute(&doc, "j", "1").unwrap_err(),
        DocumentError::BadXml(description::DUPLICATE_ATTRIBUTE)
    );
    let _ = doc
        .insert_tag("b")
        .set_attribute(&doc, "i", Some("1"))
        .unwrap()
        .set_attribute(&doc, "i", Some("2"))
        .unwrap();
    check_doc_xml(&doc, "<doc><a i=\"1\" j=\"2\"/><b i=\"2\"/></doc>");

    let _ = doc.root().first_child().set_attribute(&doc, "k", Some("3"));
    check_doc_xml(&doc, "<doc><a i=\"1\" j=\"2\" k=\"3\"/><b i=\"2\"/></doc>");

    assert_eq!(doc.find_tag("a").attribute("i"), Some("1"));
    assert_eq!(doc.find_tag("a").attribute("j"), Some("2"));
    assert_eq!(doc.find_tag("a").attribute("k"), Some("3"));
    assert_eq!(doc.find_tag("b").attribute("i"), Some("2"));

    let _ = doc.find_tag("a").set_attribute(&doc, "i", None);
    let _ = doc.find_tag("a").set_attribute(&doc, "x", None);
    check_doc_xml(&doc, "<doc><a j=\"2\" k=\"3\"/><b i=\"2\"/></doc>");
    let _ = doc.find_tag("a").set_attribute(&doc, "k", None);
    check_doc_xml(&doc, "<doc><a j=\"2\"/><b i=\"2\"/></doc>");
    let _ = doc.find_tag("a").set_attribute(&doc, "j", None);
    check_doc_xml(&doc, "<doc><a/><b i=\"2\"/></doc>");
}

#[test]
fn navigation() {
    let doc = Document::from_str("<a><b>123<c/>456</b>.,;<d/> <e x='1' y='2'> lala<f/></e>789</a>")
        .unwrap();
    assert_eq!(doc.root().first_tag().first_tag().to_string(), "<c/>");
    assert_eq!(doc.root().first_child().next().to_string(), ".,;");
    assert_eq!(doc.root().first_child().next().next().to_string(), "<d/>");
    assert_eq!(doc.root().first_child().next_tag().to_string(), "<d/>");
    assert_eq!(doc.root().first_tag().last_child().cdata(), "456");
    assert_eq!(doc.root().last_child().to_string(), "789");
    assert_eq!(
        doc.root().last_child().previous().previous().to_string(),
        " "
    );
    assert_eq!(
        doc.root()
            .last_child()
            .previous_tag()
            .previous_tag()
            .to_string(),
        "<d/>"
    );
    assert_eq!(
        doc.root().last_child().previous().first_tag().to_string(),
        "<f/>"
    );
    assert_eq!(
        doc.first_child()
            .first_tag()
            .parent()
            .next_tag()
            .next_tag()
            .find_tag("f")
            .root()
            .find_tag("e")
            .first_child()
            .to_string(),
        " lala"
    );
    assert_eq!(doc.first_tag().first_tag().to_string(), "<c/>");
    assert_eq!(
        doc.find_tag("e").to_string(),
        "<e x=\"1\" y=\"2\"> lala<f/></e>"
    );
}

#[test]
fn doc_parser() {
    let doc = Document::from_str("<a><b>123<c/>456</b><d x='1' y='2'>lala</d></a>");
    println!("{}", doc.unwrap());
    // FIXME:
}

#[test]
fn serialize_subset() {
    let doc = Document::from_str("<a><b>lala</b><c>bibi</c><d><e>123</e></d></a>").unwrap();
    assert_eq!(doc.first_child().to_string(), "<b>lala</b>");
    assert_eq!(doc.find_tag("c").to_string(), "<c>bibi</c>");
    assert_eq!(doc.find_tag("d").to_string(), "<d><e>123</e></d>");
    assert_eq!(doc.find_tag("d").first_child().to_string(), "<e>123</e>");
}

#[test]
fn cursor_clone() {
    let doc = Document::from_str("<a><b>lala</b><c>bibi</c><d><e>123</e></d></a>").unwrap();

    let c4: Cursor;
    {
        let c1 = doc.root();
        c4 = c1.clone();
        let c2 = c1.clone().find_tag("d").first_child();
        assert_eq!(c2.first_child().to_string(), "123");
        let c3 = c1.find_tag("b").first_child();
        assert_eq!(c3.to_string(), "lala");
    }
    assert_eq!(c4.find_tag("c").first_child().to_string(), "bibi");
}

#[test]
fn null_checks() {
    let doc = Document::new("a");

    // property
    assert_eq!(doc.root().next().is_null(), true);
    assert_eq!(doc.root().next().is_tag(), false);
    assert_eq!(doc.root().next().name(), "");
    assert_eq!(doc.root().next().attribute("lala"), None);
    assert_eq!(doc.root().next().cdata(), "");
    assert_eq!(doc.root().next().str_size(), 0);
    // FIXME: to_string
    // FIXME: display
    assert_eq!(doc.root().next().clone().is_null(), true);
    // navigation
    assert!(doc.root().next().next().is_null());
    assert!(doc.root().next().next_tag().is_null());
    assert!(doc.root().next().previous().is_null());
    assert!(doc.root().next().previous_tag().is_null());
    assert!(doc.root().next().first_child().is_null());
    assert!(doc.root().next().last_child().is_null());
    assert!(doc.root().next().first_tag().is_null());
    assert!(doc.root().next().parent().is_null());
    assert!(doc.root().next().root().is_null());
    assert!(doc.root().next().find_tag("lala").is_null());
    // edits
    // FIXME: edits
    assert!(doc.root().next().insert_attribute(&doc, "k", "v").is_err());
    assert!(doc.root().next().set_attribute(&doc, "k", None).is_err());
}

#[test]
fn bad_doc_parser() {
    assert_eq!(
        Document::from_str("<a>lala</b>").err(),
        Some(DocumentError::BadXml(TAG_MISMATCH))
    );
    assert_eq!(
        Document::from_str("<a><b><c/></d></a>").err(),
        Some(DocumentError::BadXml(TAG_MISMATCH))
    );
    assert_eq!(
        Document::from_str("<a><b><c/></b><d></d><e></e2></a>").err(),
        Some(DocumentError::BadXml(TAG_MISMATCH))
    );
    assert_eq!(
        Document::from_str("<a><b x=\"1\" y=\"2\" x=\"abc\"/></a>").err(),
        Some(DocumentError::BadXml(DUPLICATE_ATTRIBUTE))
    );
}
