/*
** This file is a part of Iksemel (XML parser for Jabber/XMPP)
** Copyright (C) 2000-2025 Gurer Ozen
**
** Iksemel is free software: you can redistribute it and/or modify it
** under the terms of the GNU Lesser General Public License as
** published by the Free Software Foundation, either version 3 of
** the License, or (at your option) any later version.
*/

use crate::Document;
use crate::Jid;
use crate::StreamElement;
use crate::StreamError;
use crate::StreamParser;

use super::constants::FEATURES_TAG;
use super::constants::STREAM_TAG;

pub enum XmppClientProtocolEvent {
    Send(Vec<u8>),
    StartTls,
    Stanza(Document),
    End,
}

pub struct XmppClientProtocolEvents<'a> {
    protocol: &'a mut XmppClientProtocol,
    bytes: &'a [u8],
    bytes_parsed: usize,
}

impl<'a> XmppClientProtocolEvents<'a> {
    pub fn new(protocol: &'a mut XmppClientProtocol, bytes: &'a [u8]) -> Self {
        Self {
            protocol,
            bytes,
            bytes_parsed: 0,
        }
    }

    pub fn next(&mut self) -> Option<Result<XmppClientProtocolEvent, StreamError>> {
        match self
            .protocol
            .receive_bytes(&self.bytes[self.bytes_parsed..])
        {
            Ok(Some((element, bytes))) => {
                self.bytes_parsed += bytes;
                Some(Ok(element))
            }
            Ok(None) => {
                self.bytes_parsed = self.bytes.len();
                None
            }
            Err(err) => Some(Err(err)),
        }
    }
}

enum StreamState {
    Connected,
    StartSent,
    //    StartReceived,
    //    FeaturesReceived,
    //    Established,
    //    Error,
}

pub struct XmppClientProtocol {
    jid: Jid,
    stream_parser: StreamParser,
    state: StreamState,
}

impl XmppClientProtocol {
    pub fn new(jid: Jid) -> Self {
        XmppClientProtocol {
            jid,
            stream_parser: StreamParser::new(),
            state: StreamState::Connected,
        }
    }

    pub fn events<'a>(&'a mut self, bytes: &'a [u8]) -> XmppClientProtocolEvents<'a> {
        XmppClientProtocolEvents::new(self, bytes)
    }

    pub fn receive_element(&mut self, element: Document) {
        match element.root().name() {
            STREAM_TAG => {
                // Handle stream received
            }
            FEATURES_TAG => {
                // Handle features received
            }
            _ => {}
        }
    }

    pub fn send_bytes(&mut self) -> Option<Vec<u8>> {
        match self.state {
            StreamState::Connected => {
                let mut bytes = Vec::new();
                bytes.extend_from_slice(b"<?xml version='1.0'?>");
                bytes.extend_from_slice(b"<stream:stream xmlns='jabber:client' xmlns:stream='http://etherx.jabber.org/streams' version='1.0' xmllang='en' from='");
                bytes.extend_from_slice(self.jid.full().as_bytes());
                bytes.extend_from_slice(b"' to='");
                bytes.extend_from_slice(self.jid.domainpart().as_bytes());
                bytes.extend_from_slice(b"'>");
                self.state = StreamState::StartSent;
                Some(bytes)
            }
            _ => None,
        }
    }

    pub fn receive_bytes(
        &mut self,
        bytes: &[u8],
    ) -> Result<Option<(XmppClientProtocolEvent, usize)>, StreamError> {
        match self.stream_parser.parse_bytes(bytes) {
            Ok(Some((element, bytes))) => {
                let result = match element {
                    StreamElement::Element(doc) => XmppClientProtocolEvent::Stanza(doc),
                    StreamElement::End => XmppClientProtocolEvent::End,
                };
                Ok(Some((result, bytes)))
            }
            Ok(None) => {
                Ok(None)
            }
            Err(err) => Err(err.into()),
        }
    }
}
