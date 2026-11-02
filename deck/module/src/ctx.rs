//! `Ctx`: what a module sees of the world, read-only (RIG-EDITING.md §4). Writing back happens through declared
//! outputs (a control output, an event) – never by changing a global, so a module cannot break a show.
use crate::abi::{get_str, In, MAX_BLOCK, MAX_VARS};

/// The tempo (deck-core's `TempoMap`, the same formulas): a line through one anchor – frame `anchor_frame` is beat
/// `anchor_beat` – at `bpm` while it runs.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Tempo {
    pub bpm: f64,
    /// frames per second
    pub rate: f64,
    pub running: bool,
    /// beats per bar
    pub per_bar: u32,
    pub anchor_frame: f64,
    pub anchor_beat: f64,
}

impl Tempo {
    /// beats per frame (0 while stopped)
    pub fn per_frame(&self) -> f64 {
        if self.running {
            self.bpm / 60.0 / self.rate
        } else {
            0.0
        }
    }

    pub fn beat_at(&self, frame: f64) -> f64 {
        self.anchor_beat + (frame - self.anchor_frame) * self.per_frame()
    }

    /// the frame at which `beat` is reached (None while stopped)
    pub fn frame_of(&self, beat: f64) -> Option<f64> {
        let p = self.per_frame();
        (p > 0.0).then(|| self.anchor_frame + (beat - self.anchor_beat) / p)
    }
}

/// The character the module plays for (empty strings = none): the persona and mood a rig is played with.
#[derive(Debug, Clone, Copy)]
pub struct Character<'a> {
    pub id: &'a str,
    pub persona: &'a str,
    pub mood: &'a str,
}

/// One call's context: the tempo, the span of audio it covers, the character, the variables, the seed.
#[derive(Clone, Copy)]
pub struct Ctx<'a> {
    inp: &'a In,
}

impl<'a> Ctx<'a> {
    pub fn new(inp: &'a In) -> Ctx<'a> {
        Ctx { inp }
    }

    pub fn tempo(&self) -> Tempo {
        let i = self.inp;
        Tempo { bpm: i.bpm, rate: i.rate, running: i.running != 0, per_bar: i.per_bar.max(1), anchor_frame: i.tempo_frame, anchor_beat: i.tempo_beat }
    }

    /// frames per second
    pub fn rate(&self) -> f64 {
        self.inp.rate
    }

    /// the audio frame of this call's first frame
    pub fn frame(&self) -> f64 {
        self.inp.frame
    }

    /// how many frames this call covers
    pub fn frames(&self) -> usize {
        (self.inp.frames as usize).min(MAX_BLOCK)
    }

    /// the beat at this call's first frame, and at the frame after its last (equal while the tempo stands still)
    pub fn beats(&self) -> (f64, f64) {
        let t = self.tempo();
        (t.beat_at(self.inp.frame), t.beat_at(self.inp.frame + self.inp.frames as f64))
    }

    /// 0..1 within the beat at the call's first frame
    pub fn phase(&self) -> f64 {
        let b = self.beats().0;
        b - b.floor()
    }

    /// the module's seed – the same on every deck that plays the same rig
    pub fn seed(&self) -> u64 {
        self.inp.seed
    }

    pub fn character(&self) -> Character<'a> {
        Character { id: get_str(&self.inp.character_id), persona: get_str(&self.inp.character_persona), mood: get_str(&self.inp.character_mood) }
    }

    /// a scene or world variable by key (`story.act`, `being.glow` …)
    pub fn var(&self, key: &str) -> Option<f32> {
        let n = (self.inp.vars_len as usize).min(MAX_VARS);
        (0..n).find(|&i| get_str(&self.inp.var_keys[i]) == key).map(|i| self.inp.var_values[i])
    }
}
