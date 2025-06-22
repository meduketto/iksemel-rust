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
use std::fmt::Write;
use std::marker::PhantomPinned;
use std::marker::PhantomData;
use std::ptr::null_mut;

use super::arena::Arena;

pub struct Document {
    arena: Arena,
    root_node: UnsafeCell<*mut Node>,
}

#[repr(transparent)]
pub struct Cursor<'a> {
    node: UnsafeCell<*mut Node>,
    marker: PhantomData<&'a Document>,
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
    current: *mut Node,
}

impl Visitor {
    fn new(start: *mut Node) -> Visitor {
        unsafe {
            Visitor {
                level: 0,
                direction: VisitorDirection::Down,
                current: start,
            }
        }
    }

    fn take_step(&mut self) -> bool {
        if self.current.is_null() {
            return false;
        }
        unsafe {
            if let VisitorDirection::Down = self.direction {
                let child = match (*self.current).payload {
                    NodePayload::Tag(tag) => (*tag).children,
                    NodePayload::CData(_) => null_mut(),
                };
                if !child.is_null() {
                    self.level += 1;
                    self.current = child;
                    return true;
                }
            }
            let next = (*self.current).next;
            if next.is_null() {
                self.direction = VisitorDirection::Up;
                self.current = (*self.current).parent;
                if self.level == 0 {
                    return false;
                }
                self.level -= 1;
            } else {
                self.current = next;
                self.direction = VisitorDirection::Down;
            }
        }
        true
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

    pub fn root<'a>(&'a self) -> Cursor<'a> {
        unsafe {
            let node = *self.root_node.get();

            Cursor::new(node)
        }
    }

    //
    // Convenience functions to avoid typing .root() all the time
    //

    pub fn insert_tag<'a,'b>(&'a self, tag_name: &'b str) -> Cursor<'a> {
        self.root().insert_tag(self, tag_name)
    }

    pub fn insert_cdata<'a,'b>(&'a self, cdata: &'b str) -> Cursor<'a> {
        self.root().insert_cdata(self, cdata)
    }

    pub fn str_size(&self) -> usize {
        self.root().str_size()
    }

    pub fn to_string(&self) -> String {
        self.root().to_string()
    }
}

macro_rules! null_cursor {
    () => {
        Cursor::new(null_mut() as *mut Node)
    };
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

impl<'a> Cursor<'a> {
    fn new(node: *mut Node) -> Cursor<'a> {
        Cursor {
            node: node.into(),
            marker: PhantomData,
        }
    }

    pub fn insert_tag<'b,'c>(&'a self, document: &'b Document, tag_name: &'c str) -> Cursor<'b> {
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

                    Cursor::new(new_node)
                }
            }
        }
    }

    pub fn append_tag<'b,'c>(&'a self, document: &'b Document, tag_name: &'c str) -> Cursor<'b> {
        null_cursor_guard!(self);

        unsafe {
            let node = *self.node.get();
            if (*node).parent.is_null() {
                // Root tag cannot have siblings
                return null_cursor!();
            }

            let new_tag = document.arena.alloc_tag(tag_name);
            let new_node = document.arena.alloc_node(NodePayload::Tag(new_tag));

            let parent = (*node).parent;
            (*new_node).parent = parent;

            let next = (*node).next;
            (*new_node).next = next;
            if next.is_null() {
                match (*parent).payload {
                    NodePayload::CData(_) => {
                        // We never create a node under a non Tag node
                        unreachable!();
                    }
                    NodePayload::Tag(tag) => {
                        (*tag).last_child = new_node;
                    }
                }
            } else {
                (*next).previous = new_node;
            }
            (*new_node).previous = node;
            (*node).next = new_node;

            Cursor::new(new_node)
        }
    }

    pub fn insert_cdata<'b,'c>(&'a self, document: &'b Document, cdata: &'c str) -> Cursor<'b> {
        null_cursor_guard!(self);

        unsafe {
            let node = *self.node.get();
            match (*node).payload {
                NodePayload::CData(_) => {
                    // Cannot insert a tag into a cdata element
                    return null_cursor!()
                }
                NodePayload::Tag(tag) => {
                    let new_cdata = document.arena.alloc_cdata(cdata);
                    let new_node = document.arena.alloc_node(NodePayload::CData(new_cdata));

                    (*new_node).parent = node;
                    if (*tag).children.is_null() {
                        (*tag).children = new_node;
                    }
                    if !(*tag).last_child.is_null() {
                        (*(*tag).last_child).next = new_node;
                        (*new_node).previous = (*tag).last_child;
                    }
                    (*tag).last_child = new_node;

                    Cursor::new(new_node)
                }
            }
        }
    }

    pub fn append_cdata<'b,'c>(&'a self, document: &'b Document, cdata: &'c str) -> Cursor<'b> {
        null_cursor_guard!(self);

        unsafe {
            let node = *self.node.get();
            if (*node).parent.is_null() {
                // Root tag cannot have siblings
                return null_cursor!();
            }

            let new_cdata = document.arena.alloc_cdata(cdata);
            let new_node = document.arena.alloc_node(NodePayload::CData(new_cdata));

            let parent = (*node).parent;
            (*new_node).parent = parent;

            let next = (*node).next;
            (*new_node).next = next;
            if next.is_null() {
                match (*parent).payload {
                    NodePayload::CData(_) => {
                        unreachable!();
                    }
                    NodePayload::Tag(tag) => {
                        (*tag).last_child = new_node;
                    }
                }
            } else {
                (*next).previous = new_node;
            }
            (*new_node).previous = node;
            (*node).next = new_node;

            Cursor::new(new_node)
        }
    }

    //
    // Navigation methods
    //

    pub fn next(&self) -> Cursor {
        null_cursor_guard!(self);

        unsafe {
            let node = *self.node.get();

            Cursor::new((*node).next)
        }
    }

    pub fn previous(&self) -> Cursor {
        null_cursor_guard!(self);

        unsafe {
            let node = *self.node.get();

            Cursor::new((*node).previous)
        }
    }

    pub fn parent(&self) -> Cursor {
        null_cursor_guard!(self);

        unsafe {
            let node = *self.node.get();

            Cursor::new((*node).parent)
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
                    Cursor::new((*tag).children)
                },
            }
        }
    }

    //
    // Node property methods
    //

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
            let mut v = Visitor::new(*self.node.get());
            loop {
                let current: *const Node = v.current;
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
                                }
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
                        NodePayload::CData(cdata) => {
                            if let VisitorDirection::Down = v.direction {
                                size += (*cdata).value_size;
                            }
                        },
                    }
                    if !v.take_step() {
                        break;
                    }
                }
            }
        }

        size
    }

    fn to_string(&self) -> String {
        let mut buf = String::with_capacity(self.str_size());
        write!(&mut buf, "{}", self)
            .expect("a Display implementation returned an error unexpectedly");
        buf
    }
}

