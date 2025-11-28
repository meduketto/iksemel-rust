/*
** This file is a part of Iksemel (XML parser for Jabber/XMPP)
** Copyright (C) 2000-2025 Gurer Ozen
**
** Iksemel is free software: you can redistribute it and/or modify it
** under the terms of the GNU Lesser General Public License as
** published by the Free Software Foundation, either version 3 of
** the License, or (at your option) any later version.
*/

use std::alloc::LayoutError;
use std::error::Error;
use std::fmt::Display;

/// Error type for memory allocation failures.
///
/// Arena methods return this error when the underlying global
/// allocator fails to allocate a memory chunk. Best action is
/// to abort the current operation and release any other
/// allocated resources.
///
/// Details about the failed allocation are not included to make
/// this error as lightweight as possible. Usually there isn't
/// any fragmentation with the bump allocator, and the allocation
/// size is in relation to the size of the bytes given to the
/// parser. Including more exact information would harm the
/// successful hot path for very little extra help in debugging.
///
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NoMemory;

impl Display for NoMemory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "not enough memory")
    }
}

impl Error for NoMemory {}

impl From<LayoutError> for NoMemory {
    fn from(_: LayoutError) -> Self {
        NoMemory
    }
}
