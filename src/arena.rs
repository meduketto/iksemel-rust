/*
** This file is a part of Iksemel (XML parser for Jabber/XMPP)
** Copyright (C) 2000-2025 Gurer Ozen
**
** Iksemel is free software: you can redistribute it and/or modify it
** under the terms of the GNU Lesser General Public License as
** published by the Free Software Foundation, either version 3 of
** the License, or (at your option) any later version.
*/

use std::alloc::{alloc, handle_alloc_error, Layout};
use std::cmp;
use std::marker::PhantomPinned;
use std::ptr::null_mut;

const MIN_NODE_WORDS: usize = 32;

const MIN_DATA_BYTES: usize = 256;

pub struct Arena {
    info: *mut ArenaInfo,
}

struct ArenaInfo {
    allocated: usize,
    nr_allocations: u32,
    node_chunk: *mut ArenaChunk,
    data_chunk: *mut ArenaChunk,

    // Arena has a raw pointer to this struct
    _pin: PhantomPinned,
}

struct ArenaChunk {
    next: *mut ArenaChunk,
    size: usize,
    used: usize,
    last: *mut u8,
    last_size: usize,
    mem: *mut u8,

    // ArenaInfo and ArenaChunk has raw pointers to this struct
    _pin: PhantomPinned,
}


impl ArenaChunk {
    fn raw_init(self: &mut ArenaChunk, ptr: *mut u8, size: usize) -> () {
        self.next = null_mut();
        self.size = size;
        self.used = 0;
        self.last = ptr;
        self.last_size = 0;
        self.mem = ptr;
    }

    fn has_space(self: &mut ArenaChunk, size: usize) -> bool {
        return size < self.size && self.used + size <= self.size
    }

    pub fn make_space(self: &mut ArenaChunk, size: usize) -> *mut ArenaChunk {
unsafe {
        let mut current: *mut ArenaChunk = self;
        let mut expected_next_size = self.size;
        loop {
            if (*current).has_space(size) {
                // If I fits, I sits
                return current;
            }
            expected_next_size *= 2;
            if (*current).next.is_null() {
                let data_size = cmp::max(expected_next_size, size);
                let chunk_layout = Layout::new::<ArenaChunk>();
                let (data_layout, data_offset) = chunk_layout.extend(Layout::array::<u8>(data_size).unwrap()).unwrap();
                let new_layout = data_layout.pad_to_align();

                let chunk;
                unsafe {
                    let ptr = alloc(new_layout);
                    if ptr.is_null() {
                        handle_alloc_error(new_layout);
                    }
                    chunk = ptr as *mut ArenaChunk;
                    (*chunk).raw_init(ptr.byte_add(data_offset), data_size);
                }
                (*current).next = chunk;
            }
            current = (*current).next;
        }
    }
    }

    pub fn find_adjacent_space(self: &mut ArenaChunk, old_p: *const u8, old_size: usize, size: usize) -> Option<*mut ArenaChunk> {

        unsafe {
            let mut current: *mut ArenaChunk = self;
            loop {
                if old_p == (*current).last && old_size == (*current).last_size {
                    if (*current).has_space(size) {
                        return Some(current);
                    }
                }
                if (*current).next.is_null() {
                    return None;
                }
                current = (*current).next;
            }
        }

    }
}

impl Arena {
    pub fn new() -> Arena {
        // Minimums are defaults
        Self::with_chunk_sizes(0, 0)
    }

    pub fn with_chunk_sizes(node_size: usize, data_size: usize) -> Arena {
        // First node chunk should have enough capacity for at least 32 usize words.
        let min_node_layout = Layout::array::<usize>(MIN_NODE_WORDS).unwrap();
        let node_size = cmp::max(node_size, min_node_layout.size());
        let node_layout = Layout::array::<u8>(node_size).unwrap();

        // First data chunk should have enough capacity for at least 256 bytes.
        let data_size = cmp::max(data_size, MIN_DATA_BYTES);

        let info_layout = Layout::new::<ArenaInfo>();
        // Extending the layout for Chunk structures could NOT overflow the usize arithmetic,
        // therefore we just unwrap() the results.
        let (info_layout, node_offset) = info_layout.extend(Layout::new::<ArenaChunk>()).unwrap();
        let (info_layout, data_offset) = info_layout.extend(Layout::new::<ArenaChunk>()).unwrap();
        let (info_layout, node_buf_offset) = info_layout.extend(node_layout).unwrap();
        let (info_layout, data_buf_offset) = info_layout.extend(Layout::array::<u8>(data_size).unwrap()).unwrap();
        // Necessary to align the whole block to pointer/usize alignment
        let info_layout = info_layout.pad_to_align();

        let info;
        unsafe {
            let ptr = alloc(info_layout);
            if ptr.is_null() {
                handle_alloc_error(info_layout);
            }
            info = ptr as *mut ArenaInfo;
            (*info).allocated = info_layout.size();
            (*info).nr_allocations = 1;

            let node_ptr = ptr.byte_add(node_offset);
            let node = node_ptr as *mut ArenaChunk;
            (*info).node_chunk = node;

            let data_ptr = ptr.byte_add(data_offset);
            let data = data_ptr as *mut ArenaChunk;
            (*info).data_chunk = data;

            let node_buf_ptr = ptr.byte_add(node_buf_offset);
            (*node).raw_init(node_buf_ptr, node_layout.size());

            let data_buf_ptr = ptr.byte_add(data_buf_offset);
            (*data).raw_init(data_buf_ptr, data_size);
        }

        Arena {
            info: info,
        }
    }

