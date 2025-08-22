/*
** This file is a part of Iksemel (XML parser for Jabber/XMPP)
** Copyright (C) 2000-2025 Gurer Ozen
**
** Iksemel is free software: you can redistribute it and/or modify it
** under the terms of the GNU Lesser General Public License as
** published by the Free Software Foundation, either version 3 of
** the License, or (at your option) any later version.
*/

mod error;
mod parser;

use std::cell::UnsafeCell;
use std::marker::PhantomData;
use std::marker::PhantomPinned;
use std::ptr::null_mut;

use super::arena::Arena;
use super::entities::escape;
use super::entities::escape_fmt;
use super::entities::escaped_size;
use error::DocumentError;
pub use parser::DocumentParser;

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

impl Tag {
    fn as_str(&self) -> &str {
        unsafe {
            let slice = std::slice::from_raw_parts(self.name, self.name_size);
            std::str::from_utf8_unchecked(slice)
        }
    }
}

struct CData {
    value: *const u8,
    value_size: usize,

    _pin: PhantomPinned,
}

impl CData {
    fn as_str(&self) -> &str {
        unsafe {
            let slice = std::slice::from_raw_parts(self.value, self.value_size);
            std::str::from_utf8_unchecked(slice)
        }
    }
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

impl Attribute {
    fn name_as_str(&self) -> &str {
        unsafe {
            let slice = std::slice::from_raw_parts(self.name, self.name_size);
            std::str::from_utf8_unchecked(slice)
        }
    }

    fn value_as_str(&self) -> &str {
        unsafe {
            let slice = std::slice::from_raw_parts(self.value, self.value_size);
            std::str::from_utf8_unchecked(slice)
        }
    }
}

trait ArenaExt {
    fn alloc_node(&self, payload: NodePayload) -> *mut Node;
    fn alloc_tag(&self, tag_name: &str) -> *mut Tag;
    fn alloc_cdata(&self, cdata_value: &str) -> *mut CData;
    fn alloc_attribute(&self, name: &str, value: &str) -> *mut Attribute;
}

impl ArenaExt for Arena {
    fn alloc_node(&self, payload: NodePayload) -> *mut Node {
        let node = self.alloc_struct::<Node>().unwrap().as_ptr();
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
        let tag = self.alloc_struct::<Tag>().unwrap().as_ptr();
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
        let cdata = self.alloc_struct::<CData>().unwrap().as_ptr();
        unsafe {
            (*cdata).value = value.as_ptr();
            (*cdata).value_size = value.len();
        }

        cdata
    }

    fn alloc_attribute(&self, name: &str, value: &str) -> *mut Attribute {
        let name = self.push_str(name);
        let value = self.push_str(value);
        let attribute = self.alloc_struct::<Attribute>().unwrap().as_ptr();
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

struct Visitor {
    going_down: bool,
    current: *mut Node,
}

enum VisitorStep<'a> {
    StartTag(&'a Tag),
    EndTag(&'a Tag),
    CData(&'a CData),
}

impl Visitor {
    fn new(start: *mut Node) -> Visitor {
        Visitor {
            going_down: true,
            current: start,
        }
    }

    fn step(&mut self) {
        unsafe {
            if self.going_down {
                if let NodePayload::Tag(tag) = (*self.current).payload {
                    let child = (*tag).children;
                    if !child.is_null() {
                        self.current = child;
                        return;
                    }
                };
            };
            let next = (*self.current).next;
            if next.is_null() {
                self.current = (*self.current).parent;
                self.going_down = false;
            } else {
                self.current = next;
                self.going_down = true;
            }
        }
    }

    fn next(&mut self) -> Option<VisitorStep> {
        if self.current.is_null() {
            return None;
        }
        unsafe {
            let old = self.current;
            let old_going_down = self.going_down;
            self.step();
            match (*old).payload {
                NodePayload::Tag(tag) => {
                    if old_going_down {
                        Some(VisitorStep::StartTag(&*tag))
                    } else {
                        Some(VisitorStep::EndTag(&*tag))
                    }
                }
                NodePayload::CData(cdata) => Some(VisitorStep::CData(&*cdata)),
            }
        }
    }
}

impl Document {
    pub fn new(root_tag_name: &str) -> Document {
        let arena = Arena::new().unwrap();
        let tag = arena.alloc_tag(root_tag_name);
        let node = arena.alloc_node(NodePayload::Tag(tag));

        Document {
            arena,
            root_node: node.into(),
        }
    }

    pub fn from_str(xml_str: &str) -> Result<Document, DocumentError> {
        let mut parser = DocumentParser::new();
        parser.parse_bytes(xml_str.as_bytes())?;
        parser.into_document()
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

    pub fn insert_tag<'a>(&'a self, tag_name: &str) -> Cursor<'a> {
        self.root().insert_tag(self, tag_name)
    }

    pub fn insert_cdata<'a>(&'a self, cdata: &str) -> Cursor<'a> {
        self.root().insert_cdata(self, cdata)
    }

    pub fn str_size(&self) -> usize {
        self.root().str_size()
    }

    pub fn to_string(&self) -> String {
        self.root().to_string()
    }
}

impl<'a> std::fmt::Display for Document {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.root().fmt(f)
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

