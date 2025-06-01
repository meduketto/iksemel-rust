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

pub struct Arena {
    info: *mut ArenaInfo,
}

struct ArenaInfo {
    allocated: usize,
    nr_allocations: u32,
    node: *mut ArenaChunk,
    data: *mut ArenaChunk,
    _pin: PhantomPinned,  // Arena has a raw pointer to this struct
}

struct ArenaChunk {
    next: *mut ArenaChunk,
    size: usize,
    used: usize,
    last: usize,
    mem: *mut u8,
    _pin: PhantomPinned,  // ArenaInfo has raw pointers to this struct
}


// use Layout for size
// fix return type
// unsafe?
// mark allocations on the arena
impl ArenaChunk {
    pub fn find_chunk(self: &mut ArenaChunk, size: usize) -> *mut ArenaChunk {
unsafe {
        let mut current: *mut ArenaChunk = self;
        let mut expected_next_size = self.size;
        loop {
            if size < (*current).size && size + (*current).used <= (*current).size {
                // If I fits, I sits
                return self
            }
            expected_next_size *= 2;
            if (*current).next.is_null() {
                let data_size = cmp::max(expected_next_size, size);
                let chunk_layout = Layout::new::<ArenaChunk>();
                let (data_layout, data_offset) = chunk_layout.extend(Layout::array::<u8>(data_size).unwrap()).unwrap();
                let new_layout = chunk_layout.pad_to_align();

                let chunk;
                unsafe {
                    let ptr = alloc(new_layout);
                    if ptr.is_null() {
                        handle_alloc_error(new_layout);
                    }
                    chunk = ptr as *mut ArenaChunk;
                    (*chunk).size = data_size;
                    (*chunk).used = 0;
                    (*chunk).last = 0;
                    (*chunk).mem = ptr.byte_add(data_offset);
                }
                (*current).next = chunk;
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
        let min_node_layout = Layout::array::<usize>(32).unwrap();
        let node_size = cmp::max(node_size, min_node_layout.size());
        let node_layout = Layout::array::<u8>(node_size).unwrap();

        // First data chunk should have enough capacity for at least 256 bytes.
        let data_size = cmp::max(data_size, 256);

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
            (*info).allocated = 0;
            (*info).nr_allocations = 1;

            let node_ptr = ptr.byte_add(node_offset);
            let node = node_ptr as *mut ArenaChunk;
            (*info).node = node;

            let data_ptr = ptr.byte_add(data_offset);
            let data = data_ptr as *mut ArenaChunk;
            (*info).data = data;

            let node_buf_ptr = ptr.byte_add(node_buf_offset);
            (*node).next = null_mut();
            (*node).size = node_layout.size();
            (*node).used = 0;
            (*node).last = 0;
            (*node).mem = node_buf_ptr;

            let data_buf_ptr = ptr.byte_add(data_buf_offset);
            (*data).next = null_mut();
            (*data).size = node_layout.size();
            (*data).used = 0;
            (*data).last = 0;
            (*data).mem = data_buf_ptr;
        }

        Arena {
            info: info,
        }
    }

    pub fn alloc(self: &mut Arena, layout: Layout) -> *mut u8 {
    unsafe {
        let size = layout.size();
        let chunk = (*(*self.info).node).find_chunk(size);
        (*chunk).used += size;
        return (*chunk).mem.byte_add(size);
}
    }

/*
    fn push_str(&mut self, &s: str) -> &str {
    
    }

    fn push_str(&mut self, &s: str, &old_s: str) -> &str {

    }

    fn stats();
*/



}
