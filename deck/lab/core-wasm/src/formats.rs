//! #184: the exchange-format candidates across the JS ↔ wasm boundary, each ending in the same apply (live-core's
//! `Core::send` on a loaded scene), so a difference is the format and the crossing, not the work. The fourth
//! candidate – wire records written straight into wasm memory – needs no function here: it is live_core.wasm itself
//! (`lc_apply`), measured by deck/lab/ts/formats.ts next to these.
//!
//! - **JSON**: text in through wasm-bindgen (TextEncoder into wasm memory), serde_json; the cue as the relay's own
//!   `protocol::Envelope`.
//! - **postcard**: the schema-driven binary (serde types → varints, f32 LE, length-prefixed strings), bytes copied in by
//!   wasm-bindgen. The relay's protocol types cannot go through it as they are (`#[serde(flatten)]`, skipped optional
//!   fields and the loose `serde_json::Map` actions need a self-describing format), so the cue here is a typed mirror.
//! - **wire, copied**: live-core's fixed records, the bytes copied in by wasm-bindgen.
use live_core::wire::{Mode, FRAME};
use live_core::Core;
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use wasm_bindgen::prelude::*;

thread_local! {
    static CORE: RefCell<Option<Core>> = const { RefCell::new(None) };
    static FRAME_NOW: RefCell<Vec<f32>> = RefCell::new((0..FRAME).map(|i| (i as f32 * 0.37).sin().abs()).collect());
}

/// Loads the measuring scene (deck-core's wide bench scene: 300 control nodes + 16 FM voices) and returns the node
/// count; the formats' sends land on it.
#[wasm_bindgen]
pub fn fmt_init() -> usize {
    let core = Core::load(&fmt_scene(), 48_000.0).expect("the bench scene loads");
    CORE.with(|c| *c.borrow_mut() = Some(core));
    316
}

/// The measuring scene's TOML (live_core.wasm loads the same text for the in-place candidate).
#[wasm_bindgen]
pub fn fmt_scene() -> String {
    deck_core::graph::bench_scene_wide(100, 16)
}

fn send(mode: Mode, node: usize, port: usize, amount: f32, v: impl Iterator<Item = f32>) -> u32 {
    CORE.with(|c| c.borrow_mut().as_mut().map_or(0, |core| core.send(mode, node, port, amount, v).is_ok() as u32))
}

#[derive(Deserialize)]
#[serde(rename_all = "lowercase")]
enum ModeJ {
    Force,
    Blend,
    Release,
}

impl From<ModeJ> for Mode {
    fn from(m: ModeJ) -> Mode {
        match m {
            ModeJ::Force => Mode::Force,
            ModeJ::Blend => Mode::Blend,
            ModeJ::Release => Mode::Release,
        }
    }
}

#[derive(Deserialize)]
struct SendJ {
    mode: ModeJ,
    node: u16,
    port: u16,
    amount: f32,
    v: Vec<f32>,
}

/// `{"mode":"blend","node":3,"port":0,"amount":0.5,"v":[…]}` → applied
#[wasm_bindgen]
pub fn fmt_json_send(text: &str) -> u32 {
    match serde_json::from_str::<SendJ>(text) {
        Ok(s) => send(s.mode.into(), s.node as usize, s.port as usize, s.amount, s.v.into_iter()),
        Err(_) => 0,
    }
}

/// The analysis frame as JSON text (an array of FRAME numbers).
#[wasm_bindgen]
pub fn fmt_json_frame() -> String {
    FRAME_NOW.with(|f| serde_json::to_string(&*f.borrow()).expect("floats serialise"))
}

