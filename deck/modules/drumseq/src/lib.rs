//! `drumseq` – a drum machine's sequencer: eight lanes of sixteen steps on the tempo clock, each lane a `notes` output
//! (`lane1` … `lane8`) that a cable takes to an instrument's `notes` – one `analog` with a drum preset a lane. A step is
//! off, on or accented: it plays its lane's note (`note1` … `note8`, a MIDI key; the host fills the pitch from the deck's
//! tuning) at the `normal` or the `accent` velocity, lasting `length` of a step. `rate` steps a beat, the pattern wraps
//! after `steps`, `swing` makes every second step late (100 % = a third of a step, the triplet shuffle). The step comes
//! from the beat (the SDK's grid), so the transport moves it: stopped nothing plays, a jump plays from the new beat, and a
//! render is the same however the host cuts the blocks. The default pattern is a punk beat (kick, snare, closed and open
//! hat on lanes 1 – 4). Control output `step`: the step under the clock, 1 … `steps`, 0 while stopped.
//!
//! Build: `cargo build --release --target wasm32-unknown-unknown -p drumseq-module` (the .wasm in
//! `target/wasm32-unknown-unknown/release/drumseq_module.wasm`).
// one `#[port(…)]` a port: clippy takes them for duplicates (deck/module/README.md)
#![allow(clippy::duplicated_attributes)]
use deck_module::dsp::grid::{frame_in, steps};
use deck_module::{math, Block, Ctx, Ev, Module};

/// lanes and steps
pub const LANES: usize = 8;
pub const STEPS: usize = 16;
/// a cell's value: off, on, accented
pub const OFF: f32 = 0.0;
pub const ON: f32 = 1.0;
pub const ACCENT: f32 = 2.0;
/// how late `swing` 100 % makes a second step, in steps
const SWING_MAX: f64 = 1.0 / 3.0;

#[derive(Module)]
#[module(name = "drumseq", doc = "a drum sequencer: 8 lanes of 16 steps on the tempo clock, accent and swing, a notes output a lane",
    sections = ["SEQ/MAIN", "LANE 1/1-8", "LANE 1/9-16", "LANE 1/NOTE", "LANE 2/1-8", "LANE 2/9-16", "LANE 2/NOTE", "LANE 3/1-8", "LANE 3/9-16", "LANE 3/NOTE", "LANE 4/1-8", "LANE 4/9-16", "LANE 4/NOTE", "LANE 5/1-8", "LANE 5/9-16", "LANE 5/NOTE", "LANE 6/1-8", "LANE 6/9-16", "LANE 6/NOTE", "LANE 7/1-8", "LANE 7/9-16", "LANE 7/NOTE", "LANE 8/1-8", "LANE 8/9-16", "LANE 8/NOTE"])]
#[port(output, event, name = "lane1", names = "music.note", props = "pitch|vel|glide|key", capacity = 32, doc = "lane 1's steps as notes (kick by default)")]
#[port(output, event, name = "lane2", names = "music.note", props = "pitch|vel|glide|key", capacity = 32, doc = "lane 2's steps as notes (snare by default)")]
#[port(output, event, name = "lane3", names = "music.note", props = "pitch|vel|glide|key", capacity = 32, doc = "lane 3's steps as notes (closed hat by default)")]
#[port(output, event, name = "lane4", names = "music.note", props = "pitch|vel|glide|key", capacity = 32, doc = "lane 4's steps as notes (open hat by default)")]
#[port(output, event, name = "lane5", names = "music.note", props = "pitch|vel|glide|key", capacity = 32, doc = "lane 5's steps as notes (clap by default)")]
#[port(output, event, name = "lane6", names = "music.note", props = "pitch|vel|glide|key", capacity = 32, doc = "lane 6's steps as notes (low tom by default)")]
#[port(output, event, name = "lane7", names = "music.note", props = "pitch|vel|glide|key", capacity = 32, doc = "lane 7's steps as notes (crash by default)")]
#[port(output, event, name = "lane8", names = "music.note", props = "pitch|vel|glide|key", capacity = 32, doc = "lane 8's steps as notes (rim by default)")]
#[port(output, control, name = "step", doc = "the step under the clock, 1 … steps; 0 while stopped")]
#[preset(name = "skank", doc = "The punk two-beat: the kick on every beat, the snare on every 'and', the closed hat on the eighths.",
    values(l1s5 = 2.0, l1s11 = 0.0, l1s13 = 2.0, l2s3 = 2.0, l2s5 = 0.0, l2s7 = 2.0, l2s11 = 2.0, l2s13 = 0.0, l2s15 = 2.0, l3s15 = 1.0, l4s15 = 0.0))]
