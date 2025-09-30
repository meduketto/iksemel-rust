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

#[derive(Debug)]
pub enum XmppClientError {
    StreamError(StreamError),
    IOError(std::io::Error),
}

impl Display for XmppClientError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            XmppClientError::StreamError(err) => err.fmt(f),
            XmppClientError::IOError(err) => err.fmt(f),
        }
    }
}

impl Error for XmppClientError {}

impl From<StreamError> for XmppClientError {
    fn from(err: StreamError) -> Self {
        XmppClientError::StreamError(err)
    }
}

impl From<std::io::Error> for XmppClientError {
    fn from(err: std::io::Error) -> Self {
        XmppClientError::IOError(err)
    }
}

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
