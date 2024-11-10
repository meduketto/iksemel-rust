/*
** This file is a part of Iksemel (XML parser for Jabber/XMPP)
** Copyright (C) 2000-2024 Gurer Ozen
**
** Iksemel is free software: you can redistribute it and/or modify it
** under the terms of the GNU Lesser General Public License as
** published by the Free Software Foundation, either version 3 of
** the License, or (at your option) any later version.
*/

use std::alloc::{alloc, handle_alloc_error, Layout};
use std::marker::PhantomPinned;

pub struct Arena {
    info: *mut ArenaInfo,
}

struct ArenaInfo {
    size: usize,
    node: *mut ArenaChunk,
    data: *mut ArenaChunk,
    _pin: PhantomPinned,  // Arena has a raw pointer to this struct
}

struct ArenaChunk {
    next: *mut ArenaChunk,
    size: usize,
    used: usize,
    last: usize,
    _pin: PhantomPinned,  // ArenaInfo has raw pointers to this struct
}

impl Arena {
    pub fn new() -> Arena {
        // FIXME: defaults
        Self::with_chunk_sizes(10, 10)
    }

    pub fn with_chunk_sizes(node_size: u64, data_size: u64) -> Arena {
        // FIXME: fix min/aligns

        let info_layout = Layout::new::<ArenaInfo>();
        // Extending the layout for Chunk structures could NOT overflow the usize arithmetic,
        // therefore we just unwrap() the results.
        let (node_layout, node_offset) = info_layout.extend(Layout::new::<ArenaChunk>()).unwrap();
        let (data_layout, data_offset) = node_layout.extend(Layout::new::<ArenaChunk>()).unwrap();

        // FIXME: add data areas

        let info;
        unsafe {
            let ptr = alloc(data_layout);
            if ptr.is_null() {
                handle_alloc_error(data_layout);
            }
            info = ptr as *mut ArenaInfo;
            (*info).size = 0;

            let node_ptr = ptr.byte_add(node_offset);
            let node = node_ptr as *mut ArenaChunk;
            (*info).node = node;

            let data_ptr = ptr.byte_add(data_offset);
            let data = data_ptr as *mut ArenaChunk;
            (*info).data = data;
        }

        Arena {
            info: info,
        }
    }

/*
    fn push_str(&mut self, &s: str) -> &str {
    
    }

    fn push_str(&mut self, &s: str, &old_s: str) -> &str {

    }

    fn alloc();
    fn stats();
*/
    pub fn check(&self) {
        unsafe {
            println!("{:#?}", self.info);
            println!("{:#?}", (*self.info).node);
            println!("{:#?}", (*self.info).data);
        }
    }

}
