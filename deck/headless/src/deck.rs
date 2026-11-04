//! The virtual deck: live-core's `Host` (the adapter API the browser's worker drives through the `lc_*` exports) with a
//! virtual clock and a virtual audio sink instead of a device. Time is the page's ms (`T0` on, the same base every run);
//! the sink "plays" what was rendered at the sample rate, `lead` frames behind the render (the worklet's ring), and stamps
//! the heard moment back like `getOutputTimestamp`. Scripted inputs ride along: a MIDI clock source, a relay leader that
//! answers pings and sends anchors, the page's picker and keys. Nothing here decides anything a deck decides – the
//! numbers go in, the core's state comes out.
use live_core::clock::{ANCHOR, LEADING, LOCKED, MIDI_LOST, PING, SEND, STATE};
use live_core::wire::{self, Mode, VALUES_AT};
use live_core::Host;
use serde_json::{json, Map, Value};

/// the page's ms at the start of every run (a fixed epoch: the reports are the same on every machine)
pub const T0: f64 = 1_762_000_000_000.0;
/// a clock frame every this many ms (the worker's)
pub const FRAME_MS: f64 = 25.0;
/// what the sink renders at a time (one engine quantum, as the worklet asks)
pub const QUANTUM: usize = 128;

/// a scripted MIDI clock source: ticks at 24 per quarter note from `next` on
struct MidiIn {
    bpm: f64,
    next: f64,
    started: bool,
}

/// a scripted relay leader: answers this deck's pings `rtt_ms` later with its clock `offset_ms` ahead
struct Leader {
    offset_ms: f64,
    rtt_ms: f64,
    /// the page ms this deck's last ping went out (answered at the next frame)
    ping_at: Option<f64>,
}

/// A 64-bit FNV-1a over the samples' bits – the audio's fingerprint (the same on every target when the samples are)
#[derive(Clone, Copy)]
pub struct Hash(u64);

impl Default for Hash {
    fn default() -> Hash {
        Hash(0xcbf2_9ce4_8422_2325)
    }
}

impl Hash {
    pub fn samples(&mut self, x: &[f32]) {
        for s in x {
            for b in s.to_bits().to_le_bytes() {
                self.0 ^= u64::from(b);
                self.0 = self.0.wrapping_mul(0x0100_0000_01b3);
            }
        }
    }
    pub fn hex(self) -> String {
        format!("{:016x}", self.0)
    }
}

pub struct Deck {
    pub rate: f32,
    pub channels: usize,
    host: Host,
    /// the page's ms now
    pub now: f64,
    /// when the scene's audio began (page ms) and the frames rendered since
    started: Option<f64>,
    rendered: u64,
    /// the output lead (frames between the render and the ear)
    pub lead: usize,
    /// the load table (live-core's JSON) of the loaded scene
    table: Option<Value>,
    midi: Option<MidiIn>,
    leader: Option<Leader>,
    last_flags: u32,
    /// the last analysis frame
    frame: Vec<f32>,
    /// a pace for a real-time run: called with the ms of each virtual frame (the CLI sleeps; None = as fast as it goes)
    pub pace: Option<Box<dyn FnMut(f64)>>,
}

/// what one `run` saw
#[derive(Default)]
pub struct Seen {
    pub audio: Hash,
    pub peak: f32,
    pub frames: u64,
    pub clock_out: Vec<[f64; 4]>,
    pub anchors: Vec<Value>,
    pub pings: u32,
    pub signals: usize,
    pub signal_bytes: Hash,
    pub flags_any: u32,
}

impl Deck {
    pub fn new(rate: f32, channels: usize) -> Deck {
        Deck {
            rate,
            channels,
            host: Host::new(rate, channels),
            now: T0,
            started: None,
            rendered: 0,
            lead: 1024,
            table: None,
            midi: None,
            leader: None,
            last_flags: 0,
            frame: Vec::new(),
            pace: None,
        }
    }

