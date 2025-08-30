/*
** This file is a part of Iksemel (XML parser for Jabber/XMPP)
** Copyright (C) 2000-2025 Gurer Ozen
**
** Iksemel is free software: you can redistribute it and/or modify it
** under the terms of the GNU Lesser General Public License as
** published by the Free Software Foundation, either version 3 of
** the License, or (at your option) any later version.
*/

use crate::Document;

#[derive(Clone, Copy, Debug)]
enum Axis {
    Child,
    Parent,
    Ancestor,
    Descendant,
    Following,
    Preceding,
    FollowingSibling,
    PrecedingSibling,
    Attribute,
    DescendantOrSelf,
    AncestorOrSelf,
}

#[derive(Debug)]
struct AxisStep {
    axis: Axis,
    name: String,
}

pub struct XPath {
    steps: Vec<AxisStep>,
}

enum State {
    Start,
    Slash,
    AxisStart,
    Axis,
}

impl XPath {
    pub fn new(expression: &str) -> Self {
        let bytes = expression.as_bytes();
        let mut pos: usize = 0;
        let mut back: usize = 0;
        let mut state: State = State::Start;
        let mut axis = Axis::Child;
        let mut steps: Vec<AxisStep> = Vec::new();

        while pos < bytes.len() {
            let c = bytes[pos];

            match state {
                State::Start => {
                    if c == b'/' {
                        state = State::Slash;
                    }
                }
                State::Slash => {
                    if c == b'/' {
                        axis = Axis::DescendantOrSelf;
                        state = State::AxisStart;
                    } else if c == b'@' {
                        back = pos + 1;
                        axis = Axis::Attribute;
                        state = State::Axis;
                    } else {
                        back = pos;
                        axis = Axis::Child;
                        state = State::Axis;
                    }
                }
                State::AxisStart => {
                    if c == b'@' {
                        back = pos + 1;
                        axis = Axis::Attribute;
                        state = State::Axis;
                    } else {
                        back = pos;
                        state = State::Axis;
                    }
                }
                State::Axis => {
                    if c == b':' {
                    } else if c == b'/' {
                        steps.push(AxisStep {
                            axis,
                            name: String::from_utf8_lossy(&bytes[back..pos]).to_string(),
                        });
                        state = State::Slash;
                    }
                }
            }

            pos += 1;
        }

        match state {
            State::Start => {}
            State::Slash => {}
            State::AxisStart => {}
            State::Axis => {
                steps.push(AxisStep {
                    axis,
                    name: String::from_utf8_lossy(&bytes[back..pos]).to_string(),
                });
            }
        }

        XPath { steps }
    }

    pub fn apply(&self, document: Document) {
        for step in &self.steps {
            println!("{:?}", step);
        }
    }
}
