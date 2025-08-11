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
    expected: &'a [SaxElement<'a>],
    current: usize,
    cdata_buf: String,
}

impl<'a> Tester<'a> {
    fn new(expected: &'a [SaxElement]) -> Tester<'a> {
        Tester {
            expected,
            current: 0,
            cdata_buf: String::new(),
        }
    }

    fn check(&mut self, s: &str) {
        let nr_lines = s.matches("\n").count();
        let nr_column = s.lines().last().unwrap().len();

        let mut parser = SaxParser::new();
        assert!(parser.parse_bytes_finish(self, &s.as_bytes()).is_ok());
        assert_eq!(self.current, self.expected.len());
        assert_eq!(parser.nr_lines(), nr_lines);
        assert_eq!(parser.nr_column(), nr_column);
        assert_eq!(parser.nr_bytes(), s.len());

        // now try byte by byte
        parser.reset();
        self.current = 0;
        self.cdata_buf.clear();
        for i in 0..s.len() {
            assert!(parser.parse_bytes(self, &s.as_bytes()[i..i + 1]).is_ok());
        }
        assert!(parser.parse_finish().is_ok());
        assert_eq!(self.current, self.expected.len());
        assert_eq!(parser.nr_lines(), nr_lines);
        assert_eq!(parser.nr_column(), nr_column);
        assert_eq!(parser.nr_bytes(), s.len());
    }
}

impl<'a> SaxHandler for Tester<'a> {
    fn handle_element(&mut self, element: &SaxElement) -> Result<(), SaxError> {
        assert!(self.current < self.expected.len());
        if let SaxElement::CData(cdata) = element {
            if let SaxElement::CData(cdata2) = self.expected[self.current] {
                self.cdata_buf.push_str(cdata);
                if self.cdata_buf.len() >= cdata2.len() {
                    assert_eq!(self.cdata_buf, cdata2);
                    self.current += 1;
                    self.cdata_buf.clear();
                }
            } else {
                assert_eq!(element, &self.expected[self.current]);
            }
        } else {
            assert_eq!(element, &self.expected[self.current]);
            self.current += 1;
        }
        Ok(())
    }
}

struct BadTester {
    bad_byte: usize,
}

impl BadTester {
    fn new(bad_byte: usize) -> BadTester {
        BadTester { bad_byte }
    }

    fn check(&mut self, s: &str) {
        self.check_with_error(s, SaxError::BadXml)
    }

    fn check_with_error(&mut self, s: &str, expected_error: SaxError) {
        let mut parser = SaxParser::new();
        assert_eq!(
            parser.parse_bytes_finish(self, &s.as_bytes()),
            Err(expected_error)
        );
        assert_eq!(parser.nr_bytes(), self.bad_byte);
    }

