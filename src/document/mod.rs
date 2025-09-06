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
mod iterators;
mod parser;

use std::cell::UnsafeCell;
use std::fmt::Debug;
use std::marker::PhantomPinned;
use std::ptr::NonNull;
use std::ptr::null_mut;
use std::str::FromStr;

use crate::NoMemory;
use crate::document::error::description;

use super::arena::Arena;
use super::arena::ArenaStats;
use super::entities::escape;
use super::entities::escape_fmt;
use super::entities::escaped_size;
pub use error::DocumentError;
pub use iterators::Attributes;
pub use iterators::Children;
pub use iterators::DescendantOrSelf;
pub use parser::DocumentParser;

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
    fn alloc_node(&self, payload: NodePayload) -> Result<NonNull<Node>, NoMemory>;
    fn alloc_tag(&self, tag_name: &str) -> Result<NonNull<Tag>, NoMemory>;
    fn alloc_cdata(&self, cdata_value: &str) -> Result<NonNull<CData>, NoMemory>;
    fn alloc_attribute(&self, name: &str, value: &str) -> Result<NonNull<Attribute>, NoMemory>;
}

impl ArenaExt for Arena {
    fn alloc_node(&self, payload: NodePayload) -> Result<NonNull<Node>, NoMemory> {
        let node = self.alloc_struct::<Node>()?.as_ptr();
        unsafe {
            (*node).next = null_mut();
            (*node).previous = null_mut();
            (*node).parent = null_mut();
            (*node).payload = payload;
        }

        Ok(NonNull::new(node).unwrap())
    }

    fn alloc_tag(&self, tag_name: &str) -> Result<NonNull<Tag>, NoMemory> {
        let name = self.push_str(tag_name)?;
        let tag = self.alloc_struct::<Tag>()?.as_ptr();
        unsafe {
            (*tag).children = null_mut();
            (*tag).last_child = null_mut();
            (*tag).attributes = null_mut();
            (*tag).last_attribute = null_mut();
            (*tag).name = name.as_ptr();
            (*tag).name_size = name.len();
        }

        Ok(NonNull::new(tag).unwrap())
    }

    fn alloc_cdata(&self, cdata_value: &str) -> Result<NonNull<CData>, NoMemory> {
        let value = self.push_str(cdata_value)?;
        let cdata = self.alloc_struct::<CData>()?.as_ptr();
        unsafe {
            (*cdata).value = value.as_ptr();
            (*cdata).value_size = value.len();
        }

        Ok(NonNull::new(cdata).unwrap())
    }

    fn alloc_attribute(&self, name: &str, value: &str) -> Result<NonNull<Attribute>, NoMemory> {
        let name = self.push_str(name)?;
        let value = self.push_str(value)?;
        let attribute = self.alloc_struct::<Attribute>()?.as_ptr();
        unsafe {
            (*attribute).next = null_mut();
            (*attribute).previous = null_mut();
            (*attribute).name = name.as_ptr();
            (*attribute).name_size = name.len();
            (*attribute).value = value.as_ptr();
            (*attribute).value_size = value.len();

            Ok(NonNull::new_unchecked(attribute))
        }
    }
}

struct Visitor {
    going_down: bool,
    current: *mut Node,
    level: usize,
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
            level: 0,
        }
    }

    fn step(&mut self) {
        unsafe {
            if self.going_down
                && let NodePayload::Tag(tag) = (*self.current).payload
            {
                let child = (*tag).children;
                if !child.is_null() {
                    self.current = child;
                    self.level += 1;
                    return;
                }
            }
            if self.level == 0 {
                self.current = null_mut();
                return;
            }
            let next = (*self.current).next;
            if next.is_null() {
                self.level -= 1;
                self.current = (*self.current).parent;
                self.going_down = false;
            } else {
                self.current = next;
                self.going_down = true;
            }
        }
    }

    fn next(&mut self) -> Option<VisitorStep<'_>> {
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

pub struct Document {
    arena: Arena,
    root_node: UnsafeCell<*mut Node>,
}

