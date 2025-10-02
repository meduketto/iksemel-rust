/*
** This file is a part of Iksemel (XML parser for Jabber/XMPP)
** Copyright (C) 2000-2025 Gurer Ozen
**
** Iksemel is free software: you can redistribute it and/or modify it
** under the terms of the GNU Lesser General Public License as
** published by the Free Software Foundation, either version 3 of
** the License, or (at your option) any later version.
*/

mod client;
pub(crate) mod constants;
mod error;
mod jid;
mod parser;
mod protocol;

pub use client::XmppClient;
pub use error::BadJid;
pub use error::StreamError;
pub use error::XmppClientError;
pub use jid::Jid;
pub use parser::StreamElement;
pub use parser::StreamParser;
pub use protocol::XmppClientProtocol;

#[cfg(test)]
mod tests;
