/*
** This file is a part of Iksemel (XML parser for Jabber/XMPP)
** Copyright (C) 2000-2025 Gurer Ozen
**
** Iksemel is free software: you can redistribute it and/or modify it
** under the terms of the GNU Lesser General Public License as
** published by the Free Software Foundation, either version 3 of
** the License, or (at your option) any later version.
*/

use std::marker::Send;
use std::ptr::null_mut;
use std::sync::Arc;
use std::sync::Mutex;

use super::Attribute;
use super::Node;
use super::NodePayload;
use super::sync_iterators::SyncChildren;
use crate::Cursor;
use crate::Document;
use crate::ParseError;

pub struct SyncAttributes {
    sync_cursor: SyncCursor,
    current: *mut Attribute,
}

impl SyncAttributes {
    pub fn new(sync_cursor: &SyncCursor) -> Self {
        let _document = sync_cursor.document.lock().unwrap();
        unsafe {
            let attr = if sync_cursor.node.is_null() {
                null_mut::<Attribute>()
            } else {
                match (*sync_cursor.node).payload {
                    NodePayload::Tag(tag) => (*tag).attributes,
                    NodePayload::CData(_) => null_mut::<Attribute>(),
                }
            };
            SyncAttributes {
                sync_cursor: sync_cursor.clone(),
                current: attr,
            }
        }
    }
}

impl Iterator for SyncAttributes {
    type Item = (String, String);

    fn next(&mut self) -> Option<Self::Item> {
        if self.current.is_null() {
            return None;
        }
        let _document = self.sync_cursor.document.lock().unwrap();
        unsafe {
            let result = Some((
                (*self.current).name_as_str().to_string(),
                (*self.current).value_as_str().to_string(),
            ));
            self.current = (*self.current).next;
            result
        }
    }
}

pub struct SyncCursor {
    document: Arc<Mutex<Document>>,
    node: *mut Node,
}

macro_rules! tag_edit_method {
    ($method:ident) => {
        pub fn $method(mut self, tag_name: &str) -> Result<Self, ParseError> {
            {
                let document = self.document.lock().unwrap();
                let current = Cursor::new(self.node, &document.arena);
                let new = current.$method(tag_name)?;
                self.node = new.get_node_ptr();
            }
            Ok(self)
        }
    };
}

macro_rules! cdata_edit_method {
    ($method:ident) => {
        pub fn $method(mut self, cdata: &str) -> Result<Self, ParseError> {
            {
                let document = self.document.lock().unwrap();
                let current = Cursor::new(self.node, &document.arena);
                let new = current.$method(cdata)?;
                self.node = new.get_node_ptr();
            }
            Ok(self)
        }
    };
}

macro_rules! navigation_method {
    ($method:ident) => {
        pub fn $method(mut self) -> Self {
            {
                let document = self.document.lock().unwrap();
                let new = Cursor::new(self.node, &document.arena).$method();
                self.node = new.get_node_ptr();
            }
            self
        }
    };
}

impl SyncCursor {
    pub fn new(document: Document) -> Self {
        let node = document.root().get_node_ptr();
        let document = Arc::new(Mutex::new(document));
        Self { document, node }
    }

    //
    // Edit
    //

    tag_edit_method!(insert_tag);
    tag_edit_method!(append_tag);
    tag_edit_method!(prepend_tag);
    cdata_edit_method!(insert_cdata);
    cdata_edit_method!(append_cdata);
    cdata_edit_method!(prepend_cdata);

