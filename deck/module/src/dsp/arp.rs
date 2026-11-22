//! The arpeggiator's rule, one copy for the `arp` module and deck-events' `arp` generator: the held notes (a fixed set,
//! latch), the steps on the beat grid and which note a step plays. Step `k` sits at beat `k / per_beat` and its number
//! alone picks the note, so a pattern is on the grid however the blocks are cut.
use super::arith::log2;

/// how many notes the set holds; a key pressed on a full set pushes the oldest out
pub const HELD: usize = 16;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// low to high, then the next octave
    Up,
    Down,
    /// up and back without repeating the ends
    UpDown,
    /// in the order the keys went down
    Played,
}

impl Mode {
    /// a `choices = "up|down|updown|played"` parameter's value
    pub fn from_index(v: f32) -> Mode {
        match v as i32 {
            1 => Mode::Down,
            2 => Mode::UpDown,
            3 => Mode::Played,
            _ => Mode::Up,
        }
    }
}

/// a held note: its id (the note on's), pitch both ways (NaN = absent), velocity and glide
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Note {
    pub id: u32,
    pub hz: f32,
    pub key: f32,
    pub velocity: f32,
    pub glide: f32,
}

impl Note {
    pub const SILENT: Note = Note { id: 0, hz: f32::NAN, key: f32::NAN, velocity: f32::NAN, glide: 0.0 };

    /// the note `semitones` higher, Hz and key alike (an absent one stays absent)
    pub fn moved(&self, semitones: f64) -> Note {
        Note { hz: (f64::from(self.hz) * super::arith::exp2(semitones / 12.0)) as f32, key: (f64::from(self.key) + semitones) as f32, ..*self }
    }

    /// the order up / down sorts by: the key, or the key of the Hz
    fn height(&self) -> f64 {
        if self.key.is_finite() {
            f64::from(self.key)
        } else {
            69.0 + 12.0 * log2(f64::from(self.hz) / 440.0)
        }
    }
}

/// The held notes in play order. With latch on, a released key keeps sounding until the first key pressed after all
/// were let go starts a new set.
#[derive(Debug, Clone, Copy)]
pub struct Held {
    notes: [Note; HELD],
    down: [bool; HELD],
    len: usize,
    latch: bool,
}

impl Default for Held {
    fn default() -> Held {
        Held::new()
    }
}

