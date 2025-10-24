/*
** This file is a part of Iksemel (XML parser for Jabber/XMPP)
** Copyright (C) 2025 Gurer Ozen
**
** Iksemel is free software: you can redistribute it and/or modify it
** under the terms of the GNU Lesser General Public License as
** published by the Free Software Foundation, either version 3 of
** the License, or (at your option) any later version.
*/

use crate::SyncCursor;

pub struct SyncChildren {
    current: SyncCursor,
}

impl SyncChildren {
    pub fn new(sync_cursor: &SyncCursor) -> Self {
        SyncChildren {
            current: sync_cursor.clone().first_child(),
        }
    }
}

impl Iterator for SyncChildren {
    type Item = SyncCursor;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current.is_null() {
            return None;
        }
        let result = self.current.clone();
        self.current = self.current.clone().next();
        Some(result)
    }
}
