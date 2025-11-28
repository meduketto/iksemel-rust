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
use std::net::SocketAddr;
use std::net::TcpStream;
use std::net::ToSocketAddrs;
use std::sync::Arc;
use std::time::Duration;

use rustls::RootCertStore;
use webpki_roots::TLS_SERVER_ROOTS;

use crate::Document;
use crate::Jid;
use crate::XMPP_CLIENT_PORT;
use crate::XmppClientError;
use crate::XmppClientProtocol;
use crate::xmpp::protocol::XmppClientProtocolEvent;

pub(super) fn need_port(host: &str) -> bool {
    // Rust resolver does require a port number but does NOT provide
    // a way to provide a default one :(
    let column_pos = host.rfind(':');
    let bracket_pos = host.find(']');
    match (column_pos, bracket_pos) {
        (None, None) | (None, Some(_)) => true,
        (Some(_), None) => false,
        (Some(column), Some(bracket)) => column < bracket,
    }
}

fn resolve_host_with_default_port(
    host: &str,
    default_port: u16,
) -> std::io::Result<std::vec::IntoIter<SocketAddr>> {
    if need_port(host) {
        (host, default_port).to_socket_addrs()
    } else {
        host.to_socket_addrs()
    }
}

pub struct XmppClientBuilder {
    jid: Jid,
    server: Option<String>,
    password: String,
    connection_timeout: Duration,
    debug: bool,
}

impl XmppClientBuilder {
    pub fn new(jid: Jid, password: String) -> Self {
        XmppClientBuilder {
            jid,
            server: None,
            password,
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
        let result = resolve_host_with_default_port(host, XMPP_CLIENT_PORT)?;
        for addr in result {
            if self.debug {
                println!("Connecting to: {addr:?}");
            }
            match TcpStream::connect_timeout(&addr, self.connection_timeout) {
                Ok(tcp_stream) => {
                    return Ok(XmppClient {
                        protocol: XmppClientProtocol::new(self.jid, self.password),
                        stream: XmppStream::new(tcp_stream),
                        read_buffer: [0; 4096],
                        consumed: 0,
                        read: 0,
                        debug: self.debug,
                    });
                }
                Err(err) => {
                    if self.debug {
                        println!("Failed to connect to {addr:?}: {err}");
                    }
                }
            }
        }
        Err(XmppClientError::BadStream("cannot connect"))
    }
}

struct XmppStream {
    tcp_stream: Option<TcpStream>,
    tls_stream: Option<rustls::StreamOwned<rustls::ClientConnection, TcpStream>>,
}

impl XmppStream {
    fn new(tcp_stream: TcpStream) -> Self {
        XmppStream {
            tcp_stream: Some(tcp_stream),
            tls_stream: None,
        }
    }

    fn read(&mut self, buf: &mut [u8]) -> Result<usize, std::io::Error> {
        if let Some(tcp) = &mut self.tcp_stream {
            Ok(tcp.read(buf)?)
        } else if let Some(tls) = &mut self.tls_stream {
            Ok(tls.read(buf)?)
        } else {
            Err(std::io::Error::other("No stream"))
        }
    }

    fn write_all(&mut self, buf: &[u8]) -> Result<(), std::io::Error> {
        if let Some(tcp) = &mut self.tcp_stream {
            Ok(tcp.write_all(buf)?)
        } else if let Some(tls) = &mut self.tls_stream {
            Ok(tls.write_all(buf)?)
        } else {
            Err(std::io::Error::other("No stream"))
        }
    }

    fn upgrade(&mut self, jid: &Jid) -> Result<(), XmppClientError> {
        let root_store = RootCertStore {
            roots: TLS_SERVER_ROOTS.into(),
        };
        let config = rustls::ClientConfig::builder_with_provider(
            rustls::crypto::aws_lc_rs::default_provider().into(),
        )
        .with_safe_default_protocol_versions()?
        .with_root_certificates(root_store)
        .with_no_client_auth();

        let server_name = jid.domainpart().to_owned().try_into()?;
        let conn = rustls::ClientConnection::new(Arc::new(config), server_name)?;
        match self.tcp_stream.take() {
            Some(tcp_stream) => {
                let tls = rustls::StreamOwned::new(conn, tcp_stream);
                self.tls_stream = Some(tls);
                Ok(())
            }
            None => Err(XmppClientError::BadStream("not possible")),
        }
    }
}

pub struct XmppClient {
    protocol: XmppClientProtocol,
    stream: XmppStream,
    read_buffer: [u8; 4096],
    consumed: usize,
    read: usize,
    debug: bool,
}

impl XmppClient {
    pub fn build(jid: Jid, password: String) -> XmppClientBuilder {
        XmppClientBuilder::new(jid, password)
    }

    pub fn send_bytes(&mut self, bytes: Vec<u8>) -> Result<(), XmppClientError> {
        if self.debug {
            println!("Sending bytes: {}", String::from_utf8_lossy(&bytes));
        }
        self.stream.write_all(bytes.as_slice())?;
        Ok(())
    }

    pub fn send_stanza(&mut self, stanza: Document) -> Result<(), XmppClientError> {
        self.send_bytes(stanza.to_string().into_bytes())
    }

    pub fn send_message(&mut self, jid: Jid, body: &str) -> Result<(), XmppClientError> {
        let stanza = Document::new("message")?;
        stanza
            .root()
            .set_attribute("to", Some(jid.full()))?
            .insert_tag("body")?
            .insert_cdata(body)?;
        self.send_stanza(stanza)
    }

    pub fn request_roster(&mut self) -> Result<(), XmppClientError> {
        let stanza = Document::new("iq")?;
        stanza
            .root()
            .set_attribute("type", Some("get"))?
            .set_attribute("from", Some(self.protocol.jid().full()))?
            .set_attribute("id", Some("roster"))?
            .insert_tag("query")?
            .set_attribute("xmlns", Some("jabber:iq:roster"))?;
        self.send_stanza(stanza)
    }

    pub fn wait_for_stanza(&mut self) -> Result<Document, XmppClientError> {
        loop {
            if let Some(bytes) = self.protocol.send_bytes() {
                self.send_bytes(bytes)?;
            }
            let bytes = if self.read > self.consumed {
                &self.read_buffer[self.consumed..self.read]
            } else {
                let nr_read = self.stream.read(&mut self.read_buffer)?;
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
                    self.consumed += processed;
                    match event {
                        XmppClientProtocolEvent::Send(bytes) => self.send_bytes(bytes)?,
                        XmppClientProtocolEvent::StartTls => {
                            self.stream.upgrade(self.protocol.jid())?;
                        }
                        XmppClientProtocolEvent::Continue => {}
                        XmppClientProtocolEvent::Stanza(doc) => {
                            return Ok(doc);
                        }
                        XmppClientProtocolEvent::End => {}
                    }
                }
                Ok(None) => self.consumed = self.read,
                Err(err) => return Err(err.into()),
            }
        }
    }
}
