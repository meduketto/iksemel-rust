/*
** This file is a part of Iksemel (XML parser for Jabber/XMPP)
** Copyright (C) 2000-2025 Gurer Ozen
**
** Iksemel is free software: you can redistribute it and/or modify it
** under the terms of the GNU Lesser General Public License as
** published by the Free Software Foundation, either version 3 of
** the License, or (at your option) any later version.
*/

use std::alloc::{Layout, alloc, dealloc, handle_alloc_error};
use std::cell::UnsafeCell;
use std::cmp;
use std::marker::PhantomPinned;
use std::ptr::null_mut;

const MIN_NODE_WORDS: usize = 32;

const MIN_DATA_BYTES: usize = 256;

pub struct Arena {
    info: UnsafeCell<*mut ArenaInfo>,
}

struct ArenaInfo {
    nr_allocated_bytes: usize,
    nr_allocations: u32,
    node_chunk: *mut ArenaChunk,
    data_chunk: *mut ArenaChunk,
    chunk_layout: Layout,

    // Arena has a raw pointer to this struct
    _pin: PhantomPinned,
}

struct ArenaChunk {
    next: *mut ArenaChunk,
    size: usize,
    used: usize,
    last: *mut u8,
    mem: *mut u8,
    chunk_layout: Layout,

    // ArenaInfo and ArenaChunk has raw pointers to this struct
    _pin: PhantomPinned,
}

impl ArenaChunk {
    fn raw_init(self: &mut ArenaChunk, ptr: *mut u8, size: usize, chunk_layout: Layout) {
        self.next = null_mut();
        self.size = size;
        self.used = 0;
        self.last = ptr;
        self.mem = ptr;
        self.chunk_layout = chunk_layout;
    }

    fn has_space(self: &mut ArenaChunk, size: usize) -> bool {
        size < self.size && self.used + size <= self.size
    }

    fn make_space(self: &mut ArenaChunk, info: &mut ArenaInfo, size: usize) -> *mut ArenaChunk {
        let mut expected_next_size = self.size;
        let mut current: *mut ArenaChunk = self;
        unsafe {
            loop {
                if (*current).has_space(size) {
                    // If I fits, I sits
                    return current;
                }
                expected_next_size *= 2;
                if (*current).next.is_null() {
                    let data_size = cmp::max(expected_next_size, size);
                    let chunk_layout = Layout::new::<ArenaChunk>();
                    let (data_layout, data_offset) = chunk_layout
                        .extend(Layout::array::<u8>(data_size).unwrap())
                        .unwrap();
                    let new_layout = data_layout.pad_to_align();

                    let ptr = alloc(new_layout);
                    if ptr.is_null() {
                        handle_alloc_error(new_layout);
                    }
                    info.nr_allocations += 1;
                    info.nr_allocated_bytes += new_layout.size();

                    let chunk = ptr as *mut ArenaChunk;
                    (*chunk).raw_init(ptr.byte_add(data_offset), data_size, new_layout);
                    (*current).next = chunk;
                }
                current = (*current).next;
            }
        }
    }

