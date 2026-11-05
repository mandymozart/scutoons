//! The live context (#409, T2 of #314, agents/research/RIG-MODULE-GROUPS.md §4): what a deck's frame thread hands every
//! module node of its engine – the mood, the persona and the being's senses (energy, touch, strain, 0 … 1). The deck's
//! `Deck::context` (deck.rs) or the native being makes one per frame; deck-engine's control face sends it through its
//! command ring and the engine applies it to every module node at the next block's start (deck-engine module.rs: the
//! module's `ctx.character()`). Plain bytes, `Copy`: it crosses the ring without allocating. Scene variables are a later
//! field (the seam: `Command::Context` carries this whole value).
//!
//! A1 (the one ABI addition, no header change): a module lists the moods it knows (`#[module(moods = "thinking|sad")]`,
//! deck-catalog's `Descriptor::moods`); `mood_index` is the mood's place in that list + 1, 0 for a mood the list lacks –
//! the module's `ctx.character().mood_index`.

/// bytes of a mood / persona name (the module header's `STR_LEN`): longer names are cut on a char boundary
pub const NAME: usize = 32;

/// The context a deck's module nodes play in (see the module's head).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Context {
    mood: [u8; NAME],
    persona: [u8; NAME],
    /// the being's energy (the mood ladder's walk), 0 … 1
    pub energy: f32,
    /// a hand on the deck, 0 … 1
    pub touch: f32,
    /// how hard the output's limiter works (the being's red flush), 0 … 1
    pub strain: f32,
}

impl Default for Context {
    fn default() -> Self {
        Context { mood: [0; NAME], persona: [0; NAME], energy: 0.0, touch: 0.0, strain: 0.0 }
    }
}

fn put(field: &mut [u8; NAME], s: &str) {
    let mut n = s.len().min(NAME);
    while !s.is_char_boundary(n) {
        n -= 1;
    }
    field[..n].copy_from_slice(&s.as_bytes()[..n]);
    field[n..].fill(0);
}

fn get(field: &[u8; NAME]) -> &str {
    let n = field.iter().position(|&b| b == 0).unwrap_or(NAME);
    core::str::from_utf8(&field[..n]).unwrap_or("")
}

impl Context {
    pub fn new(mood: &str, persona: &str, energy: f32, touch: f32, strain: f32) -> Context {
        let mut c = Context { energy, touch, strain, ..Context::default() };
        put(&mut c.mood, mood);
        put(&mut c.persona, persona);
        c
    }

    /// the mood ("" = none)
    pub fn mood(&self) -> &str {
        get(&self.mood)
    }

    /// the persona ("" = none)
    pub fn persona(&self) -> &str {
        get(&self.persona)
    }
}

/// A1: `mood`'s index in a module's own `moods` list – its position + 1; 0 = a mood the list lacks (or none).
pub fn mood_index<S: AsRef<str>>(moods: &[S], mood: &str) -> u32 {
    if mood.is_empty() {
        return 0;
    }
    moods.iter().position(|m| m.as_ref() == mood).map_or(0, |i| i as u32 + 1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_context_carries_its_names_as_bytes() {
        let c = Context::new("laughing", "dream", 0.5, 0.25, 1.0);
        assert_eq!((c.mood(), c.persona(), c.energy, c.touch, c.strain), ("laughing", "dream", 0.5, 0.25, 1.0));
        assert_eq!(Context::default().mood(), "");
        // cut on a char boundary at 32 bytes, never in the middle of a char
        let long = "ü".repeat(20);
        assert_eq!(Context::new(&long, "", 0.0, 0.0, 0.0).mood(), "ü".repeat(16));
    }

    #[test]
    fn a_mood_counts_in_the_module_s_own_list() {
        let moods = ["thinking", "sad", "happy", "laughing", "surprised"];
        assert_eq!(mood_index(&moods, "thinking"), 1);
        assert_eq!(mood_index(&moods, "surprised"), 5);
        assert_eq!(mood_index(&moods, "neutral"), 0, "a mood the module does not name");
        assert_eq!(mood_index(&moods, ""), 0);
        assert_eq!(mood_index::<&str>(&[], "happy"), 0, "a module without a list");
    }
}