impl Held {
    pub const fn new() -> Held {
        Held { notes: [Note::SILENT; HELD], down: [false; HELD], len: 0, latch: false }
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn clear(&mut self) {
        self.len = 0;
    }

    /// turning latch off lets go of the released keys
    pub fn set_latch(&mut self, on: bool) {
        if self.latch && !on {
            self.retain_down();
        }
        self.latch = on;
    }

    /// a key down: the same id again replaces its note
    pub fn press(&mut self, note: Note) {
        if self.latch && !self.down[..self.len].contains(&true) {
            self.len = 0;
        }
        if let Some(i) = self.notes[..self.len].iter().position(|n| n.id == note.id) {
            self.notes[i] = note;
            self.down[i] = true;
            return;
        }
        if self.len == HELD {
            self.remove(0);
        }
        self.notes[self.len] = note;
        self.down[self.len] = true;
        self.len += 1;
    }

    /// a key up: the note leaves, or with latch on stays, released
    pub fn release(&mut self, id: u32) {
        let Some(i) = self.notes[..self.len].iter().position(|n| n.id == id) else { return };
        if self.latch {
            self.down[i] = false;
        } else {
            self.remove(i);
        }
    }

    fn remove(&mut self, i: usize) {
        self.notes.copy_within(i + 1..self.len, i);
        self.down.copy_within(i + 1..self.len, i);
        self.len -= 1;
    }

    fn retain_down(&mut self) {
        let mut i = 0;
        while i < self.len {
            if self.down[i] {
                i += 1;
            } else {
                self.remove(i);
            }
        }
    }

    /// step `k`'s note and its octave above the held one (0 … octaves − 1); None while nothing is held
    pub fn step(&self, k: i64, mode: Mode, octaves: u32) -> Option<(Note, u32)> {
        if self.len == 0 {
            return None;
        }
        let mut order = [0u8; HELD];
        for (i, o) in order[..self.len].iter_mut().enumerate() {
            *o = i as u8;
        }
        if mode != Mode::Played {
            // insertion sort, stable: equal heights keep their play order
            for i in 1..self.len {
                let mut j = i;
                while j > 0 && self.notes[order[j - 1] as usize].height() > self.notes[order[j] as usize].height() {
                    order.swap(j - 1, j);
                    j -= 1;
                }
            }
        }
        let i = rung(mode, k, self.len * octaves.max(1) as usize);
        Some((self.notes[order[i % self.len] as usize], (i / self.len) as u32))
    }
}

/// step `k`'s place on a ladder of `n` rungs, low to high
pub fn rung(mode: Mode, k: i64, n: usize) -> usize {
    let n = n.max(1) as i64;
    match mode {
        Mode::Up | Mode::Played => k.rem_euclid(n) as usize,
        Mode::Down => (n - 1 - k.rem_euclid(n)) as usize,
        Mode::UpDown if n < 2 => 0,
        Mode::UpDown => {
            let period = 2 * n - 2;
            let i = k.rem_euclid(period);
            (if i < n { i } else { period - i }) as usize
        }
    }
}

/// the steps on a grid of `per_beat` steps a beat whose start falls in [from, to): (step, its beat)
pub fn steps(from: f64, to: f64, per_beat: f64) -> impl Iterator<Item = (i64, f64)> {
    let ok = per_beat > 0.0 && to > from;
    let first = if ok { libm::ceil(from * per_beat) as i64 } else { 0 };
    let end = if ok { libm::ceil(to * per_beat) as i64 } else { 0 };
    (first..end).map(move |k| (k, k as f64 / per_beat))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn note(id: u32, key: f32) -> Note {
        Note { id, key, ..Note::SILENT }
    }

    fn keys(h: &Held, mode: Mode, octaves: u32, n: i64) -> Vec<f32> {
        (0..n).map(|k| h.step(k, mode, octaves).map(|(n, o)| n.key + 12.0 * o as f32).unwrap()).collect()
    }

    #[test]
    fn the_modes_walk_the_held_notes_through_the_octaves() {
        let mut h = Held::new();
        for (id, key) in [(1, 64.0), (2, 60.0), (3, 67.0)] {
            h.press(note(id, key));
        }
        assert_eq!(keys(&h, Mode::Up, 2, 7), [60.0, 64.0, 67.0, 72.0, 76.0, 79.0, 60.0]);
        assert_eq!(keys(&h, Mode::Down, 1, 4), [67.0, 64.0, 60.0, 67.0]);
        assert_eq!(keys(&h, Mode::UpDown, 1, 6), [60.0, 64.0, 67.0, 64.0, 60.0, 64.0]);
        assert_eq!(keys(&h, Mode::Played, 1, 4), [64.0, 60.0, 67.0, 64.0]);
        assert_eq!(rung(Mode::UpDown, 5, 1), 0);
    }

    #[test]
    fn a_released_key_leaves_unless_latched() {
        let mut h = Held::new();
        h.press(note(1, 60.0));
        h.press(note(2, 64.0));
        h.release(1);
        assert_eq!(keys(&h, Mode::Up, 1, 2), [64.0, 64.0]);
        h.release(2);
        assert!(h.step(0, Mode::Up, 1).is_none());
        h.set_latch(true);
        h.press(note(3, 60.0));
        h.press(note(4, 63.0));
        h.release(3);
        h.release(4);
        assert_eq!(keys(&h, Mode::Up, 1, 2), [60.0, 63.0], "latched: the chord stays");
        h.press(note(5, 70.0));
        assert_eq!(keys(&h, Mode::Up, 1, 2), [70.0, 70.0], "the next key after all were let go starts a new set");
        h.press(note(6, 72.0));
        h.release(5);
        h.set_latch(false);
        assert_eq!(keys(&h, Mode::Up, 1, 1), [72.0], "latch off lets go of the released keys");
    }

    #[test]
    fn a_full_set_pushes_the_oldest_out_and_hz_sorts_like_its_key() {
        let mut h = Held::new();
        for id in 0..HELD as u32 + 1 {
            h.press(note(id, 40.0 + id as f32));
        }
        assert_eq!((h.len(), h.step(0, Mode::Up, 1).unwrap().0.id), (HELD, 1));
        let mut h = Held::new();
        h.press(Note { id: 1, hz: 440.0, ..Note::SILENT });
        h.press(note(2, 60.0));
        assert_eq!(h.step(0, Mode::Up, 1).unwrap().0.id, 2);
        let up = Note { id: 1, hz: 220.0, key: 57.0, ..Note::SILENT }.moved(12.0);
        assert_eq!((up.hz, up.key), (440.0, 69.0));
    }

    #[test]
    fn steps_cover_a_span_once() {
        let a: Vec<i64> = steps(0.0, 1.0, 4.0).map(|(k, _)| k).collect();
        let b: Vec<i64> = steps(0.0, 0.3, 4.0).chain(steps(0.3, 1.0, 4.0)).map(|(k, _)| k).collect();
        assert_eq!(a, [0, 1, 2, 3]);
        assert_eq!(a, b, "cut anywhere, the same steps");
        assert_eq!(steps(1.0, 1.0, 4.0).count(), 0, "a stopped tempo has no steps");
    }
}
