/*
** This file is a part of Iksemel (XML parser for Jabber/XMPP)
** Copyright (C) 2000-2025 Gurer Ozen
**
** Iksemel is free software: you can redistribute it and/or modify it
** under the terms of the GNU Lesser General Public License as
** published by the Free Software Foundation, either version 3 of
** the License, or (at your option) any later version.
*/

use std::fmt::Display;
use std::hash::Hash;
use std::hash::Hasher;
use std::num::NonZero;

use super::error::BadJid;
use super::error::description;

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

#[derive(Debug, Clone, Eq)]
pub struct Jid {
    full: String,
    slash_pos: Option<NonZero<u16>>,
    at_pos: Option<NonZero<u16>>,
}

impl Jid {
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
            at_pos = Some(unsafe {
                // SAFETY: We just pushed a part verified to be 1-1023 bytes
                // in length in the JidParts.
                NonZero::new_unchecked(full.len() as u16)
            });
            full.push('@');
        }
        full.push_str(parts.domain);
        if let Some(resource) = parts.resource {
            slash_pos = Some(unsafe {
                // SAFETY: We pushed parts which must be between 1-2047 bytes
                // in length, verified by JidParts.
                NonZero::new_unchecked(full.len() as u16)
            });
            full.push('/');
            full.push_str(resource);
        }

        Ok(Jid {
            full,
            slash_pos,
            at_pos,
        })
    }

    pub fn full(&self) -> &str {
        &self.full
    }

    pub fn bare(&self) -> &str {
        match self.slash_pos {
            Some(pos) => &self.full[..pos.get() as usize],
            None => &self.full,
        }
    }

    pub fn localpart(&self) -> Option<&str> {
        match self.at_pos {
            Some(pos) => self.full.get(..pos.get() as usize),
            None => None,
        }
    }

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

    pub fn resourcepart(&self) -> Option<&str> {
        match self.slash_pos {
            Some(pos) => self.full.get(pos.get() as usize + 1..),
            None => None,
        }
    }

    pub fn is_bare(&self) -> bool {
        self.slash_pos.is_none()
    }

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
            slash_pos: Some(unsafe {
                // SAFETY: slash_pos cannot be shorter that current full
                // which was checked to contain at least the non-empty domain
                NonZero::new_unchecked(slash_pos as u16)
            }),
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
