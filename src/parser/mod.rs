/*
** This file is a part of Iksemel (XML parser for Jabber/XMPP)
** Copyright (C) 2000-2025 Gurer Ozen
**
** Iksemel is free software: you can redistribute it and/or modify it
** under the terms of the GNU Lesser General Public License as
** published by the Free Software Foundation, either version 3 of
** the License, or (at your option) any later version.
*/

mod error;
mod location;

pub use error::ParseError;
use error::description;
pub use location::Location;

/// An XML element returned from the parser.
#[derive(Debug, Eq, PartialEq)]
pub enum SaxElement<'a> {
    /// A start tag or empty element tag.
    ///
    /// The argument is the full name of the tag. This element is sent to
    /// the handler as soon as the name is parsed.
    ///
    /// This is always followed by zero or more [Attribute](SaxElement::Attribute)
    /// elements, and then either a [StartTagContent](SaxElement::StartTagContent)
    /// or [StartTagEmpty](SaxElement::StartTagEmpty) element.
    StartTag(&'a str),

    /// A tag attribute for the last StartTag.
    ///
    /// First argument is the attribute name and the second argument is the
    /// attribute value. All references in the attribute value are replaced
    /// with the actual characters. Each attribute is sent as a separate
    /// element for efficiency.
    Attribute(&'a str, &'a str),

    /// Indicates that the last StartTag was not an empty element tag.
    ///
    /// Note that you might still get an [EndTag](SaxElement::EndTag)
    /// immediately after this, as `<tag/>` and `<tag></tag>` are
    /// distinct in the XML specification, and not normalized.
    StartTagContent,

    /// Indicates that the last StartTag was an empty element tag and will have no content.
    StartTagEmpty,

    /// An end tag element.
    ///
    /// The argument is the full name of the end tag.
    EndTag(&'a str),

    /// A character data element.
    ///
    /// The argument is the text content. Note that you might get this element several times
    /// with different parts of the content for a single continous block of text. When you parse
    /// the document in multiple parse calls, or when the parser encounters a reference to
    /// substitute, collected content is flushed. The [DocumentParser](crate::DocumentParser)
    /// of iksemel automatically concatenates these parts to build a seamless document model.
    CData(&'a str),
}

pub struct SaxElements<'a> {
    parser: &'a mut SaxParser,
    bytes: &'a [u8],
    bytes_parsed: usize,
}

impl<'a> SaxElements<'a> {
    pub fn new(parser: &'a mut SaxParser, bytes: &'a [u8]) -> Self {
        SaxElements {
            parser,
            bytes,
            bytes_parsed: 0,
        }
    }

    #[allow(
        clippy::should_implement_trait,
        reason = "Iterator trait does not support lending iterator pattern"
    )]
    pub fn next(&mut self) -> Option<Result<SaxElement<'_>, ParseError>> {
        if self.bytes_parsed == self.bytes.len() {
            None
        } else {
            match self.parser.parse_bytes(&self.bytes[self.bytes_parsed..]) {
                Ok(Some((element, bytes))) => {
                    self.bytes_parsed += bytes;
                    Some(Ok(element))
                }
                Ok(None) => {
                    self.bytes_parsed = self.bytes.len();
                    None
                }
                Err(err) => Some(Err(err)),
            }
        }
    }
}