    /// `lc_load`: a scene's TOML → the load table (or the core's error)
    pub fn load(&mut self, toml: &str) -> Result<Value, String> {
        let bytes = toml.as_bytes();
        self.host.inbox(bytes.len())[..bytes.len()].copy_from_slice(bytes);
        let n = self.host.load(bytes.len());
        let text = String::from_utf8_lossy(&self.host.outbox()[..n.unsigned_abs() as usize]).into_owned();
        if n < 0 {
            return Err(text);
        }
        let table: Value = serde_json::from_str(&text).map_err(|e| format!("the load table: {e}"))?;
        self.table = Some(table.clone());
        self.started = Some(self.now);
        self.rendered = 0;
        Ok(table)
    }

    /// `lc_warm`: the scene played silently by a throwaway twin
    pub fn warm(&mut self, toml: &str, quanta: usize) -> Result<(), String> {
        let bytes = toml.as_bytes();
        self.host.inbox(bytes.len())[..bytes.len()].copy_from_slice(bytes);
        match self.host.warm(bytes.len(), quanta) {
            0 => Ok(()),
            _ => Err(String::from_utf8_lossy(self.host.outbox()).into_owned()),
        }
    }

    /// a path `node.port` → (the node's row, the port's index among its inputs) by the load table
    fn port(&self, path: &str) -> Result<(usize, usize), String> {
        let t = self.table.as_ref().ok_or("no scene loaded")?;
        let (node, port) = path.split_once('.').ok_or_else(|| format!("'{path}' is no node.port"))?;
        let nodes = t["nodes"].as_array().ok_or("a load table without nodes")?;
        let i = nodes.iter().position(|n| n["id"] == node).ok_or_else(|| format!("no node '{node}'"))?;
        let p = nodes[i]["ports"].as_array().and_then(|ps| ps.iter().position(|p| p == port)).ok_or_else(|| format!("node '{node}' has no input '{port}'"))?;
        Ok((i, p))
    }

    /// `lc_apply`: one wire record (a knob, a stick, MIDI learn's value) onto an input
    pub fn send(&mut self, path: &str, value: f32, mode: Mode, amount: f32) -> Result<usize, String> {
        let (node, port) = self.port(path)?;
        let mut rec = Vec::new();
        wire::encode_send(&mut rec, mode, node as u16, port as u16, amount, &[value]);
        self.host.inbox(rec.len())[..rec.len()].copy_from_slice(&rec);
        let n = self.host.apply(rec.len());
        if n < 0 {
            return Err(String::from_utf8_lossy(self.host.outbox()).into_owned());
        }
        Ok(n as usize)
    }

    /// what the core heard last at `node.port` (an input) or `node.<output>` – the analysis frame's values
    pub fn heard(&self, path: &str) -> Result<f32, String> {
        let t = self.table.as_ref().ok_or("no scene loaded")?;
        let (node, port) = path.split_once('.').ok_or_else(|| format!("'{path}' is no node.port"))?;
        let n = t["nodes"].as_array().and_then(|ns| ns.iter().find(|n| n["id"] == node)).ok_or_else(|| format!("no node '{node}'"))?;
        let ins = n["ports"].as_array().map_or(0, Vec::len);
        let k = match n["ports"].as_array().and_then(|ps| ps.iter().position(|p| p == port)) {
            Some(k) => k,
            None => ins + n["outs"].as_array().and_then(|os| os.iter().position(|p| p == port)).ok_or_else(|| format!("node '{node}' has no port '{port}'"))?,
        };
        let at = n["at"].as_i64().filter(|&a| a >= 0).ok_or("past the frame's room")? as usize;
        self.frame.get(VALUES_AT + at + k).copied().ok_or_else(|| "no analysis frame yet (run first)".into())
    }

