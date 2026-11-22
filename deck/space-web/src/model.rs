//! #773: a model file's facts (deck-space `model_facts`) for the creator's 3D place – the bytes in, JSON out; the page
//! draws the model and holds no rule of it.
use wasm_bindgen::prelude::*;

/// a GLB's bytes → `{ ok: true, triangles, bones: [{ name, parent }], clips: [{ name, seconds }], spots, missing }` or `{ ok: false, why }`
#[wasm_bindgen]
pub fn model_facts(bytes: &[u8]) -> String {
    deck_space::space::model_facts::facts_json(bytes)
}
