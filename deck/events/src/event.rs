//! The event (research/EVENTS.md §2, §14): a start, a duration, a name from the dictionary, a few fast props, children
//! (compound events: a phrase of notes, a cue of actions) and the hops it has made (a loop guard, graph.rs).
//!
//! Time (§13, owner 2026-11-01): a **trigger** happens `Now`; a **timed** event sits on the timeline in `Beats` (music,
//! deck-core's tempo map) or `Seconds` (clips, video – the audio timebase: frame = seconds × rate). Big data never
//! rides on an event (§16): a prop is a number; a reference is an id the receiver resolves itself.
use crate::names::Sym;
use serde_json::Value;
use deck_core::tempo::TempoMap;

/// fast props per event (real-time safe: no allocation)
pub const MAX_PROPS: usize = 6;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Time {
    /// a trigger: as soon as possible
    Now,
    /// on the tempo map (absolute beats; a child's: beats after its parent's start)
    Beats(f64),
    /// on the audio timebase (absolute seconds; a child's: seconds after its parent's start)
    Seconds(f64),
}

/// up to MAX_PROPS (key, value) pairs, in insertion order
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Props {
    keys: [Sym; MAX_PROPS],
    vals: [f32; MAX_PROPS],
    len: u8,
}

impl Props {
    /// set `key` (replacing its value); false when all slots are taken by other keys
    pub fn set(&mut self, key: Sym, value: f32) -> bool {
        let n = self.len as usize;
        if let Some(i) = self.keys[..n].iter().position(|k| *k == key) {
            self.vals[i] = value;
            return true;
        }
        if n == MAX_PROPS {
            return false;
        }
        self.keys[n] = key;
        self.vals[n] = value;
        self.len += 1;
        true
    }

    pub fn get(&self, key: Sym) -> Option<f32> {
        let n = self.len as usize;
        self.keys[..n].iter().position(|k| *k == key).map(|i| self.vals[i])
    }