    /// Insert an attribute into the current tag element.
    ///
    /// # Errors:
    ///
    /// Returns `ParseError::BadXml` if the attribute already exists.
    ///
    /// # Panics
    ///
    /// Panics if the mutex is poisoned.
    ///
    pub fn insert_attribute<'b>(
        mut self,
        name: &'b str,
        value: &'b str,
    ) -> Result<Self, ParseError> {
        {
            let document = self.document.lock().unwrap();
            let current = Cursor::new(self.node, &document.arena);
            let new = current.insert_attribute(name, value)?;
            self.node = new.get_node_ptr();
        }
        Ok(self)
    }

    /// Sets or clears an attribute of the current tag element.
    ///
    /// # Panics
    ///
    /// Panics if the mutex is poisoned.
    ///
    pub fn set_attribute<'b>(
        mut self,
        name: &'b str,
        value: Option<&'b str>,
    ) -> Result<Self, ParseError> {
        {
            let document = self.document.lock().unwrap();
            let current = Cursor::new(self.node, &document.arena);
            let new = current.set_attribute(name, value)?;
            self.node = new.get_node_ptr();
        }
        Ok(self)
    }

    /// Removes the tag element from the document.
    ///
    /// # Panics
    ///
    /// Panics if the mutex is poisoned.
    ///
    pub fn remove(self) {
        let document = self.document.lock().unwrap();
        let current = Cursor::new(self.node, &document.arena);
        current.remove();
    }

    //
    // Navigation
    //

    navigation_method!(next);
    navigation_method!(next_tag);
    navigation_method!(previous);
    navigation_method!(previous_tag);
    navigation_method!(parent);
    navigation_method!(root);
    navigation_method!(first_child);
    navigation_method!(last_child);
    navigation_method!(first_tag);

    //
    // Iterators
    //

    pub fn attributes(self) -> SyncAttributes {
        SyncAttributes::new(&self)
    }

    pub fn children(&self) -> SyncChildren {
        SyncChildren::new(self)
    }

    /// Returns the first child tag element with the given name.
    ///
    /// # Panics
    ///
    /// Panics if the mutex is poisoned.
    ///
    pub fn find_tag(mut self, tag_name: &str) -> Self {
        {
            let document = self.document.lock().unwrap();
            let next = Cursor::new(self.node, &document.arena).find_tag(tag_name);
            self.node = next.get_node_ptr();
        }
        self
    }

    //
    // Properties
    //

    pub fn is_null(&self) -> bool {
        self.node.is_null()
    }

    pub fn is_tag(&self) -> bool {
        unsafe {
            if self.node.is_null() {
                return false;
            }
            match (*self.node).payload {
                NodePayload::CData(_) => false,
                NodePayload::Tag(_) => true,
            }
        }
    }

    pub fn name(&self) -> &str {
        unsafe {
            // SAFETY: Arc guarantees the node memory is not dropped.
            // Since the edits can only unlink a node, and can never change
            // the tag name of the node, it is safe to access without
            // locking the mutex. Leaked reference is also pointing
            // to an immutable memory as long as self is alive.
            if self.node.is_null() {
                return "";
            }
            match (*self.node).payload {
                NodePayload::CData(_) => {
                    // Not a tag
                    ""
                }
                NodePayload::Tag(tag) => (*tag).as_str(),
            }
        }
    }

    /// Returns the value of the given attribute.
    ///
    /// # Panics
    ///
    /// Panics if the mutex is poisoned.
    ///
    pub fn attribute(&self, name: &str) -> Option<&str> {
        if self.node.is_null() {
            return None;
        }
        unsafe {
            if let NodePayload::Tag(tag) = (*self.node).payload {
                let _document = self.document.lock().unwrap();
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
            if self.node.is_null() {
                return "";
            }
            match (*self.node).payload {
                NodePayload::CData(cdata) => (*cdata).as_str(),
                NodePayload::Tag(_) => {
                    // Not a CData
                    ""
                }
            }
        }
    }

    /// Returns the length of the XML string representation.
    ///
    /// # Panics
    ///
    /// Panics if the mutex is poisoned.
    ///
    pub fn str_size(&self) -> usize {
        let document = self.document.lock().unwrap();
        Cursor::new(self.node, &document.arena).str_size()
    }

    /// Returns the XML string representation.
    ///
    /// # Panics
    ///
    /// Panics if the mutex is poisoned.
    ///
    #[expect(
        clippy::inherent_to_string_shadow_display,
        reason = "prereserving exact capacity makes this method significantly faster"
    )]
    pub fn to_string(&self) -> String {
        let document = self.document.lock().unwrap();
        Cursor::new(self.node, &document.arena).to_string()
    }
}

impl Clone for SyncCursor {
    fn clone(&self) -> Self {
        Self {
            document: self.document.clone(),
            node: self.node,
        }
    }
}

impl std::fmt::Display for SyncCursor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let document = self.document.lock().unwrap();
        let cursor = Cursor::new(self.node, &document.arena);
        std::fmt::Display::fmt(&cursor, f)
    }
}

unsafe impl Send for SyncCursor {}

unsafe impl Sync for SyncCursor {}
