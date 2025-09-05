/*
** This file is a part of Iksemel (XML parser for Jabber/XMPP)
** Copyright (C) 2000-2025 Gurer Ozen
**
** Iksemel is free software: you can redistribute it and/or modify it
** under the terms of the GNU Lesser General Public License as
** published by the Free Software Foundation, either version 3 of
** the License, or (at your option) any later version.
*/

use std::marker::PhantomData;
use std::ptr::null_mut;

use crate::Cursor;

use super::Attribute;
use super::NodePayload;

pub struct Attributes<'a> {
    current: *mut Attribute,
    marker: PhantomData<Cursor<'a>>,
}

impl<'a> Attributes<'a> {
    pub fn new(cursor: Cursor<'a>) -> Self {
        let node = cursor.get_node_ptr();
        if node.is_null() {
            return Attributes {
                current: null_mut(),
                marker: PhantomData,
            };
        }
        unsafe {
            let attr = match (*node).payload {
                NodePayload::Tag(tag) => (*tag).attributes,
                NodePayload::CData(_) => null_mut::<Attribute>(),
            };
            Attributes {
                current: attr,
                marker: PhantomData,
            }
        }
    }
}

impl<'a> Iterator for Attributes<'a> {
    type Item = (&'a str, &'a str);

    fn next(&mut self) -> Option<Self::Item> {
        if self.current.is_null() {
            return None;
        }
        unsafe {
            let result = Some((
                (*self.current).name_as_str(),
                (*self.current).value_as_str(),
            ));
            self.current = (*self.current).next;
            result
        }
    }
}

pub struct Children<'a> {
    current: Cursor<'a>,
}

impl<'a> Children<'a> {
    pub fn new(cursor: Cursor<'a>) -> Self {
        Children { current: cursor }
    }
}

impl<'a> Iterator for Children<'a> {
    type Item = Cursor<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current.is_null() {
            return None;
        }
        let result = self.current.clone();
        self.current = self.current.clone().next();
        Some(result)
    }
}

pub struct DescendantOrSelf<'a> {
    current: Cursor<'a>,
    level: usize,
    going_down: bool,
}

impl<'a> DescendantOrSelf<'a> {
    pub fn new(cursor: Cursor<'a>) -> Self {
        DescendantOrSelf {
            current: cursor,
            level: 0,
            going_down: true,
        }
    }
}

impl<'a> Iterator for DescendantOrSelf<'a> {
    type Item = Cursor<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current.is_null() {
            return None;
        }
        let result = self.current.clone();
        loop {
            if self.going_down {
                if self.current.is_tag() {
                    let child = self.current.clone().first_child();
                    if !child.is_null() {
                        self.current = child;
                        self.level += 1;
                        return Some(result);
                    }
                }
            };
            if self.level == 0 {
                self.current.clear();
                break;
            }
            let next = self.current.clone().next();
            if next.is_null() {
                self.level -= 1;
                self.current = self.current.clone().parent();
                self.going_down = false;
            } else {
                self.current = next;
                self.going_down = true;
                break;
            }
        }
        Some(result)
    }
}
