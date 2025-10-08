/*
** This file is a part of Iksemel (XML parser for Jabber/XMPP)
** Copyright (C) 2000-2025 Gurer Ozen
**
** Iksemel is free software: you can redistribute it and/or modify it
** under the terms of the GNU Lesser General Public License as
** published by the Free Software Foundation, either version 3 of
** the License, or (at your option) any later version.
*/

/// # Must not compile tests
///
/// Returned element cannot outlive the bytes buffer:
/// ```compile_fail
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// use iks::SaxParser;
/// use iks::SaxElement;
/// let mut parser = SaxParser::new();
/// let element: SaxElement;
/// {
///     let bytes = b"<root/>".clone();
///     let size: usize;
///     (element, size) = parser.parse_bytes(&bytes).unwrap().unwrap();
/// }
/// println!("{:?}", element);
/// # Ok(())
/// # }
/// ```
///
/// Returned element cannot outlive the parser:
/// ```compile_fail
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// use iks::SaxParser;
/// use iks::SaxElement;
/// let bytes = b"<root/>";
/// let element: SaxElement;
/// {
///     let mut parser = SaxParser::new();
///     let size: usize;
///     (element, size) = parser.parse_bytes(bytes).unwrap().unwrap();
/// }
/// println!("{:?}", element);
/// # Ok(())
/// # }
/// ```
///
/// Returned element cannot outlive the iterator:
/// ```compile_fail
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// use iks::SaxParser;
/// use iks::SaxElement;
/// let bytes = b"<root/>";
/// let mut parser = SaxParser::new();
/// let element: SaxElement;
/// {
///     let mut elements = parser.elements(bytes);
///     element = elements.next().unwrap()?;
/// }
/// println!("{:?}", element);
/// # Ok(())
/// # }
/// ```
///
#[cfg(doctest)]
struct MustNotCompileTests;
