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

use std::alloc::{Layout, alloc, dealloc};
use std::cmp;
use std::fmt::Debug;
use std::fmt::Display;
use std::marker::PhantomPinned;
use std::ptr::NonNull;
use std::ptr::null_mut;

pub use error::NoMemory;

const MIN_STRUCT_WORDS: usize = 32;

const MIN_CDATA_BYTES: usize = 256;

// This global is necessary to test the Drop impl in a stable
// way, and does NOT compiled in for the non-test profiles.
#[cfg(test)]
use std::cell::RefCell;

#[cfg(test)]
thread_local! {
    static IKSEMEL_ALLOCATED: RefCell<usize> = const { RefCell::new(0) };
}

#[cfg(test)]
fn test_allocated_add(bytes: usize) {
    IKSEMEL_ALLOCATED.with_borrow_mut(|cell| *cell += bytes);
}

#[cfg(not(test))]
fn test_allocated_add(_: usize) {}

#[cfg(test)]
fn test_allocated_sub(bytes: usize) {
    IKSEMEL_ALLOCATED.with_borrow_mut(|cell| *cell -= bytes);
}

#[cfg(not(test))]
fn test_allocated_sub(_: usize) {}

#[cfg(test)]
pub(self) fn test_allocated() -> usize {
    IKSEMEL_ALLOCATED.with_borrow(|cell| *cell)
}

/// Statistics about the memory usage of the arena.
///
/// These numbers are limited to the most useful metrics for
/// programmatic access to avoid introducing an API dependency
/// into the implementation details of the arena. Debug trait
/// of the [Arena] prints more detailed information about the
/// internal state but not guaranteed to be stable across
/// different versions of the library.
///
/// The 'chunks' is the number of alloc() calls from the system
/// allocator. The ratio of 'used_bytes' to 'allocated_bytes'
/// shows how much memory is wasted. The goal is to keep the
/// allocations as few as possible with the minimal waste.
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
/// to the rest of the crate. The [alloc_struct()](Arena::alloc_struct) method
/// requires a bit more care since it has to return a raw pointer. Read its
/// safety section for the safe usage guidelines.
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
    head_ptr: *mut Head,
}

struct Head {
    struct_chunk: *mut Chunk,
    cdata_chunk: *mut Chunk,
    alloc_layout: Layout,
    _pin: PhantomPinned,
}

struct Chunks {
    next: *mut Chunk,
}

impl Iterator for Chunks {
    type Item = *mut Chunk;

    fn next(&mut self) -> Option<Self::Item> {
        if self.next.is_null() {
            None
        } else {
            let chunk = self.next;
            self.next = unsafe { (*chunk).next };
            Some(chunk)
        }
    }
}

impl Head {
    fn struct_chunks(&mut self) -> Chunks {
        Chunks {
            next: self.struct_chunk,
        }
    }

    fn extra_struct_chunks(&mut self) -> Chunks {
        Chunks {
            next: unsafe { (*self.struct_chunk).next },
        }
    }

    fn cdata_chunks(&mut self) -> Chunks {
        Chunks {
            next: self.cdata_chunk,
        }
    }

    fn extra_cdata_chunks(&mut self) -> Chunks {
        Chunks {
            next: unsafe { (*self.cdata_chunk).next },
        }
    }
}

struct Chunk {
    next: *mut Chunk,
    size: usize,
    used: usize,
    last: *mut u8,
    mem: *mut u8,
    alloc_layout: Layout,
    _pin: PhantomPinned,
}

impl Chunk {
    fn raw_init(self: &mut Chunk, ptr: *mut u8, size: usize, alloc_layout: Layout) {
        self.next = null_mut();
        self.size = size;
        self.used = 0;
        self.last = ptr;
        self.mem = ptr;
        self.alloc_layout = alloc_layout;
    }

    fn clear(&mut self) {
        self.used = 0;
        self.last = self.mem;
    }

