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

use crate::NoMemory;
use crate::SaxError;

#[derive(Debug, Eq, PartialEq, Copy, Clone)]
pub enum DocumentError {
    NoMemory,
    BadXml(&'static str),
}

impl Display for DocumentError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DocumentError::NoMemory => write!(f, "not enough memory"),
            DocumentError::BadXml(msg) => write!(f, "invalid XML syntax: {}", msg),
        }
    }
}

impl Error for DocumentError {}

impl From<SaxError> for DocumentError {
    fn from(err: SaxError) -> Self {
        match err {
            SaxError::NoMemory => DocumentError::NoMemory,
            SaxError::BadXml(msg) => DocumentError::BadXml(msg),
            SaxError::HandlerAbort => DocumentError::BadXml(description::UNEXPECTED_HANDLER_ABORT),
        }
    }
}

impl From<DocumentError> for SaxError {
    fn from(err: DocumentError) -> Self {
        match err {
            DocumentError::NoMemory => SaxError::NoMemory,
            DocumentError::BadXml(msg) => SaxError::BadXml(msg),
        }
    }
}

impl From<NoMemory> for DocumentError {
    fn from(_: NoMemory) -> Self {
        DocumentError::NoMemory
    }
}

pub(super) mod description {
    pub(super) const UNEXPECTED_HANDLER_ABORT: &str = "unexpected handler abort";
    pub(in super::super) const NO_DOCUMENT: &str = "no document parsed yet";
    pub(in super::super) const TAG_MISMATCH: &str = "start and end tags have different names";
    pub(in super::super) const DUPLICATE_ATTRIBUTE: &str =
        "attribute name already used in this tag";
    pub(in super::super) const CDATA_ATTRIBUTE: &str = "attributes cannot be set on CDATA elements";
    pub(in super::super) const CDATA_CHILDREN: &str =
        "child elements cannot be added on CDATA elements";
    pub(in super::super) const NULL_CURSOR_EDIT: &str = "null cursor cannot edit the document";
    pub(in super::super) const REMOVED_EDIT: &str = "cannot edit a removed element";
    pub(in super::super) const ROOT_SIBLING: &str = "root element cannot have siblings";
}
