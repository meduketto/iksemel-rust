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

use super::stream::StreamError;

#[derive(Debug)]
pub enum XmppClientError {
    NoMemory,
    BadXml(&'static str),
    BadStream(&'static str),
    IOError(std::io::Error),
    TlsError(rustls::Error),
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

impl Error for XmppClientError {}

impl From<StreamError> for XmppClientError {
    fn from(err: StreamError) -> Self {
        match err {
            StreamError::NoMemory => XmppClientError::NoMemory,
            StreamError::BadXml(msg) => XmppClientError::BadXml(msg),
            StreamError::BadStream(msg) => XmppClientError::BadStream(msg),
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
        XmppClientError::TlsError(err)
    }
}

impl From<rustls::pki_types::InvalidDnsNameError> for XmppClientError {
    fn from(_err: rustls::pki_types::InvalidDnsNameError) -> Self {
        XmppClientError::BadStream("Invalid dns name")
    }
}
