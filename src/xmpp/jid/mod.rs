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

use std::fmt::Display;
use std::hash::Hash;
use std::hash::Hasher;
use std::num::NonZero;

pub use error::BadJid;
use error::description;

struct JidParts<'a> {
    local: Option<&'a str>,
    domain: &'a str,
    resource: Option<&'a str>,
}

impl<'a> JidParts<'a> {
    fn new(jid: &'a str) -> Result<JidParts<'a>, BadJid> {
        let slash_pos = jid.find('/');
        let at_pos = match slash_pos {
            Some(pos) => {
                let (bare, _resource) = jid.split_at(pos);
                bare.find('@')
            }
            None => jid.find('@'),
        };
        let mut domain = match (at_pos, slash_pos) {
            (None, None) => jid,
            (Some(pos), None) => &jid[pos + 1..],
            (None, Some(pos)) => &jid[..pos],
            (Some(pos), Some(pos2)) => &jid[pos + 1..pos2],
        };
        if domain.is_empty() {
            return Err(BadJid(description::DOMAIN_EMPTY));
        }
        if domain.len() > 1023 {
            return Err(BadJid(description::DOMAIN_TOO_LONG));
        }
        if domain.ends_with('.') {
            // Remove final dot as per RFC 7622 section 3.2
            domain = &domain[..domain.len() - 1];
        }
        let mut local = None;
        if let Some(pos) = at_pos {
            if pos == 0 {
                return Err(BadJid(description::LOCAL_EMPTY));
            }
            if pos > 1023 {
                return Err(BadJid(description::LOCAL_TOO_LONG));
            }
            local = Some(&jid[..pos]);
        }
        let mut resource = None;
        if let Some(pos) = slash_pos {
            let part = &jid[pos + 1..];
            if part.is_empty() {
                return Err(BadJid(description::RESOURCE_EMPTY));
            }
            if part.len() > 1023 {
                return Err(BadJid(description::RESOURCE_TOO_LONG));
            }
            resource = Some(part);
        }

        Ok(JidParts {
            local,
            domain,
            resource,
        })
    }
}

/// The address of an entity in the XMPP protocol.
///
/// Each JID has three parts:
/// - Local part: Optionally identifies a local entity on the domain.
/// - Domain part: Identifies an XMPP server.
/// - Resource part: Optionally identifies a service or an object.
///
/// More details can be found in [RFC7622](https://datatracker.ietf.org/doc/rfc7622/)
///
#[derive(Debug, Clone, Eq)]
pub struct Jid {
    full: String,
    slash_pos: Option<NonZero<u16>>,
    at_pos: Option<NonZero<u16>>,
}

impl Jid {
    /// Create a JID from a string.
    pub fn new(jid: &str) -> Result<Self, BadJid> {
        let parts = JidParts::new(jid)?;

        let mut full_size = parts.domain.len();
        if let Some(local) = parts.local {
            full_size += local.len() + 1;
        }
        if let Some(resource) = parts.resource {
            full_size += resource.len() + 1;
        }
        let mut full = String::with_capacity(full_size);
        let mut slash_pos = None;
        let mut at_pos = None;
        if let Some(local) = parts.local {
            full.push_str(local);
            at_pos = Some(
                // SAFETY:
                // Invariant: full length cannot be zero.
                // Guard: local part is already pushed and verified to be
                // at least one character long in JidParts.
                unsafe { NonZero::new_unchecked(full.len() as u16) },
            );
            full.push('@');
        }
        full.push_str(parts.domain);
        if let Some(resource) = parts.resource {
            slash_pos = Some(
                // SAFETY:
                // Invariant: full length cannot be zero.
                // Guard: domain part is already pushed and verified to be
                // at least one character long in JidParts.
                unsafe { NonZero::new_unchecked(full.len() as u16) },
            );
            full.push('/');
            full.push_str(resource);
        }

        Ok(Jid {
            full,
            slash_pos,
            at_pos,
        })
    }

    /// Full form of the JID with all the components.
    pub fn full(&self) -> &str {
        &self.full
    }

    /// Bare form of the JID without the resource part.
    pub fn bare(&self) -> &str {
        match self.slash_pos {
            Some(pos) => &self.full[..pos.get() as usize],
            None => &self.full,
        }
    }

    /// Only the local part of the JID.
    pub fn localpart(&self) -> Option<&str> {
        match self.at_pos {
            Some(pos) => self.full.get(..pos.get() as usize),
            None => None,
        }
    }

    /// Only the domain part of the JID.
    pub fn domainpart(&self) -> &str {
        let start = match self.at_pos {
            Some(pos) => pos.get() as usize + 1,
            None => 0,
        };
        let end = match self.slash_pos {
            Some(pos) => pos.get() as usize,
            None => self.full.len(),
        };
        &self.full[start..end]
    }

    /// Only the resource part of the JID.
    pub fn resourcepart(&self) -> Option<&str> {
        match self.slash_pos {
            Some(pos) => self.full.get(pos.get() as usize + 1..),
            None => None,
        }
    }

    /// True if the JID does not contain a resource part.
    pub fn is_bare(&self) -> bool {
        self.slash_pos.is_none()
    }

    /// Creates another JID by overriding the resource part.
    pub fn with_resource(self, resource: &str) -> Result<Jid, BadJid> {
        if resource.is_empty() {
            return Err(BadJid(description::RESOURCE_EMPTY));
        }
        if resource.len() > 1023 {
            return Err(BadJid(description::RESOURCE_TOO_LONG));
        }
        let slash_pos = match self.slash_pos {
            Some(pos) => pos.get() as usize,
            None => self.full.len(),
        };
        let size = slash_pos + 1 + resource.len();
        let mut full = String::with_capacity(size);
        full.push_str(self.bare());
        full.push('/');
        full.push_str(resource);
        Ok(Jid {
            full,
            slash_pos: Some(
                // SAFETY:
                // Invariant: slash_pos cannot be null
                // Guard: A valid JID must have a non-empty domain part
                // which will always comes before the slash_pos.
                unsafe { NonZero::new_unchecked(slash_pos as u16) },
            ),
            at_pos: self.at_pos,
        })
    }
}

impl Display for Jid {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.full)?;
        Ok(())
    }
}

impl PartialEq for Jid {
    fn eq(&self, other: &Jid) -> bool {
        self.full == other.full
    }
}

impl PartialOrd for Jid {
    fn partial_cmp(&self, other: &Jid) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Jid {
    fn cmp(&self, other: &Jid) -> std::cmp::Ordering {
        self.full.cmp(&other.full)
    }
}

impl Hash for Jid {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.full.hash(state)
    }
}

#[cfg(test)]
mod tests;
