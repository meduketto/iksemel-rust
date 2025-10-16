/*
** This file is a part of Iksemel (XML parser for Jabber/XMPP)
** Copyright (C) 2025 Gurer Ozen
**
** Iksemel is free software: you can redistribute it and/or modify it
** under the terms of the GNU Lesser General Public License as
** published by the Free Software Foundation, either version 3 of
** the License, or (at your option) any later version.
*/

use std::error::Error;
use std::fmt::Display;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BadXPath;

impl Display for BadXPath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "XPath syntax error")
    }
}

impl Error for BadXPath {}