pub struct DrumSeq {
    #[param(choices = "off|on|accent", default = 2, section = "LANE 1/1-8", doc = "lane 1, step 1")]
    pub l1s1: f32,
    #[param(choices = "off|on|accent", default = 0, section = "LANE 1/1-8", doc = "lane 1, step 2")]
    pub l1s2: f32,
    #[param(choices = "off|on|accent", default = 0, section = "LANE 1/1-8", doc = "lane 1, step 3")]
    pub l1s3: f32,
    #[param(choices = "off|on|accent", default = 0, section = "LANE 1/1-8", doc = "lane 1, step 4")]
    pub l1s4: f32,
    #[param(choices = "off|on|accent", default = 0, section = "LANE 1/1-8", doc = "lane 1, step 5")]
    pub l1s5: f32,
    #[param(choices = "off|on|accent", default = 0, section = "LANE 1/1-8", doc = "lane 1, step 6")]
    pub l1s6: f32,
    #[param(choices = "off|on|accent", default = 0, section = "LANE 1/1-8", doc = "lane 1, step 7")]
    pub l1s7: f32,
    #[param(choices = "off|on|accent", default = 0, section = "LANE 1/1-8", doc = "lane 1, step 8")]
    pub l1s8: f32,
    #[param(choices = "off|on|accent", default = 2, section = "LANE 1/9-16", doc = "lane 1, step 9")]
    pub l1s9: f32,
    #[param(choices = "off|on|accent", default = 0, section = "LANE 1/9-16", doc = "lane 1, step 10")]
    pub l1s10: f32,
    #[param(choices = "off|on|accent", default = 1, section = "LANE 1/9-16", doc = "lane 1, step 11")]
    pub l1s11: f32,
    #[param(choices = "off|on|accent", default = 0, section = "LANE 1/9-16", doc = "lane 1, step 12")]
    pub l1s12: f32,
    #[param(choices = "off|on|accent", default = 0, section = "LANE 1/9-16", doc = "lane 1, step 13")]
    pub l1s13: f32,
    #[param(choices = "off|on|accent", default = 0, section = "LANE 1/9-16", doc = "lane 1, step 14")]
    pub l1s14: f32,
    #[param(choices = "off|on|accent", default = 0, section = "LANE 1/9-16", doc = "lane 1, step 15")]
    pub l1s15: f32,
    #[param(choices = "off|on|accent", default = 0, section = "LANE 1/9-16", doc = "lane 1, step 16")]
    pub l1s16: f32,
    #[param(choices = "off|on|accent", default = 0, section = "LANE 2/1-8", doc = "lane 2, step 1")]
    pub l2s1: f32,
    #[param(choices = "off|on|accent", default = 0, section = "LANE 2/1-8", doc = "lane 2, step 2")]
    pub l2s2: f32,
    #[param(choices = "off|on|accent", default = 0, section = "LANE 2/1-8", doc = "lane 2, step 3")]
    pub l2s3: f32,
    #[param(choices = "off|on|accent", default = 0, section = "LANE 2/1-8", doc = "lane 2, step 4")]
    pub l2s4: f32,
    #[param(choices = "off|on|accent", default = 2, section = "LANE 2/1-8", doc = "lane 2, step 5")]
    pub l2s5: f32,
    #[param(choices = "off|on|accent", default = 0, section = "LANE 2/1-8", doc = "lane 2, step 6")]
    pub l2s6: f32,
    #[param(choices = "off|on|accent", default = 0, section = "LANE 2/1-8", doc = "lane 2, step 7")]
    pub l2s7: f32,
    #[param(choices = "off|on|accent", default = 0, section = "LANE 2/1-8", doc = "lane 2, step 8")]
    pub l2s8: f32,
    #[param(choices = "off|on|accent", default = 0, section = "LANE 2/9-16", doc = "lane 2, step 9")]
    pub l2s9: f32,
    #[param(choices = "off|on|accent", default = 0, section = "LANE 2/9-16", doc = "lane 2, step 10")]
    pub l2s10: f32,
    #[param(choices = "off|on|accent", default = 0, section = "LANE 2/9-16", doc = "lane 2, step 11")]
    pub l2s11: f32,
    #[param(choices = "off|on|accent", default = 0, section = "LANE 2/9-16", doc = "lane 2, step 12")]
    pub l2s12: f32,
    #[param(choices = "off|on|accent", default = 2, section = "LANE 2/9-16", doc = "lane 2, step 13")]
    pub l2s13: f32,
    #[param(choices = "off|on|accent", default = 0, section = "LANE 2/9-16", doc = "lane 2, step 14")]
    pub l2s14: f32,
    #[param(choices = "off|on|accent", default = 0, section = "LANE 2/9-16", doc = "lane 2, step 15")]
    pub l2s15: f32,
    #[param(choices = "off|on|accent", default = 0, section = "LANE 2/9-16", doc = "lane 2, step 16")]
    pub l2s16: f32,
    #[param(choices = "off|on|accent", default = 2, section = "LANE 3/1-8", doc = "lane 3, step 1")]
    pub l3s1: f32,
    #[param(choices = "off|on|accent", default = 0, section = "LANE 3/1-8", doc = "lane 3, step 2")]
    pub l3s2: f32,
    #[param(choices = "off|on|accent", default = 1, section = "LANE 3/1-8", doc = "lane 3, step 3")]
    pub l3s3: f32,
    #[param(choices = "off|on|accent", default = 0, section = "LANE 3/1-8", doc = "lane 3, step 4")]
    pub l3s4: f32,
    #[param(choices = "off|on|accent", default = 2, section = "LANE 3/1-8", doc = "lane 3, step 5")]
    pub l3s5: f32,
    #[param(choices = "off|on|accent", default = 0, section = "LANE 3/1-8", doc = "lane 3, step 6")]
    pub l3s6: f32,
    #[param(choices = "off|on|accent", default = 1, section = "LANE 3/1-8", doc = "lane 3, step 7")]
    pub l3s7: f32,
    #[param(choices = "off|on|accent", default = 0, section = "LANE 3/1-8", doc = "lane 3, step 8")]
    pub l3s8: f32,
    #[param(choices = "off|on|accent", default = 2, section = "LANE 3/9-16", doc = "lane 3, step 9")]
    pub l3s9: f32,
    #[param(choices = "off|on|accent", default = 0, section = "LANE 3/9-16", doc = "lane 3, step 10")]
    pub l3s10: f32,
    #[param(choices = "off|on|accent", default = 1, section = "LANE 3/9-16", doc = "lane 3, step 11")]
    pub l3s11: f32,
    #[param(choices = "off|on|accent", default = 0, section = "LANE 3/9-16", doc = "lane 3, step 12")]
    pub l3s12: f32,
    #[param(choices = "off|on|accent", default = 2, section = "LANE 3/9-16", doc = "lane 3, step 13")]
    pub l3s13: f32,
    #[param(choices = "off|on|accent", default = 0, section = "LANE 3/9-16", doc = "lane 3, step 14")]
    pub l3s14: f32,
    #[param(choices = "off|on|accent", default = 0, section = "LANE 3/9-16", doc = "lane 3, step 15")]
    pub l3s15: f32,
    #[param(choices = "off|on|accent", default = 0, section = "LANE 3/9-16", doc = "lane 3, step 16")]
    pub l3s16: f32,
    #[param(choices = "off|on|accent", default = 0, section = "LANE 4/1-8", doc = "lane 4, step 1")]
    pub l4s1: f32,
    #[param(choices = "off|on|accent", default = 0, section = "LANE 4/1-8", doc = "lane 4, step 2")]
    pub l4s2: f32,
    #[param(choices = "off|on|accent", default = 0, section = "LANE 4/1-8", doc = "lane 4, step 3")]
    pub l4s3: f32,
    #[param(choices = "off|on|accent", default = 0, section = "LANE 4/1-8", doc = "lane 4, step 4")]
    pub l4s4: f32,
    #[param(choices = "off|on|accent", default = 0, section = "LANE 4/1-8", doc = "lane 4, step 5")]
    pub l4s5: f32,
    #[param(choices = "off|on|accent", default = 0, section = "LANE 4/1-8", doc = "lane 4, step 6")]
    pub l4s6: f32,
    #[param(choices = "off|on|accent", default = 0, section = "LANE 4/1-8", doc = "lane 4, step 7")]
    pub l4s7: f32,
    #[param(choices = "off|on|accent", default = 0, section = "LANE 4/1-8", doc = "lane 4, step 8")]
    pub l4s8: f32,
    #[param(choices = "off|on|accent", default = 0, section = "LANE 4/9-16", doc = "lane 4, step 9")]
    pub l4s9: f32,
    #[param(choices = "off|on|accent", default = 0, section = "LANE 4/9-16", doc = "lane 4, step 10")]
    pub l4s10: f32,
    #[param(choices = "off|on|accent", default = 0, section = "LANE 4/9-16", doc = "lane 4, step 11")]
    pub l4s11: f32,
    #[param(choices = "off|on|accent", default = 0, section = "LANE 4/9-16", doc = "lane 4, step 12")]
    pub l4s12: f32,
    #[param(choices = "off|on|accent", default = 0, section = "LANE 4/9-16", doc = "lane 4, step 13")]
    pub l4s13: f32,
    #[param(choices = "off|on|accent", default = 0, section = "LANE 4/9-16", doc = "lane 4, step 14")]
    pub l4s14: f32,
    #[param(choices = "off|on|accent", default = 1, section = "LANE 4/9-16", doc = "lane 4, step 15")]
    pub l4s15: f32,
    #[param(choices = "off|on|accent", default = 0, section = "LANE 4/9-16", doc = "lane 4, step 16")]
    pub l4s16: f32,
    #[param(choices = "off|on|accent", default = 0, section = "LANE 5/1-8", doc = "lane 5, step 1")]
    pub l5s1: f32,
    #[param(choices = "off|on|accent", default = 0, section = "LANE 5/1-8", doc = "lane 5, step 2")]
    pub l5s2: f32,
    #[param(choices = "off|on|accent", default = 0, section = "LANE 5/1-8", doc = "lane 5, step 3")]
    pub l5s3: f32,
    #[param(choices = "off|on|accent", default = 0, section = "LANE 5/1-8", doc = "lane 5, step 4")]
    pub l5s4: f32,
    #[param(choices = "off|on|accent", default = 0, section = "LANE 5/1-8", doc = "lane 5, step 5")]
    pub l5s5: f32,
    #[param(choices = "off|on|accent", default = 0, section = "LANE 5/1-8", doc = "lane 5, step 6")]
    pub l5s6: f32,
    #[param(choices = "off|on|accent", default = 0, section = "LANE 5/1-8", doc = "lane 5, step 7")]
    pub l5s7: f32,
    #[param(choices = "off|on|accent", default = 0, section = "LANE 5/1-8", doc = "lane 5, step 8")]
    pub l5s8: f32,
    #[param(choices = "off|on|accent", default = 0, section = "LANE 5/9-16", doc = "lane 5, step 9")]
    pub l5s9: f32,
    #[param(choices = "off|on|accent", default = 0, section = "LANE 5/9-16", doc = "lane 5, step 10")]
    pub l5s10: f32,
    #[param(choices = "off|on|accent", default = 0, section = "LANE 5/9-16", doc = "lane 5, step 11")]
    pub l5s11: f32,
    #[param(choices = "off|on|accent", default = 0, section = "LANE 5/9-16", doc = "lane 5, step 12")]
    pub l5s12: f32,
    #[param(choices = "off|on|accent", default = 0, section = "LANE 5/9-16", doc = "lane 5, step 13")]
    pub l5s13: f32,
    #[param(choices = "off|on|accent", default = 0, section = "LANE 5/9-16", doc = "lane 5, step 14")]
    pub l5s14: f32,
    #[param(choices = "off|on|accent", default = 0, section = "LANE 5/9-16", doc = "lane 5, step 15")]
    pub l5s15: f32,
    #[param(choices = "off|on|accent", default = 0, section = "LANE 5/9-16", doc = "lane 5, step 16")]
    pub l5s16: f32,
    #[param(choices = "off|on|accent", default = 0, section = "LANE 6/1-8", doc = "lane 6, step 1")]
    pub l6s1: f32,
    #[param(choices = "off|on|accent", default = 0, section = "LANE 6/1-8", doc = "lane 6, step 2")]
    pub l6s2: f32,
    #[param(choices = "off|on|accent", default = 0, section = "LANE 6/1-8", doc = "lane 6, step 3")]
    pub l6s3: f32,
    #[param(choices = "off|on|accent", default = 0, section = "LANE 6/1-8", doc = "lane 6, step 4")]
    pub l6s4: f32,
    #[param(choices = "off|on|accent", default = 0, section = "LANE 6/1-8", doc = "lane 6, step 5")]
    pub l6s5: f32,
    #[param(choices = "off|on|accent", default = 0, section = "LANE 6/1-8", doc = "lane 6, step 6")]
    pub l6s6: f32,
    #[param(choices = "off|on|accent", default = 0, section = "LANE 6/1-8", doc = "lane 6, step 7")]
    pub l6s7: f32,
    #[param(choices = "off|on|accent", default = 0, section = "LANE 6/1-8", doc = "lane 6, step 8")]
    pub l6s8: f32,
    #[param(choices = "off|on|accent", default = 0, section = "LANE 6/9-16", doc = "lane 6, step 9")]
    pub l6s9: f32,
    #[param(choices = "off|on|accent", default = 0, section = "LANE 6/9-16", doc = "lane 6, step 10")]
    pub l6s10: f32,
    #[param(choices = "off|on|accent", default = 0, section = "LANE 6/9-16", doc = "lane 6, step 11")]
    pub l6s11: f32,
    #[param(choices = "off|on|accent", default = 0, section = "LANE 6/9-16", doc = "lane 6, step 12")]
    pub l6s12: f32,
    #[param(choices = "off|on|accent", default = 0, section = "LANE 6/9-16", doc = "lane 6, step 13")]
    pub l6s13: f32,
    #[param(choices = "off|on|accent", default = 0, section = "LANE 6/9-16", doc = "lane 6, step 14")]
    pub l6s14: f32,
    #[param(choices = "off|on|accent", default = 0, section = "LANE 6/9-16", doc = "lane 6, step 15")]
    pub l6s15: f32,
    #[param(choices = "off|on|accent", default = 0, section = "LANE 6/9-16", doc = "lane 6, step 16")]
    pub l6s16: f32,
    #[param(choices = "off|on|accent", default = 0, section = "LANE 7/1-8", doc = "lane 7, step 1")]
    pub l7s1: f32,
    #[param(choices = "off|on|accent", default = 0, section = "LANE 7/1-8", doc = "lane 7, step 2")]
    pub l7s2: f32,
    #[param(choices = "off|on|accent", default = 0, section = "LANE 7/1-8", doc = "lane 7, step 3")]
    pub l7s3: f32,
    #[param(choices = "off|on|accent", default = 0, section = "LANE 7/1-8", doc = "lane 7, step 4")]
    pub l7s4: f32,
    #[param(choices = "off|on|accent", default = 0, section = "LANE 7/1-8", doc = "lane 7, step 5")]
    pub l7s5: f32,
    #[param(choices = "off|on|accent", default = 0, section = "LANE 7/1-8", doc = "lane 7, step 6")]
    pub l7s6: f32,
    #[param(choices = "off|on|accent", default = 0, section = "LANE 7/1-8", doc = "lane 7, step 7")]
    pub l7s7: f32,
    #[param(choices = "off|on|accent", default = 0, section = "LANE 7/1-8", doc = "lane 7, step 8")]
    pub l7s8: f32,
    #[param(choices = "off|on|accent", default = 0, section = "LANE 7/9-16", doc = "lane 7, step 9")]
    pub l7s9: f32,
    #[param(choices = "off|on|accent", default = 0, section = "LANE 7/9-16", doc = "lane 7, step 10")]
    pub l7s10: f32,
    #[param(choices = "off|on|accent", default = 0, section = "LANE 7/9-16", doc = "lane 7, step 11")]
    pub l7s11: f32,
    #[param(choices = "off|on|accent", default = 0, section = "LANE 7/9-16", doc = "lane 7, step 12")]
    pub l7s12: f32,
    #[param(choices = "off|on|accent", default = 0, section = "LANE 7/9-16", doc = "lane 7, step 13")]
    pub l7s13: f32,
    #[param(choices = "off|on|accent", default = 0, section = "LANE 7/9-16", doc = "lane 7, step 14")]
    pub l7s14: f32,
    #[param(choices = "off|on|accent", default = 0, section = "LANE 7/9-16", doc = "lane 7, step 15")]
    pub l7s15: f32,
    #[param(choices = "off|on|accent", default = 0, section = "LANE 7/9-16", doc = "lane 7, step 16")]
    pub l7s16: f32,
    #[param(choices = "off|on|accent", default = 0, section = "LANE 8/1-8", doc = "lane 8, step 1")]
    pub l8s1: f32,
    #[param(choices = "off|on|accent", default = 0, section = "LANE 8/1-8", doc = "lane 8, step 2")]
    pub l8s2: f32,
    #[param(choices = "off|on|accent", default = 0, section = "LANE 8/1-8", doc = "lane 8, step 3")]
    pub l8s3: f32,
    #[param(choices = "off|on|accent", default = 0, section = "LANE 8/1-8", doc = "lane 8, step 4")]
    pub l8s4: f32,
    #[param(choices = "off|on|accent", default = 0, section = "LANE 8/1-8", doc = "lane 8, step 5")]
    pub l8s5: f32,
    #[param(choices = "off|on|accent", default = 0, section = "LANE 8/1-8", doc = "lane 8, step 6")]
    pub l8s6: f32,
    #[param(choices = "off|on|accent", default = 0, section = "LANE 8/1-8", doc = "lane 8, step 7")]
    pub l8s7: f32,
    #[param(choices = "off|on|accent", default = 0, section = "LANE 8/1-8", doc = "lane 8, step 8")]
    pub l8s8: f32,
    #[param(choices = "off|on|accent", default = 0, section = "LANE 8/9-16", doc = "lane 8, step 9")]
    pub l8s9: f32,
    #[param(choices = "off|on|accent", default = 0, section = "LANE 8/9-16", doc = "lane 8, step 10")]
    pub l8s10: f32,
    #[param(choices = "off|on|accent", default = 0, section = "LANE 8/9-16", doc = "lane 8, step 11")]
    pub l8s11: f32,
    #[param(choices = "off|on|accent", default = 0, section = "LANE 8/9-16", doc = "lane 8, step 12")]
    pub l8s12: f32,
    #[param(choices = "off|on|accent", default = 0, section = "LANE 8/9-16", doc = "lane 8, step 13")]
    pub l8s13: f32,
    #[param(choices = "off|on|accent", default = 0, section = "LANE 8/9-16", doc = "lane 8, step 14")]
    pub l8s14: f32,
    #[param(choices = "off|on|accent", default = 0, section = "LANE 8/9-16", doc = "lane 8, step 15")]
    pub l8s15: f32,
    #[param(choices = "off|on|accent", default = 0, section = "LANE 8/9-16", doc = "lane 8, step 16")]
    pub l8s16: f32,
    #[param(range = 0.0..127.0, default = 36.0, section = "LANE 1/NOTE", doc = "lane 1's note, a MIDI key (kick: 36)")]
    pub note1: f32,
    #[param(range = 1.0..100.0, default = 50.0, unit = "%", section = "LANE 1/NOTE", doc = "of a step: how long lane 1's note holds")]
    pub length1: f32,
    #[param(range = 0.0..127.0, default = 38.0, section = "LANE 2/NOTE", doc = "lane 2's note, a MIDI key (snare: 38)")]
    pub note2: f32,
    #[param(range = 1.0..100.0, default = 50.0, unit = "%", section = "LANE 2/NOTE", doc = "of a step: how long lane 2's note holds")]
    pub length2: f32,
    #[param(range = 0.0..127.0, default = 42.0, section = "LANE 3/NOTE", doc = "lane 3's note, a MIDI key (closed hat: 42)")]
    pub note3: f32,
    #[param(range = 1.0..100.0, default = 50.0, unit = "%", section = "LANE 3/NOTE", doc = "of a step: how long lane 3's note holds")]
    pub length3: f32,
    #[param(range = 0.0..127.0, default = 46.0, section = "LANE 4/NOTE", doc = "lane 4's note, a MIDI key (open hat: 46)")]
    pub note4: f32,
    #[param(range = 1.0..100.0, default = 50.0, unit = "%", section = "LANE 4/NOTE", doc = "of a step: how long lane 4's note holds")]
    pub length4: f32,
    #[param(range = 0.0..127.0, default = 39.0, section = "LANE 5/NOTE", doc = "lane 5's note, a MIDI key (clap: 39)")]
    pub note5: f32,
    #[param(range = 1.0..100.0, default = 50.0, unit = "%", section = "LANE 5/NOTE", doc = "of a step: how long lane 5's note holds")]
    pub length5: f32,
    #[param(range = 0.0..127.0, default = 45.0, section = "LANE 6/NOTE", doc = "lane 6's note, a MIDI key (low tom: 45)")]
    pub note6: f32,
    #[param(range = 1.0..100.0, default = 50.0, unit = "%", section = "LANE 6/NOTE", doc = "of a step: how long lane 6's note holds")]
    pub length6: f32,
    #[param(range = 0.0..127.0, default = 49.0, section = "LANE 7/NOTE", doc = "lane 7's note, a MIDI key (crash: 49)")]
    pub note7: f32,
    #[param(range = 1.0..100.0, default = 50.0, unit = "%", section = "LANE 7/NOTE", doc = "of a step: how long lane 7's note holds")]
    pub length7: f32,
    #[param(range = 0.0..127.0, default = 37.0, section = "LANE 8/NOTE", doc = "lane 8's note, a MIDI key (rim: 37)")]
    pub note8: f32,
    #[param(range = 1.0..100.0, default = 50.0, unit = "%", section = "LANE 8/NOTE", doc = "of a step: how long lane 8's note holds")]
    pub length8: f32,
    #[param(range = 1.0..16.0, default = 16.0, unit = "steps", section = "SEQ/MAIN", doc = "the pattern's length: it wraps after this many steps")]
    pub steps: f32,
    #[param(range = 1.0..16.0, default = 4.0, unit = "steps/beat", section = "SEQ/MAIN", doc = "steps a beat (4: sixteenths)")]
    pub rate: f32,
    #[param(range = 0.0..100.0, default = 0.0, unit = "%", section = "SEQ/MAIN", doc = "every second step late: 0 straight, 100 a third of a step (the triplet shuffle)")]
    pub swing: f32,
    #[param(range = 0.0..100.0, default = 100.0, unit = "%", section = "SEQ/MAIN", doc = "the velocity of an accented step")]
    pub accent: f32,
    #[param(range = 0.0..100.0, default = 70.0, unit = "%", section = "SEQ/MAIN", doc = "the velocity of a step that is on")]
    pub normal: f32,
}