    fn check_bytes(&mut self, bytes: &[u8]) {
        let mut parser = SaxParser::new();
        assert_eq!(
            parser.parse_bytes_finish(self, bytes),
            Err(SaxError::BadXml)
        );
        assert_eq!(parser.nr_bytes(), self.bad_byte);
    }
}

impl SaxHandler for BadTester {
    fn handle_element(&mut self, _element: &SaxElement) -> Result<(), SaxError> {
        Ok(())
    }
}

#[test]
fn tags() {
    Tester::new(&[SaxElement::StartTag("lonely"), SaxElement::EmptyElementTag]).check("<lonely/>");

    Tester::new(&[SaxElement::StartTag("lonely"), SaxElement::EmptyElementTag])
        .check("   <lonely/>    ");

    Tester::new(&[
        SaxElement::StartTag("parent"),
        SaxElement::StartTag("child"),
        SaxElement::EmptyElementTag,
        SaxElement::StartTag("child"),
        SaxElement::EmptyElementTag,
        SaxElement::CData("child"),
        SaxElement::EndTag("parent"),
    ])
    .check("<?xml version='1.0'?><parent><child/><child/>child</parent>");

    Tester::new(&[
        SaxElement::StartTag("parent"),
        SaxElement::StartTag("empty"),
        SaxElement::EmptyElementTag,
        SaxElement::StartTag("b"),
        SaxElement::CData("lala"),
        SaxElement::EndTag("b"),
        SaxElement::EndTag("parent"),
    ])
    .check("<parent  ><empty \t /><b>lala</b \n></parent>");

    Tester::new(&[
        SaxElement::StartTag("mytag"),
        SaxElement::Attribute("abc", "123"),
        SaxElement::Attribute("id", "XC72"),
        SaxElement::EndTag("mytag"),
    ])
    .check("<mytag abc='123' id=\"XC72\"></mytag>");

    Tester::new(&[
        SaxElement::StartTag("a"),
        SaxElement::StartTag("b"),
        SaxElement::Attribute("x1", "lala"),
        SaxElement::EmptyElementTag,
        SaxElement::StartTag("c"),
        SaxElement::Attribute("x2", "bibi"),
        SaxElement::EmptyElementTag,
        SaxElement::EndTag("a"),
    ])
    .check("<a><b x1 ='lala'/><c x2\t= \t'bibi'/></a>");

    Tester::new(&[
        SaxElement::StartTag("tag"),
        SaxElement::Attribute("a", "1"),
        SaxElement::Attribute("b", "2"),
        SaxElement::Attribute("c", "3"),
        SaxElement::Attribute("d", "4"),
        SaxElement::Attribute("e", "5"),
        SaxElement::Attribute("f", "6"),
        SaxElement::Attribute("g", "7"),
        SaxElement::Attribute("id", "xyz9"),
        SaxElement::StartTag("sub"),
        SaxElement::EndTag("sub"),
        SaxElement::EndTag("tag"),
    ])
    .check("<tag a  =  '1' b  ='2' c=  '3' d='4'   e='5' f='6' g='7' id='xyz9'><sub></sub></tag>");

    Tester::new(&[
        SaxElement::StartTag("tag"),
        SaxElement::Attribute("a", "12\"34"),
        SaxElement::Attribute("b", "123'456"),
        SaxElement::EmptyElementTag,
    ])
    .check("<tag a='12\"34' b=\"123'456\" />");

    Tester::new(&[
        SaxElement::StartTag("a"),
        SaxElement::StartTag("b"),
        SaxElement::CData("john&mary"),
        SaxElement::StartTag("c"),
        SaxElement::StartTag("d"),
        SaxElement::Attribute("e", "f"),
        SaxElement::Attribute("g", "123456"),
        SaxElement::Attribute("h", "madcat"),
        SaxElement::Attribute("klm", "nop"),
        SaxElement::EmptyElementTag,
        SaxElement::EndTag("c"),
        SaxElement::EndTag("b"),
        SaxElement::EndTag("a"),
    ])
    .check("<a><b>john&amp;mary<c><d e='f' g='123456' h='madcat' klm='nop'/></c></b></a>");
}

#[test]
fn comments() {
    Tester::new(&[
        SaxElement::StartTag("item"),
        SaxElement::Attribute("url", "http://jabber.org"),
        SaxElement::CData("Jabber Site"),
        SaxElement::EndTag("item"),
    ])
    .check("<item url='http://jabber.org'><!-- little comment -->Jabber Site</item>");

    Tester::new(&[
        SaxElement::StartTag("index"),
        SaxElement::StartTag("item"),
        SaxElement::Attribute("name", "lala"),
        SaxElement::Attribute("page", "42"),
        SaxElement::EmptyElementTag,
        SaxElement::EndTag("index"),
    ])
    .check("<index><!-- <item> - tag has no childs --><item name='lala' page='42'/></index>");

    Tester::new(&[SaxElement::StartTag("empty"), SaxElement::EmptyElementTag])
        .check("<!-- comment --> <empty/> <!-- lala -->");
}

#[test]
fn cdatas() {
    Tester::new(&[
        SaxElement::StartTag("ka"),
        SaxElement::CData("1234 <ka> lala ] ]] ]]] 4321"),
        SaxElement::EndTag("ka"),
    ])
    .check("<ka>1234<![CDATA[ <ka> lala ] ]] ]]] ]]>4321</ka>");

    Tester::new(&[
        SaxElement::StartTag("data"),
        SaxElement::CData("[TEST]"),
        SaxElement::EndTag("data"),
    ])
    .check("<data><![CDATA[[TEST]]]></data>");

    Tester::new(&[
        SaxElement::StartTag("data"),
        SaxElement::CData("[TEST]]"),
        SaxElement::EndTag("data"),
    ])
    .check("<data><![CDATA[[TEST]]]]></data>");

    Tester::new(&[
        SaxElement::StartTag("a"),
        SaxElement::CData("[[bg:Чингис хан]][[bn:চেঙ্গিজ খান]]"),
        SaxElement::EndTag("a"),
    ])
    .check("<a>[[bg:Чингис хан]][[bn:চেঙ্গিজ খান]]</a>");
}

#[test]
fn dtds() {
    Tester::new(&[
        SaxElement::StartTag("x"),
        SaxElement::CData("foo"),
        SaxElement::EndTag("x"),
    ])
    .check(" <!DOCTYPE greeting [ <!ELEMENT greeting (#PCDATA)> ]> <x>foo</x>");
}

#[test]
fn pi() {
    Tester::new(&[
        SaxElement::StartTag("a"),
        SaxElement::CData("bibi"),
        SaxElement::EndTag("a"),
    ])
    .check("<a><?xml lala?>bibi</a>");
}

#[test]
fn entities() {
    Tester::new(&[
            SaxElement::StartTag("body"),
            SaxElement::CData("I'm fixing parser&tester for \"<\" and \">\" chars."),
            SaxElement::EndTag("body"),
        ])
        .check("<body>I&apos;m fixing parser&amp;tester for &quot;&lt;&quot; and &quot;&gt;&quot; chars.</body>");

    Tester::new(&[
        SaxElement::StartTag("test"),
        SaxElement::StartTag("standalone"),
        SaxElement::Attribute("be", "happy"),
        SaxElement::EmptyElementTag,
        SaxElement::CData("abcd"),
        SaxElement::StartTag("br"),
        SaxElement::EmptyElementTag,
        SaxElement::CData("<escape>"),
        SaxElement::EndTag("test"),
    ])
    .check("<test><standalone be='happy'/>abcd<br/>&lt;escape&gt;</test>");

    Tester::new(&[
        SaxElement::StartTag("a"),
        SaxElement::CData(";AB;"),
        SaxElement::EndTag("a"),
    ])
    .check("<a>&#x3B;&#65;&#x42;&#x3b;</a>");

    Tester::new(&[
        SaxElement::StartTag("a"),
        SaxElement::CData(" \u{90} \u{900} \u{10abc} "),
        SaxElement::EndTag("a"),
    ])
    .check("<a> &#x90; &#x900; &#x10abc; </a>");
}

#[test]
fn attribute_entities() {
    Tester::new(&[
        SaxElement::StartTag("a"),
        SaxElement::Attribute("b", "a&b BA"),
        SaxElement::EndTag("a"),
    ])
    .check("<a b='a&amp;b &#x42;&#65;'></a>");
}

#[test]
fn long_tag() {
    let name = "abc".repeat(500);
    let xml = format!("<{}></{}>", name, name);

    Tester::new(&[SaxElement::StartTag(&name), SaxElement::EndTag(&name)]).check(&xml);
}

#[test]
fn location() {
    Tester::new(&[
        SaxElement::StartTag("a"),
        SaxElement::CData("\n\n "),
        SaxElement::EndTag("a"),
    ])
    .check("<a>\n\n </a>");
}

#[test]
fn bad_tags() {
    BadTester::new(4).check("<a>< b/></a>");
    BadTester::new(6).check("<a><b/ ></a>");
    BadTester::new(8).check("<a></ccc/></a>");
    BadTester::new(13).check("<a><b/><c></c/></a>");
    BadTester::new(1).check("</a>");
    BadTester::new(9).check("<a> </a  b>");
    BadTester::new(8).check("<a></a><b/>");
    BadTester::new(10).check("<a a='1' b></a>");
    BadTester::new(11).check("<a a='1' b=></a>");
    BadTester::new(12).check("<a a='12' b '2'></a>");
    BadTester::new(13).check("<a a='123' b c='5'></a>");
    BadTester::new(14).check("<a a='12'></a b='1'>");
    BadTester::new(17).check("<g><test a='123'/ b='lala'></g>");
    BadTester::new(13).check("<a a='1' b='></a>");
    BadTester::new(13).check("<a a='1' b=\"></a>");
    BadTester::new(5).check("<a> <> </a>");
    BadTester::new(6).check("<a> </> </a>");
}

#[test]
fn bad_comments() {
    BadTester::new(10).check("<e><!-- -- --></e>");
    BadTester::new(22).check("<ha><!-- <lala> --><!- comment -></ha>");
    BadTester::new(12).check("<!-- c1 --> lala <ha/>");
    BadTester::new(31).check("<!-- c1 --> <ha/> <!-- pika -->c");
    BadTester::new(9).check("<!-- c ---> <ha/>");
}

#[test]
fn bad_pi() {
    BadTester::new(12).check("<e/> <?xml >");
    BadTester::new(13).check("<e/> <?xml ?>lala");
}

#[test]
fn bad_cdatas() {
    BadTester::new(2).check("  lala <a></a>");
    BadTester::new(10).check("  <a></a> lala");
    BadTester::new(11).check("  <a></a > lala");
    BadTester::new(2).check("<![CDATA[lala]> <a/>");
    BadTester::new(8).check(" <a/> <![CDATA[lala]>");
    BadTester::new(7).check("<a> <![DATA[lala]> </a>");
    BadTester::new(9).check("<a> <![CDaTA[lala]> </a>");
    BadTester::new(12).check("<a> <![CDATAlala]> </a>");
}

#[test]
fn bad_entities() {
    BadTester::new(8).check_with_error("<a>&lala;</a>", SaxError::NotSupported);
    BadTester::new(12).check_with_error("<a>&lala           </a>", SaxError::NotSupported);
    BadTester::new(16).check("<lol>&lt;<&gt;</lol>");
    BadTester::new(6).check("<a>&#1a;</a>");
    BadTester::new(6).check("<a>&#Xaa;</a>");
    BadTester::new(8).check("<a>&#xa5g;</a>");
    BadTester::new(6).check("<a>&#8;</a>");
    BadTester::new(7).check("<a>&#11;</a>");
    BadTester::new(7).check("<a>&#15;</a>");
    BadTester::new(10).check("<a>&#xD800;</a>");
    BadTester::new(10).check("<a>&#xDfFf;</a>");
    BadTester::new(10).check("<a>&#xfFfE;</a>");
    BadTester::new(10).check("<a>&#xFFff;</a>");
    BadTester::new(12).check("<a>&#x110000;</a>");
}

#[test]
fn bad_chars() {
    BadTester::new(6).check_bytes(b"<test>\xFF</test>");
    BadTester::new(6).check_bytes(b"<test>\xFE</test>");
    BadTester::new(2).check_bytes(b"<t\x00></t>");
    BadTester::new(2).check_bytes(b"<t\x19></t>");
    BadTester::new(8).check_bytes(b"<test>\xe3\x8fa</test>");
    BadTester::new(7).check_bytes(b"<test>\xC0\x80</test>");
    BadTester::new(7).check_bytes(b"<test>\xC0\xaf</test>");
    BadTester::new(8).check_bytes(b"<test>\xe0\x80\xaf</test>");
    BadTester::new(9).check_bytes(b"<test>\xf0\x80\x80\xaf</test>");
    BadTester::new(7).check_bytes(b"<test>\xc1\xbf</test>");
    BadTester::new(8).check_bytes(b"<test>\xe0\x9f\xbf</test>");
    BadTester::new(9).check_bytes(b"<test>\xf0\x8f\xbf\xbf</test>");
    BadTester::new(1).check_bytes(b"<\x8f\x85></\x8f\x85>");
    BadTester::new(7).check_bytes(
        b"<utf8>\xC1\x80<br/>\xED\x95\x9C\xEA\xB5\xAD\xEC\x96\xB4<err>\xC1\x65</err></utf8>",
    );
}

#[test]
fn bad_unfinished() {
    BadTester::new(5).check(" <a> ");
    BadTester::new(20).check("  <!-- lala -->     ");
    BadTester::new(27).check(" <a></a> <!-- open comment ");
    BadTester::new(23).check(" <a></a> <?app open pi ");
}