/// SAX (Simple API for XML) based XML parser.
///
/// This struct implements a SAX parser which processes the incoming
/// bytes and invokes a handler method for each encountered
/// XML element.
///
/// # SAX Limitations
///
/// This kind of parsing is extremely memory efficient, but it cannot
/// do certain validations such as checking tag mismatches, or cannot
/// run complex queries on the document without storing the information
/// in external memory. The [DocumentParser](crate::DocumentParser) does
/// all of that by building a [Document](crate::Document) tree in memory
/// from the SAX events, and should be preferred when you don't have
/// extreme memory constraints.
///
/// # Iksemel Limitations
///
/// Iksemel parser has some additional limitations listed below. See
/// the DESIGN.md file for reasons.
///
/// - Only the UTF-8 encoded byte streams are supported. You can parse
///   other encodings by converting them to UTF-8 before the parsing.
///
/// - DTDs are syntactically parsed but not used for validation.
///
/// - Custom entity references within DTDs are not supported whether
///   they are internal or external.
///
/// - Processing instructions and comments are parsed but they
///   do not generate any elements.
///
/// # Examples
///
/// Typical usage:
/// ```
/// use iks::{SaxElement, ParseError, SaxParser};
/// # fn main() -> Result<(), ParseError> {
///
/// let mut parser = SaxParser::new();
///
/// let mut elements = parser.elements(b"<doc>example</doc>");
/// while let Some(result) = elements.next() {
///     let element = result?;
///     println!("Element parsed: {:?}", element);
/// }
/// parser.parse_finish()?;
/// # Ok(())
/// # }
/// ```
///
/// Alternatively you can pass the input in multiple blocks:
/// ```
/// # use iks::SaxElement;
/// # use iks::ParseError;
/// # fn main() -> Result<(), ParseError> {
/// # use iks::SaxParser;
/// # let mut parser = SaxParser::new();
/// # use std::io::Read;
/// # let mut binding = vec!(b'<', b'a', b'/', b'>');
/// # let mut xml_file = binding.as_slice();
/// let mut buffer = [0u8; 1024];
/// loop {
///     let len = xml_file.read(&mut buffer).expect("io error");
///     if len == 0 {
///         break;
///     }
///     let mut elements = parser.elements(&buffer[0..len]);
///     while let Some(result) = elements.next() {
///         let element = result?;
///         println!("Element parsed: {:?}", element);
///     }
/// }
/// // This is to check if there is any incomplete XML construct at the end
/// parser.parse_finish()?;
/// # Ok(())
/// # }
/// ```
pub struct SaxParser {
    state: State,
    uni_len: u32,
    uni_left: u32,
    uni_char: u32,
    depth: usize,
    is_end_tag: bool,
    is_quot_value: bool,
    seen_content: bool,
    value_pos: usize,
    buffer: Vec<u8>,
    ref_buffer: Vec<u8>,
    char_ref_value: u32,
    char_ref_buffer: [u8; 4],
    is_value_ref: bool,
    location: Location,
}

#[derive(Eq, PartialEq)]
enum State {
    Prolog,
    TagStart,
    PI,
    PIEnd,
    Markup,
    CDataSectionC,
    CDataSectionCD,
    CDataSectionCDA,
    CDataSectionCDAT,
    CDataSectionCDATA,
    CDataSectionCDATAb,
    CDataSectionBody,
    CDataSectionMaybeEnd,
    CDataSectionMaybeEnd2,
    CommentStart,
    CommentBody,
    CommentMaybeEnd,
    CommentEnd,
    DoctypeDO,
    DoctypeDOC,
    DoctypeDOCT,
    DoctypeDOCTY,
    DoctypeDOCTYP,
    DoctypeDOCTYPE,
    DoctypeWhitespace,
    DoctypeSkip,
    DoctypeMarkupDecl,
    TagName,
    TagNameContinue,
    EndTagWhitespace,
    EmptyTagEnd,
    AttributeWhitespace,
    AttributeName,
    AttributeValueStart,
    AttributeValue,
    AttributeValueContinue,
    AttributeEq,
    CData,
    CDataContinue,
    Reference,
    CharReference,
    CharReferenceBody,
    HexCharReference,
    Entity,
    Epilog,
}

const INITIAL_BUFFER_CAPACITY: usize = 128;

const REF_BUFFER_SIZE: usize = 8;

macro_rules! whitespace {
    () => {
        b' ' | b'\t' | b'\r' | b'\n'
    };
}

fn is_valid_xml_char(c: u32) -> bool {
    matches!(c, 0x09 | 0x0a | 0x0d | 0x20..=0xd7ff | 0xe000..=0xfffd | 0x10000..=0x10_ffff)
}

macro_rules! xml_error {
    ($a:ident) => {
        return Err(ParseError::BadXml(description::$a));
    };
}