impl DrumSeq {
    /// every cell, lane by lane
    fn cells(&self) -> [[f32; STEPS]; LANES] {
        [
            [self.l1s1, self.l1s2, self.l1s3, self.l1s4, self.l1s5, self.l1s6, self.l1s7, self.l1s8, self.l1s9, self.l1s10, self.l1s11, self.l1s12, self.l1s13, self.l1s14, self.l1s15, self.l1s16],
            [self.l2s1, self.l2s2, self.l2s3, self.l2s4, self.l2s5, self.l2s6, self.l2s7, self.l2s8, self.l2s9, self.l2s10, self.l2s11, self.l2s12, self.l2s13, self.l2s14, self.l2s15, self.l2s16],
            [self.l3s1, self.l3s2, self.l3s3, self.l3s4, self.l3s5, self.l3s6, self.l3s7, self.l3s8, self.l3s9, self.l3s10, self.l3s11, self.l3s12, self.l3s13, self.l3s14, self.l3s15, self.l3s16],
            [self.l4s1, self.l4s2, self.l4s3, self.l4s4, self.l4s5, self.l4s6, self.l4s7, self.l4s8, self.l4s9, self.l4s10, self.l4s11, self.l4s12, self.l4s13, self.l4s14, self.l4s15, self.l4s16],
            [self.l5s1, self.l5s2, self.l5s3, self.l5s4, self.l5s5, self.l5s6, self.l5s7, self.l5s8, self.l5s9, self.l5s10, self.l5s11, self.l5s12, self.l5s13, self.l5s14, self.l5s15, self.l5s16],
            [self.l6s1, self.l6s2, self.l6s3, self.l6s4, self.l6s5, self.l6s6, self.l6s7, self.l6s8, self.l6s9, self.l6s10, self.l6s11, self.l6s12, self.l6s13, self.l6s14, self.l6s15, self.l6s16],
            [self.l7s1, self.l7s2, self.l7s3, self.l7s4, self.l7s5, self.l7s6, self.l7s7, self.l7s8, self.l7s9, self.l7s10, self.l7s11, self.l7s12, self.l7s13, self.l7s14, self.l7s15, self.l7s16],
            [self.l8s1, self.l8s2, self.l8s3, self.l8s4, self.l8s5, self.l8s6, self.l8s7, self.l8s8, self.l8s9, self.l8s10, self.l8s11, self.l8s12, self.l8s13, self.l8s14, self.l8s15, self.l8s16],
        ]
    }