impl Document {
    pub fn new(root_tag_name: &str) -> Result<Document, DocumentError> {
        let arena = Arena::new()?;
        let tag = arena.alloc_tag(root_tag_name)?.as_ptr();
        let node = arena.alloc_node(NodePayload::Tag(tag))?.as_ptr();

        Ok(Document {
            arena,
            root_node: node.into(),
        })
    }

    pub fn root<'a>(&'a self) -> Cursor<'a> {
        unsafe {
            let node = *self.root_node.get();
            Cursor::new(node, &self.arena)
        }
    }

    pub fn arena_stats(&self) -> ArenaStats {
        self.arena.stats()
    }

    //
    // Convenience functions to avoid typing .root() all the time
    //

    pub fn insert_tag<'a>(&'a self, tag_name: &str) -> Result<Cursor<'a>, DocumentError> {
        self.root().insert_tag(tag_name)
    }

    pub fn insert_cdata<'a>(&'a self, cdata: &str) -> Result<Cursor<'a>, DocumentError> {
        self.root().insert_cdata(cdata)
    }

    pub fn first_child<'a>(&'a self) -> Cursor<'a> {
        self.root().first_child()
    }

    pub fn first_tag<'a>(&'a self) -> Cursor<'a> {
        self.root().first_tag()
    }

    pub fn find_tag<'a>(&'a self, name: &str) -> Cursor<'a> {
        self.root().find_tag(name)
    }

    pub fn str_size(&self) -> usize {
        self.root().str_size()
    }

    #[allow(
        clippy::inherent_to_string_shadow_display,
        reason = "prereserving exact capacity makes this function significantly faster"
    )]
    pub fn to_string(&self) -> String {
        self.root().to_string()
    }
}

impl std::fmt::Display for Document {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(&self.root(), f)
    }
}

impl FromStr for Document {
    type Err = DocumentError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut parser = DocumentParser::new();
        parser.parse_bytes(s.as_bytes())?;
        parser.into_document()
    }
}

macro_rules! null_cursor {
    ($x:expr) => {
        Cursor::new(null_mut() as *mut Node, $x.arena)
    };
}

macro_rules! null_cursor_guard {
    ($x:expr) => {
        unsafe {
            if (*$x.node.get()).is_null() {
                return null_cursor!($x);
            }
        }
    };
}

macro_rules! cursor_edit_guards {
    ($self:ident) => {{
        let node = $self.get_node_ptr();
        if node.is_null() {
            return Err(DocumentError::BadXml(description::NULL_CURSOR_EDIT));
        }
        node
    }};
}

pub struct Cursor<'a> {
    node: UnsafeCell<*mut Node>,
    arena: &'a Arena,
}

impl<'a> Cursor<'a> {
    fn new(node: *mut Node, arena: &'a Arena) -> Cursor<'a> {
        Cursor {
            node: node.into(),
            arena,
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

    pub fn insert_tag<'b>(self, tag_name: &'b str) -> Result<Cursor<'a>, DocumentError> {
        let node = cursor_edit_guards!(self);

        unsafe {
            match (*node).payload {
                NodePayload::CData(_) => {
                    // Cannot insert a tag into a cdata element
                    Err(DocumentError::BadXml(description::CDATA_CHILDREN))
                }
                NodePayload::Tag(tag) => {
                    let new_tag = self.arena.alloc_tag(tag_name)?.as_ptr();
                    let new_node = self.arena.alloc_node(NodePayload::Tag(new_tag))?.as_ptr();

                    (*new_node).parent = node;
                    if (*tag).children.is_null() {
                        (*tag).children = new_node;
                    }
                    if !(*tag).last_child.is_null() {
                        (*(*tag).last_child).next = new_node;
                        (*new_node).previous = (*tag).last_child;
                    }
                    (*tag).last_child = new_node;

                    Ok(Cursor::new(new_node, self.arena))
                }
            }
        }
    }

    pub fn append_tag<'b>(self, tag_name: &'b str) -> Result<Cursor<'a>, DocumentError> {
        let node = cursor_edit_guards!(self);

