//! The name dictionary (owner 2026-11-01: "the event name dictionary with domains … makes it really easy to know where
//! things go and what they will trigger"): an event's name is `<domain>.<name>` (`story.fx`, `music.note`, `relay.cue`,
//! `deck.mood`); a prop key is a plain word (`pitch`, `vel`). Both are interned once, at load – the hot path compares
//! integers (`Sym`), never strings.
use std::collections::HashMap;
use std::fmt;

/// an interned event name or prop key
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Sym(pub u32);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NameError {
    /// an event name needs a domain: `<domain>.<name>`
    NoDomain(String),
    /// lower-case letters, digits, `_`, and `.` between the parts
    BadChars(String),
}

impl fmt::Display for NameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NameError::NoDomain(n) => write!(f, "event name '{n}' has no domain (write <domain>.<name>, e.g. story.fx)"),
            NameError::BadChars(n) => write!(f, "'{n}': only a–z, 0–9 and _ in its parts, '.' between them"),
        }
    }
}

impl std::error::Error for NameError {}

#[derive(Debug, Clone, Default)]
pub struct Names {
    by_name: HashMap<String, Sym>,
    names: Vec<String>,
}

fn word_ok(w: &str) -> bool {
    !w.is_empty() && w.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
}

impl Names {
    pub fn new() -> Names {
        Names::default()
    }

    fn intern(&mut self, s: &str) -> Sym {
        if let Some(&sym) = self.by_name.get(s) {
            return sym;
        }
        let sym = Sym(self.names.len() as u32);
        self.names.push(s.to_string());
        self.by_name.insert(s.to_string(), sym);
        sym
    }

    /// an event name: `<domain>.<name>` (more dots allowed: `media.video.seek`)
    pub fn event(&mut self, name: &str) -> Result<Sym, NameError> {
        let parts: Vec<&str> = name.split('.').collect();
        if parts.len() < 2 {
            return Err(NameError::NoDomain(name.into()));
        }
        if !parts.iter().all(|p| word_ok(p)) {
            return Err(NameError::BadChars(name.into()));
        }
        Ok(self.intern(name))
    }

    /// a prop key: one plain word
    pub fn key(&mut self, key: &str) -> Result<Sym, NameError> {
        if !word_ok(key) {
            return Err(NameError::BadChars(key.into()));
        }
        Ok(self.intern(key))
    }

    /// an already interned name or key
    pub fn get(&self, name: &str) -> Option<Sym> {
        self.by_name.get(name).copied()
    }

    pub fn name(&self, sym: Sym) -> &str {
        self.names.get(sym.0 as usize).map_or("?", String::as_str)
    }

    /// the domain of an event name (`story` of `story.fx`); a key has none
    pub fn domain(&self, sym: Sym) -> Option<&str> {
        self.name(sym).split_once('.').map(|(d, _)| d)
    }

    pub fn in_domain(&self, sym: Sym, domain: &str) -> bool {
        self.domain(sym) == Some(domain)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_have_domains_and_intern_once() {
        let mut n = Names::new();
        let fx = n.event("story.fx").unwrap();
        assert_eq!(n.event("story.fx").unwrap(), fx, "interned once");
        assert_eq!(n.domain(fx), Some("story"));
        assert!(n.in_domain(fx, "story") && !n.in_domain(fx, "relay"));
        let seek = n.event("media.video.seek").unwrap();
        assert_eq!(n.domain(seek), Some("media"));
        assert_eq!(n.name(seek), "media.video.seek");
        assert_eq!(n.event("fx"), Err(NameError::NoDomain("fx".into())));
        assert!(matches!(n.event("Story.FX"), Err(NameError::BadChars(_))));
        assert!(matches!(n.event("story..fx"), Err(NameError::BadChars(_))));
        let pitch = n.key("pitch").unwrap();
        assert_eq!(n.domain(pitch), None);
        assert!(n.key("a.b").is_err(), "a key is one word");
        assert_eq!(n.get("pitch"), Some(pitch));
    }
}
