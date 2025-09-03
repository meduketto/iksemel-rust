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
/// Returned Cursor cannot outlive the Document:
/// ```compile_fail
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// use iksemel::Document;
/// use iksemel::Cursor;
/// let c: Cursor;
/// {
///     let doc = Document::from_str("<a><b/></a>")?;
///     c = doc.root();
/// }
/// println!("{}", c);
/// # Ok(())
/// # }
/// ```
///
/// Cursor clone cannot outlive the Document:
/// ```compile_fail
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// use iksemel::Document;
/// use iksemel::Cursor;
/// let c2: Cursor;
/// {
///     let doc = Document::from_str("<a><b/></a>")?;
///     let c1 = doc.root();
///     c2 = c1.clone();
/// }
/// println!("{}", c2);
/// # Ok(())
/// # }
/// ```
///
/// Returned Cursor cannot outlive the Document:
/// ```compile_fail
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// use iksemel::Document;
/// use iksemel::Cursor;
/// let c2: Cursor;
/// {
///     let doc = Document::from_str("<a><b/></a>")?;
///     let c1 = doc.root().find_tag("b");
///     c2 = c1;
/// }
/// println!("{}", c2);
/// # Ok(())
/// # }
/// ```
///
#[cfg(doctest)]
struct MustNotCompileTests;