    fn find_adjacent_space(
        self: &mut ArenaChunk,
        old_p: *const u8,
        old_size: usize,
        size: usize,
    ) -> Option<*mut ArenaChunk> {
        let mut current: *mut ArenaChunk = self;
        unsafe {
            loop {
                if std::ptr::addr_eq(old_p, (*current).last) {
                    let chunk_end = (*current).mem.byte_add((*current).used);
                    let old_end = old_p.byte_add(old_size);
                    if std::ptr::addr_eq(chunk_end, old_end) && (*current).has_space(size) {
                        return Some(current);
                    }
                    return None;
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
        let data_buf_layout = Layout::array::<u8>(data_size).unwrap();

        let info_layout = Layout::new::<ArenaInfo>();
        // Extending the layout for Chunk structures could NOT overflow the usize arithmetic,
        // therefore we just unwrap() the results.
        let (info_layout, node_offset) = info_layout.extend(Layout::new::<ArenaChunk>()).unwrap();
        let (info_layout, data_offset) = info_layout.extend(Layout::new::<ArenaChunk>()).unwrap();
        let (info_layout, node_buf_offset) = info_layout.extend(node_layout).unwrap();
        let (info_layout, data_buf_offset) = info_layout.extend(data_buf_layout).unwrap();
        // Necessary to align the whole block to pointer/usize alignment
        let info_layout = info_layout.pad_to_align();

        let info;
        unsafe {
            let ptr = alloc(info_layout);
            if ptr.is_null() {
                handle_alloc_error(info_layout);
            }
            info = ptr as *mut ArenaInfo;
            (*info).nr_allocated_bytes = info_layout.size();
            (*info).nr_allocations = 1;
            (*info).chunk_layout = info_layout;

            let node_ptr = ptr.byte_add(node_offset);
            let node = node_ptr as *mut ArenaChunk;
            (*info).node_chunk = node;

            let data_ptr = ptr.byte_add(data_offset);
            let data = data_ptr as *mut ArenaChunk;
            (*info).data_chunk = data;

            let node_buf_ptr = ptr.byte_add(node_buf_offset);
            (*node).raw_init(node_buf_ptr, node_size, node_layout);

            let data_buf_ptr = ptr.byte_add(data_buf_offset);
            (*data).raw_init(data_buf_ptr, data_size, data_buf_layout);
        }

        Arena { info: info.into() }
    }

    pub fn alloc(&self, layout: Layout) -> *mut u8 {
        let size = layout.size();
        unsafe {
            let info = &mut **self.info.get();
            let chunk = (*info.node_chunk).make_space(info, size);
            (*chunk).used += size;
            (*chunk).mem.byte_add(size)
        }
    }

    pub fn push_str<'a>(&'a self, s: &str) -> &'a str {
        let size = s.len();
        unsafe {
            let info = &mut **self.info.get();
            let chunk = (*info.data_chunk).make_space(info, size);
            let p = (*chunk).mem.byte_add((*chunk).used);
            (*chunk).last = p;
            (*chunk).used += size;
            std::ptr::copy_nonoverlapping(s.as_ptr(), p, size);
            let r = std::slice::from_raw_parts(p, size);
            std::str::from_utf8_unchecked(r)
        }
    }

    pub fn concat_str<'a>(&'a self, old_s: &str, s: &str) -> &'a str {
        unsafe {
            let info = &mut **self.info.get();
            let data_chunk = info.data_chunk;
            if let Some(chunk) =
                (*data_chunk).find_adjacent_space(old_s.as_ptr(), old_s.len(), s.len())
            {
                // Enough space to extend the str
                let p = (*chunk).mem.byte_add((*chunk).used);
                (*chunk).used += s.len();
                std::ptr::copy_nonoverlapping(s.as_ptr(), p, s.len());
                let r = std::slice::from_raw_parts(p.byte_sub(old_s.len()), old_s.len() + s.len());
                std::str::from_utf8_unchecked(r)
            } else {
                let chunk = (*data_chunk).make_space(info, old_s.len() + s.len());
                let p = (*chunk).mem.byte_add((*chunk).used);
                (*chunk).last = p;
                (*chunk).used += old_s.len() + s.len();
                std::ptr::copy_nonoverlapping(old_s.as_ptr(), p, old_s.len());
                let p2 = p.byte_add(old_s.len());
                std::ptr::copy_nonoverlapping(s.as_ptr(), p2, s.len());
                let r = std::slice::from_raw_parts(p, old_s.len() + s.len());
                std::str::from_utf8_unchecked(r)
            }
        }
    }

    pub fn nr_allocations(&self) -> u32 {
        unsafe {
            let info = &mut **self.info.get();
            info.nr_allocations
        }
    }

    pub fn nr_allocated_bytes(&self) -> usize {
        unsafe {
            let info = &mut **self.info.get();
            info.nr_allocated_bytes
        }
    }
}