macro_rules! yield_element {
    ($self:ident, $c:ident, $pos:ident, $elem:expr) => {
        $self.location.advance($c);
        return Ok(Some(($elem, $pos + 1)));
    };
}

macro_rules! yield_element_inplace {
    ($pos:ident, $elem:expr) => {
        return Ok(Some(($elem, $pos)));
    };
}

impl SaxParser {
    /// Creates a new SAX parser instance.
    ///
    /// The instance can be reused for multiple documents with the [reset()](SaxParser::reset) method.
    pub fn new() -> SaxParser {
        SaxParser {
            state: State::Prolog,
            uni_len: 0,
            uni_left: 0,
            uni_char: 0,
            depth: 0,
            is_end_tag: false,
            is_quot_value: false,
            seen_content: false,
            value_pos: 0,
            buffer: Vec::<u8>::with_capacity(INITIAL_BUFFER_CAPACITY),
            ref_buffer: Vec::<u8>::with_capacity(REF_BUFFER_SIZE),
            char_ref_value: 0,
            char_ref_buffer: [0; 4],
            is_value_ref: false,
            location: Location::new(),
        }
    }

    /// Resets the parser into a clean state.
    pub fn reset(&mut self) {
        self.state = State::Prolog;
        self.uni_len = 0;
        self.uni_left = 0;
        self.uni_char = 0;
        self.depth = 0;
        self.is_end_tag = false;
        self.is_quot_value = false;
        self.seen_content = false;
        self.value_pos = 0;
        self.buffer.clear();
        self.ref_buffer.clear();
        self.char_ref_value = 0;
        self.is_value_ref = false;
        self.location = Location::new();
    }

    fn extend_buffer(&mut self, bytes: &[u8]) -> Result<(), ParseError> {
        let space = self.buffer.capacity() - self.buffer.len();
        if bytes.len() > space {
            let additional = bytes.len() - space;
            let result = self.buffer.try_reserve_exact(additional);
            if result.is_err() {
                return Err(ParseError::NoMemory);
            }
        }
        self.buffer.extend_from_slice(bytes);
        Ok(())
    }

    fn u32_to_cdata(&mut self) -> usize {
        const DATA_MASK: u32 = 0b0011_1111;
        const DATA_PREFIX: u8 = 0b1000_0000;

        let mut size = 0;
        let value = self.char_ref_value;
        let buf = &mut self.char_ref_buffer;
        match value {
            0..=0x7f => {
                buf[0] = value as u8;
                size = 1;
            }
            0x80..=0x7ff => {
                buf[0] = 0b1100_0000 | ((value >> 6) as u8);
                buf[1] = DATA_PREFIX | ((value & DATA_MASK) as u8);
                size = 2;
            }
            0x800..=0xffff => {
                buf[0] = 0b1110_0000 | ((value >> 12) as u8);
                buf[1] = DATA_PREFIX | (((value >> 6) & DATA_MASK) as u8);
                buf[2] = DATA_PREFIX | ((value & DATA_MASK) as u8);
                size = 3;
            }
            0x10000..=0x10_ffff => {
                buf[0] = 0b1111_0000 | ((value >> 18) as u8);
                buf[1] = DATA_PREFIX | (((value >> 12) & DATA_MASK) as u8);
                buf[2] = DATA_PREFIX | (((value >> 6) & DATA_MASK) as u8);
                buf[3] = DATA_PREFIX | ((value & DATA_MASK) as u8);
                size = 4;
            }
            _ => (),
        }
        size
    }

    /// Checks if the document is complete.
    ///
    /// A completed document should have a root tag and should not have any
    /// unfinished XML constructs, such as open comments or markup.
    pub fn parse_finish(&self) -> Result<(), ParseError> {
        if !self.seen_content {
            xml_error!(DOC_NO_CONTENT);
        }
        if self.depth > 0 {
            xml_error!(DOC_OPEN_TAGS);
        }
        if self.state != State::Epilog {
            xml_error!(DOC_OPEN_MARKUP);
        }
        Ok(())
    }

    pub fn elements<'a>(&'a mut self, bytes: &'a [u8]) -> SaxElements<'a> {
        SaxElements::new(self, bytes)
    }

