/*
** This file is a part of Iksemel (XML parser for Jabber/XMPP)
** Copyright (C) 2000-2025 Gurer Ozen
**
** Iksemel is free software: you can redistribute it and/or modify it
** under the terms of the GNU Lesser General Public License as
** published by the Free Software Foundation, either version 3 of
** the License, or (at your option) any later version.
*/

use super::error::description;
use super::*;

fn check_jid(
    jid: Jid,
    full: &str,
    bare: &str,
    local: Option<&str>,
    domain: &str,
    resource: Option<&str>,
) {
    assert_eq!(jid.full(), full);
    assert_eq!(jid.bare(), bare);
    assert_eq!(jid.localpart(), local);
    assert_eq!(jid.domainpart(), domain);
    assert_eq!(jid.resourcepart(), resource);
    assert_eq!(jid.is_bare(), resource.is_none());
    assert_eq!(jid.full.capacity(), jid.full().len());
}

#[test]
fn good_jids() {
    check_jid(
        Jid::new("juliet@example.com").unwrap(),
        "juliet@example.com",
        "juliet@example.com",
        Some("juliet"),
        "example.com",
        None,
    );
    check_jid(
        Jid::new("juliet@example.com/foo").unwrap(),
        "juliet@example.com/foo",
        "juliet@example.com",
        Some("juliet"),
        "example.com",
        Some("foo"),
    );
    check_jid(
        Jid::new("juliet@example.com/foo@bar").unwrap(),
        "juliet@example.com/foo@bar",
        "juliet@example.com",
        Some("juliet"),
        "example.com",
        Some("foo@bar"),
    );
    check_jid(
        Jid::new("example.com").unwrap(),
        "example.com",
        "example.com",
        None,
        "example.com",
        None,
    );
    check_jid(
        Jid::new("example.com/foobar").unwrap(),
        "example.com/foobar",
        "example.com",
        None,
        "example.com",
        Some("foobar"),
    );
    check_jid(
        Jid::new("a.example.com/b@example.net").unwrap(),
        "a.example.com/b@example.net",
        "a.example.com",
        None,
        "a.example.com",
        Some("b@example.net"),
    );
}

#[test]
fn resource_change() {
    let jid = Jid::new("juliet@example.com/balcony").unwrap();
    check_jid(
        jid.with_resource("orchard").unwrap(),
        "juliet@example.com/orchard",
        "juliet@example.com",
        Some("juliet"),
        "example.com",
        Some("orchard"),
    );

    let jid = Jid::new("juliet@example.com").unwrap();
    check_jid(
        jid.with_resource("street").unwrap(),
        "juliet@example.com/street",
        "juliet@example.com",
        Some("juliet"),
        "example.com",
        Some("street"),
    );

    let jid = Jid::new("juliet@example.com").unwrap();
    assert_eq!(
        jid.with_resource(""),
        Err(BadJid(description::RESOURCE_EMPTY))
    );

    let jid = Jid::new("juliet@example.com").unwrap();
    assert_eq!(
        jid.with_resource(&"b".repeat(1024)),
        Err(BadJid(description::RESOURCE_TOO_LONG))
    );

    let jid = Jid::new("juliet@example.com").unwrap();
    let _ = jid.with_resource(&"e".repeat(1023)).unwrap();
}

#[test]
fn bad_jids() {
    assert_eq!(Jid::new(""), Err(BadJid(description::DOMAIN_EMPTY)));
    assert_eq!(
        Jid::new("/resource"),
        Err(BadJid(description::DOMAIN_EMPTY))
    );
    assert_eq!(
        Jid::new("local@/resource"),
        Err(BadJid(description::DOMAIN_EMPTY))
    );
    assert_eq!(Jid::new("local@"), Err(BadJid(description::DOMAIN_EMPTY)));

    assert_eq!(
        Jid::new("@example.com"),
        Err(BadJid(description::LOCAL_EMPTY))
    );

    assert_eq!(
        Jid::new("example.com/"),
        Err(BadJid(description::RESOURCE_EMPTY))
    );
}

#[test]
fn comparisons() {
    let j1a = Jid::new("juliet@example.com").unwrap();
    let j1b = Jid::new("juliet@example.com.").unwrap();
    let j1c = Jid::new("juliet@example.com/balcony").unwrap();
    let j2 = Jid::new("romeo@example.com").unwrap();

    assert_eq!(j1a, j1a);
    assert_eq!(j1a, j1b);
    assert_ne!(j1a, j1c);
    assert_ne!(j1a, j2);
    assert!(j1a < j2);
    assert!(j2 > j1a);
}

#[test]
fn prints() {
    let j1 = Jid::new("lala@example.com/bibi").unwrap();
    let s1 = format!("{:?}", j1);
    assert!(s1.len() > 0);
    let s2 = format!("{}", j1);
    assert!(s2.len() > 0);

    let j2 = Jid::new("lala@example.com/bibi").unwrap();
    let s3 = format!("{:?}", j2);
    assert!(s3.len() > 0);
    let s4 = format!("{}", j2);
    assert!(s4.len() > 0);
}
