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

use crate::ParseError;

#[derive(Debug, Eq, PartialEq, Copy, Clone)]
pub enum StreamError {
    NoMemory,
    BadXml(&'static str),
    BadStream(&'static str),
}

impl Display for StreamError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StreamError::NoMemory => write!(f, "not enough memory"),
            StreamError::BadXml(msg) => write!(f, "invalid XML syntax: {msg}"),
            StreamError::BadStream(msg) => write!(f, "invalid stream protocol: {msg}"),
        }
    }
}

impl Error for StreamError {}

impl From<ParseError> for StreamError {
    fn from(err: ParseError) -> Self {
        match err {
            ParseError::NoMemory => StreamError::NoMemory,
            ParseError::BadXml(msg) => StreamError::BadXml(msg),
        }
    }
}
