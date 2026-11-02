//! The scene's `[outputs]` in the browser engine (#282, #211): deck-outputs' platform-free `Engine` runs inside the core
//! on the audio clock and its messages leave as **signal records** – the resolved device messages (protocol/schema.json
//! `DeviceMsg`: midi bytes, osc address + args, dmx channel values) for the page to *show* (the Sound design workspace's
//! signal strip) – the browser plays no MIDI port or DMX widget. Records, little-endian, 4-byte aligned; the TypeScript
//! twin is shared/live-core/src/signals.ts and `fixtures/signals.json` holds records with their bytes (both sides' tests
//! agree on every one):
//!
//! | offset | type | field |
//! |---|---|---|
//! | 0 | u8 | kind – 1 midi, 2 dmx, 3 osc |
//! | 1 | u8 | flags – midi: 0 = the deck's MIDI out port, 1 = the conductor (over the relay); dmx: bit 0 = a flash (sent at once) |
//! | 2 | u16 | n – the payload's bytes (without the padding) |
//! | 4 | u16 | ch – dmx: the first channel (1-based); else 0 |
//! | 6 | u16 | 0 (reserved) |
//! | 8 | u32 | at – the engine clock in ms since the scene loaded (wraps after 49 days) |
//! | 12 | n bytes | midi: status + data bytes · dmx: the channel values from `ch` on · osc: see below, then zeros up to a multiple of 4 |
//!
//! OSC payload: `u16 address length · address (UTF-8) · u8 argument count ·` per argument `u8 tag` (1 int32, 2 float32,
//! 3 string, 4 bool) and its value (`i32` / `f32` little-endian, `u16 length + UTF-8`, `u8`). deck-outputs produces no OSC
//! yet; the format is here so the page's hook and its monitor are complete the day an output does.
use deck_outputs::{Clock, Engine, MidiPort, Out, Plan, Timed};

pub const KIND_MIDI: u8 = 1;
pub const KIND_DMX: u8 = 2;
pub const KIND_OSC: u8 = 3;
pub const HEADER: usize = 12;
/// The bytes one drain holds at most (a stalled page must not grow the core's memory): more is dropped.
pub const CAP: usize = 64 * 1024;

#[derive(Debug, Clone, PartialEq)]
pub enum OscArg {
    Int(i32),
    Float(f32),
    Str(String),
    Bool(bool),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Signal<'a> {
    Midi { conductor: bool, bytes: &'a [u8] },
    Dmx { ch: u16, values: &'a [u8], flash: bool },
    Osc { address: &'a str, args: &'a [OscArg] },
}

/// Appends `s` (at `at_ms`) to `out` as a record; returns false (nothing written) when `out` would pass `CAP`.
pub fn encode(out: &mut Vec<u8>, at_ms: u32, s: Signal) -> bool {
    let start = out.len();
    let (kind, flags, ch): (u8, u8, u16) = match s {
        Signal::Midi { conductor, .. } => (KIND_MIDI, u8::from(conductor), 0),
        Signal::Dmx { ch, flash, .. } => (KIND_DMX, u8::from(flash), ch),
        Signal::Osc { .. } => (KIND_OSC, 0, 0),
    };
    out.extend_from_slice(&[kind, flags, 0, 0]);
    out.extend_from_slice(&ch.to_le_bytes());
    out.extend_from_slice(&[0, 0]);
    out.extend_from_slice(&at_ms.to_le_bytes());
    match s {
        Signal::Midi { bytes, .. } => out.extend_from_slice(bytes),
        Signal::Dmx { values, .. } => out.extend_from_slice(values),
        Signal::Osc { address, args } => {
            out.extend_from_slice(&(address.len().min(0xFFFF) as u16).to_le_bytes());
            out.extend_from_slice(&address.as_bytes()[..address.len().min(0xFFFF)]);
            out.push(args.len().min(0xFF) as u8);
            for a in args.iter().take(0xFF) {
                match a {
                    OscArg::Int(v) => {
                        out.push(1);
                        out.extend_from_slice(&v.to_le_bytes());
                    }
                    OscArg::Float(v) => {
                        out.push(2);
                        out.extend_from_slice(&v.to_le_bytes());
                    }
                    OscArg::Str(v) => {
                        out.push(3);
                        out.extend_from_slice(&(v.len().min(0xFFFF) as u16).to_le_bytes());
                        out.extend_from_slice(&v.as_bytes()[..v.len().min(0xFFFF)]);
                    }
                    OscArg::Bool(v) => {
                        out.push(4);
                        out.push(u8::from(*v));
                    }
                }
            }
        }
    }
    let n = out.len() - start - HEADER;
    if n > 0xFFFF || out.len() > CAP {
        out.truncate(start);
        return false;
    }
    out[start + 2..start + 4].copy_from_slice(&(n as u16).to_le_bytes());
    while (out.len() - start) % 4 != 0 {
        out.push(0);
    }
    true
}

/// Where a parameter path's value comes from in the batch graph: a node's input (`node.port`, as the last step left
/// it) or its output (`node`, `node.out`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Slot {
    In(usize, usize),
    Out(usize),
}

/// A scene's running `[outputs]`: the engine, where its values come from and the clock.
pub struct Outputs {
    engine: Engine,
    slots: Vec<Slot>,
    values: Vec<f32>,
    /// the engine clock (s): the audio rendered since the load
    now: f64,
    /// when the engine wants its next step
    wake: f64,
    timed: Vec<Timed>,
}

impl Outputs {
    /// `None` = the scene has nothing to drive. `slots[i]` is `plan.paths[i]`.
    pub fn new(plan: Plan, slots: Vec<Slot>) -> Option<Outputs> {
        if plan.is_empty() {
            return None;
        }
        let values = vec![0.0; slots.len()];
        Some(Outputs { engine: Engine::new(plan), slots, values, now: 0.0, wake: 0.0, timed: Vec::with_capacity(64) })
    }

