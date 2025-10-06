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

#[derive(Debug, Eq, PartialEq, Copy, Clone)]
pub struct BadJid(pub &'static str);

impl Display for BadJid {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "invalid JabberID: {}", self.0)
    }
}

impl Error for BadJid {}

pub(super) mod description {
    pub(in super::super) const DOMAIN_EMPTY: &str = "domainpart is empty";
    pub(in super::super) const DOMAIN_TOO_LONG: &str = "domainpart is longer than 1023 octets";
    pub(in super::super) const LOCAL_EMPTY: &str = "localpart is empty";
    pub(in super::super) const LOCAL_TOO_LONG: &str = "localpart is longer than 1023 octets";
    pub(in super::super) const RESOURCE_EMPTY: &str = "resourcepart is empty";
    pub(in super::super) const RESOURCE_TOO_LONG: &str = "resourcepart is longer than 1023 octets";
}
