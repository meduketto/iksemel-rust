/*
** This file is a part of Iksemel (XML parser for Jabber/XMPP)
** Copyright (C) 2000-2026 Gurer Ozen
**
** Iksemel is free software: you can redistribute it and/or modify it
** under the terms of the GNU Lesser General Public License as
** published by the Free Software Foundation, either version 3 of
** the License, or (at your option) any later version.
*/

use std::error::Error;
use std::fmt::Display;

use crate::ParseError;

use super::stream::StreamError;

#[derive(Debug)]
pub enum XmppClientError {
    NoMemory,
    BadXml(&'static str),
    BadStream(&'static str),
    IOError(std::io::Error),
    TlsError(Box<dyn std::error::Error + Send + Sync>),
}

impl Display for XmppClientError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            XmppClientError::NoMemory => write!(f, "not enough memory"),
            XmppClientError::BadXml(msg) => write!(f, "invalid XML syntax: {msg}"),
            XmppClientError::BadStream(msg) => write!(f, "invalid stream protocol: {msg}"),
            XmppClientError::IOError(err) => err.fmt(f),
            XmppClientError::TlsError(err) => err.fmt(f),
        }
    }
}

impl Error for XmppClientError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::TlsError(e) => Some(e.as_ref()),
            Self::IOError(e) => Some(e),
            _ => None,
        }
    }
}

impl From<StreamError> for XmppClientError {
    fn from(err: StreamError) -> Self {
        match err {
            StreamError::NoMemory => XmppClientError::NoMemory,
            StreamError::BadXml(msg) => XmppClientError::BadXml(msg),
            StreamError::BadStream(msg) => XmppClientError::BadStream(msg),
        }
    }
}

impl From<ParseError> for XmppClientError {
    fn from(err: ParseError) -> Self {
        match err {
            ParseError::NoMemory => XmppClientError::NoMemory,
            ParseError::BadXml(msg) => XmppClientError::BadXml(msg),
        }
    }
}

impl From<std::io::Error> for XmppClientError {
    fn from(err: std::io::Error) -> Self {
        XmppClientError::IOError(err)
    }
}

impl From<rustls::Error> for XmppClientError {
    fn from(err: rustls::Error) -> Self {
        XmppClientError::TlsError(Box::new(err))
    }
}

impl From<rustls::pki_types::InvalidDnsNameError> for XmppClientError {
    fn from(_err: rustls::pki_types::InvalidDnsNameError) -> Self {
        XmppClientError::BadStream("Invalid dns name")
    }
}