    /// cell `step` (0-based) of lane `lane` (0-based): `OFF`, `ON` or `ACCENT`
    pub fn cell(&self, lane: usize, step: usize) -> f32 {
        math::round(self.cells()[lane.min(LANES - 1)][step.min(STEPS - 1)]).clamp(OFF, ACCENT)
    }

    /// lane `lane`'s (0-based) MIDI key and its length as a part of a step
    pub fn lane(&self, lane: usize) -> (f32, f32) {
        let (notes, lengths) = (
            [self.note1, self.note2, self.note3, self.note4, self.note5, self.note6, self.note7, self.note8],
            [self.length1, self.length2, self.length3, self.length4, self.length5, self.length6, self.length7, self.length8],
        );
        let i = lane.min(LANES - 1);
        (math::round(notes[i]).clamp(0.0, 127.0), lengths[i].clamp(1.0, 100.0) / 100.0)
    }

    /// the pattern's length in steps (1 … 16)
    pub fn length(&self) -> i64 {
        (math::round(self.steps) as i64).clamp(1, STEPS as i64)
    }

    /// steps a beat
    pub fn per_beat(&self) -> f64 {
        f64::from(math::round(self.rate).clamp(1.0, 16.0))
    }

    /// how late a second step plays, in beats
    fn late(&self) -> f64 {
        f64::from(self.swing.clamp(0.0, 100.0)) / 100.0 * SWING_MAX / self.per_beat()
    }
}