    fn add_chunk(self: &mut Chunk, size: usize) -> Result<NonNull<Chunk>, NoMemory> {
        let data_layout = Layout::array::<u8>(size).unwrap();

        let chunk_layout = Layout::new::<Chunk>();
        let (chunk_layout, data_offset) = chunk_layout.extend(data_layout).unwrap();
        let chunk_layout = chunk_layout.pad_to_align();

        unsafe {
            let ptr = alloc(chunk_layout);
            if ptr.is_null() {
                return Err(NoMemory);
            }
            test_allocated_add(chunk_layout.size());
            let chunk = ptr as *mut Chunk;
            (*chunk).raw_init(ptr.byte_add(data_offset), size, chunk_layout);
            self.next = chunk;

            Ok(NonNull::new_unchecked(chunk))
        }
    }

    fn has_space(self: &mut Chunk, size: usize) -> bool {
        size <= self.size && self.used + size <= self.size
    }

    fn has_aligned_space(self: &mut Chunk, layout: Layout) -> bool {
        let size = layout.size();
        let used_layout = Layout::from_size_align(self.used, layout.align()).unwrap();
        let used_layout = used_layout.pad_to_align();

        size <= self.size && used_layout.size() + size <= self.size
    }

    fn make_aligned_space(self: &mut Chunk, layout: Layout) -> Result<NonNull<u8>, NoMemory> {
        let mut expected_next_size = self.size;
        let mut current: *mut Chunk = self;
        unsafe {
            while !(*current).has_aligned_space(layout) {
                expected_next_size *= 2;
                let mut next = (*current).next;
                if next.is_null() {
                    let data_size = cmp::max(expected_next_size, layout.size());
                    next = (*current).add_chunk(data_size)?.as_ptr();
                }
                current = next;
            }

            let used_layout = Layout::from_size_align((*current).used, layout.align()).unwrap();
            let used_layout = used_layout.pad_to_align();
            let offset = used_layout.size() - (*current).used;
            let ptr = (*current).mem.byte_add((*current).used + offset);
            (*current).last = ptr;
            (*current).used += layout.size() + offset;

            Ok(NonNull::new_unchecked(ptr))
        }
    }

    fn make_space(self: &mut Chunk, size: usize) -> Result<NonNull<u8>, NoMemory> {
        let mut expected_next_size = self.size;
        let mut current: *mut Chunk = self;
        unsafe {
            while !(*current).has_space(size) {
                expected_next_size *= 2;
                let mut next = (*current).next;
                if next.is_null() {
                    let data_size = cmp::max(expected_next_size, size);
                    next = (*current).add_chunk(data_size)?.as_ptr();
                }
                current = next;
            }

            let ptr = (*current).mem.byte_add((*current).used);
            (*current).last = ptr;
            (*current).used += size;

            Ok(NonNull::new_unchecked(ptr))
        }
    }

