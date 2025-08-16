/*
** This file is a part of Iksemel (XML parser for Jabber/XMPP)
** Copyright (C) 2000-2025 Gurer Ozen
**
** Iksemel is free software: you can redistribute it and/or modify it
** under the terms of the GNU Lesser General Public License as
** published by the Free Software Foundation, either version 3 of
** the License, or (at your option) any later version.
*/

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

    let _p = arena.alloc(Layout::new::<Layout>());
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

    let _s3 = arena.concat_str(s2, "x");
    assert_eq!(arena.nr_allocations(), 2);
}

#[test]
fn concat_copy_allocates_right() {
    let arena = Arena::new();
    assert_eq!(arena.nr_allocations(), 1);

    let _s1 = arena.push_str(&"x".repeat(MIN_DATA_BYTES - 8));
    assert_eq!(arena.nr_allocations(), 1);
    let s2 = "abcd";
    let _s3 = arena.concat_str(s2, s2);
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

#[test]
fn alloc_alignments() {
    let arena = Arena::new();

    let lay1 = Layout::from_size_align(2, 2).unwrap();
    let lay2 = Layout::from_size_align(8, 8).unwrap();

    let p1 = arena.alloc(lay1);
    assert_eq!(p1.align_offset(2), 0);
    let p2 = arena.alloc(lay2);
    assert_eq!(p2.align_offset(8), 0);
    let p3 = arena.alloc(lay1);
    assert_eq!(p3.align_offset(2), 0);
    let p4 = arena.alloc(lay1);
    assert_eq!(p4.align_offset(2), 0);
}

#[test]
fn alloc_weird_alignments() {
    let arena = Arena::new();

    // Rust types don't have sizes like these but they can be created
    // manually or via #[packed] so let's test them too.
    let lay1 = Layout::from_size_align(3, 2).unwrap();
    let lay2 = Layout::from_size_align(5, 8).unwrap();
    let lay3 = Layout::from_size_align(13, 8).unwrap();

    let p1 = arena.alloc(lay1);
    assert_eq!(p1.align_offset(2), 0);
    let p2 = arena.alloc(lay2);
    assert_eq!(p2.align_offset(8), 0);
    let p3 = arena.alloc(lay1);
    assert_eq!(p3.align_offset(2), 0);
    let p4 = arena.alloc(lay3);
    assert_eq!(p4.align_offset(8), 0);
}

#[test]
fn alloc_chunk_border() {
    let arena = Arena::new();
    assert_eq!(arena.nr_allocations(), 1);

    let lay1 = Layout::array::<*const usize>(MIN_NODE_WORDS - 2).unwrap();
    let lay2 = Layout::array::<*const usize>(2).unwrap();

    let _p1 = arena.alloc(lay1);
    assert_eq!(arena.nr_allocations(), 1);
    let _p2 = arena.alloc(lay2);
    assert_eq!(arena.nr_allocations(), 1);
    let _p3 = arena.alloc(lay2);
    assert_eq!(arena.nr_allocations(), 2);
}

fn old_iksemel_test_step(size: usize) {
    let arena = Arena::with_chunk_sizes(size, size);

    let mut s = "";

    for i in 0..CHARS.len() {
        arena.push_str(&CHARS[..i]);
        let ptr = arena.alloc(Layout::from_size_align(i, 8).unwrap());
        assert_eq!(ptr.align_offset(8), 0);
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
