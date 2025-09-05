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

use crate::Cursor;
use crate::Document;

use error::BadXPath;

#[derive(Clone, Copy, Debug)]
enum Axis {
    Child,
    Descendant,
    Attribute,
    Self_,
    DescendantOrSelf,
    FollowingSibling,
    Following,
    Namespace,
    Parent,
    Ancestor,
    PrecedingSibling,
    Preceding,
    AncestorOrSelf,
}

#[derive(Debug)]
struct AxisStep {
    axis: Axis,
    name: String,
}

#[derive(Debug)]
pub enum XPathValue<'a> {
    Node(Cursor<'a>),
}

#[derive(Debug)]
pub struct XPathSequence<'a> {
    pub items: Vec<XPathValue<'a>>,
}

impl XPathSequence<'_> {
    pub fn new() -> Self {
        XPathSequence { items: Vec::new() }
    }
}

impl std::fmt::Display for XPathSequence<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for value in self.items.iter() {
            match value {
                XPathValue::Node(node) => {
                    write!(f, "{}\n", node)?;
                }
            }
        }
        Ok(())
    }
}

pub struct XPath {
    steps: Vec<AxisStep>,
}

enum State {
    Start,
    Slash,
    AxisStart,
    Axis,
    AxisColumn,
    NodeTest,
}

impl XPath {
    pub fn new(expression: &str) -> Result<Self, BadXPath> {
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
                        axis = match &bytes[back..pos] {
                            b"self" => Axis::Self_,
                            b"parent" => Axis::Parent,
                            b"ancestor" => Axis::Ancestor,
                            b"descendant" => Axis::Descendant,
                            b"following" => Axis::Following,
                            b"preceding" => Axis::Preceding,
                            b"following-sibling" => Axis::FollowingSibling,
                            b"preceding-sibling" => Axis::PrecedingSibling,
                            b"attribute" => Axis::Attribute,
                            b"namespace" => Axis::Namespace,
                            b"child" => Axis::Child,
                            b"descendant-or-self" => Axis::DescendantOrSelf,
                            b"ancestor-or-self" => Axis::AncestorOrSelf,
                            _ => return Err(BadXPath),
                        };
                        state = State::AxisColumn;
                    } else if c == b'/' {
                        steps.push(AxisStep {
                            axis,
                            name: String::from_utf8_lossy(&bytes[back..pos]).to_string(),
                        });
                        state = State::Slash;
                    }
                }
                State::AxisColumn => {
                    if c == b':' {
                        back = pos + 1;
                        state = State::NodeTest;
                    } else {
                        return Err(BadXPath);
                    }
                }
                State::NodeTest => {
                    if c == b'/' {
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
            State::AxisColumn => {
                return Err(BadXPath);
            }
            State::Axis | State::NodeTest => {
                steps.push(AxisStep {
                    axis,
                    name: String::from_utf8_lossy(&bytes[back..pos]).to_string(),
                });
            }
        }

        Ok(XPath { steps })
    }

    fn child_step<'a>(
        document: &'a Document,
        context: &XPathSequence<'a>,
        step: &AxisStep,
    ) -> Result<XPathSequence<'a>, BadXPath> {
        let mut new_context = XPathSequence::new();
        if context.items.is_empty() {
            let root = document.root();
            if step.name == "*" || step.name == root.name() {
                new_context.items.push(XPathValue::Node(root));
            }
        } else {
            for value in context.items.as_slice() {
                match value {
                    XPathValue::Node(cursor) => {
                        if step.name == "*" {
                            let mut child = cursor.clone().first_child();
                            while !child.is_null() {
                                new_context.items.push(XPathValue::Node(child.clone()));
                                child = child.next();
                            }
                        } else {
                            let mut child = cursor.clone().first_child();
                            while !child.is_null() {
                                if step.name == child.name() {
                                    new_context.items.push(XPathValue::Node(child.clone()));
                                }
                                child = child.next();
                            }
                        }
                    }
                }
            }
        }
        Ok(new_context)
    }

    fn descendant_or_self_step<'a>(
        document: &'a Document,
        context: &XPathSequence<'a>,
        step: &AxisStep,
    ) -> Result<XPathSequence<'a>, BadXPath> {
        let mut new_context = XPathSequence::new();
        if context.items.is_empty() {
            let root = document.root();
            for descendant in root.clone().descendant_or_self() {
                if step.name == "*" || step.name == descendant.name() {
                    new_context.items.push(XPathValue::Node(descendant.clone()));
                }
            }
        } else {
            for value in context.items.as_slice() {
                match value {
                    XPathValue::Node(cursor) => {
                        for descendant in cursor.clone().descendant_or_self() {
                            if step.name == "*" || step.name == descendant.name() {
                                new_context.items.push(XPathValue::Node(descendant.clone()));
                            }
                        }
                    }
                }
            }
        }
        Ok(new_context)
    }

    pub fn apply<'a, 'b>(&'a self, document: &'b Document) -> Result<XPathSequence<'b>, BadXPath> {
        let mut context = XPathSequence::new();
        for step in &self.steps {
            context = match step.axis {
                Axis::Child => XPath::child_step(document, &context, step)?,
                Axis::DescendantOrSelf => XPath::descendant_or_self_step(document, &context, step)?,
                _ => {
                    return Err(BadXPath);
                }
            };
            if context.items.is_empty() {
                break;
            }
        }
        Ok(context)
    }
}

#[cfg(test)]
mod tests;
