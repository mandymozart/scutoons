//! #773: a model file's facts (deck-space `model_facts`) for the creator's 3D place – the bytes in, JSON out; the page
//! draws the model and holds no rule of it.
use wasm_bindgen::prelude::*;

/// a GLB's bytes → `{ ok: true, triangles, bones: [{ name, parent }], clips: [{ name, seconds }], spots, missing }` or `{ ok: false, why }`
#[wasm_bindgen]
pub fn model_facts(bytes: &[u8]) -> String {
    deck_space::space::model_facts::facts_json(bytes)
}

/// #817: whether a character's idle plays on its model – `{ animation?: Skeleton, clip?: string, model: Skeleton }` →
/// the problems as JSON (`[]`: it plays); deck-catalog's `FitCheck`, the rule the API and the stage ask too
#[wasm_bindgen]
pub fn idle_problems(check: &str) -> String {
    let problems = match serde_json::from_str::<deck_catalog::FitCheck>(check) {
        Ok(c) => c.problems(),
        Err(e) => vec![format!("not an idle check: {e}")],
    };
    serde_json::to_string(&problems).unwrap_or_else(|_| "[]".into())
}
