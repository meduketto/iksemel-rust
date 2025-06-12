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
use std::marker::PhantomPinned;
use std::ptr::null_mut;

use super::arena::Arena;

pub struct Document {
    arena: Arena,
    root: *mut Node,
}

enum NodePayload {
    Tag(*mut Tag),
    CData(*mut CData),
}

struct Node {
    next: *mut Node,
    prev: *mut Node,
    parent: *mut Node,
    payload: NodePayload,

    _pin: PhantomPinned,
}

struct Tag {
    children: *mut Node,
    last_child: *mut Node,
    attributes: *mut Attribute,
    last_attribute: *mut Attribute,
    name: *const u8,
    name_size: usize,

    _pin: PhantomPinned,
}

struct CData {
    cdata: *mut u8,
    cdata_size: usize,

    _pin: PhantomPinned,
}

struct Attribute {
    next: *mut Attribute,
    prev: *mut Attribute,
    name: *mut u8,
    name_size: usize,
    value: *mut u8,
    value_size: usize,

    _pin: PhantomPinned,
}

impl Document {
    fn create_node(arena: &Arena, payload: NodePayload) -> *mut Node {
        let node = arena.alloc(Layout::new::<Node>()) as *mut Node;
        unsafe {
            (*node).next = null_mut();
            (*node).prev = null_mut();
            (*node).parent = null_mut();
            (*node).payload = payload;
        }
        node
    }

    fn create_tag(arena: &Arena, tag_name: &str) -> *mut Node {
        let name = arena.push_str(tag_name);

        let tag = arena.alloc(Layout::new::<Tag>()) as *mut Tag;
        unsafe {
            (*tag).children = null_mut();
            (*tag).last_child = null_mut();
            (*tag).attributes = null_mut();
            (*tag).last_attribute = null_mut();
            (*tag).name = name.as_ptr();
            (*tag).name_size = name.len();
        }

        Document::create_node(arena, NodePayload::Tag(tag))
    }

    pub fn new(root_tag_name: &str) -> Document {
        let arena = Arena::new();
        let node = Document::create_tag(&arena, root_tag_name);
        Document { arena, root: node }
    }

}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let doc = Document::new("test");
    }
}


// FIXME: MaybeUninit?
// FIXME: NodePayload niche optimization
// FIXME: unit tests
// FIXME: docs
// FIXME: insert/append/prepend funcs
// FIXME: Cursor and navigation funcs
// FIXME: serializer
// FIXME: Drop
// FIXME: find/xpath funcs
// FIXME: delete funcs
// FIXME: duplicate
