/*
** This file is a part of Iksemel (XML parser for Jabber/XMPP)
** Copyright (C) 2000-2025 Gurer Ozen
**
** Iksemel is free software: you can redistribute it and/or modify it
** under the terms of the GNU Lesser General Public License as
** published by the Free Software Foundation, either version 3 of
** the License, or (at your option) any later version.
*/

use base64::Engine;
use base64::engine::general_purpose::STANDARD;

use crate::Document;
use crate::Jid;
use crate::StreamElement;
use crate::StreamError;
use crate::StreamParser;
use crate::xmpp::constants::PROCEED_TAG;
use crate::xmpp::constants::SUCCESS_TAG;

use super::constants::FEATURES_TAG;
use super::constants::IQ_TAG;
use super::constants::MESSAGE_TAG;
use super::constants::PRESENCE_TAG;
use super::constants::STREAM_TAG;

pub enum XmppClientProtocolEvent {
    Send(Vec<u8>),
    StartTls,
    Continue,
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

#[derive(Debug, PartialEq, Eq)]
enum StreamState {
    Connected,
    StartSent,
    StartReceived,
    FeaturesReceived,
    Handshake,
    SecureStartSent,
    SecureStartReceived,
    SecureFeaturesReceived,
    AuthStartSent,
    AuthStartReceived,
    Online,
    Error,
}

pub struct XmppClientProtocol {
    jid: Jid,
    stream_parser: StreamParser,
    state: StreamState,
    password: String,
}

impl XmppClientProtocol {
    pub fn new(jid: Jid, password: String) -> Self {
        XmppClientProtocol {
            jid,
            stream_parser: StreamParser::new(),
            state: StreamState::Connected,
            password,
        }
    }

    pub fn jid(&self) -> &Jid {
        &self.jid
    }

    pub fn events<'a>(&'a mut self, bytes: &'a [u8]) -> XmppClientProtocolEvents<'a> {
        XmppClientProtocolEvents::new(self, bytes)
    }

    fn extend_with_header(&mut self, bytes: &mut Vec<u8>) {
        bytes.extend_from_slice(b"<stream:stream xmlns='jabber:client' xmlns:stream='http://etherx.jabber.org/streams' version='1.0' xmllang='en' from='");
        bytes.extend_from_slice(self.jid.full().as_bytes());
        bytes.extend_from_slice(b"' to='");
        bytes.extend_from_slice(self.jid.domainpart().as_bytes());
        bytes.extend_from_slice(b"'>");
    }

    pub fn receive_element(
        &mut self,
        element: Document,
    ) -> Result<(XmppClientProtocolEvent, bool), StreamError> {
        match element.root().name() {
            STREAM_TAG => match self.state {
                StreamState::StartSent => {
                    self.state = StreamState::StartReceived;
                    Ok((XmppClientProtocolEvent::Continue, false))
                }
                StreamState::SecureStartSent => {
                    self.state = StreamState::SecureStartReceived;
                    Ok((XmppClientProtocolEvent::Continue, false))
                }
                StreamState::AuthStartSent => {
                    self.state = StreamState::AuthStartReceived;
                    Ok((XmppClientProtocolEvent::Continue, false))
                }
                _ => {
                    self.state = StreamState::Error;
                    Err(StreamError::BadStream("Unexpected stream tag"))
                }
            },
            FEATURES_TAG => match self.state {
                StreamState::StartReceived => {
                    self.state = StreamState::FeaturesReceived;
                    let mut bytes = Vec::new();
                    bytes.extend_from_slice(b"<starttls xmlns='urn:ietf:params:xml:ns:xmpp-tls'/>");
                    Ok((XmppClientProtocolEvent::Send(bytes), false))
                }
                StreamState::SecureStartReceived => {
                    self.state = StreamState::SecureFeaturesReceived;
                    let mut bytes = Vec::new();
                    bytes.extend_from_slice(
                        b"<auth xmlns='urn:ietf:params:xml:ns:xmpp-sasl' mechanism='PLAIN'>",
                    );
                    let mut userpass = Vec::new();
                    userpass.extend_from_slice(b"\0");
                    let localpart = match self.jid.localpart() {
                        Some(localpart) => localpart,
                        None => return Err(StreamError::BadStream("no localpart for auth")),
                    };
                    userpass.extend_from_slice(localpart.as_bytes());
                    userpass.extend_from_slice(b"\0");
                    userpass.extend_from_slice(self.password.as_bytes());
                    bytes.extend_from_slice(STANDARD.encode(userpass).as_bytes());
                    bytes.extend_from_slice(b"</auth>");
                    Ok((XmppClientProtocolEvent::Send(bytes), false))
                }
                StreamState::AuthStartReceived => {
                    self.state = StreamState::Online;
                    let mut bytes = Vec::new();
                    bytes.extend_from_slice(
                        b"<iq type='set' id='bind'><bind xmlns='urn:ietf:params:xml:ns:xmpp-bind'>",
                    );
                    if let Some(resourcepart) = self.jid.resourcepart() {
                        bytes.extend_from_slice(b"<resource>");
                        bytes.extend_from_slice(resourcepart.as_bytes());
                        bytes.extend_from_slice(b"</resource>");
                    }
                    bytes.extend_from_slice(b"</bind></iq>");
                    Ok((XmppClientProtocolEvent::Send(bytes), false))
                }
                _ => {
                    self.state = StreamState::Error;
                    Err(StreamError::BadStream("Unexpected features tag"))
                }
            },
            PROCEED_TAG => {
                self.state = StreamState::Handshake;
                self.stream_parser.reset();
                Ok((XmppClientProtocolEvent::StartTls, true))
            }
            SUCCESS_TAG => {
                self.state = StreamState::AuthStartSent;
                self.stream_parser.reset();
                let mut bytes = Vec::new();
                self.extend_with_header(&mut bytes);
                Ok((XmppClientProtocolEvent::Send(bytes), true))
            }
            MESSAGE_TAG | PRESENCE_TAG | IQ_TAG => {
                Ok((XmppClientProtocolEvent::Stanza(element), false))
            }
            _ => {
                self.state = StreamState::Error;
                Err(StreamError::BadStream("Unknown tag"))
            }
        }
    }

    pub fn send_bytes(&mut self) -> Option<Vec<u8>> {
        match self.state {
            StreamState::Connected => {
                let mut bytes = Vec::new();
                bytes.extend_from_slice(b"<?xml version='1.0'?>");
                self.extend_with_header(&mut bytes);
                self.state = StreamState::StartSent;
                Some(bytes)
            }
            StreamState::Handshake => {
                let mut bytes = Vec::new();
                self.extend_with_header(&mut bytes);
                self.state = StreamState::SecureStartSent;
                Some(bytes)
            }
            _ => None,
        }
    }

    pub fn receive_bytes(
        &mut self,
        bytes: &[u8],
    ) -> Result<Option<(XmppClientProtocolEvent, usize)>, StreamError> {
        if self.state == StreamState::Error {
            return Err(StreamError::BadStream("already errored"));
        }
        match self.stream_parser.parse_bytes(bytes) {
            Ok(Some((element, parsed))) => {
                let (result, reset) = match element {
                    StreamElement::Element(doc) => self.receive_element(doc)?,
                    StreamElement::End => (XmppClientProtocolEvent::End, true),
                };
                let parsed = if reset { bytes.len() } else { parsed };
                Ok(Some((result, parsed)))
            }
            Ok(None) => Ok(None),
            Err(err) => Err(err.into()),
        }
    }
}