    pub fn len(&self) -> usize {
        self.len as usize
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn iter(&self) -> impl Iterator<Item = (Sym, f32)> + '_ {
        self.keys[..self.len()].iter().copied().zip(self.vals[..self.len()].iter().copied())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Event {
    pub start: Time,
    /// none = a moment
    pub dur: Option<Time>,
    pub name: Sym,
    pub props: Props,
    /// compound: starts relative to this event's start (`flatten` makes them absolute)
    pub children: Vec<Event>,
    /// edges crossed so far (graph.rs drops an event past MAX_HOPS)
    pub hops: u8,
    /// loose side table (text, ids, references – never on the audio thread; the adapters refuse > `MAX_DATA` bytes)
    pub data: Option<Box<Value>>,
    /// a stable id (the story's `id`, lineage)
    pub id: Option<String>,
    /// where it came from: `story:catpope/2`, `relay:c7`, `midi:keys`
    pub source: Option<String>,
}

/// bytes of serialised `data` an adapter lets through (§16: "send a reference")
pub const MAX_DATA: usize = 1024;

impl Event {
    pub fn new(name: Sym, start: Time) -> Event {
        Event { start, dur: None, name, props: Props::default(), children: Vec::new(), hops: 0, data: None, id: None, source: None }
    }

    /// a trigger
    pub fn now(name: Sym) -> Event {
        Event::new(name, Time::Now)
    }

    pub fn beats(name: Sym, beat: f64) -> Event {
        Event::new(name, Time::Beats(beat))
    }

    /// with a prop (ignored when all slots are taken – MAX_PROPS)
    pub fn with(mut self, key: Sym, value: f32) -> Event {
        self.props.set(key, value);
        self
    }

    pub fn lasting(mut self, dur: Time) -> Event {
        self.dur = Some(dur);
        self
    }

    pub fn containing(mut self, children: Vec<Event>) -> Event {
        self.children = children;
        self
    }

    /// its start on the audio timebase (frames); a trigger is `now`, a beat while the tempo stops is `now` too
    pub fn frame(&self, tempo: &TempoMap, now: f64) -> f64 {
        match self.start {
            Time::Now => now,
            Time::Beats(b) => tempo.frame_of(b).unwrap_or(now),
            Time::Seconds(s) => s * tempo.rate,
        }
    }

    /// its length in frames (0 = a moment, or beats while the tempo stops)
    pub fn dur_frames(&self, tempo: &TempoMap) -> f64 {
        match self.dur {
            None | Some(Time::Now) => 0.0,
            Some(Time::Beats(b)) => {
                let p = tempo.per_frame();
                if p > 0.0 {
                    b / p
                } else {
                    0.0
                }
            }
            Some(Time::Seconds(s)) => s * tempo.rate,
        }
    }

    /// This event and its children (recursively) as flat events with absolute starts, parents first. A child's start
    /// is an offset in its own unit after the parent's start (a `Now` child starts with its parent).
    pub fn flatten(&self, tempo: &TempoMap, now: f64, out: &mut Vec<Event>) {
        let at = self.frame(tempo, now);
        out.push(Event { children: Vec::new(), ..self.clone() });
        for c in &self.children {
            let start = match c.start {
                Time::Now => self.start,
                Time::Beats(off) => Time::Beats(tempo.beat_at(at) + off),
                Time::Seconds(off) => Time::Seconds(at / tempo.rate + off),
            };
            Event { start, ..c.clone() }.flatten(tempo, now, out);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::names::Names;

    fn tempo() -> TempoMap {
        // 120 bpm at 48 kHz: a beat = 24 000 frames
        TempoMap::new(120.0, 4, 48_000.0)
    }

    #[test]
    fn props_set_replace_and_fill() {
        let mut n = Names::new();
        let keys: Vec<Sym> = (0..7).map(|i| n.key(&format!("k{i}")).unwrap()).collect();
        let mut p = Props::default();
        for (i, k) in keys.iter().take(MAX_PROPS).enumerate() {
            assert!(p.set(*k, i as f32));
        }
        assert!(!p.set(keys[6], 9.0), "full");
        assert!(p.set(keys[0], 5.0), "replacing is always possible");
        assert_eq!(p.get(keys[0]), Some(5.0));
        assert_eq!(p.len(), MAX_PROPS);
        assert_eq!(p.get(keys[6]), None);
    }

    #[test]
    fn times_resolve_to_frames() {
        let mut n = Names::new();
        let note = n.event("music.note").unwrap();
        let t = tempo();
        assert_eq!(Event::beats(note, 2.0).frame(&t, 7.0), 48_000.0);
        assert_eq!(Event::new(note, Time::Seconds(0.5)).frame(&t, 7.0), 24_000.0);
        assert_eq!(Event::now(note).frame(&t, 7.0), 7.0);
        assert_eq!(Event::now(note).lasting(Time::Beats(0.5)).dur_frames(&t), 12_000.0);
        let stopped = TempoMap { running: false, ..t };
        assert_eq!(Event::beats(note, 2.0).frame(&stopped, 7.0), 7.0, "a stopped clock plays it now");
    }

    #[test]
    fn compound_events_flatten_to_absolute_starts() {
        let mut n = Names::new();
        let (phrase, note) = (n.event("music.phrase").unwrap(), n.event("music.note").unwrap());
        let e = Event::beats(phrase, 4.0).containing(vec![
            Event::beats(note, 0.0),
            Event::beats(note, 0.5).containing(vec![Event::beats(note, 0.25)]),
            Event::new(note, Time::Seconds(1.0)),
        ]);
        let mut out = Vec::new();
        e.flatten(&tempo(), 0.0, &mut out);
        let starts: Vec<Time> = out.iter().map(|e| e.start).collect();
        assert_eq!(starts, [Time::Beats(4.0), Time::Beats(4.0), Time::Beats(4.5), Time::Beats(4.75), Time::Seconds(3.0)]);
        assert!(out.iter().all(|e| e.children.is_empty()));
    }
}
