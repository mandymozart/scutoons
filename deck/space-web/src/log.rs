//! #767: the Log (deck-ui `messages`) for the creator's Log tab and status bar – the standing problems, the passing
//! events, the filters, the counts, the scope pills, the types and their legend. JSON in and out; the page holds no
//! rule of its own (no GPU: it works when the 3D can't).
use deck_ui::messages::{self, Messages};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct Log {
    all: Messages,
}

impl Default for Log {
    fn default() -> Self {
        Log::new()
    }
}

#[wasm_bindgen]
impl Log {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Log {
        Log { all: Messages::default() }
    }

    /// a line (`{ level, source, text, at, scope?, key?, kind?, jump?, ref?, detail?, accepted? }`): standing with a key
    pub fn post(&mut self, message: &str) -> Result<(), JsValue> {
        self.all.push_json(message).map_err(|e| JsValue::from_str(&e))
    }

    /// a poster's standing lines (a JSON array), replacing every standing line whose key starts with `prefix`
    pub fn stand(&mut self, prefix: &str, lines: &str) -> Result<(), JsValue> {
        self.all.stand_json(prefix, lines).map_err(|e| JsValue::from_str(&e))
    }

    /// the standing line with `key` goes
    pub fn clear(&mut self, key: &str) {
        self.all.clear(key);
    }

    /// `{ level, show, kinds, scope, notes, find, now, max }` → `{ problems, events, counts, notes, newest }`
    pub fn view(&self, request: &str) -> Result<String, JsValue> {
        self.all.view_json(request).map_err(|e| JsValue::from_str(&e))
    }

    /// a filter → `{ errors, warnings }`
    pub fn counts(&self, filter: &str) -> Result<String, JsValue> {
        self.all.counts_json(filter).map_err(|e| JsValue::from_str(&e))
    }

    /// a filter with `find` → Copy all's text
    pub fn text(&self, request: &str) -> Result<String, JsValue> {
        self.all.text_json(request).map_err(|e| JsValue::from_str(&e))
    }
}

/// `{ item, selection }` → the scope pills `[{ scope, words, prefix }]`
#[wasm_bindgen]
pub fn log_scopes(request: &str) -> Result<String, JsValue> {
    messages::scopes_json(request).map_err(|e| JsValue::from_str(&e))
}

/// the Type pills, their legend and the Show pills: `{ kinds: [{ kind, label, legend }], shows: [{ show, words }] }`
#[wasm_bindgen]
pub fn log_kinds() -> String {
    messages::kinds_json()
}