    pub fn plan(&self) -> &Plan {
        self.engine.plan()
    }

    pub fn slots(&self) -> &[Slot] {
        &self.slots
    }

    /// `secs` of audio were rendered: when the engine is due, its values are read through `read` and what it sends is
    /// appended to `out` as records. The browser has no tempo clock yet, so a strobe stays dark (its rest values go out).
    pub fn advance(&mut self, secs: f64, read: impl Fn(Slot) -> f32, out: &mut Vec<u8>) {
        self.now += secs;
        if self.now < self.wake - 1e-9 {
            return;
        }
        for (v, s) in self.values.iter_mut().zip(&self.slots) {
            *v = read(*s);
        }
        self.timed.clear();
        let clock: Option<&Clock> = None;
        self.wake = self.engine.step(self.now, &self.values, clock, &mut self.timed);
        // DMX values on consecutive channels leave as one range (the protocol's `dmx` message), like a look does
        let mut run: Vec<u8> = Vec::new();
        let mut run_at = (0u16, 0u32, false);
        let flush = |run: &mut Vec<u8>, at: (u16, u32, bool), out: &mut Vec<u8>| {
            if !run.is_empty() {
                encode(out, at.1, Signal::Dmx { ch: at.0, values: run, flash: at.2 });
                run.clear();
            }
        };
        for t in &self.timed {
            let at_ms = (t.at.max(0.0) * 1000.0) as u32;
            match t.out {
                Out::Midi { port, bytes } => {
                    flush(&mut run, run_at, out);
                    encode(out, at_ms, Signal::Midi { conductor: port == MidiPort::Conductor, bytes: &bytes });
                }
                Out::Dmx { ch, value, now } => {
                    if !run.is_empty() && run_at.2 == now && run_at.0 as usize + run.len() == ch as usize {
                        run.push(value);
                    } else {
                        flush(&mut run, run_at, out);
                        run_at = (ch, at_ms, now);
                        run.push(value);
                    }
                }
            }
        }
        flush(&mut run, run_at, out);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    const FIXTURE: &str = include_str!("../fixtures/signals.json");

    fn hex(b: &[u8]) -> String {
        b.iter().map(|x| format!("{x:02x}")).collect()
    }

    fn osc_args(v: &Value) -> Vec<OscArg> {
        v.as_array()
            .unwrap()
            .iter()
            .map(|a| match a["kind"].as_str().unwrap() {
                "int" => OscArg::Int(a["p"].as_i64().unwrap() as i32),
                "float" => OscArg::Float(a["p"].as_f64().unwrap() as f32),
                "string" => OscArg::Str(a["p"].as_str().unwrap().to_owned()),
                "bool" => OscArg::Bool(a["p"].as_bool().unwrap()),
                k => panic!("{k}"),
            })
            .collect()
    }

    /// Every fixture row encodes to its bytes (shared/live-core's test decodes the same file to the same messages).
    #[test]
    fn fixture_records_encode_to_their_bytes() {
        let rows: Vec<Value> = serde_json::from_str(FIXTURE).unwrap();
        assert!(rows.len() >= 5);
        let mut all = vec![];
        for row in &rows {
            let at = row["at"].as_u64().unwrap() as u32;
            let m = &row["msg"];
            let bytes = |v: &Value| v.as_array().unwrap().iter().map(|x| x.as_u64().unwrap() as u8).collect::<Vec<u8>>();
            let mut out = vec![];
            let ok = match m["kind"].as_str().unwrap() {
                "midi" => encode(&mut out, at, Signal::Midi { conductor: m["port"] == "conductor", bytes: &bytes(&m["bytes"]) }),
                "dmx" => encode(&mut out, at, Signal::Dmx { ch: m["ch"].as_u64().unwrap() as u16, values: &bytes(&m["values"]), flash: m["flash"] == true }),
                "osc" => encode(&mut out, at, Signal::Osc { address: m["address"].as_str().unwrap(), args: &osc_args(&m["args"]) }),
                k => panic!("{k}"),
            };
            assert!(ok);
            assert_eq!(hex(&out), row["hex"].as_str().unwrap(), "{}", row["name"]);
            assert_eq!(out.len() % 4, 0);
            all.extend_from_slice(&out);
        }
        assert!(all.len() > 40);
    }

    #[test]
    fn a_full_buffer_drops_the_record_whole() {
        let mut out = vec![0u8; CAP - 8];
        assert!(!encode(&mut out, 0, Signal::Midi { conductor: false, bytes: &[0xB0, 7, 100] }));
        assert_eq!(out.len(), CAP - 8, "nothing half-written");
        assert!(!encode(&mut Vec::new(), 0, Signal::Dmx { ch: 1, values: &vec![0; 70_000], flash: false }));
    }
}