        unsafe {
            if (*node).parent.is_null() {
                return Err(DocumentError::BadXml(description::ROOT_SIBLING));
            }

            let new_tag = self.arena.alloc_tag(tag_name)?.as_ptr();
            let new_node = self.arena.alloc_node(NodePayload::Tag(new_tag))?.as_ptr();

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

            Ok(Cursor::new(new_node, self.arena))
        }
    }

    pub fn prepend_tag<'b>(self, tag_name: &'b str) -> Result<Cursor<'a>, DocumentError> {
        let node = cursor_edit_guards!(self);

        unsafe {
            if (*node).parent.is_null() {
                return Err(DocumentError::BadXml(description::ROOT_SIBLING));
            }

            let new_tag = self.arena.alloc_tag(tag_name)?.as_ptr();
            let new_node = self.arena.alloc_node(NodePayload::Tag(new_tag))?.as_ptr();

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

            Ok(Cursor::new(new_node, self.arena))
        }
    }

    pub fn insert_attribute<'b>(
        &self,
        name: &'b str,
        value: &'b str,
    ) -> Result<Cursor<'a>, DocumentError> {
        let node = cursor_edit_guards!(self);

        unsafe {
            match (*node).payload {
                NodePayload::CData(_) => Err(DocumentError::BadXml(description::CDATA_ATTRIBUTE)),
                NodePayload::Tag(tag) => {
                    let mut attr = (*tag).attributes;
                    while !attr.is_null() {
                        if name == (*attr).name_as_str() {
                            // Two attributes with the same name
                            return Err(DocumentError::BadXml(description::DUPLICATE_ATTRIBUTE));
                        }
                        attr = (*attr).next;
                    }
                    // Add the new attribute
                    let attribute = self.arena.alloc_attribute(name, value)?.as_ptr();
                    if (*tag).attributes.is_null() {
                        (*tag).attributes = attribute;
                    }
                    if !(*tag).last_attribute.is_null() {
                        (*(*tag).last_attribute).next = attribute;
                        (*attribute).previous = (*tag).last_attribute;
                    }
                    (*tag).last_attribute = attribute;

                    Ok(Cursor::new(node, self.arena))
                }
            }
        }
    }

    pub fn set_attribute<'b>(
        &self,
        name: &'b str,
        value: Option<&'b str>,
    ) -> Result<Cursor<'a>, DocumentError> {
        let node = cursor_edit_guards!(self);

        unsafe {
            match (*node).payload {
                NodePayload::CData(_) => Err(DocumentError::BadXml(description::CDATA_ATTRIBUTE)),
                NodePayload::Tag(tag) => {
                    let mut attr = (*tag).attributes;
                    while !attr.is_null() {
                        if name == (*attr).name_as_str() {
                            // Existing attribute, change the value
                            match value {
                                None => {
                                    if !(*attr).next.is_null() {
                                        (*(*attr).next).previous = (*attr).previous;
                                    }
                                    if !(*attr).previous.is_null() {
                                        (*(*attr).previous).next = (*attr).next;
                                    }
                                    if (*tag).attributes == attr {
                                        (*tag).attributes = (*attr).next;
                                    }
                                    if (*tag).last_attribute == attr {
                                        (*tag).last_attribute = (*attr).previous;
                                    }
                                }
                                Some(value) => {
                                    let value = self.arena.push_str(value)?;
                                    (*attr).value = value.as_ptr();
                                    (*attr).value_size = value.len();
                                    return Ok(Cursor::new(node, self.arena));
                                }
                            }
                        }
                        attr = (*attr).next;
                    }
                    match value {
                        None => {
                            // Attribute already non existent
                            Ok(Cursor::new(node, self.arena))
                        }
                        Some(value) => {
                            // Add a new attribute
                            let attribute = self.arena.alloc_attribute(name, value)?.as_ptr();
                            if (*tag).attributes.is_null() {
                                (*tag).attributes = attribute;
                            }
                            if !(*tag).last_attribute.is_null() {
                                (*(*tag).last_attribute).next = attribute;
                                (*attribute).previous = (*tag).last_attribute;
                            }
                            (*tag).last_attribute = attribute;

                            Ok(Cursor::new(node, self.arena))
                        }
                    }
                }
            }
        }
    }

    pub fn insert_cdata<'b>(self, cdata: &'b str) -> Result<Cursor<'a>, DocumentError> {
        let node = cursor_edit_guards!(self);

        unsafe {
            match (*node).payload {
                NodePayload::CData(_) => Err(DocumentError::BadXml(description::CDATA_CHILDREN)),
                NodePayload::Tag(tag) => {
                    let last = (*tag).last_child;
                    if !last.is_null()
                        && let NodePayload::CData(cdata_node) = (*last).payload
                    {
                        let old_s = (*cdata_node).as_str();
                        let s = self.arena.concat_str(old_s, cdata)?;
                        (*cdata_node).value = s.as_ptr();
                        (*cdata_node).value_size = s.len();

                        return Ok(Cursor::new(last, self.arena));
                    }

                    let new_cdata = self.arena.alloc_cdata(cdata)?.as_ptr();
                    let new_node = self
                        .arena
                        .alloc_node(NodePayload::CData(new_cdata))?
                        .as_ptr();

                    (*new_node).parent = node;
                    if (*tag).children.is_null() {
                        (*tag).children = new_node;
                    }
                    if !last.is_null() {
                        (*last).next = new_node;
                        (*new_node).previous = last;
                    }
                    (*tag).last_child = new_node;

                    Ok(Cursor::new(new_node, self.arena))
                }
            }
        }
    }

    pub fn append_cdata<'b>(self, cdata: &'b str) -> Result<Cursor<'a>, DocumentError> {
        let node = cursor_edit_guards!(self);

        unsafe {
            if (*node).parent.is_null() {
                return Err(DocumentError::BadXml(description::ROOT_SIBLING));
            }

            let new_cdata = self.arena.alloc_cdata(cdata)?.as_ptr();
            let new_node = self
                .arena
                .alloc_node(NodePayload::CData(new_cdata))?
                .as_ptr();

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

            Ok(Cursor::new(new_node, self.arena))
        }
    }

    pub fn prepend_cdata<'b>(self, cdata: &'b str) -> Result<Cursor<'a>, DocumentError> {
        let node = cursor_edit_guards!(self);

        unsafe {
            if (*node).parent.is_null() {
                return Err(DocumentError::BadXml(description::ROOT_SIBLING));
            }

            let new_cdata = self.arena.alloc_cdata(cdata)?.as_ptr();
            let new_node = self
                .arena
                .alloc_node(NodePayload::CData(new_cdata))?
                .as_ptr();

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

            Ok(Cursor::new(new_node, self.arena))
        }
    }

    pub fn remove(self) {
        let node = self.get_node_ptr();
        if node.is_null() {
            return;
        }
        unsafe {
            let parent = (*node).parent;
            if parent.is_null() {
                // Cannot remove the root element
                return;
            }
            // Fix siblings
            if !(*node).next.is_null() {
                (*(*node).next).previous = (*node).previous;
            }
            if !(*node).previous.is_null() {
                (*(*node).previous).next = (*node).next;
            }
            // Fix parent
            match (*parent).payload {
                NodePayload::Tag(tag) => {
                    if (*tag).children == node {
                        (*tag).children = (*node).next;
                    }
                    if (*tag).last_child == node {
                        (*tag).last_child = (*node).previous;
                    }
                }
                NodePayload::CData(_) => {}
            }
            // Fix self
            (*node).parent = null_mut();
            (*node).next = null_mut();
            (*node).previous = null_mut();
        }
    }

    //
    // Navigation methods
    //

    fn clear(&mut self) {
        self.node = null_mut::<Node>().into();
    }

    pub fn next(self) -> Cursor<'a> {
        null_cursor_guard!(self);

        unsafe {
            let node = *self.node.get();
            Cursor::new((*node).next, self.arena)
        }
    }

    pub fn next_tag(self) -> Cursor<'a> {
        null_cursor_guard!(self);

        let mut next = self.next();
        loop {
            if next.is_null() || next.is_tag() {
                break;
            }
            next = next.next();
        }
        next
    }

    pub fn previous(self) -> Cursor<'a> {
        null_cursor_guard!(self);

        unsafe {
            let node = *self.node.get();
            Cursor::new((*node).previous, self.arena)
        }
    }

    pub fn previous_tag(self) -> Cursor<'a> {
        null_cursor_guard!(self);

        let mut next = self.previous();
        loop {
            if next.is_null() || next.is_tag() {
                break;
            }
            next = next.previous();
        }
        next
    }

    pub fn parent(self) -> Cursor<'a> {
        null_cursor_guard!(self);

        unsafe {
            let node = *self.node.get();
            Cursor::new((*node).parent, self.arena)
        }
    }

    pub fn root(self) -> Cursor<'a> {
        null_cursor_guard!(self);

        let mut current = self;
        loop {
            let parent = current.clone().parent();
            if parent.is_null() {
                break;
            }
            current = parent;
        }
        current
    }

    pub fn first_child(self) -> Cursor<'a> {
        null_cursor_guard!(self);

        unsafe {
            let node = *self.node.get();
            match (*node).payload {
                NodePayload::CData(_) => {
                    null_cursor!(self)
                }
                NodePayload::Tag(tag) => Cursor::new((*tag).children, self.arena),
            }
        }
    }

    pub fn last_child(self) -> Cursor<'a> {
        null_cursor_guard!(self);

        unsafe {
            let node = *self.node.get();
            match (*node).payload {
                NodePayload::CData(_) => {
                    null_cursor!(self)
                }
                NodePayload::Tag(tag) => Cursor::new((*tag).last_child, self.arena),
            }
        }
    }

    pub fn first_tag(self) -> Cursor<'a> {
        null_cursor_guard!(self);

        let child = self.clone().first_child();
        if child.is_null() {
            null_cursor!(self)
        } else if child.is_tag() {
            child
        } else {
            child.next_tag()
        }
    }

    pub fn find_tag(self, name: &str) -> Cursor<'a> {
        let mut child = self.first_child();
        while !child.is_null() {
            if child.name() == name {
                break;
            }
            child = child.next();
        }
        child
    }

    //
    // Iterator methods
    //

    pub fn children(self) -> Children<'a> {
        Children::new(self.first_child())
    }

    pub fn attributes(self) -> Attributes<'a> {
        Attributes::new(self.clone())
    }

    pub fn descendant_or_self(self) -> DescendantOrSelf<'a> {
        DescendantOrSelf::new(self.clone())
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

    pub fn is_tag(&self) -> bool {
        unsafe {
            let node = *self.node.get();
            if node.is_null() {
                return false;
            }
            match (*node).payload {
                NodePayload::CData(_) => false,
                NodePayload::Tag(_) => true,
            }
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

    pub fn attribute(&self, name: &str) -> Option<&str> {
        let node = self.get_node_ptr();
        if node.is_null() {
            return None;
        }
        unsafe {
            if let NodePayload::Tag(tag) = (*node).payload {
                let mut attr = (*tag).attributes;
                while !attr.is_null() {
                    let attr_name = (*attr).name_as_str();
                    if attr_name == name {
                        return Some((*attr).value_as_str());
                    }
                    attr = (*attr).next;
                }
            }
        }
        None
    }

    pub fn cdata(&self) -> &str {
        unsafe {
            let node = *self.node.get();
            if node.is_null() {
                return "";
            }
            match (*node).payload {
                NodePayload::CData(cdata) => (*cdata).as_str(),
                NodePayload::Tag(_) => {
                    // Not a CData
                    ""
                }
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
                    let mut attr = tag.attributes;
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

    #[allow(
        clippy::inherent_to_string_shadow_display,
        reason = "prereserving exact capacity makes this function significantly faster"
    )]
    fn to_string(&self) -> String {
        let mut buf = String::with_capacity(self.str_size());

        let mut visitor = self.visitor();
        while let Some(step) = visitor.next() {
            match step {
                VisitorStep::StartTag(tag) => {
                    buf.push('<');
                    buf.push_str(tag.as_str());
                    let mut attr = tag.attributes;
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

impl Clone for Cursor<'_> {
    fn clone(&self) -> Self {
        Cursor {
            node: self.get_node_ptr().into(),
            arena: self.arena,
        }
    }
}

impl Debug for Cursor<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Cursor ({:?})", self.get_node_ptr())
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
                    let mut attr = tag.attributes;
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

mod nocompile;
