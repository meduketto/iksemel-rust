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
use std::cell::UnsafeCell;
use std::marker::PhantomPinned;
use std::ptr::null_mut;

use super::arena::Arena;

pub struct Document {
    arena: Arena,
    root_node: UnsafeCell<*mut Node>,
}

#[repr(transparent)]
pub struct Cursor {
    node: UnsafeCell<*mut Node>,
}

pub trait OptionCursorExt {
    fn next(&self) -> Option<Cursor>;
    fn insert_tag(&self, document: &mut Document, tag_name: &str) -> Option<Cursor>;
}

enum NodePayload {
    Tag(*mut Tag),
    CData(*mut CData),
}

struct Node {
    next: *mut Node,
    previous: *mut Node,
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
    value: *const u8,
    value_size: usize,

    _pin: PhantomPinned,
}

struct Attribute {
    next: *mut Attribute,
    previous: *mut Attribute,
    name: *const u8,
    name_size: usize,
    value: *const u8,
    value_size: usize,

    _pin: PhantomPinned,
}

trait ArenaExt {
    fn alloc_node(&self, payload: NodePayload) -> *mut Node;
    fn alloc_tag(&self, tag_name: &str) -> *mut Tag;
    fn alloc_cdata(&self, cdata_value: &str) -> *mut CData;
}

impl ArenaExt for Arena {
    fn alloc_node(&self, payload: NodePayload) -> *mut Node {
        let node = self.alloc(Layout::new::<Node>()) as *mut Node;
        unsafe {
            (*node).next = null_mut();
            (*node).previous = null_mut();
            (*node).parent = null_mut();
            (*node).payload = payload;
        }

        node
    }

    fn alloc_tag(&self, tag_name: &str) -> *mut Tag {
        let name = self.push_str(tag_name);
        let tag = self.alloc(Layout::new::<Tag>()) as *mut Tag;
        unsafe {
            (*tag).children = null_mut();
            (*tag).last_child = null_mut();
            (*tag).attributes = null_mut();
            (*tag).last_attribute = null_mut();
            (*tag).name = name.as_ptr();
            (*tag).name_size = name.len();
        }

        tag
    }

    fn alloc_cdata(&self, cdata_value: &str) -> *mut CData {
        let value = self.push_str(cdata_value);
        let cdata = self.alloc(Layout::new::<CData>()) as *mut CData;
        unsafe {
            (*cdata).value = value.as_ptr();
            (*cdata).value_size = value.len();
        }

        cdata
    }

}

impl Document {
    pub fn new(root_tag_name: &str) -> Document {
        let arena = Arena::new();
        let tag = arena.alloc_tag(root_tag_name);
        let node = arena.alloc_node(NodePayload::Tag(tag));

        unsafe {
            Document { arena, root_node: node.into() }
        }
    }

    pub fn root(&self) -> Cursor {
        unsafe {
            let node = *self.root_node.get();

            Cursor { node: node.into() }
        }
    }

    pub fn insert_tag(&mut self, tag_name: &str) -> Option<Cursor> {
        self.root().insert_tag(self, tag_name)
    }
/*
    pub fn str_size(&self) -> usize {
        Cursor { arena: self.arena, node: self.root }.str_size()
    }
*/
}

impl Cursor {
    pub fn insert_tag(&self, document: &mut Document, tag_name: &str) -> Option<Cursor> {
        unsafe {
            let node = *self.node.get();
            // FIXME: check it is a tag node
        }

        let tag = document.arena.alloc_tag(tag_name);
        let node = document.arena.alloc_node(NodePayload::Tag(tag));
/*
        (*node).parent = self.node;
        if (*self.tag).children.is_null() {
            (*self.tag).children = node;
        }
        if !(*self.node).last_child.is_null() {
            (*(*self.node).last_child).next = node;
            (*node).previous = (*self.node).last_child;
        }
        (*self.node).last_child = node;
*/
        Some(Cursor { node: node.into() })
    }

    pub fn next(&self) -> Cursor {
        unsafe {
            let node = *self.node.get();

            Cursor { node: (*node).next.into() }
        }
    }

    pub fn previous(&self) -> Cursor {
        unsafe {
            let node = *self.node.get();

            Cursor { node: (*node).previous.into() }
        }
    }

    pub fn parent(&self) -> Cursor {
        unsafe {
            let node = *self.node.get();

            Cursor { node: (*node).parent.into() }
        }
    }

/*

    pub fn str_size(&self) -> usize {
        let mut size = 0;
        let mut current: *const Node = self.node;

        unsafe {
            match (*current).payload {
                NodePayload::Tag(tag) => {
                    size += 1;  // Tag opening '<'
                    size += (*tag).name_size;
                    if (*tag).children.is_null() {
                        size += 2;  // Standalone tag closing '/>'
                    } else {
                        // FIXME: not implemented
                    }
                },
                NodePayload::CData(cdata) => (),
            }
        }

        size
    }
*/
}


impl OptionCursorExt for Option<Cursor> {
    fn next(&self) -> Option<Cursor> {
        None
    }
    
    fn insert_tag(&self, document: &mut Document, tag_name: &str) -> Option<Cursor> {
        match self {
            Some(cursor) => cursor.insert_tag(document, tag_name),
            None => None,
        }
    }
}




#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let mut doc = Document::new("html");

        let p = doc.insert_tag("p");
        let b = p.insert_tag(&mut doc, "b").next();
        let c = b.insert_tag(&mut doc, "lala");
//        let b = doc.insert_tag(p, "b");

//        assert_eq!(doc.str_size(), 7);
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
// FIXME: clone


// FIXME: Option vs built-in Cursor checks
