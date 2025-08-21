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
    let arena = Arena::new().unwrap();
    let stats = arena.stats();
    assert_eq!(stats.chunks, 1);
    assert!(stats.allocated_bytes > 0);
    assert_eq!(stats.used_bytes, 0);

    let s = arena.push_str("test");
    assert_eq!(s, "test");
    assert_eq!(arena.stats().used_bytes, 4);

    let s2 = arena.concat_str(s, "moretest");
    assert_eq!(s2, "testmoretest");
    assert_eq!(arena.stats().used_bytes, 12);

    let _p = arena.alloc_struct::<Layout>();
}

#[test]
fn many_pushes() {
    let arena = Arena::new().unwrap();
    let old_bytes = arena.stats().allocated_bytes;

    for _ in 0..1000 {
        for j in 0..CHARS.len() {
            arena.push_str(&CHARS[..j]);
        }
    }
    let stats = arena.stats();
    assert!(stats.chunks > 1);
    assert!(stats.allocated_bytes > old_bytes);
    assert_eq!(stats.used_bytes, 1000 * CHARS.len() * (CHARS.len() - 1) / 2);
}

#[test]
fn many_1char_pushes() {
    let arena = Arena::new().unwrap();

    for _ in 0..10000 {
        arena.push_str("+");
    }
}

#[test]
fn concat_saves_space() {
    let arena = Arena::new().unwrap();
    assert_eq!(arena.stats().chunks, 1);

    let s1 = arena.push_str(&"x".repeat(MIN_DATA_BYTES - 4));
    assert_eq!(arena.stats().chunks, 1);

    let s2 = arena.concat_str(s1, "abcd");
    assert_eq!(arena.stats().chunks, 1);

    let _s3 = arena.concat_str(s2, "x");
    assert_eq!(arena.stats().chunks, 2);
}

#[test]
fn concat_copy_allocates_right() {
    let arena = Arena::new().unwrap();
    assert_eq!(arena.stats().chunks, 1);

    let _s1 = arena.push_str(&"x".repeat(MIN_DATA_BYTES - 8));
    assert_eq!(arena.stats().chunks, 1);
    let s2 = "abcd";
    let _s3 = arena.concat_str(s2, s2);
    assert_eq!(arena.stats().chunks, 1);
}

#[test]
fn concat_copy_with_large_str() {
    let arena = Arena::new().unwrap();
    assert_eq!(arena.stats().chunks, 1);

    let s1 = arena.push_str(&"x".repeat(MIN_DATA_BYTES - 8));
    assert_eq!(arena.stats().chunks, 1);
    let s2 = &"a".repeat(MIN_DATA_BYTES * 10);
    let _s3 = arena.concat_str(s1, s2);
    let stats = arena.stats();
    assert_eq!(stats.chunks, 2);
    assert_eq!(
        stats.used_bytes,
        (MIN_DATA_BYTES * 10) + ((MIN_DATA_BYTES - 8) * 2)
    );
    let _s4 = arena.push_str("abcd");
    assert_eq!(stats.chunks, 2);
}

#[test]
fn concats_from_same_base() {
    let arena = Arena::new().unwrap();
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
    let arena = Arena::new().unwrap();

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
    let arena = Arena::new().unwrap();
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
    let arena = Arena::new().unwrap();

    #[repr(C, align(2))]
    struct Lay1([u8; 2]);

    #[repr(C, align(8))]
    struct Lay2([u8; 8]);

    #[repr(C)]
    struct Lay3([u8; 3]);

    let p1 = arena.alloc_struct::<Lay1>();
    assert_eq!(p1.align_offset(2), 0);
    let p2 = arena.alloc_struct::<Lay2>();
    assert_eq!(p2.align_offset(8), 0);
    let p3 = arena.alloc_struct::<Lay1>();
    assert_eq!(p3.align_offset(2), 0);
    let p4 = arena.alloc_struct::<Lay2>();
    assert_eq!(p4.align_offset(8), 0);
    let _p5 = arena.alloc_struct::<Lay3>();
    let p6 = arena.alloc_struct::<Lay2>();
    assert_eq!(p6.align_offset(8), 0);
}

#[test]
fn alloc_chunk_border() {
    let arena = Arena::new().unwrap();
    assert_eq!(arena.stats().chunks, 1);

    #[repr(C)]
    struct Lay1([usize; MIN_NODE_WORDS - 2]);

    #[repr(C)]
    struct Lay2([usize; 2]);

    let _p1 = arena.alloc_struct::<Lay1>();
    assert_eq!(arena.stats().chunks, 1);
    let _p2 = arena.alloc_struct::<Lay2>();
    assert_eq!(arena.stats().chunks, 1);
    let _p3 = arena.alloc_struct::<Lay2>();
    assert_eq!(arena.stats().chunks, 2);
}

fn old_iksemel_test_step(size: usize) {
    let arena = Arena::with_chunk_sizes(size, size).unwrap();

    #[repr(C, align(8))]
    struct Lay([u8; 1]);

    let mut s = "";
    for i in 0..CHARS.len() {
        arena.push_str(&CHARS[..i]);
        let ptr = arena.alloc_struct::<Lay>();
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
