/*
** This file is a part of Iksemel (XML parser for Jabber/XMPP)
** Copyright (C) 2000-2025 Gurer Ozen
**
** Iksemel is free software: you can redistribute it and/or modify it
** under the terms of the GNU Lesser General Public License as
** published by the Free Software Foundation, either version 3 of
** the License, or (at your option) any later version.
*/

use std::error::Error;
use std::fmt::Display;

#[derive(Debug, Eq, PartialEq)]
pub enum DocumentError {
    NoMemory,
    BadXml(&'static str),
}

impl Display for DocumentError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "document error")
    }
}

impl Error for DocumentError {}

#[derive(Debug, Eq, PartialEq)]
pub enum FileDocumentError {
    NoMemory,
    BadXml(&'static str),
    IOError,
}

impl Display for FileDocumentError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "document error")
    }
}

impl Error for FileDocumentError {}

pub(super) mod description {
    pub(in super::super) const TAG_MISMATCH: &str = "Start and end tags have different names";
    pub(in super::super) const DUPLICATE_ATTRIBUTE: &str =
        "Attribute name already used in this tag";
}