    pub fn choose(&mut self, source: usize, lead: bool) {
        self.host.clock.choose(source, lead);
    }
    /// the page's picker (#370): `ClockMode::ALL[mode]`
    pub fn mode(&mut self, mode: usize) {
        self.host.clock.set_mode(mode);
    }
    pub fn key(&mut self, key: usize) {
        self.host.clock.key(key, self.now);
    }
    pub fn clock_out(&mut self, on: bool) {
        self.host.clock.set_out(on);
    }
    pub fn joined(&mut self) {
        self.host.clock.joined(self.now);
    }
    pub fn line(&mut self) -> String {
        self.host.clock.line(self.now)
    }
    /// the tempo line of the last frame and the panel's whole line now (#370)
    pub fn tempo_line(&self) -> String {
        self.host.clock.controller().tempo_line()
    }
    pub fn hud_line(&mut self) -> String {
        self.host.clock.hud_line(self.now)
    }
    /// the device's clock from now at `bpm` (None: its ticks stop); `start` = it begins with a Start (0xFA), else it
    /// only ticks (after a Continue)
    pub fn midi_in(&mut self, bpm: Option<f64>, start: bool) {
        self.midi = bpm.map(|bpm| MidiIn { bpm, next: self.now, started: !start });
    }
    /// a MIDI message from the device now (Stop, Continue, a song position)
    pub fn midi(&mut self, bytes: [u8; 3]) {
        self.host.clock.midi(bytes, self.now);
    }
    pub fn leader(&mut self, offset_ms: f64, rtt_ms: f64) {
        self.leader = Some(Leader { offset_ms, rtt_ms, ping_at: None });
    }
    /// the relay's `tempo`: the leader's anchor, stamped with the relay's ms now
    pub fn anchor(&mut self, bpm: f64, per_bar: u32, beat: f64, running: bool) {
        let at = (self.now + self.leader.as_ref().map_or(0.0, |l| l.offset_ms)).round();
        self.host.clock.anchor(deck_core::tempo::Anchor { bpm, per_bar, beat, at: at as i64, running }, self.now);
    }

    /// `ms` of the show: clock frames every `FRAME_MS`, the sink fed a quantum at a time up to `lead` ahead of the ear
    pub fn run(&mut self, ms: f64) -> Seen {
        let mut seen = Seen::default();
        let end = self.now + ms;
        let rate = f64::from(self.rate);
        while self.now < end - 1e-9 {
            let prev = self.now;
            self.now = (self.now + FRAME_MS).min(end);
            if let Some(p) = self.pace.as_mut() {
                p(self.now - prev);
            }
            // the scripted MIDI clock: the start, then 24 ticks a beat, each stamped when it came
            if let Some(m) = self.midi.as_mut() {
                let mut due = Vec::new();
                while m.next <= self.now {
                    if !m.started {
                        due.push(([0xFA, 0, 0], m.next));
                        m.started = true;
                    }
                    due.push(([0xF8, 0, 0], m.next));
                    m.next += 60_000.0 / (m.bpm * 24.0);
                }
                for (b, at) in due {
                    self.host.clock.midi(b, at);
                }
            }
            // the leader answers the last frame's ping
            if let Some(l) = self.leader.as_mut() {
                if let Some(at) = l.ping_at.take() {
                    self.host.clock.pong(at * 1e3, at + l.rtt_ms / 2.0 + l.offset_ms, at + l.rtt_ms);
                }
            }
            // the sink: render until the ring holds `lead` frames past the ear, the ear stamped like getOutputTimestamp
            if let Some(start) = self.started {
                let target = ((self.now - start) * rate / 1e3).round() as u64 + self.lead as u64;
                while self.rendered < target {
                    let Some(block) = self.host.render(QUANTUM) else { break };
                    seen.audio.samples(block);
                    seen.peak = block.iter().fold(seen.peak, |m, x| m.max(x.abs()));
                    self.rendered += QUANTUM as u64;
                    seen.frames += QUANTUM as u64;
                }
                let heard = self.rendered.saturating_sub(self.lead as u64) as f64;
                self.host.clock.stamp(heard / rate, start + heard * 1e3 / rate, 0.0);
            }
            let state = *self.host.clock.frame(self.now);
            let flags = state[6] as u32;
            seen.flags_any |= flags;
            self.last_flags = flags;
            for m in self.host.clock.out().chunks_exact(4) {
                seen.clock_out.push([m[0], m[1], m[2], m[3]]);
            }
            if flags & PING != 0 {
                seen.pings += 1;
                if let Some(l) = self.leader.as_mut() {
                    l.ping_at = Some(self.now);
                }
            }
            if flags & SEND != 0 {
                seen.anchors.push(json!({ "bpm": round(state[10], 3), "per_bar": state[11], "beat": round(state[12], 3), "running": state[14] != 0.0 }));
            }
            if self.started.is_some() {
                if let Some(f) = self.host.analyse((FRAME_MS / 1e3) as f32, self.lead) {
                    self.frame.clear();
                    self.frame.extend_from_slice(f);
                }
                if let Some(core) = self.host.core() {
                    let s = core.take_signals();
                    if !s.is_empty() {
                        seen.signals += records(s);
                        seen.signal_bytes.samples(&s.iter().map(|&b| f32::from(b)).collect::<Vec<_>>());
                    }
                }
            }
        }
        seen
    }

