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

use std::alloc::{Layout, alloc, dealloc, handle_alloc_error};
use std::cell::UnsafeCell;
use std::cmp;
use std::fmt::Debug;
use std::fmt::Display;
use std::marker::PhantomPinned;
use std::ptr::null_mut;

pub use error::NoMemory;

const MIN_NODE_WORDS: usize = 32;

const MIN_DATA_BYTES: usize = 256;

/// Statistics about the memory usage of the arena.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct ArenaStats {
    pub chunks: u32,
    pub allocated_bytes: usize,
    pub used_bytes: usize,
}

impl Display for ArenaStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} chunks, {} bytes allocated, {} bytes used",
            self.chunks, self.allocated_bytes, self.used_bytes
        )
    }
}

/// A memory arena for the XML structures and character data.
///
/// This struct implements a custom memory allocation system to pack
/// XML structures and character data efficiently for the purpose of
/// fast querying and modification.
///
/// Assumption is that the XML data is coming as a stream or series
/// of chunks, therefore the [Document](crate::Document) structure
/// needs to copy and store the character data. This is also helpful
/// for unescaping the entities and combining the CDATA sections.
/// You can access the character data as a continuous block.
///
/// Since new character data is often appended together, and structs
/// have higher alignment requirements, allocation is done in separate
/// character data and struct chunks. Each chunk is managed by a simple
/// bump allocator. Resulting compact packing is good for performance,
/// and since the document is generally freed as a whole after the
/// processing, individual object freeing is not necessary.
///
/// Initial chunks and meta data are allocated together. When there is
/// a need for more memory, a double sized chunk is allocated. This
/// strategy reduces the number of allocations to O(log2 N) while
/// limiting the memory waste to less than half of the allocated space.
/// The [with_chunk_sizes()](Arena::with_chunk_sizes) constructor
/// can be used to fine tune the initial chunk sizes for even
/// better performance. The defaults are optimized for the typical
/// [XMMP stanzas](https://xmpp.org/rfcs/rfc6120.html#streams-fundamentals).
///
/// # Safety
///
/// The arena struct encapsulates the unsafe sections and provides a safe API
/// to the rest of the crate. The [alloc()](Arena::alloc) method requires a bit
/// more care since it has to return a raw pointer. Read its safety section for
/// the safe usage guidelines.
///
/// Iksemel has been tested under
/// [Miri](https://github.com/rust-lang/miri) for any
/// [Unsound Behavior](https://doc.rust-lang.org/reference/behavior-considered-undefined.html).
/// The unit tests are tested with [Cargo Mutants](https://mutants.rs) to
/// ensure that they cover all possible mutations and edge cases, and allow
/// Miri to fully see the runtime behavior of the crate.
///
#[repr(transparent)]
pub struct Arena {
    head: UnsafeCell<*mut ArenaHead>,
}

struct ArenaHead {
    struct_chunk: *mut ArenaChunk,
    cdata_chunk: *mut ArenaChunk,
    alloc_layout: Layout,

    // Arena has a raw pointer to this struct
    _pin: PhantomPinned,
}

struct ArenaChunk {
    next: *mut ArenaChunk,
    size: usize,
    used: usize,
    last: *mut u8,
    mem: *mut u8,
    alloc_layout: Layout,

    // ArenaHead and previous ArenaChunk have raw pointers to this struct
    _pin: PhantomPinned,
}

impl ArenaChunk {
    fn raw_init(self: &mut ArenaChunk, ptr: *mut u8, size: usize, alloc_layout: Layout) {
        self.next = null_mut();
        self.size = size;
        self.used = 0;
        self.last = ptr;
        self.mem = ptr;
        self.alloc_layout = alloc_layout;
    }

    fn add_chunk(self: &mut ArenaChunk, size: usize) -> *mut ArenaChunk {
        let data_layout = Layout::array::<u8>(size).unwrap();

        let chunk_layout = Layout::new::<ArenaChunk>();
        let (chunk_layout, data_offset) = chunk_layout.extend(data_layout).unwrap();
        let chunk_layout = chunk_layout.pad_to_align();

        unsafe {
            let ptr = alloc(chunk_layout);
            if ptr.is_null() {
                handle_alloc_error(chunk_layout);
            }
            let chunk = ptr as *mut ArenaChunk;
            (*chunk).raw_init(ptr.byte_add(data_offset), size, chunk_layout);
            self.next = chunk;

            chunk
        }
    }

    fn has_space(self: &mut ArenaChunk, size: usize) -> bool {
        size < self.size && self.used + size <= self.size
    }

    fn has_aligned_space(self: &mut ArenaChunk, layout: Layout) -> bool {
        let size = layout.size();
        let used_layout = Layout::from_size_align(self.used, layout.align()).unwrap();
        let used_layout = used_layout.pad_to_align();

        size < self.size && used_layout.size() + size <= self.size
    }