/// the beat step `k` plays at: on the grid, a second step `late` after it
fn beat_of(k: i64, per_beat: f64, late: f64) -> f64 {
    k as f64 / per_beat + if k.rem_euclid(2) == 1 { late } else { 0.0 }
}

impl Module for DrumSeq {
    fn process(&mut self, block: &mut Block, ctx: &Ctx) {
        let tempo = ctx.tempo();
        let (frame, frames) = (ctx.frame(), ctx.frames());
        let (from, to) = ctx.beats();
        let (per_beat, late, len) = (self.per_beat(), self.late(), self.length());
        let velocity = [0.0, self.normal.clamp(0.0, 100.0) / 100.0, self.accent.clamp(0.0, 100.0) / 100.0];
        // the steps whose beat may land in this call: back by the swing and a frame either side (the call a step lands in
        // is its rounded frame's, `frame_in`)
        let margin = tempo.per_frame();
        for (k, _) in steps(from - late - margin, to + margin, per_beat) {
            let Some(at) = frame_in(&tempo, frame, frames, beat_of(k, per_beat, late)) else { continue };
            let step = k.rem_euclid(len) as usize;
            for lane in 0..LANES {
                let v = self.cell(lane, step) as usize;
                if v == 0 {
                    continue;
                }
                let (key, part) = self.lane(lane);
                // the step is the note's id; the pitch is the host's to fill from the key
                let note = Ev::note_on(at, k as u32, f32::NAN, key, velocity[v]).lasting((f64::from(part) / per_beat) as f32);
                block.emit(Self::LANE1 + lane, note);
            }
        }
        let now = if tempo.running { (math::libm::floor(from * per_beat) as i64).rem_euclid(len) + 1 } else { 0 };
        block.set(Self::STEP, now as f32);
    }
}
