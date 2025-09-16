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
    let doc = Document::new("html").unwrap();
    let blink = doc
        .insert_tag("p")
        .unwrap()
        .insert_tag("b")
        .unwrap()
        .insert_tag("blink")
        .unwrap()
        .insert_cdata("lala")
        .unwrap();
    assert!(!blink.is_null());

    let p2 = doc
        .root()
        .first_child()
        .append_cdata("foo&")
        .unwrap()
        .append_tag("p2")
        .unwrap();

    p2.prepend_cdata("bar").unwrap().prepend_tag("p3").unwrap();

    check_doc_xml(
        &doc,
        "<html><p><b><blink>lala</blink></b></p>foo&amp;<p3/>bar<p2/></html>",
    );
}

#[test]
fn attributes() {
    let doc = Document::new("doc").unwrap();
    let a = doc.insert_tag("a").unwrap();
    assert!(a.insert_attribute("i", "1").unwrap().is_tag());
    assert_eq!(
        a.insert_attribute("i", "1").unwrap_err(),
        ParseError::BadXml(description::DUPLICATE_ATTRIBUTE)
    );
    assert!(a.insert_attribute("j", "2").unwrap().is_tag());
    assert_eq!(
        a.insert_attribute("i", "1").unwrap_err(),
        ParseError::BadXml(description::DUPLICATE_ATTRIBUTE)
    );
    assert_eq!(
        a.insert_attribute("j", "1").unwrap_err(),
        ParseError::BadXml(description::DUPLICATE_ATTRIBUTE)
    );
    let _ = doc
        .insert_tag("b")
        .unwrap()
        .set_attribute("i", Some("1"))
        .unwrap()
        .set_attribute("i", Some("2"))
        .unwrap();
    check_doc_xml(&doc, "<doc><a i=\"1\" j=\"2\"/><b i=\"2\"/></doc>");

    let _ = doc.root().first_child().set_attribute("k", Some("3"));
    check_doc_xml(&doc, "<doc><a i=\"1\" j=\"2\" k=\"3\"/><b i=\"2\"/></doc>");

    let mut iter = doc.first_child().attributes();
    assert_eq!(iter.next(), Some(("i", "1")));
    assert_eq!(iter.next(), Some(("j", "2")));
    assert_eq!(iter.next(), Some(("k", "3")));
    assert_eq!(iter.next(), None);

    assert_eq!(doc.find_tag("a").attribute("i"), Some("1"));
    assert_eq!(doc.find_tag("a").attribute("j"), Some("2"));
    assert_eq!(doc.find_tag("a").attribute("k"), Some("3"));
    assert_eq!(doc.find_tag("b").attribute("i"), Some("2"));

    let _ = doc.find_tag("a").set_attribute("i", None);
    let _ = doc.find_tag("a").set_attribute("x", None);
    check_doc_xml(&doc, "<doc><a j=\"2\" k=\"3\"/><b i=\"2\"/></doc>");
    let _ = doc.find_tag("a").set_attribute("k", None);
    check_doc_xml(&doc, "<doc><a j=\"2\"/><b i=\"2\"/></doc>");
    let _ = doc.find_tag("a").set_attribute("j", None);
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
fn removals() {
    let doc = Document::from_str("<a>123<b/>456<c/><d><e/></d>789<f/></a>").unwrap();

    doc.find_tag("d").remove();
    assert_eq!(doc.to_string(), "<a>123<b/>456<c/>789<f/></a>");

    doc.find_tag("f").remove();
    assert_eq!(doc.to_string(), "<a>123<b/>456<c/>789</a>");

    doc.first_child().remove();
    assert_eq!(doc.to_string(), "<a><b/>456<c/>789</a>");

    doc.root().last_child().remove();
    assert_eq!(doc.to_string(), "<a><b/>456<c/></a>");

    for child in doc.root().children() {
        child.remove();
    }
    assert_eq!(doc.to_string(), "<a/>");
}

#[test]
fn iterators() {
    let doc = Document::from_str("<a>lala<b><c>bibi</c><d><e>123</e></d>456</b>foo</a>").unwrap();

    let mut iter = doc.find_tag("b").descendant_or_self();
    assert_eq!(iter.next().unwrap().name(), "b");
    assert_eq!(iter.next().unwrap().name(), "c");
    assert_eq!(iter.next().unwrap().cdata(), "bibi");
    assert_eq!(iter.next().unwrap().name(), "d");
    assert_eq!(iter.next().unwrap().name(), "e");
    assert_eq!(iter.next().unwrap().cdata(), "123");
    assert_eq!(iter.next().unwrap().cdata(), "456");
    assert!(iter.next().is_none());

    let mut iter = doc.find_tag("b").children();
    assert_eq!(iter.next().unwrap().name(), "c");
    assert_eq!(iter.next().unwrap().name(), "d");
    assert_eq!(iter.next().unwrap().cdata(), "456");
    assert!(iter.next().is_none());

    let doc = Document::from_str("<a>lala<b/>123<c>101</c>456<d/>abc<e><f/></e></a>").unwrap();
    let mut iter = doc.find_tag("d").following_sibling();
    assert_eq!(iter.next().unwrap().cdata(), "abc");
    assert_eq!(iter.next().unwrap().name(), "e");
    assert!(iter.next().is_none());

    let doc = Document::from_str("<a>lala<b/>123<c>101</c>456<d/>abc<e><f/></e></a>").unwrap();
    let mut iter = doc.find_tag("d").preceding_sibling();
    assert_eq!(iter.next().unwrap().cdata(), "456");
    assert_eq!(iter.next().unwrap().name(), "c");
    assert_eq!(iter.next().unwrap().cdata(), "123");
    assert_eq!(iter.next().unwrap().name(), "b");
    assert_eq!(iter.next().unwrap().cdata(), "lala");
    assert!(iter.next().is_none());
}

#[test]
fn null_checks() {
    let doc = Document::new("a").unwrap();

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
    // iterators
    assert!(doc.root().next().children().next().is_none());
    assert!(doc.root().next().descendant_or_self().next().is_none());
    assert!(doc.root().next().following_sibling().next().is_none());
    assert!(doc.root().next().attributes().next().is_none());
    // edits
    assert!(doc.root().next().insert_tag("k").is_err());
    assert!(doc.root().next().append_tag("k").is_err());
    assert!(doc.root().next().prepend_tag("k").is_err());
    assert!(doc.root().next().insert_attribute("k", "v").is_err());
    assert!(doc.root().next().set_attribute("k", None).is_err());
    assert!(doc.root().next().insert_cdata("k").is_err());
    assert!(doc.root().next().append_cdata("k").is_err());
    assert!(doc.root().next().prepend_cdata("k").is_err());
    doc.root().next().remove();
}

#[test]
fn bad_doc_parser() {
    assert_eq!(
        Document::from_str("<a>lala</b>").err(),
        Some(ParseError::BadXml(TAG_MISMATCH))
    );
    assert_eq!(
        Document::from_str("<a><b><c/></d></a>").err(),
        Some(ParseError::BadXml(TAG_MISMATCH))
    );
    assert_eq!(
        Document::from_str("<a><b><c/></b><d></d><e></e2></a>").err(),
        Some(ParseError::BadXml(TAG_MISMATCH))
    );
    assert_eq!(
        Document::from_str("<a><b x=\"1\" y=\"2\" x=\"abc\"/></a>").err(),
        Some(ParseError::BadXml(DUPLICATE_ATTRIBUTE))
    );
}
