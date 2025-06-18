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
    fn alloc_attribute(&self, name: &str, value: &str) -> *mut Attribute;
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

    fn alloc_attribute(&self, name: &str, value: &str) -> *mut Attribute {
        let name = self.push_str(name);
        let value = self.push_str(value);
        let attribute = self.alloc(Layout::new::<Attribute>()) as *mut Attribute;
        unsafe {
            (*attribute).next = null_mut();
            (*attribute).previous = null_mut();
            (*attribute).name = name.as_ptr();
            (*attribute).name_size = name.len();
            (*attribute).value = value.as_ptr();
            (*attribute).value_size = value.len();
        }

        attribute
    }
}

enum VisitorDirection {
    Up,
    Down,
}

struct Visitor {
    level: usize,
    direction: VisitorDirection,
    current: Cursor,
}

impl Visitor {
    fn new(start: Cursor) -> Visitor {
        Visitor {
            level: 0,
            direction: VisitorDirection::Down,
            current: start,
        }
    }

    fn take_step(&mut self) -> bool {
        if self.current.is_null() {
            return false;
        }
        if let VisitorDirection::Down = self.direction {
            let child = self.current.first_child();
            if !child.is_null() {
                self.level += 1;
                self.current = child;
                return true;
            }
        }
        let next = self.current.next();
        if next.is_null() {
            self.direction = VisitorDirection::Up;
            self.current = self.current.parent();
            if self.level == 0 {
                return false;
            }
            self.level -= 1;
        } else {
            self.current = next;
            self.direction = VisitorDirection::Down;
        }
        return true;
    }
}

impl Document {
    pub fn new(root_tag_name: &str) -> Document {
        let arena = Arena::new();
        let tag = arena.alloc_tag(root_tag_name);
        let node = arena.alloc_node(NodePayload::Tag(tag));

        unsafe {
            Document {
                arena,
                root_node: node.into(),
            }
        }
    }

    pub fn root(&self) -> Cursor {
        unsafe {
            let node = *self.root_node.get();

            Cursor { node: node.into() }
        }
    }

    pub fn insert_tag(&mut self, tag_name: &str) -> Cursor {
        self.root().insert_tag(self, tag_name)
    }

    pub fn str_size(&self) -> usize {
        self.root().str_size()
    }
}

macro_rules! null_cursor {
    () => {
        Cursor {
            node: (null_mut() as *mut Node).into(),
        }
    }
}

macro_rules! null_cursor_guard {
    ($x:expr) => {
        unsafe {
            if (*$x.node.get()).is_null() {
                return null_cursor!();
            }
        }
    };
}

impl Cursor {
    pub fn insert_tag(&self, document: &mut Document, tag_name: &str) -> Cursor {
        null_cursor_guard!(self);

        unsafe {
            let node = *self.node.get();
            match (*node).payload {
                NodePayload::CData(_) => {
                    // Cannot insert a tag into a cdata element
                    null_cursor!()
                }
                NodePayload::Tag(tag) => {
                    let new_tag = document.arena.alloc_tag(tag_name);
                    let new_node = document.arena.alloc_node(NodePayload::Tag(new_tag));

                    (*new_node).parent = node;
                    if (*tag).children.is_null() {
                        (*tag).children = new_node;
                    }
                    if !(*tag).last_child.is_null() {
                        (*(*tag).last_child).next = new_node;
                        (*new_node).previous = (*tag).last_child;
                    }
                    (*tag).last_child = new_node;

                    Cursor {
                        node: new_node.into(),
                    }
                }
            }
        }
    }

    //
    // Navigation methods
    //

    pub fn next(&self) -> Cursor {
        null_cursor_guard!(self);

        unsafe {
            let node = *self.node.get();

            Cursor {
                node: (*node).next.into(),
            }
        }
    }

    pub fn previous(&self) -> Cursor {
        null_cursor_guard!(self);

        unsafe {
            let node = *self.node.get();

            Cursor {
                node: (*node).previous.into(),
            }
        }
    }

    pub fn parent(&self) -> Cursor {
        null_cursor_guard!(self);

        unsafe {
            let node = *self.node.get();

            Cursor {
                node: (*node).parent.into(),
            }
        }
    }

    pub fn first_child(&self) -> Cursor {
        null_cursor_guard!(self);

        unsafe {
            let node = *self.node.get();
            match (*node).payload {
                NodePayload::CData(_) => {
                    null_cursor!()
                }
                NodePayload::Tag(tag) => {
                    Cursor {
                        node: (*tag).children.into(),
                    }
                }
            }
        }
    }

    pub fn is_null(&self) -> bool {
        unsafe {
            let node = *self.node.get();
            node.is_null()
        }
    }

    pub fn str_size(&self) -> usize {
        unsafe {
            if (*self.node.get()).is_null() {
                return 0;
            }
        }

        let mut size = 0;
        unsafe {
            let mut v = Visitor::new(Cursor { node: (*self.node.get()).into() });
            loop {
                let current: *const Node = *v.current.node.get();
                unsafe {
                    match (*current).payload {
                        NodePayload::Tag(tag) => {
                            match v.direction {
                                VisitorDirection::Down => {
                                    size += 1; // Tag opening '<'
                                    size += (*tag).name_size;
                                    if (*tag).children.is_null() {
                                        size += 2; // Standalone tag closing '/>'
                                    } else {
                                        size += 1;
                                    }
                                },
                                VisitorDirection::Up => {
                                    if (*tag).children.is_null() {
                                        // Already handled
                                    } else {
                                        size += 2; // End tag opening '</'
                                        size += (*tag).name_size;
                                        size += 1; // End tag closing '>'
                                    }
                                }
                            }
                        }
                        NodePayload::CData(cdata) => (),
                    }
                    if !v.take_step() { break; }
                }
            }
        }

        size
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let mut doc = Document::new("html");

        let p = doc.insert_tag("p");
        let b = p.insert_tag(&mut doc, "b");
        let c = b.insert_tag(&mut doc, "lala");

        // <html><p><b><lala/></b></p></html>
        assert_eq!(doc.str_size(), 34);
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