impl<'a> std::fmt::Display for Cursor<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        unsafe {
            if (*self.node.get()).is_null() {
                return Result::Ok(());
            }
        }
        unsafe {
            let mut v = Visitor::new(*self.node.get());

            loop {
                let current: *const Node = v.current;
                match (*current).payload {
                    NodePayload::Tag(tag) => {
                        match v.direction {
                            VisitorDirection::Down => {
                                f.write_str("<");
                                let slice =
                                    std::slice::from_raw_parts((*tag).name, (*tag).name_size);
                                let s = std::str::from_utf8_unchecked(slice);
                                f.write_str(s);
                                if (*tag).children.is_null() {
                                    f.write_str("/>");
                                } else {
                                    f.write_str(">");
                                }
                            }
                            VisitorDirection::Up => {
                                if (*tag).children.is_null() {
                                    // Already handled
                                } else {
                                    f.write_str("</");
                                    let slice =
                                        std::slice::from_raw_parts((*tag).name, (*tag).name_size);
                                    let s = std::str::from_utf8_unchecked(slice);
                                    f.write_str(s);
                                    f.write_str(">");
                                }
                            }
                        }
                    }
                    NodePayload::CData(cdata) => {
                        if let VisitorDirection::Down = v.direction {
                            let slice =
                                std::slice::from_raw_parts((*cdata).value, (*cdata).value_size);
                            let s = std::str::from_utf8_unchecked(slice);
                            f.write_str(s);
                        }
                    },

                }

                if !v.take_step() {
                    break;
                }
            }
        }
        Result::Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let doc = Document::new("html");
        let blink = doc.insert_tag("p").insert_tag(&doc, "b").insert_tag(&doc, "blink").insert_cdata(&doc, "lala");
        assert_eq!(blink.is_null(), false);

        doc.root().first_child().append_cdata(&doc, "foo").append_tag(&doc, "p2");

        let xml = doc.to_string();
        assert_eq!(xml, "<html><p><b><blink>lala</blink></b></p>foo<p2/></html>");
        // Verify that the capacity is measured correctly
        assert_eq!(xml.len(), xml.capacity());
    }

}

// FIXME: MaybeUninit?
// FIXME: NodePayload niche optimization
// FIXME: unit tests
// FIXME: docs
// FIXME: prepend funcs
// FIXME: Cursor and navigation funcs
// FIXME: Drop
// FIXME: find/xpath funcs
// FIXME: delete funcs
// FIXME: clone
// FIXME: node property funcs

// FIXME: string escape/unescape
