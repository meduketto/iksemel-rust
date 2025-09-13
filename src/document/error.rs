/*
** This file is a part of Iksemel (XML parser for Jabber/XMPP)
** Copyright (C) 2000-2025 Gurer Ozen
**
** Iksemel is free software: you can redistribute it and/or modify it
** under the terms of the GNU Lesser General Public License as
** published by the Free Software Foundation, either version 3 of
** the License, or (at your option) any later version.
*/

use crate::{NoMemory, ParseError};

impl From<NoMemory> for ParseError {
    fn from(_: NoMemory) -> Self {
        ParseError::NoMemory
    }
}

pub(super) mod description {
    pub(in super::super) const NO_DOCUMENT: &str = "no document parsed yet";
    pub(in super::super) const NO_START_TAG: &str = "document must start with a StartTag element";
    pub(in super::super) const TAG_MISMATCH: &str = "start and end tags have different names";
    pub(in super::super) const DUPLICATE_ATTRIBUTE: &str =
        "attribute name already used in this tag";
    pub(in super::super) const CDATA_ATTRIBUTE: &str = "attributes cannot be set on CDATA elements";
    pub(in super::super) const CDATA_CHILDREN: &str =
        "child elements cannot be added on CDATA elements";
    pub(in super::super) const NULL_CURSOR_EDIT: &str = "null cursor cannot edit the document";
    pub(in super::super) const ROOT_SIBLING: &str = "root element cannot have siblings";
}