    fn make_aligned_space(self: &mut ArenaChunk, layout: Layout) -> *mut u8 {
        let mut expected_next_size = self.size;
        let mut current: *mut ArenaChunk = self;
        unsafe {
            while !(*current).has_aligned_space(layout) {
                expected_next_size *= 2;
                let mut next = (*current).next;
                if next.is_null() {
                    let data_size = cmp::max(expected_next_size, layout.size());
                    next = (*current).add_chunk(data_size);
                }
                current = next;
            }

            let used_layout = Layout::from_size_align((*current).used, layout.align()).unwrap();
            let used_layout = used_layout.pad_to_align();
            let offset = used_layout.size() - (*current).used;
            let ptr = (*current).mem.byte_add((*current).used + offset);
            (*current).last = ptr;
            (*current).used += layout.size() + offset;

            ptr
        }
    }

    fn make_space(self: &mut ArenaChunk, size: usize) -> *mut u8 {
        let mut expected_next_size = self.size;
        let mut current: *mut ArenaChunk = self;
        unsafe {
            while !(*current).has_space(size) {
                expected_next_size *= 2;
                let mut next = (*current).next;
                if next.is_null() {
                    let data_size = cmp::max(expected_next_size, size);
                    next = (*current).add_chunk(data_size);
                }
                current = next;
            }

            let ptr = (*current).mem.byte_add((*current).used);
            (*current).last = ptr;
            (*current).used += size;

            ptr
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
    /// Creates a new 'Arena' with the default initial chunk sizes.
    ///
    /// If there is not enough memory for the initial chunk,
    /// [NoMemory] error is returned.
    ///
    /// # Examples
    /// ```
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// use iksemel::Arena;
    ///
    /// let arena : Arena = Arena::new()?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    pub fn new() -> Result<Arena, NoMemory> {
        // Minimums are defaults
        Self::with_chunk_sizes(0, 0)
    }

    /// Creates a new 'Arena' with the given chunk sizes.
    ///
    /// If there is not enough memory for the initial chunk,
    /// [NoMemory] error is returned.
    ///
    /// Structs often need to be aligned on word boundaries, so
    /// the chunk size for structs is given in machine words, i.e.
    /// 10 words on an 64bit system would be equal to 80 bytes.
    /// Character data chunk size is given in bytes. If the numbers
    /// are smaller than the default sizes, the defaults are used
    /// instead.
    ///
    /// # Examples
    /// ```
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # use iksemel::Arena;
    /// let arena : Arena = Arena::with_chunk_sizes(128, 4096)?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    pub fn with_chunk_sizes(struct_words: usize, cdata_bytes: usize) -> Result<Arena, NoMemory> {
        // First node chunk should have capacity for this many pointer words.
        let struct_words = cmp::max(struct_words, MIN_NODE_WORDS);
        let struct_buf_layout = Layout::array::<*const usize>(struct_words).unwrap();

        // First data chunk should have capacity for this many bytes.
        let cdata_bytes = cmp::max(cdata_bytes, MIN_DATA_BYTES);
        let cdata_buf_layout = Layout::array::<u8>(cdata_bytes).unwrap();

        let head_layout = Layout::new::<ArenaHead>();
        let (head_layout, struct_offset) = head_layout.extend(Layout::new::<ArenaChunk>()).unwrap();
        let (head_layout, cdata_offset) = head_layout.extend(Layout::new::<ArenaChunk>()).unwrap();
        let (head_layout, struct_buf_offset) = head_layout.extend(struct_buf_layout).unwrap();
        let (head_layout, cdata_buf_offset) = head_layout.extend(cdata_buf_layout).unwrap();
        // Necessary to align the whole block to pointer/usize alignment
        let head_layout = head_layout.pad_to_align();

        let head;
        unsafe {
            let ptr = alloc(head_layout);
            if ptr.is_null() {
                return Err(NoMemory);
            }
            head = ptr as *mut ArenaHead;
            (*head).alloc_layout = head_layout;

            let struct_ptr = ptr.byte_add(struct_offset);
            let struct_chunk = struct_ptr as *mut ArenaChunk;
            (*head).struct_chunk = struct_chunk;

            let cdata_ptr = ptr.byte_add(cdata_offset);
            let cdata_chunk = cdata_ptr as *mut ArenaChunk;
            (*head).cdata_chunk = cdata_chunk;

            let struct_buf_ptr = ptr.byte_add(struct_buf_offset);
            (*struct_chunk).raw_init(struct_buf_ptr, struct_buf_layout.size(), head_layout);

            let cdata_buf_ptr = ptr.byte_add(cdata_buf_offset);
            (*cdata_chunk).raw_init(cdata_buf_ptr, cdata_buf_layout.size(), head_layout);
        }

        Ok(Arena { head: head.into() })
    }

