/*
** This file is a part of Iksemel (XML parser for Jabber/XMPP)
** Copyright (C) 2025-2026 Gurer Ozen
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
enum Predicate {
    Index(usize),
    HasAttribute { name: String },
    AttributeIs { name: String, value: String },
}

#[derive(Debug)]
struct AxisStep {
    axis: Axis,
    name: String,
    predicates: Vec<Predicate>,
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

impl Default for XPathSequence<'_> {
    fn default() -> Self {
        XPathSequence::new()
    }
}

impl std::fmt::Display for XPathSequence<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for value in self.items.iter() {
            match value {
                XPathValue::Node(node) => {
                    writeln!(f, "{node}")?;
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
    Predicate,
}

impl XPath {
    fn fix_steps(steps: &mut [AxisStep]) {
        if let Some(step) = steps.first_mut()
            && let Axis::Child = step.axis
        {
            // Iksemel does not have the extra indirection between the
            // document and the root element like XPath, so we map axes
            // to work with the iksemel model when they are at the
            // beginning of the expression.
            step.axis = Axis::Self_;
        }
    }

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
                    } else if c == b'[' {
                        back = pos + 1;
                        state = State::Predicate;
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
                    } else if c == b'[' {
                        back = pos + 1;
                        state = State::Predicate;
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
                    } else if c == b'/' || c == b'[' {
                        steps.push(AxisStep {
                            axis,
                            name: String::from_utf8_lossy(&bytes[back..pos]).to_string(),
                            predicates: Vec::new(),
                        });
                        if c == b'[' {
                            back = pos + 1;
                            state = State::Predicate;
                        } else {
                            state = State::Slash;
                        }
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
                            predicates: Vec::new(),
                        });
                        state = State::Slash;
                    }
                }
                State::Predicate => {
                    if c == b']' {
                        let pred = &bytes[back..pos];
                        let predicate = if let Some(eq_pos) = pred.iter().position(|&b| b == b'=') {
                            let name_bytes = &pred[..eq_pos];
                            let value_bytes = &pred[eq_pos + 1..];
                            let name_bytes = name_bytes.strip_prefix(b"@").unwrap_or(name_bytes);
                            let name = String::from_utf8_lossy(name_bytes).to_string();
                            let value_bytes = if value_bytes.len() >= 2
                                && ((value_bytes[0] == b'\''
                                    && value_bytes[value_bytes.len() - 1] == b'\'')
                                    || (value_bytes[0] == b'"'
                                        && value_bytes[value_bytes.len() - 1] == b'"'))
                            {
                                &value_bytes[1..value_bytes.len() - 1]
                            } else {
                                value_bytes
                            };
                            let value = String::from_utf8_lossy(value_bytes).to_string();
                            Some(Predicate::AttributeIs { name, value })
                        } else if let Some(rest) = pred.strip_prefix(b"@") {
                            let name = String::from_utf8_lossy(rest).to_string();
                            Some(Predicate::HasAttribute { name })
                        } else if let Ok(s) = std::str::from_utf8(pred) {
                            s.trim().parse::<usize>().ok().map(Predicate::Index)
                        } else {
                            None
                        };
                        if let (Some(p), Some(last)) = (predicate, steps.last_mut()) {
                            last.predicates.push(p);
                        }
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
            State::Predicate => {}
            State::AxisColumn => {
                return Err(BadXPath);
            }
            State::Axis | State::NodeTest => {
                steps.push(AxisStep {
                    axis,
                    name: String::from_utf8_lossy(&bytes[back..pos]).to_string(),
                    predicates: Vec::new(),
                });
            }
        }

        XPath::fix_steps(&mut steps);

        Ok(XPath { steps })
    }

    fn run_step_for_item<'a>(
        cursor: Cursor<'a>,
        new_context: &mut XPathSequence<'a>,
        step: &AxisStep,
    ) -> Result<(), BadXPath> {
        let mut candidates: Vec<Cursor<'a>> = Vec::new();
        match step.axis {
            Axis::Child => {
                for child in cursor.clone().children() {
                    if step.name == "*" || step.name == child.name() {
                        candidates.push(child);
                    }
                }
            }
            Axis::DescendantOrSelf => {
                for descendant in cursor.clone().descendant_or_self() {
                    if step.name == "*" || step.name == descendant.name() {
                        candidates.push(descendant);
                    }
                }
            }
            Axis::FollowingSibling => {
                for sibling in cursor.clone().following_sibling() {
                    if step.name == "*" || step.name == sibling.name() {
                        candidates.push(sibling);
                    }
                }
            }
            Axis::Ancestor => {
                for ancestor in cursor.clone().ancestor() {
                    if step.name == "*" || step.name == ancestor.name() {
                        candidates.push(ancestor);
                    }
                }
            }
            Axis::PrecedingSibling => {
                for sibling in cursor.clone().preceding_sibling() {
                    if step.name == "*" || step.name == sibling.name() {
                        candidates.push(sibling);
                    }
                }
            }
            Axis::Self_ if (step.name == "*" || step.name == cursor.name()) => {
                candidates.push(cursor);
            }
            _ => {}
        }
        for predicate in &step.predicates {
            match predicate {
                Predicate::HasAttribute { name } => {
                    candidates.retain(|c| c.attribute(name).is_some());
                }
                Predicate::AttributeIs { name, value } => {
                    candidates.retain(|c| c.attribute(name) == Some(value.as_str()));
                }
                Predicate::Index(n) => {
                    candidates = if *n >= 1 && *n <= candidates.len() {
                        vec![candidates.remove(*n - 1)]
                    } else {
                        Vec::new()
                    };
                }
            }
        }
        for c in candidates {
            new_context.items.push(XPathValue::Node(c));
        }
        Ok(())
    }

    fn run_step<'a>(
        document: &'a Document,
        context: &XPathSequence<'a>,
        step: &AxisStep,
    ) -> Result<XPathSequence<'a>, BadXPath> {
        let mut new_context = XPathSequence::new();
        if context.items.is_empty() {
            XPath::run_step_for_item(document.root(), &mut new_context, step)?;
        } else {
            for item in &context.items {
                match item {
                    XPathValue::Node(cursor) => {
                        XPath::run_step_for_item(cursor.clone(), &mut new_context, step)?;
                    }
                }
            }
        }
        Ok(new_context)
    }

    pub fn apply<'b>(&self, document: &'b Document) -> Result<XPathSequence<'b>, BadXPath> {
        let mut context = XPathSequence::new();
        for step in &self.steps {
            context = XPath::run_step(document, &context, step)?;
            if context.items.is_empty() {
                break;
            }
        }
        Ok(context)
    }
}

#[cfg(test)]
mod tests;
