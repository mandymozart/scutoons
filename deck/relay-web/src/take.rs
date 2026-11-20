//! The handheld camera's follow and take for a page that draws its own 3D (the creator's storyboard): deck-space's
//! `dop::take::Recorder` behind wasm-bindgen. The page hands in the phone's poses and Record / Stop; the core follows the
//! phone from the shot camera's pose, keeps the poses of a take and thins them to keys. Natively empty.
use deck_space::space::dop::take::{anchor_of, flat, Recorder};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct TakeFollow {
    r: Recorder,
}

#[wasm_bindgen]
impl TakeFollow {
    /// follows a phone from the camera at `position` (3) / `quaternion` (4); `scale` scene metres per phone metre
    #[wasm_bindgen(constructor)]
    pub fn new(position: &[f64], quaternion: &[f64], scale: f64) -> Result<TakeFollow, JsValue> {
        let anchor = anchor_of(position, quaternion).ok_or_else(|| JsValue::from_str("a malformed camera"))?;
        Ok(TakeFollow { r: Recorder::live("handheld", anchor, scale as f32) })
    }

    /// a pose of the phone at `t` (its clock, ms); false: late or malformed
    pub fn pose(&mut self, t: f64, p: &[f64], q: &[f64]) -> bool {
        self.r.pose(t, p, q)
    }

    pub fn recenter(&mut self) {
        self.r.recenter();
    }

    /// Record (true) and Stop (false)
    pub fn keep(&mut self, on: bool) {
        self.r.keep(on);
    }

    /// how long the take runs so far (s)
    pub fn secs(&self) -> f64 {
        self.r.secs() as f64
    }

    /// the camera now: position (3) and rotation (4); empty before the first pose
    pub fn held(&self) -> Vec<f64> {
        self.r.held().map(|(e, q)| flat(e, q).to_vec()).unwrap_or_default()
    }

    /// the take as JSON `[{ t, position, quaternion }]` (s from its first pose); empty: too short to be a move
    pub fn take(&self) -> String {
        let Some(keys) = self.r.keys() else { return String::new() };
        let keys: Vec<serde_json::Value> = keys
            .iter()
            .map(|k| {
                let f = flat(k.eye, k.rot);
                serde_json::json!({ "t": (k.secs as f64 * 1e4).round() / 1e4, "position": &f[..3], "quaternion": &f[3..] })
            })
            .collect();
        serde_json::to_string(&keys).unwrap_or_default()
    }
}