    fn find_adjacent_space(
        self: &mut Chunk,
        old_p: *const u8,
        old_size: usize,
        size: usize,
    ) -> Option<*mut Chunk> {
        let mut current: *mut Chunk = self;
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
    /// use iks::Arena;
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
    /// # use iks::Arena;
    /// let arena : Arena = Arena::with_chunk_sizes(128, 4096)?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    #[allow(
        clippy::missing_panics_doc,
        reason = "None of these Layout unwraps can fail"
    )]
    pub fn with_chunk_sizes(struct_words: usize, cdata_bytes: usize) -> Result<Arena, NoMemory> {
        // First node chunk should have capacity for this many pointer words.
        let struct_words = cmp::max(struct_words, MIN_STRUCT_WORDS);
        let struct_buf_layout = Layout::array::<*const usize>(struct_words).unwrap();

        // First data chunk should have capacity for this many bytes.
        let cdata_bytes = cmp::max(cdata_bytes, MIN_CDATA_BYTES);
        let cdata_buf_layout = Layout::array::<u8>(cdata_bytes).unwrap();

        let head_layout = Layout::new::<Head>();
        let (head_layout, struct_offset) = head_layout.extend(Layout::new::<Chunk>()).unwrap();
        let (head_layout, cdata_offset) = head_layout.extend(Layout::new::<Chunk>()).unwrap();
        let (head_layout, struct_buf_offset) = head_layout.extend(struct_buf_layout).unwrap();
        let (head_layout, cdata_buf_offset) = head_layout.extend(cdata_buf_layout).unwrap();
        // Necessary to align the whole block to pointer/usize alignment
        let head_layout = head_layout.pad_to_align();

        let head_ptr;
        unsafe {
            let ptr = alloc(head_layout);
            if ptr.is_null() {
                return Err(NoMemory);
            }
            test_allocated_add(head_layout.size());
            head_ptr = ptr as *mut Head;
            (*head_ptr).alloc_layout = head_layout;

            let struct_ptr = ptr.byte_add(struct_offset);
            let struct_chunk = struct_ptr as *mut Chunk;
            (*head_ptr).struct_chunk = struct_chunk;

            let cdata_ptr = ptr.byte_add(cdata_offset);
            let cdata_chunk = cdata_ptr as *mut Chunk;
            (*head_ptr).cdata_chunk = cdata_chunk;

            let struct_buf_ptr = ptr.byte_add(struct_buf_offset);
            (*struct_chunk).raw_init(struct_buf_ptr, struct_buf_layout.size(), head_layout);

            let cdata_buf_ptr = ptr.byte_add(cdata_buf_offset);
            (*cdata_chunk).raw_init(cdata_buf_ptr, cdata_buf_layout.size(), head_layout);
        }

        Ok(Arena { head_ptr })
    }

    /// Allocate memory for a struct in the arena.
    ///
    /// If there is not enough space for the struct in the arena,
    /// and a new chunk could not be allocated, a [NoMemory] error
    /// is returned.
    ///
    /// # Safety
    ///
    /// This method returns a raw pointer to the allocated but
    /// uninitialized memory. The care must be taken to ensure
    /// that the memory is properly initialized before the
    /// pointer is shared with the rest of the program.
    ///
    /// If you are going to use the
    /// [into_empty_arena()](Arena::into_empty_arena)
    /// method, you must be careful to add proper lifetimes to
    /// your structures and keep track of their scopes to avoid
    /// reusing the memory pointed by them. See the method's
    /// safety section for more details.
    ///
    /// The best way to do that is to extend the Arena with your
    /// own trait and implement individual alloc methods for your
    /// own types. Rest of your program can use these abstractions
    /// safely. The example below illustrates this method which was
    /// also used in the [Document](crate::Document) implementation.
    ///
    /// Note that your struct will be allocated within the arena,
    /// and dropped together with other structs in one dealloc call.
    /// No Drop method will be called on your struct or fields
    /// individually. If you have a field with non-trivial Drop
    /// implementation, you must use addr_of_mut!(ptr.field).write()
    /// to initialize it to avoid a Drop call on the unitialized
    /// field, and manually drop the field to avoid any leaks.
    /// It is better to construct such complex structs normally,
    /// rather than allocating inside the arena, since the
    /// workarounds are error-prone and reduce the performance
    /// benefits. See the second example below in case you must
    /// have them.
    ///
    /// # Examples
    /// Best practice:
    /// ```
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # use iks::Arena;
    /// # let arena : Arena = Arena::new()?;
    /// use iks::NoMemory;
    ///
    /// struct MyStruct<'a> {
    ///     a: i32,
    ///     b: &'a str,
    /// }
    ///
    /// trait ArenaExt {
    ///     fn alloc_my_struct(&self, a: i32, b: &str) -> Result<&mut MyStruct, NoMemory>;
    /// }
    ///
    /// impl ArenaExt for Arena {
    ///     fn alloc_my_struct(&self, a: i32, b: &str) -> Result<&mut MyStruct, NoMemory> {
    ///         let ptr = self.alloc_struct::<MyStruct>()?.as_ptr();
    ///         unsafe {
    ///             (*ptr).a = a;
    ///             (*ptr).b = self.push_str(b)?;
    ///             Ok(&mut *ptr)
    ///         }
    ///     }
    /// }
    ///
    /// let mut my_struct = arena.alloc_my_struct(42, "Hello");
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// Struct with Drop fields:
    /// ```
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # use iks::Arena;
    /// # let arena : Arena = Arena::new()?;
    /// use std::ptr::addr_of_mut;
    /// use iks::NoMemory;
    ///
    /// struct MyStruct {
    ///     a: i32,
    ///     b: Option<String>,
    /// }
    ///
    /// trait ArenaExt {
    ///     fn alloc_my_struct(&self, a: i32, b: &str) -> Result<&mut MyStruct, NoMemory>;
    /// }
    ///
    /// impl ArenaExt for Arena {
    ///     fn alloc_my_struct(&self, a: i32, b: &str) -> Result<&mut MyStruct, NoMemory> {
    ///         let ptr = self.alloc_struct::<MyStruct>()?.as_ptr();
    ///         unsafe {
    ///             (*ptr).a = a;
    ///             // (*ptr).b = b.to_string() would first try to Drop the old value
    ///             // which could crash with the uninitialized memory.
    ///             addr_of_mut!((*ptr).b).write(Some(b.to_string()));
    ///             Ok(&mut *ptr)
    ///         }
    ///     }
    /// }
    ///
    /// let mut my_struct = arena.alloc_my_struct(42, "Hello")?;
    /// // Manually drop the String to avoid memory leaks before exit
    /// my_struct.b = None;
    /// # Ok(())
    /// # }
    /// ```
    pub fn alloc_struct<T>(&self) -> Result<NonNull<T>, NoMemory> {
        unsafe {
            let head = &mut *self.head_ptr;
            let layout = Layout::new::<T>();
            let ptr = (*head.struct_chunk).make_aligned_space(layout)?;
            Ok(NonNull::new_unchecked(ptr.as_ptr() as *mut T))
        }
    }

    /// Copies given string slice into the arena and returns a reference.
    ///
    /// If there is not enough space for the struct in the arena,
    /// and a new chunk could not be allocated, a [NoMemory] error
    /// is returned.
    ///
    /// # Examples
    /// ```
    /// # use iks::Arena;
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let arena = Arena::new()?;
    /// let s = arena.push_str("Hello")?;
    /// assert_eq!(s, "Hello");
    /// # Ok(())
    /// # }
    /// ```
    pub fn push_str<'a>(&'a self, s: &str) -> Result<&'a str, NoMemory> {
        let size = s.len();
        unsafe {
            let head = &mut *self.head_ptr;
            let ptr = (*head.cdata_chunk).make_space(size)?.as_ptr();
            std::ptr::copy_nonoverlapping(s.as_ptr(), ptr, size);
            let slice = std::slice::from_raw_parts(ptr, size);

            Ok(std::str::from_utf8_unchecked(slice))
        }
    }

    /// Concatenates two strings into a new string in the arena.
    ///
    /// If there is not enough space for the struct in the arena,
    /// and a new chunk could not be allocated, a [NoMemory] error
    /// is returned.
    ///
    /// If the first string is already in the arena and there is enough
    /// space after it, only the second string is copied to extend the
    /// first string. If there is not enough space, or both strings are
    /// not in the arena, they are copied into a suitable space.
    ///
    /// [Document](crate::Document) uses this when inserting character
    /// data to efficiently concatenate the [CData](crate::SaxElement::CData)
    /// elements coming from the [SaxParser](crate::SaxParser).
    ///
    /// # Examples
    /// ```
    /// # use iks::Arena;
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let arena = Arena::new()?;
    /// let s1 = arena.push_str("Hello, ")?;
    /// let s2 = arena.concat_str(s1, "world!")?;
    /// assert_eq!(s2, "Hello, world!");
    /// # Ok(())
    /// # }
    /// ```
    pub fn concat_str<'a>(&'a self, old_s: &str, s: &str) -> Result<&'a str, NoMemory> {
        unsafe {
            let head = &mut *self.head_ptr;
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
                let ptr = (*data_chunk).make_space(old_s.len() + s.len())?.as_ptr();
                std::ptr::copy_nonoverlapping(old_s.as_ptr(), ptr, old_s.len());
                let ptr2 = ptr.byte_add(old_s.len());
                std::ptr::copy_nonoverlapping(s.as_ptr(), ptr2, s.len());
                slice = std::slice::from_raw_parts(ptr, old_s.len() + s.len());
            }

            Ok(std::str::from_utf8_unchecked(slice))
        }
    }

    /// Returns statistics about the arena.
    ///
    /// See [ArenaStats](ArenaStats) for the details of the
    /// returned information.
    ///
    /// Note that this method iterates over the chunks to calculate
    /// the numbers, so best not to be called from a hot path.
    pub fn stats(&self) -> ArenaStats {
        let mut stats = ArenaStats {
            chunks: 1,
            allocated_bytes: 0,
            used_bytes: 0,
        };
        unsafe {
            let head = &mut *self.head_ptr;
            stats.allocated_bytes += head.alloc_layout.size();
            stats.used_bytes += (*head.struct_chunk).used;
            stats.used_bytes += (*head.cdata_chunk).used;
            for chunk in head.extra_struct_chunks() {
                stats.chunks += 1;
                stats.allocated_bytes += (*chunk).alloc_layout.size();
                stats.used_bytes += (*chunk).used;
            }
            for chunk in head.extra_cdata_chunks() {
                stats.chunks += 1;
                stats.allocated_bytes += (*chunk).alloc_layout.size();
                stats.used_bytes += (*chunk).used;
            }
        }
        stats
    }

    /// Marks all chunks as empty without deallocating memory.
    ///
    /// If you are parsing a series of documents, or XML stanzas
    /// coming through a stream, you can use the same arena to
    /// build them as documents which would avoid constant
    /// memory allocations.
    ///
    /// # Safety
    ///
    /// Accessing any previously returned pointer or reference
    /// into the arena after the arena is reused, would result in
    /// undefined behavior.
    ///
    /// Since [push_str()](Arena::push_str) and
    /// [concat_str()](Arena::concat_str) returns references with
    /// lifetimes tied into the arena, the compiler will stop you
    /// from making this error. The
    /// [alloc_struct()](Arena::alloc_struct()), on the other
    /// hand, requires you to setup proper lifetimes and track this
    /// by your own means.
    ///
    /// # Examples
    /// ```
    /// # use iks::Arena;
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let arena = Arena::new()?;
    /// // use the arena
    /// arena.push_str("foo")?;
    /// // clear it
    /// let arena2 = arena.into_empty_arena();
    /// // can be reused now
    /// arena2.push_str("bar");
    /// # Ok(())
    /// # }
    /// ```
    ///
    pub fn into_empty_arena(self) -> Arena {
        unsafe {
            let head = &mut *self.head_ptr;
            for chunk in (*head).struct_chunks() {
                (*chunk).clear();
            }
            for chunk in (*head).cdata_chunks() {
                (*chunk).clear();
            }
        }
        self
    }
}