impl Drop for Arena {
    fn drop(&mut self) {
        unsafe {
            let info = &mut **self.info.get_mut();
            let mut chunk = (*info.node_chunk).next;
            while !chunk.is_null() {
                let next = (*chunk).next;
                dealloc(chunk as *mut u8, (*chunk).chunk_layout);
                chunk = next;
            }
            let mut chunk = (*info.data_chunk).next;
            while !chunk.is_null() {
                let next = (*chunk).next;
                dealloc(chunk as *mut u8, (*chunk).chunk_layout);
                chunk = next;
            }
            dealloc(*self.info.get_mut() as *mut u8, info.chunk_layout);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const CHARS: &str = "1234567890abcdefghijklmnopqrstuv";

    #[test]
    fn it_works() {
        let arena = Arena::new();
        assert_eq!(arena.nr_allocations(), 1);
        assert!(arena.nr_allocated_bytes() > 0);

        let s = arena.push_str("test");
        assert_eq!(s, "test");

        let s2 = arena.concat_str(s, "moretest");
        assert_eq!(s2, "testmoretest");
    }

    #[test]
    fn many_pushes() {
        let arena = Arena::new();
        let old_bytes = arena.nr_allocated_bytes();

        for _ in 0..1000 {
            for j in 0..CHARS.len() {
                arena.push_str(&CHARS[..j]);
            }
        }
        assert!(arena.nr_allocations() > 1);
        assert!(arena.nr_allocated_bytes() > old_bytes);
    }

    #[test]
    fn many_1char_pushes() {
        let arena = Arena::new();

        for _ in 0..10000 {
            arena.push_str("+");
        }
    }

    #[test]
    fn concat_saves_space() {
        let arena = Arena::new();
        assert_eq!(arena.nr_allocations(), 1);

        let s1 = arena.push_str(&"x".repeat(MIN_DATA_BYTES - 4));
        assert_eq!(arena.nr_allocations(), 1);

        let s2 = arena.concat_str(s1, "abcd");
        assert_eq!(arena.nr_allocations(), 1);

        let s3 = arena.concat_str(s2, "x");
        assert_eq!(arena.nr_allocations(), 2);
    }

    #[test]
    fn concat_copy_allocates_right() {
        let arena = Arena::new();
        assert_eq!(arena.nr_allocations(), 1);

        let s1 = arena.push_str(&"x".repeat(MIN_DATA_BYTES - 8));
        assert_eq!(arena.nr_allocations(), 1);
        let s2 = "abcd";
        let s3 = arena.concat_str(s2, s2);
        assert_eq!(arena.nr_allocations(), 1);
    }

    #[test]
    fn concats_from_same_base() {
        let arena = Arena::new();
        let s = arena.push_str("lala");

        let s2 = arena.concat_str(s, "bibi");
        let s3 = arena.concat_str(s, "foo");

        let s4 = arena.concat_str(s3, "123");
        let s5 = arena.concat_str(s4, "abc");
        let s6 = arena.concat_str(s2, "123");

        assert_eq!(s, "lala");
        assert_eq!(s2, "lalabibi");
        assert_eq!(s3, "lalafoo");
        assert_eq!(s4, "lalafoo123");
        assert_eq!(s5, "lalafoo123abc");
        assert_eq!(s6, "lalabibi123");
    }

    #[test]
    fn concats_from_non_arena() {
        let arena = Arena::new();

        let s1 = arena.concat_str("lala", "bibi");
        let s2 = arena.concat_str(s1, "foo");
        assert_eq!(s2, "lalabibifoo");

        let s3 = arena.concat_str("pika", s1);
        assert_eq!(s2, "lalabibifoo");
        assert_eq!(s3, "pikalalabibi");

        let s4 = arena.concat_str(s3, "123");
        assert_eq!(s2, "lalabibifoo");
        assert_eq!(s3, "pikalalabibi");
        assert_eq!(s4, "pikalalabibi123");

        let s5 = arena.concat_str(s4, s1);
        assert_eq!(s2, "lalabibifoo");
        assert_eq!(s3, "pikalalabibi");
        assert_eq!(s4, "pikalalabibi123");
        assert_eq!(s5, "pikalalabibi123lalabibi");
    }

    #[test]
    fn many_1char_concats() {
        let arena = Arena::new();
        let mut s = arena.push_str("");

        for i in 0..1000 {
            for n in 0..CHARS.len() {
                s = arena.concat_str(s, &CHARS.chars().nth(n).unwrap().to_string());
            }
            assert_eq!(s, CHARS.repeat(i + 1));
        }
    }

    fn old_iksemel_test_step(size: usize) {
        let arena = Arena::with_chunk_sizes(size, size);

        let mut s = "";

        for i in 0..CHARS.len() {
            arena.push_str(&CHARS[..i]);
            // alloc
            //iks_stack_alloc (s, i);
            //if (((unsigned long) mem) & ALIGN_MASK) {
            s = arena.concat_str(s, &CHARS.chars().nth(i).unwrap().to_string())
        }
        assert_eq!(s, CHARS);
    }

    #[test]
    fn old_iksemel_test() {
        old_iksemel_test_step(0);
        old_iksemel_test_step(16);
        old_iksemel_test_step(237);
        old_iksemel_test_step(1024);
    }
}

// FIXME: CI unittests
// FIXME: alloc alignment
// FIXME: test for str bad lifetime shouldnt compile (rustdoc compile_fail)
// FIXME: sizing units in with_sizes
// FIXME: docs
// FIXME: non-null opts
