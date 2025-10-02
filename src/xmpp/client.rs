/*
** This file is a part of Iksemel (XML parser for Jabber/XMPP)
** Copyright (C) 2000-2025 Gurer Ozen
**
** Iksemel is free software: you can redistribute it and/or modify it
** under the terms of the GNU Lesser General Public License as
** published by the Free Software Foundation, either version 3 of
** the License, or (at your option) any later version.
*/

use std::io::Read;
use std::io::Write;
use std::net::TcpStream;
use std::net::ToSocketAddrs;
use std::time::Duration;

use crate::Document;
use crate::Jid;
use crate::XMPP_CLIENT_PORT;
use crate::XmppClientError;
use crate::XmppClientProtocol;
use crate::xmpp::protocol::XmppClientProtocolEvent;

pub struct XmppClientBuilder {
    jid: Jid,
    server: Option<String>,
    connection_timeout: Duration,
    debug: bool,
}

impl XmppClientBuilder {
    pub fn new(jid: Jid) -> Self {
        XmppClientBuilder {
            jid,
            server: None,
            connection_timeout: Duration::from_secs(30),
            debug: false,
        }
    }

    pub fn server(mut self, server: Option<String>) -> Self {
        self.server = server;
        self
    }

    pub fn connection_timeout(mut self, timeout: Duration) -> Self {
        self.connection_timeout = timeout;
        self
    }

    pub fn debug(mut self, debug: bool) -> Self {
        self.debug = debug;
        self
    }

    pub fn connect(self) -> Result<XmppClient, XmppClientError> {
        let host = match &self.server {
            Some(server) => server,
            None => self.jid.domainpart(),
        };
        // Rust resolver does require a port number but does NOT provide
        // a way to provide a default one :(
        let column_pos = host.find(':');
        let bracket_pos = host.find(']');
        let need_port = match (column_pos, bracket_pos) {
            (None, None) | (None, Some(_)) => true,
            (Some(_), None) => false,
            (Some(column), Some(bracket)) => column < bracket,
        };
        let mut result = if need_port {
            (host, XMPP_CLIENT_PORT).to_socket_addrs()
        } else {
            host.to_socket_addrs()
        }?;
        if self.debug {
            println!("Connecting to: {result:?}");
        }
        let tcp_stream =
            TcpStream::connect_timeout(&result.next().unwrap(), self.connection_timeout)?;
        Ok(XmppClient {
            protocol: XmppClientProtocol::new(self.jid),
            tcp_stream,
            read_buffer: [0; 4096],
            consumed: 0,
            read: 0,
            debug: self.debug,
        })
    }
}

pub struct XmppClient {
    protocol: XmppClientProtocol,
    tcp_stream: TcpStream,
    read_buffer: [u8; 4096],
    consumed: usize,
    read: usize,
    debug: bool,
}

impl XmppClient {
    pub fn build(jid: Jid) -> XmppClientBuilder {
        XmppClientBuilder::new(jid)
    }

    pub fn send_bytes(&mut self, bytes: Vec<u8>) -> Result<(), XmppClientError> {
        if self.debug {
            println!("Sending bytes: {}", String::from_utf8_lossy(&bytes));
        }
        self.tcp_stream.write_all(bytes.as_slice())?;
        Ok(())
    }

    pub fn wait_for_stanza(&mut self) -> Result<Document, XmppClientError> {
        if let Some(bytes) = self.protocol.send_bytes() {
            self.send_bytes(bytes)?;
        }
        loop {
            let bytes = if self.read > self.consumed {
                &self.read_buffer[self.consumed..self.read]
            } else {
                let nr_read = self.tcp_stream.read(&mut self.read_buffer)?;
                if self.debug {
                    println!(
                        "Received bytes: {}",
                        String::from_utf8_lossy(&self.read_buffer[..nr_read])
                    );
                }
                self.read = nr_read;
                self.consumed = 0;
                &self.read_buffer[..self.read]
            };
            match self.protocol.receive_bytes(bytes) {
                Ok(Some((event, processed))) => {
                    match event {
                        XmppClientProtocolEvent::Send(bytes) => self.send_bytes(bytes)?,
                        XmppClientProtocolEvent::StartTls => {}
                        XmppClientProtocolEvent::Stanza(doc) => {
                            return Ok(doc);
                        }
                        XmppClientProtocolEvent::End => {}
                    }
                    self.consumed += processed;
                }
                Ok(None) => self.consumed = self.read,
                Err(err) => return Err(XmppClientError::StreamError(err)),
            }
        }
    }
}