impl Drop for Arena {
    fn drop(&mut self) {
        unsafe {
            let head = &mut *self.head_ptr;
            for chunk in (*head).extra_struct_chunks() {
                test_allocated_sub((*chunk).alloc_layout.size());
                let layout = (*chunk).alloc_layout;
                dealloc(chunk as *mut u8, layout);
            }
            for chunk in (*head).extra_cdata_chunks() {
                test_allocated_sub((*chunk).alloc_layout.size());
                let layout = (*chunk).alloc_layout;
                dealloc(chunk as *mut u8, layout);
            }
            test_allocated_sub(head.alloc_layout.size());
            let layout = head.alloc_layout;
            dealloc(self.head_ptr as *mut u8, layout);
        }
    }
}

impl Display for Arena {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Arena ({})", self.stats())
    }
}

impl Debug for Arena {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        unsafe {
            let head = &mut *self.head_ptr;
            write!(f, "Arena (head[alloc: {}]", head.alloc_layout.size())?;
            for chunk in (*head).struct_chunks() {
                write!(
                    f,
                    ", struct[alloc: {}, used: {}, size: {}]",
                    (*chunk).alloc_layout.size(),
                    (*chunk).used,
                    (*chunk).size
                )?;
            }
            for chunk in (*head).cdata_chunks() {
                write!(
                    f,
                    ", cdata[alloc: {}, used: {}, size: {}]",
                    (*chunk).alloc_layout.size(),
                    (*chunk).used,
                    (*chunk).size
                )?;
            }
            write!(f, ")")
        }
    }
}

#[cfg(test)]
mod tests;

/// # Must not compile tests
///
/// Returned &str cannot outlive the arena:
/// ```compile_fail
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// use iks::Arena;
/// let mut s : &str = "";
/// {
///     let arena = Arena::new()?;
///     s = arena.push_str("will dangle").unwrap();
/// }
/// println!("{}", s);
/// # Ok(())
/// # }
/// ```
///
/// into_empty_arena cannot be called with existing references
/// ```compile_fail
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// use iks::Arena;
/// let arena = Arena::new()?;
/// let s = arena.push_str("dangling")?;
/// let arena2 = arena.into_empty_arena();
/// println!("{}", s);
/// # Ok(())
/// # }
/// ```
///
#[cfg(doctest)]
struct MustNotCompileTests;
