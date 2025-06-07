/*
** This file is a part of Iksemel (XML parser for Jabber/XMPP)
** Copyright (C) 2000-2025 Gurer Ozen
**
** Iksemel is free software: you can redistribute it and/or modify it
** under the terms of the GNU Lesser General Public License as
** published by the Free Software Foundation, either version 3 of
** the License, or (at your option) any later version.
*/

use std::alloc::Layout;

use super::arena::Arena;

pub struct Document {
    arena: Arena,
    root: usize,
}

enum NodeContent {
    TAG,
    CDATA,
}

struct Node {
    next: usize,
    prev: usize,
    parent: usize,
    content: NodeContent,
}

struct Tag {
    children: usize,
    last_child: usize,
    attributes: usize,
    last_attribute: usize,
    name: usize,
}

struct CData {
    cdata: usize,
}

struct Attribute {
    next: usize,
    prev: usize,
    name: usize,
    value: usize,
}

impl Document {
    pub fn new(root_tag_name: &str) -> Document {
        let arena = Arena::new();

        arena.alloc(Layout::new::<Tag>());
        arena.alloc(Layout::new::<CData>());
        arena.alloc(Layout::new::<Attribute>());

        Document {
            arena: arena,
            root: 0,
        }
    }
    /*
        fn create_tag(arena: *mut Arena, tag_name: &str) -> Tag {
            let mut node = arena.alloc();
        }
    */
}
