/*
** This file is a part of Iksemel (XML parser for Jabber/XMPP)
** Copyright (C) 2000-2024 Gurer Ozen
**
** Iksemel is free software: you can redistribute it and/or modify it
** under the terms of the GNU Lesser General Public License as
** published by the Free Software Foundation, either version 3 of
** the License, or (at your option) any later version.
*/

mod arena;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let arena = arena::Arena::new();
        arena.check();
    }
}