    pub fn alloc(self: &mut Arena, layout: Layout) -> *mut u8 {
        let size = layout.size();
        unsafe {
            let chunk = (*(*self.info).node_chunk).make_space(size);
            (*chunk).used += size;
            return (*chunk).mem.byte_add(size);
        }
    }

    pub fn push_str<'a, 'b, 'c>(&'a mut self, s: &'b str) -> &'c str {
        let size = s.len();
        unsafe {
            let chunk = (*(*self.info).data_chunk).make_space(size);
            let p = (*chunk).mem.byte_add((*chunk).used);
            (*chunk).last = p;
            (*chunk).last_size = size;
            (*chunk).used += size;
            std::ptr::copy_nonoverlapping(s.as_ptr(), p, size);
            let r = std::slice::from_raw_parts(p, size);
            return std::str::from_utf8_unchecked(r);
        }
    }

    pub fn concat_str<'a,'b,'c,'d>(&'a mut self, old_s: &'b str, s: &'c str) -> &'d str {
        unsafe {
            let mut data_chunk = (*self.info).data_chunk;
            if let Some(chunk) = (*data_chunk).find_adjacent_space(old_s.as_ptr(), old_s.len(), s.len()) {
                // Enough space to extend the str
                let p = (*chunk).mem.byte_add((*chunk).used);
                (*chunk).used += s.len();
                (*chunk).last_size = old_s.len() + s.len();
                std::ptr::copy_nonoverlapping(s.as_ptr(), p, s.len());
                let r = std::slice::from_raw_parts(old_s.as_ptr(), old_s.len() + s.len());
                return std::str::from_utf8_unchecked(r);
            } else {
                let chunk = (*data_chunk).make_space(old_s.len() + s.len());
                let p = (*chunk).mem.byte_add((*chunk).used);
                (*chunk).last = p;
                (*chunk).last_size = old_s.len() + s.len();
                (*chunk).used += old_s.len() + s.len();
                std::ptr::copy_nonoverlapping(old_s.as_ptr(), p, old_s.len());
                let p2 = p.byte_add(old_s.len());
                std::ptr::copy_nonoverlapping(s.as_ptr(), p2, s.len());
                let r = std::slice::from_raw_parts(p, old_s.len() + s.len());
                return std::str::from_utf8_unchecked(r);
            }
        }
    }



    pub fn nr_allocations(&self) -> u32 {
        unsafe {
            return (*self.info).nr_allocations;
        }
    }

    pub fn nr_total_bytes(&self) -> usize {
        unsafe {
            return (*self.info).allocated;
        }
    }

}








#[cfg(test)]
mod tests {
    use super::*;

    const CHARS: &str = "1234567890abcdefghijklmnopqrstuv";

    #[test]
    fn it_works() {
        let mut arena = Arena::new();
        assert_eq!(arena.nr_allocations(), 1);
        assert!(arena.nr_total_bytes() > 0);

        let s = arena.push_str("test");
        assert_eq!(s, "test");

        let s2 = arena.concat_str(s, "moretest");
        assert_eq!(s2, "testmoretest");
    }

    #[test]
    fn many_pushes() {
        let mut arena = Arena::new();

        for _ in 0..1000 {
            for j in 0..CHARS.len() {
                arena.push_str(&CHARS[..j]);
            }
        }
        // FIXME: enable
        //assert!(arena.nr_allocations() > 1);
    }

    #[test]
    fn many_1char_pushes() {
        let mut arena = Arena::new();

        for _ in 0..10000 {
            arena.push_str("+");
        }
    }

    #[test]
    fn many_1char_concats() {
        let mut arena = Arena::new();
        let mut s = arena.push_str("");

        for i in 0..1000 {
            for n in 0..CHARS.len() {
                s = arena.concat_str(s, &CHARS.chars().nth(n).unwrap().to_string());
            }
            assert_eq!(s, CHARS.repeat(i + 1));
        }
    }

}


// FIXME: deal with last_size concat problem
// FIXME: ensure &str return lifetimes + bad tests dont compile

// FIXME: make_space unsafe reduction and format
// FIXME: mark allocations on the arena + enable test above
// FIXME: more unittests
// FIXME: docs