    /// the clock's state block now, as named numbers (lc_clock_frame's block, the last frame)
    pub fn state(&self) -> Map<String, Value> {
        let s: &[f64; STATE] = self.host.clock.state();
        let f = s[6] as u32;
        let source = deck_core::tempo::Source::ALL.get(s[5] as usize).map_or("?", |x| x.name());
        let mut m = Map::new();
        m.insert("beat".into(), json!(round(s[1], 3)));
        m.insert("bpm".into(), json!(round(s[2], 3)));
        m.insert("per_bar".into(), json!(s[3]));
        m.insert("running".into(), json!(s[4] != 0.0));
        m.insert("source".into(), json!(source));
        m.insert("locked".into(), json!(f & LOCKED != 0));
        m.insert("anchor".into(), json!(f & ANCHOR != 0));
        m.insert("leading".into(), json!(f & LEADING != 0));
        m.insert("midi_lost".into(), json!(f & MIDI_LOST != 0));
        m.insert("click".into(), json!(s[15] != 0.0));
        // #359: the transport's epoch the engine plays on (a start, stop or position jump each count one)
        m.insert("epoch".into(), json!(self.host.clock.controller().shared().transport().map_or(0, |t| t.2)));
        // #370: the picker's choice, the MIDI clock out, the strobe's flash (its length, 0 = none; its edge in ms from the heard moment)
        let mode = deck_core::tempo::ClockMode::ALL.get(s[16] as usize).filter(|_| s[16] >= 0.0).map(|m| m.name());
        m.insert("mode".into(), json!(mode));
        m.insert("out".into(), json!(s[17] != 0.0));
        m.insert("flash_ms".into(), json!(s[19]));
        m.insert("flash_edge_ms".into(), json!(if s[19] > 0.0 { round(s[18] - s[0], 3) } else { 0.0 }));
        m
    }

    /// #374: what this deck's HUD reads, for deck-ui to say: the scene's name, deck-core's monitor at an `fps` target fed
    /// one second of its frames, the virtual sink as the audio device (`audio`), the clock's tempo line, source and status
    pub fn hud_status(&mut self, scene: &str, fps: f32, audio: bool) -> deck_ui::Status {
        let mut m = deck_core::monitor::Monitor::new();
        m.target(fps);
        let frames = fps.max(0.0) as u32;
        for k in 0..=frames {
            m.frame(f64::from(k) * 1000.0 / f64::from(fps.max(1.0)), 0.0, true);
        }
        let out = self.host.clock.state()[17] != 0.0;
        deck_ui::Status {
            scene: scene.into(),
            report: m.report(1000.0),
            audio: audio.then(|| deck_ui::Audio { name: "virtual".into(), rate: self.rate as u32, buffer: QUANTUM as u32, lead: self.lead, clock_rate: self.rate }),
            tempo: self.tempo_line(),
            source: deck_ui::Source { name: self.host.clock.controller().source.name().into(), status: self.line(), midi_out: out },
            ..deck_ui::Status::default()
        }
    }

    /// the analysis frame's bands (level, bass, mid, high) of the last frame
    pub fn bands(&self) -> [f32; 4] {
        let mut b = [0.0; 4];
        for (i, x) in self.frame.iter().take(4).enumerate() {
            b[i] = *x;
        }
        b
    }
}

/// signal records in `bytes` (signals.rs: a 12-byte header with the payload's length at 2..4, padded to 4)
fn records(bytes: &[u8]) -> usize {
    let mut n = 0;
    let mut at = 0;
    while at + live_core::signals::HEADER <= bytes.len() {
        let len = usize::from(u16::from_le_bytes([bytes[at + 2], bytes[at + 3]]));
        at += live_core::signals::HEADER + len.div_ceil(4) * 4;
        n += 1;
    }
    n
}

/// a number to `digits` decimals (the report's numbers stay readable and the same on every target)
pub fn round(x: f64, digits: i32) -> f64 {
    let p = 10f64.powi(digits);
    (x * p).round() / p
}