    /// Allocate memory for a struct in the arena.
    ///
    /// # Safety
    ///
    /// # Examples
    /// ```
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # use iksemel::Arena;
    /// # let arena : Arena = Arena::new()?;
    /// use std::alloc::Layout;
    ///
    /// struct MyStruct {
    ///     a: i32,
    ///     b: String,
    /// }
    ///
    /// let my_struct_ptr = arena.alloc(Layout::new::<MyStruct>());
    /// # Ok(())
    /// # }
    /// ```
    ///
    pub fn alloc(&self, layout: Layout) -> *mut u8 {
        unsafe {
            let head = &mut **self.head.get();
            (*head.struct_chunk).make_aligned_space(layout)
        }
    }

    pub fn push_str<'a>(&'a self, s: &str) -> &'a str {
        let size = s.len();
        unsafe {
            let head = &mut **self.head.get();
            let ptr = (*head.cdata_chunk).make_space(size);
            std::ptr::copy_nonoverlapping(s.as_ptr(), ptr, size);
            let slice = std::slice::from_raw_parts(ptr, size);

            std::str::from_utf8_unchecked(slice)
        }
    }

    pub fn concat_str<'a>(&'a self, old_s: &str, s: &str) -> &'a str {
        unsafe {
            let head = &mut **self.head.get();
            let data_chunk = head.cdata_chunk;
            let slice;
            if let Some(chunk) =
                (*data_chunk).find_adjacent_space(old_s.as_ptr(), old_s.len(), s.len())
            {
                // Enough space to extend the str
                let p = (*chunk).mem.byte_add((*chunk).used);
                (*chunk).used += s.len();
                std::ptr::copy_nonoverlapping(s.as_ptr(), p, s.len());
                slice = std::slice::from_raw_parts(p.byte_sub(old_s.len()), old_s.len() + s.len());
            } else {
                let ptr = (*data_chunk).make_space(old_s.len() + s.len());
                std::ptr::copy_nonoverlapping(old_s.as_ptr(), ptr, old_s.len());
                let ptr2 = ptr.byte_add(old_s.len());
                std::ptr::copy_nonoverlapping(s.as_ptr(), ptr2, s.len());
                slice = std::slice::from_raw_parts(ptr, old_s.len() + s.len());
            }

            std::str::from_utf8_unchecked(slice)
        }
    }

    pub fn stats(&self) -> ArenaStats {
        let mut stats = ArenaStats {
            chunks: 1,
            allocated_bytes: 0,
            used_bytes: 0,
        };
        unsafe {
            let head = &mut **self.head.get();
            stats.allocated_bytes += (*head).alloc_layout.size();
            stats.used_bytes += (*head.struct_chunk).used;
            let mut chunk = (*head.struct_chunk).next;
            while !chunk.is_null() {
                let next = (*chunk).next;
                stats.chunks += 1;
                stats.allocated_bytes += (*chunk).alloc_layout.size();
                stats.used_bytes += (*chunk).used;
                chunk = next;
            }
            stats.used_bytes += (*head.cdata_chunk).used;
            let mut chunk = (*head.cdata_chunk).next;
            while !chunk.is_null() {
                let next = (*chunk).next;
                stats.chunks += 1;
                stats.allocated_bytes += (*chunk).alloc_layout.size();
                stats.used_bytes += (*chunk).used;
                chunk = next;
            }
        }
        stats
    }
}

impl Drop for Arena {
    fn drop(&mut self) {
        unsafe {
            let head = &mut **self.head.get_mut();
            let mut chunk = (*head.struct_chunk).next;
            while !chunk.is_null() {
                let next = (*chunk).next;
                dealloc(chunk as *mut u8, (*chunk).alloc_layout);
                chunk = next;
            }
            let mut chunk = (*head.cdata_chunk).next;
            while !chunk.is_null() {
                let next = (*chunk).next;
                dealloc(chunk as *mut u8, (*chunk).alloc_layout);
                chunk = next;
            }
            dealloc(*self.head.get_mut() as *mut u8, head.alloc_layout);
        }
    }
}

impl Debug for Arena {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Arena ({})", self.stats())
    }
}

#[cfg(test)]
mod tests;

/// # Must not compile tests
///
/// Returned &str cannot outlive the arena:
/// ```compile_fail
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// use iksemel::Arena;
/// let mut s : &str = "";
/// {
///     let arena = Arena::new()?;
///     s = arena.push_str("will dangle")
/// }
/// println!("{}", s);
/// # Ok(())
/// # }
/// ```
#[cfg(doctest)]
struct MustNotCompileTests;