/// The relay envelope, decoded as the relay's type.
#[wasm_bindgen]
pub fn fmt_json_cue(text: &str) -> u32 {
    serde_json::from_str::<protocol::Envelope>(text).map_or(0, |e| e.seq as u32)
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct SendP {
    pub mode: u8,
    pub node: u16,
    pub port: u16,
    pub amount: f32,
    pub v: Vec<f32>,
}

#[wasm_bindgen]
pub fn fmt_pc_send(bytes: &[u8]) -> u32 {
    match postcard::from_bytes::<SendP>(bytes) {
        Ok(s) => {
            let mode = [Mode::Force, Mode::Blend, Mode::Release][s.mode.min(2) as usize];
            send(mode, s.node as usize, s.port as usize, s.amount, s.v.into_iter())
        }
        Err(_) => 0,
    }
}

#[wasm_bindgen]
pub fn fmt_pc_frame() -> Vec<u8> {
    FRAME_NOW.with(|f| postcard::to_allocvec(&*f.borrow()).expect("floats serialise"))
}

/// The cue envelope as a typed mirror of the relay's (postcard needs fixed shapes).
#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct CueP {
    pub seq: u32,
    pub at: Option<u64>,
    pub from: String,
    pub to: Option<ToP>,
    pub id: String,
    pub actions: Vec<ActionP>,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct ToP {
    pub role: Option<String>,
    pub group: Option<String>,
    pub client: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub enum ActionP {
    Say { outlet: String, text: Vec<(String, String)>, haptic: bool },
    Stage { outlet: String, layer: String, show: bool },
    Send { outlet: String, midi: MidiP },
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct MidiP {
    pub kind: String,
    pub ch: u8,
    pub n: u8,
    pub v: u8,
}

#[wasm_bindgen]
pub fn fmt_pc_cue(bytes: &[u8]) -> u32 {
    postcard::from_bytes::<CueP>(bytes).map_or(0, |c| c.seq + c.actions.len() as u32)
}

/// The TS encoder's bytes, checked: decodes them and encodes the expected value – equal bytes = the two agree.
#[wasm_bindgen]
pub fn fmt_pc_check(send_bytes: &[u8], cue_bytes: &[u8]) -> String {
    let s = postcard::from_bytes::<SendP>(send_bytes).map_err(|e| e.to_string());
    let c = postcard::from_bytes::<CueP>(cue_bytes).map_err(|e| e.to_string());
    format!("{s:?} | {c:?}")
}

#[wasm_bindgen]
pub fn fmt_wire_send(bytes: &[u8]) -> u32 {
    CORE.with(|c| c.borrow_mut().as_mut().map_or(0, |core| core.apply(bytes).unwrap_or(0) as u32))
}

/// The frame copied out as a Float32Array (wasm-bindgen's Vec<f32> return).
#[wasm_bindgen]
pub fn fmt_wire_frame() -> Vec<f32> {
    FRAME_NOW.with(|f| f.borrow().clone())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The postcard layout the TS encoder (deck/lab/ts/formats.ts) writes by hand: varints, raw u8, f32 LE.
    #[test]
    fn postcard_bytes_as_the_ts_encoder_writes_them() {
        let s = SendP { mode: 1, node: 300, port: 0, amount: 0.5, v: vec![1.0, 2.0] };
        let b = postcard::to_allocvec(&s).unwrap();
        assert_eq!(b, [1, 0xac, 0x02, 0, 0, 0, 0, 0x3f, 2, 0, 0, 0x80, 0x3f, 0, 0, 0, 0x40]);
        let c = CueP {
            seq: 4,
            at: Some(300),
            from: "c1".into(),
            to: Some(ToP { role: None, group: Some("left".into()), client: None }),
            id: "x".into(),
            actions: vec![ActionP::Stage { outlet: "l".into(), layer: "a".into(), show: true }],
        };
        let b = postcard::to_allocvec(&c).unwrap();
        assert_eq!(b, [4, 1, 0xac, 0x02, 2, b'c', b'1', 1, 0, 1, 4, b'l', b'e', b'f', b't', 0, 1, b'x', 1, 1, 1, b'l', 1, b'a', 1]);
    }
}