    fn get_node_ptr(&self) -> *mut Node {
        unsafe { *self.node.get() }
    }

    fn visitor(&self) -> Visitor {
        unsafe { Visitor::new(*self.node.get()) }
    }

    //
    // Edit methods
    //

    pub fn insert_tag<'b, 'c>(&'a self, document: &'b Document, tag_name: &'c str) -> Cursor<'b> {
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

    pub fn append_tag<'b, 'c>(&'a self, document: &'b Document, tag_name: &'c str) -> Cursor<'b> {
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

    pub fn prepend_tag<'b, 'c>(&'a self, document: &'b Document, tag_name: &'c str) -> Cursor<'b> {
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

            let previous = (*node).previous;
            (*new_node).previous = previous;
            if previous.is_null() {
                match (*parent).payload {
                    NodePayload::CData(_) => {
                        // We never create a node under a non Tag node
                        unreachable!();
                    }
                    NodePayload::Tag(tag) => {
                        (*tag).children = new_node;
                    }
                }
            } else {
                (*previous).next = new_node;
            }
            (*new_node).next = node;
            (*node).previous = new_node;

            Cursor::new(new_node)
        }
    }

    pub fn set_attribute<'b, 'c>(
        &'a self,
        document: &'b Document,
        name: &'c str,
        value: &'c str,
    ) -> Cursor<'b> {
        null_cursor_guard!(self);

        let value = document.arena.push_str(value);
        unsafe {
            let node = *self.node.get();
            match (*node).payload {
                NodePayload::CData(_) => {
                    // Cannot set attributes on a cdata element
                    null_cursor!()
                }
                NodePayload::Tag(tag) => {
                    let mut attr = (*tag).attributes;
                    while !attr.is_null() {
                        if name == (*attr).name_as_str() {
                            // Existing attribute, change the value
                            (*attr).value = value.as_ptr();
                            (*attr).value_size = value.len();
                            return Cursor::new(node);
                        }
                        attr = (*attr).next;
                    }
                    // Add a new attribute
                    let attribute = document.arena.alloc_attribute(name, value);
                    if (*tag).attributes.is_null() {
                        (*tag).attributes = attribute;
                    }
                    if !(*tag).last_attribute.is_null() {
                        (*(*tag).last_attribute).next = attribute;
                        (*attribute).previous = (*tag).last_attribute;
                    }
                    (*tag).last_attribute = attribute;

                    Cursor::new(node)
                }
            }
        }
    }

    pub fn insert_cdata<'b, 'c>(&'a self, document: &'b Document, cdata: &'c str) -> Cursor<'b> {
        null_cursor_guard!(self);

        unsafe {
            let node = *self.node.get();
            match (*node).payload {
                NodePayload::CData(_) => {
                    // Cannot insert a tag into a cdata element
                    null_cursor!()
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

    pub fn append_cdata<'b, 'c>(&'a self, document: &'b Document, cdata: &'c str) -> Cursor<'b> {
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

    pub fn prepend_cdata<'b, 'c>(&'a self, document: &'b Document, cdata: &'c str) -> Cursor<'b> {
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

            let previous = (*node).previous;
            (*new_node).previous = previous;
            if previous.is_null() {
                match (*parent).payload {
                    NodePayload::CData(_) => {
                        // We never create a node under a non Tag node
                        unreachable!();
                    }
                    NodePayload::Tag(tag) => {
                        (*tag).children = new_node;
                    }
                }
            } else {
                (*previous).next = new_node;
            }
            (*new_node).next = node;
            (*node).previous = new_node;

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
                NodePayload::Tag(tag) => Cursor::new((*tag).children),
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

    pub fn name(&self) -> &str {
        unsafe {
            let node = *self.node.get();
            if node.is_null() {
                return "";
            }
            match (*node).payload {
                NodePayload::CData(_) => {
                    // Not a tag
                    ""
                }
                NodePayload::Tag(tag) => (*tag).as_str(),
            }
        }
    }

    pub fn str_size(&self) -> usize {
        unsafe {
            if (*self.node.get()).is_null() {
                return 0;
            }
        }

        let mut size = 0;
        let mut visitor = self.visitor();
        while let Some(step) = visitor.next() {
            match step {
                VisitorStep::StartTag(tag) => {
                    size += 1; // Tag opening '<'
                    size += tag.name_size;
                    let mut attr = (*tag).attributes;
                    while !attr.is_null() {
                        size += 1; // space
                        unsafe {
                            size += (*attr).name_size;
                            size += 2; // =" characters
                            size += escaped_size((*attr).value_as_str());
                            size += 1; // " character
                            attr = (*attr).next;
                        }
                    }
                    if tag.children.is_null() {
                        size += 2; // Standalone tag closing '/>'
                    } else {
                        size += 1;
                    }
                }
                VisitorStep::EndTag(tag) => {
                    if tag.children.is_null() {
                        // Already handled
                    } else {
                        size += 2; // End tag opening '</'
                        size += tag.name_size;
                        size += 1; // End tag closing '>'
                    }
                }
                VisitorStep::CData(cdata) => {
                    size += escaped_size(cdata.as_str());
                }
            }
        }

        size
    }

    fn to_string(&self) -> String {
        let mut buf = String::with_capacity(self.str_size());

        let mut visitor = self.visitor();
        while let Some(step) = visitor.next() {
            match step {
                VisitorStep::StartTag(tag) => {
                    buf.push('<');
                    buf.push_str(tag.as_str());
                    let mut attr = (*tag).attributes;
                    while !attr.is_null() {
                        buf.push(' ');
                        unsafe {
                            buf.push_str((*attr).name_as_str());
                            buf.push_str("=\"");
                            escape((*attr).value_as_str(), &mut buf);
                            buf.push('"');
                            attr = (*attr).next;
                        }
                    }
                    if tag.children.is_null() {
                        buf.push_str("/>");
                    } else {
                        buf.push('>');
                    }
                }
                VisitorStep::EndTag(tag) => {
                    if tag.children.is_null() {
                        // Already handled
                    } else {
                        buf.push_str("</");
                        buf.push_str(tag.as_str());
                        buf.push('>');
                    }
                }
                VisitorStep::CData(cdata) => {
                    escape(cdata.as_str(), &mut buf);
                }
            }
        }

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

        let mut visitor = self.visitor();
        while let Some(step) = visitor.next() {
            match step {
                VisitorStep::StartTag(tag) => {
                    f.write_str("<")?;
                    f.write_str(tag.as_str())?;
                    let mut attr = (*tag).attributes;
                    while !attr.is_null() {
                        f.write_str(" ")?;
                        unsafe {
                            f.write_str((*attr).name_as_str())?;
                            f.write_str("=\"")?;
                            escape_fmt((*attr).value_as_str(), f)?;
                            f.write_str("\"")?;
                            attr = (*attr).next;
                        }
                    }
                    if tag.children.is_null() {
                        f.write_str("/>")?;
                    } else {
                        f.write_str(">")?;
                    }
                }
                VisitorStep::EndTag(tag) => {
                    if tag.children.is_null() {
                        // Already handled
                    } else {
                        f.write_str("</")?;
                        f.write_str(tag.as_str())?;
                        f.write_str(">")?;
                    }
                }
                VisitorStep::CData(cdata) => {
                    escape_fmt(cdata.as_str(), f)?;
                }
            }
        }

        Result::Ok(())
    }
}

#[cfg(test)]
mod tests;

// FIXME: error line/col/byte passing
// FIXME: insert cdata merge

// FIXME: MaybeUninit?
// FIXME: NodePayload niche optimization
// FIXME: unit tests
// FIXME: docs
// FIXME: Cursor and navigation funcs
// FIXME: Drop
// FIXME: find/xpath funcs
// FIXME: delete funcs
// FIXME: clone
// FIXME: node property funcs

// FIXME: hide attribute