    /// Parses given XML bytes.
    pub fn parse_bytes<'a>(
        &'a mut self,
        bytes: &'a [u8],
    ) -> Result<Option<(SaxElement<'a>, usize)>, ParseError> {
        let mut pos: usize = 0;
        let mut back: usize = 0;

        while pos < bytes.len() {
            let mut redo: bool = false;
            let c = bytes[pos];

            if self.uni_left > 0 {
                if c & 0xc0 != 0x80 {
                    xml_error!(UTF8_INVALID_CONT_BYTE);
                }
                self.uni_char <<= 6;
                self.uni_char += c as u32 & 0x3f;
                self.uni_left -= 1;
                if self.uni_left == 0 {
                    // Sequences longer than the actual character codepoint
                    // size are security hazards.
                    if (self.uni_len == 2 && self.uni_char <= 0x7f)
                        || (self.uni_len == 3 && self.uni_char <= 0x7ff)
                        || (self.uni_len == 4 && self.uni_char <= 0xffff)
                    {
                        xml_error!(UTF8_OVERLONG_SEQUENCE);
                    }
                    if !is_valid_xml_char(self.uni_char) {
                        xml_error!(CHAR_INVALID);
                    }
                }
            } else if c & 0x80 == 0x80 {
                if c & 0x60 == 0x40 {
                    self.uni_len = 2;
                    self.uni_left = 1;
                    self.uni_char = c as u32 & 0x1f;
                } else if c & 0x70 == 0x60 {
                    self.uni_len = 3;
                    self.uni_left = 2;
                    self.uni_char = c as u32 & 0x0f;
                } else if c & 0x78 == 0x70 {
                    self.uni_len = 4;
                    self.uni_left = 3;
                    self.uni_char = c as u32 & 0x07;
                } else {
                    xml_error!(UTF8_INVALID_PREFIX_BYTE);
                }
            } else if c < 0x20 && (c != 0x09 && c != 0x0a && c != 0x0d) {
                xml_error!(CHAR_INVALID);
            }

            match self.state {
                State::Prolog => match c {
                    b'<' => self.state = State::TagStart,
                    whitespace!() => (),
                    _ => {
                        xml_error!(DOC_CDATA_WITHOUT_PARENT);
                    }
                },

                State::TagStart => match c {
                    b'!' => {
                        self.state = State::Markup;
                    }
                    b'?' => self.state = State::PI,
                    b'/' => {
                        if self.depth == 0 {
                            xml_error!(TAG_CLOSE_WITHOUT_OPEN);
                        }
                        back = pos + 1;
                        self.is_end_tag = true;
                        self.state = State::TagName;
                    }
                    whitespace!() => {
                        xml_error!(TAG_WHITESPACE_START);
                    }
                    b'>' => {
                        xml_error!(TAG_EMPTY_NAME);
                    }
                    _ => {
                        if self.depth == 0 && self.seen_content {
                            xml_error!(TAG_OUTSIDE_ROOT);
                        }
                        self.depth += 1;
                        back = pos;
                        self.is_end_tag = false;
                        self.seen_content = true;
                        self.state = State::TagName;
                    }
                },

                State::Markup => match c {
                    b'-' => self.state = State::CommentStart,
                    b'[' => {
                        if self.depth == 0 {
                            xml_error!(MARKUP_CDATA_SECTION_OUTSIDE_ROOT);
                        }
                        self.state = State::CDataSectionC;
                    }
                    b'D' => self.state = State::DoctypeDO,
                    _ => {
                        xml_error!(MARKUP_UNRECOGNIZED);
                    }
                },

                State::DoctypeDO => match c {
                    b'O' => self.state = State::DoctypeDOC,
                    _ => {
                        xml_error!(MARKUP_DOCTYPE_BAD_START);
                    }
                },

                State::DoctypeDOC => match c {
                    b'C' => self.state = State::DoctypeDOCT,
                    _ => {
                        xml_error!(MARKUP_DOCTYPE_BAD_START);
                    }
                },

                State::DoctypeDOCT => match c {
                    b'T' => self.state = State::DoctypeDOCTY,
                    _ => {
                        xml_error!(MARKUP_DOCTYPE_BAD_START);
                    }
                },

                State::DoctypeDOCTY => match c {
                    b'Y' => self.state = State::DoctypeDOCTYP,
                    _ => {
                        xml_error!(MARKUP_DOCTYPE_BAD_START);
                    }
                },

                State::DoctypeDOCTYP => match c {
                    b'P' => self.state = State::DoctypeDOCTYPE,
                    _ => {
                        xml_error!(MARKUP_DOCTYPE_BAD_START);
                    }
                },

                State::DoctypeDOCTYPE => match c {
                    b'E' => self.state = State::DoctypeWhitespace,
                    _ => {
                        xml_error!(MARKUP_DOCTYPE_BAD_START);
                    }
                },

                State::DoctypeWhitespace => match c {
                    whitespace!() => self.state = State::DoctypeSkip,
                    _ => {
                        xml_error!(MARKUP_DOCTYPE_BAD_START);
                    }
                },

                State::DoctypeSkip => match c {
                    b'<' => self.state = State::DoctypeMarkupDecl,
                    b'>' => self.state = State::Prolog,
                    _ => (),
                },

                State::DoctypeMarkupDecl => {
                    if c == b'>' {
                        self.state = State::DoctypeSkip;
                    }
                }

                State::CDataSectionC => {
                    if c != b'C' {
                        xml_error!(MARKUP_CDATA_SECTION_BAD_START);
                    }
                    self.state = State::CDataSectionCD;
                }

                State::CDataSectionCD => {
                    if c != b'D' {
                        xml_error!(MARKUP_CDATA_SECTION_BAD_START);
                    }
                    self.state = State::CDataSectionCDA;
                }

                State::CDataSectionCDA => {
                    if c != b'A' {
                        xml_error!(MARKUP_CDATA_SECTION_BAD_START);
                    }
                    self.state = State::CDataSectionCDAT;
                }

                State::CDataSectionCDAT => {
                    if c != b'T' {
                        xml_error!(MARKUP_CDATA_SECTION_BAD_START);
                    }
                    self.state = State::CDataSectionCDATA;
                }

                State::CDataSectionCDATA => {
                    if c != b'A' {
                        xml_error!(MARKUP_CDATA_SECTION_BAD_START);
                    }
                    self.state = State::CDataSectionCDATAb;
                }

                State::CDataSectionCDATAb => {
                    if c != b'[' {
                        xml_error!(MARKUP_CDATA_SECTION_BAD_START);
                    }
                    back = pos + 1;
                    self.state = State::CDataSectionBody;
                }

                State::CDataSectionBody => {
                    if c == b']' {
                        self.state = State::CDataSectionMaybeEnd;
                        if back < pos {
                            let s = unsafe { std::str::from_utf8_unchecked(&bytes[back..pos]) };
                            yield_element!(self, c, pos, SaxElement::CData(s));
                        }
                    }
                }

                State::CDataSectionMaybeEnd => match c {
                    b']' => self.state = State::CDataSectionMaybeEnd2,
                    _ => {
                        self.state = State::CDataSectionBody;
                        yield_element_inplace!(pos, SaxElement::CData("]"));
                    }
                },

                State::CDataSectionMaybeEnd2 => match c {
                    b'>' => {
                        back = pos + 1;
                        self.state = State::CData;
                    }
                    b']' => {
                        yield_element!(self, c, pos, SaxElement::CData("]"));
                    }
                    _ => {
                        self.state = State::CDataSectionBody;
                        yield_element_inplace!(pos, SaxElement::CData("]]"));
                    }
                },

                State::CommentStart => {
                    if c != b'-' {
                        xml_error!(COMMENT_MISSING_DASH);
                    }
                    self.state = State::CommentBody;
                }

                State::CommentBody => {
                    if c == b'-' {
                        self.state = State::CommentMaybeEnd;
                    }
                }

                State::CommentMaybeEnd => match c {
                    b'-' => self.state = State::CommentEnd,
                    _ => self.state = State::CommentBody,
                },

                State::CommentEnd => {
                    if c != b'>' {
                        xml_error!(COMMENT_MISSING_END);
                    }
                    if self.depth > 0 {
                        back = pos + 1;
                        self.state = State::CData;
                    } else if self.seen_content {
                        self.state = State::Epilog;
                    } else {
                        self.state = State::Prolog;
                    }
                }

                State::PI => {
                    if c == b'?' {
                        self.state = State::PIEnd;
                    }
                }

                State::PIEnd => match c {
                    b'>' => {
                        if self.seen_content {
                            if self.depth > 0 {
                                back = pos + 1;
                                self.state = State::CData;
                            } else {
                                self.state = State::Epilog;
                            }
                        } else {
                            self.state = State::Prolog;
                        }
                    }
                    _ => {
                        xml_error!(PI_MISSING_END);
                    }
                },

                State::TagName => match c {
                    b'/' | b'>' | whitespace!() => {
                        if back < pos {
                            self.extend_buffer(&bytes[back..pos])?;
                        }
                        {
                            if self.buffer.is_empty() {
                                xml_error!(TAG_EMPTY_NAME);
                            }
                            let s = unsafe { std::str::from_utf8_unchecked(&self.buffer) };
                            self.state = State::TagNameContinue;
                            if self.is_end_tag {
                                if c == b'/' {
                                    xml_error!(TAG_DOUBLE_END);
                                }
                                yield_element_inplace!(pos, SaxElement::EndTag(s));
                            } else {
                                yield_element_inplace!(pos, SaxElement::StartTag(s));
                            }
                        }
                    }
                    _ => (),
                },

                State::TagNameContinue => {
                    self.buffer.clear();
                    match c {
                        b'/' => {
                            self.state = State::EmptyTagEnd;
                            yield_element!(self, c, pos, SaxElement::StartTagEmpty);
                        }
                        b'>' => {
                            if self.is_end_tag {
                                if self.depth == 0 {
                                    xml_error!(TAG_CLOSE_WITHOUT_OPEN);
                                }
                                self.depth -= 1;
                                if self.depth == 0 {
                                    self.state = State::Epilog;
                                } else {
                                    back = pos + 1;
                                    self.state = State::CData;
                                }
                            } else {
                                self.state = State::CData;
                                yield_element!(self, c, pos, SaxElement::StartTagContent);
                            }
                        }
                        whitespace!() => {
                            if self.is_end_tag {
                                self.state = State::EndTagWhitespace;
                            } else {
                                self.state = State::AttributeWhitespace;
                            }
                        }
                        _ => unreachable!(),
                    }
                }

                State::EmptyTagEnd => match c {
                    b'>' => {
                        if self.depth == 0 {
                            xml_error!(TAG_CLOSE_WITHOUT_OPEN);
                        }
                        self.depth -= 1;
                        if self.depth == 0 {
                            self.state = State::Epilog;
                        } else {
                            back = pos + 1;
                            self.state = State::CData;
                        }
                    }
                    _ => {
                        xml_error!(TAG_EMPTY_TAG_MISSING_END);
                    }
                },

                State::EndTagWhitespace => match c {
                    b'>' => {
                        if self.depth == 0 {
                            xml_error!(TAG_CLOSE_WITHOUT_OPEN);
                        }
                        self.depth -= 1;
                        if self.depth == 0 {
                            self.state = State::Epilog;
                        } else {
                            back = pos + 1;
                            self.state = State::CData;
                        }
                    }
                    whitespace!() => (),
                    _ => {
                        xml_error!(TAG_END_TAG_ATTRIBUTES);
                    }
                },

                State::AttributeWhitespace => match c {
                    whitespace!() => (),
                    b'/' => {
                        if self.is_end_tag {
                            xml_error!(TAG_DOUBLE_END);
                        }
                        self.state = State::EmptyTagEnd;
                        yield_element!(self, c, pos, SaxElement::StartTagEmpty);
                    }
                    b'>' => {
                        self.state = State::CData;
                        yield_element!(self, c, pos, SaxElement::StartTagContent);
                    }
                    _ => {
                        back = pos;
                        self.state = State::AttributeName;
                        redo = true;
                    }
                },

                State::AttributeName => match c {
                    b'=' | whitespace!() => {
                        if back < pos {
                            self.extend_buffer(&bytes[back..pos])?;
                        }
                        if c == b'=' {
                            self.state = State::AttributeValueStart;
                        } else {
                            self.state = State::AttributeEq;
                        }
                    }
                    b'/' | b'>' | b'<' => {
                        xml_error!(TAG_ATTRIBUTE_BAD_NAME);
                    }
                    _ => (),
                },

                State::AttributeEq => match c {
                    b'=' => self.state = State::AttributeValueStart,
                    whitespace!() => (),
                    _ => {
                        xml_error!(TAG_ATTRIBUTE_WITHOUT_EQUAL);
                    }
                },

                State::AttributeValueStart => match c {
                    b'"' => {
                        self.is_quot_value = false;
                        self.value_pos = self.buffer.len();
                        back = pos + 1;
                        self.state = State::AttributeValue;
                    }
                    b'\'' => {
                        self.is_quot_value = true;
                        self.value_pos = self.buffer.len();
                        back = pos + 1;
                        self.state = State::AttributeValue;
                    }
                    whitespace!() => (),
                    _ => {
                        xml_error!(TAG_ATTRIBUTE_WITHOUT_QUOTE);
                    }
                },

                State::AttributeValue => {
                    if (self.is_quot_value && c == b'\'') || (!self.is_quot_value && c == b'"') {
                        if back < pos {
                            self.extend_buffer(&bytes[back..pos])?;
                        }
                        let attr = unsafe {
                            std::str::from_utf8_unchecked(&self.buffer[0..self.value_pos])
                        };
                        let value = unsafe {
                            std::str::from_utf8_unchecked(&self.buffer[self.value_pos..])
                        };
                        self.state = State::AttributeValueContinue;
                        yield_element_inplace!(pos, SaxElement::Attribute(attr, value));
                    } else if c == b'&' {
                        if back < pos {
                            self.extend_buffer(&bytes[back..pos])?;
                        }
                        self.ref_buffer.clear();
                        self.is_value_ref = true;
                        self.state = State::Reference;
                    } else if c == b'<' {
                        xml_error!(TAG_ATTRIBUTE_BAD_VALUE);
                    }
                }

                State::AttributeValueContinue => {
                    self.buffer.clear();
                    self.state = State::AttributeWhitespace;
                }

                State::CData => match c {
                    b'<' => {
                        if back < pos {
                            let s = unsafe { std::str::from_utf8_unchecked(&bytes[back..pos]) };
                            self.state = State::TagStart;
                            yield_element!(self, c, pos, SaxElement::CData(s));
                        }
                        back = pos + 1;
                        self.state = State::TagStart;
                    }
                    b'&' => {
                        if back < pos {
                            let s = unsafe { std::str::from_utf8_unchecked(&bytes[back..pos]) };
                            self.state = State::CDataContinue;
                            yield_element_inplace!(pos, SaxElement::CData(s));
                        }
                        self.ref_buffer.clear();
                        self.is_value_ref = false;
                        self.state = State::Reference;
                    }
                    _ => (),
                },

                State::CDataContinue => {
                    self.ref_buffer.clear();
                    self.is_value_ref = false;
                    self.state = State::Reference;
                }

                State::Reference => match c {
                    b'#' => {
                        self.char_ref_value = 0;
                        self.state = State::CharReference;
                    }
                    _ => {
                        self.ref_buffer.push(c);
                        self.state = State::Entity;
                    }
                },

                State::Entity => match c {
                    b';' => {
                        let ent = match self.ref_buffer.as_slice() {
                            b"amp" => "&",
                            b"lt" => "<",
                            b"gt" => ">",
                            b"quot" => "\"",
                            b"apos" => "'",
                            _ => {
                                xml_error!(REFERENCE_CUSTOM_ENTITY);
                            }
                        };
                        if self.is_value_ref {
                            self.extend_buffer(ent.as_bytes())?;
                            back = pos + 1;
                            self.state = State::AttributeValue;
                        } else {
                            self.state = State::CData;
                            yield_element!(self, c, pos, SaxElement::CData(ent));
                        }
                    }
                    _ => {
                        if self.ref_buffer.len() >= REF_BUFFER_SIZE {
                            xml_error!(REFERENCE_CUSTOM_ENTITY);
                        }
                        self.ref_buffer.push(c);
                    }
                },

                State::CharReference => match c {
                    b'x' => self.state = State::HexCharReference,
                    _ => {
                        let digit: u32 = (c - b'0').into();
                        self.char_ref_value = digit;
                        self.state = State::CharReferenceBody;
                    }
                },

                State::CharReferenceBody => match c {
                    b';' => {
                        if !is_valid_xml_char(self.char_ref_value) {
                            xml_error!(CHAR_INVALID);
                        }
                        if self.is_value_ref {
                            let size = self.u32_to_cdata();
                            let mut buf = [0u8; 4];
                            buf.clone_from_slice(&self.char_ref_buffer);
                            self.extend_buffer(&buf[0..size])?;
                            back = pos + 1;
                            self.state = State::AttributeValue;
                        } else {
                            let size = self.u32_to_cdata();
                            let s = unsafe {
                                std::str::from_utf8_unchecked(&self.char_ref_buffer[0..size])
                            };
                            self.state = State::CData;
                            yield_element!(self, c, pos, SaxElement::CData(s));
                        }
                    }
                    b'0'..=b'9' => {
                        let digit: u32 = (c - b'0').into();
                        self.char_ref_value = (self.char_ref_value * 10) + digit;
                    }
                    _ => {
                        xml_error!(REFERENCE_INVALID_DECIMAL);
                    }
                },

                State::HexCharReference => match c {
                    b';' => {
                        if !is_valid_xml_char(self.char_ref_value) {
                            xml_error!(CHAR_INVALID);
                        }
                        if self.is_value_ref {
                            let size = self.u32_to_cdata();
                            let mut buf = [0u8; 4];
                            buf.clone_from_slice(&self.char_ref_buffer);
                            self.extend_buffer(&buf[0..size])?;
                            back = pos + 1;
                            self.state = State::AttributeValue;
                        } else {
                            let size = self.u32_to_cdata();
                            let s = unsafe {
                                std::str::from_utf8_unchecked(&self.char_ref_buffer[0..size])
                            };
                            self.state = State::CData;
                            yield_element!(self, c, pos, SaxElement::CData(s));
                        }
                    }
                    b'0'..=b'9' => {
                        let digit: u32 = (c - b'0').into();
                        self.char_ref_value = (self.char_ref_value * 16) + digit;
                    }
                    b'a'..=b'f' => {
                        let digit: u32 = (c - b'a').into();
                        self.char_ref_value = (self.char_ref_value * 16) + digit + 10;
                    }
                    b'A'..=b'F' => {
                        let digit: u32 = (c - b'A').into();
                        self.char_ref_value = (self.char_ref_value * 16) + digit + 10;
                    }
                    _ => {
                        xml_error!(REFERENCE_INVALID_HEX);
                    }
                },

                State::Epilog => match c {
                    b'<' => self.state = State::TagStart,
                    whitespace!() => (),
                    _ => {
                        xml_error!(DOC_CDATA_WITHOUT_PARENT);
                    }
                },
            }

            if !redo {
                pos += 1;
                self.location.advance(c);
            }
        }

        if back < pos {
            match self.state {
                State::TagName | State::AttributeName | State::AttributeValue => {
                    self.extend_buffer(&bytes[back..pos])?;
                }
                State::CData | State::CDataSectionBody => {
                    let s = unsafe { std::str::from_utf8_unchecked(&bytes[back..pos]) };
                    yield_element_inplace!(pos, SaxElement::CData(s));
                }
                _ => (),
            }
        }

        Ok(None)
    }

    pub fn location(&self) -> Location {
        self.location
    }
}

impl Default for SaxParser {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests;
