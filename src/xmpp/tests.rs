/*
** This file is a part of Iksemel (XML parser for Jabber/XMPP)
** Copyright (C) 2000-2025 Gurer Ozen
**
** Iksemel is free software: you can redistribute it and/or modify it
** under the terms of the GNU Lesser General Public License as
** published by the Free Software Foundation, either version 3 of
** the License, or (at your option) any later version.
*/

use super::client::need_port;

#[test]
fn host_port_checking() {
    assert!(need_port("example.com"));
    assert!(!need_port("example.com:5222"));
    assert!(need_port("[::1]"));
    assert!(!need_port("[::1]:5222"));
}
