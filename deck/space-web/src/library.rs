//! #698: the Library's shelf (deck-catalog `library::shelf`) for the creator's Library panel, browser and picker – the
//! families, what the open item uses, the view, what a slot takes, the selection, where an upload lands. JSON in and
//! out; the page holds no rule of its own (no GPU: it works when the 3D can't).
use deck_catalog::library::open;
use deck_catalog::library::shelf::{self, Shelf};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct Library {
    shelf: Shelf,
}

#[wasm_bindgen]
impl Library {
    /// `GET /api/assets`' answer (`{ items }`) or its items as JSON
    #[wasm_bindgen(constructor)]
    pub fn new(assets: &str) -> Result<Library, JsValue> {
        Shelf::from_json(assets).map(|shelf| Library { shelf }).map_err(|e| JsValue::from_str(&e))
    }

    /// the families as JSON, stacked or every file on its own
    pub fn families(&self, single: bool) -> String {
        self.shelf.families_json(single)
    }

    /// the open item and its uses: `{ open, toon, byHome, uses: [{ path, item, part, at }], links: [[main, part]] }`
    pub fn context(&mut self, context: &str) -> Result<(), JsValue> {
        self.shelf.context_json(context).map_err(|e| JsValue::from_str(&e))
    }

    /// `{ scope, grouping, sort, query, kind, special, origin, fit, related, single, now }` → the view as JSON
    pub fn view(&self, query: &str) -> Result<String, JsValue> {
        self.shelf.view_json(query).map_err(|e| JsValue::from_str(&e))
    }

    /// whether a file's family fits `{ kinds, square }`: `{ ok }` or `{ ok: false, wants, got, square }`
    pub fn takes(&self, path: &str, fit: &str) -> String {
        self.shelf.takes_json(path, fit)
    }

    /// a family's uses by the open item: `[{ path, item, part, at }]`
    pub fn uses(&self, key: &str) -> String {
        self.shelf.uses_json(key)
    }

    /// `{ name, slot? }` → where the new file goes (JSON string or null)
    pub fn upload_path(&self, request: &str) -> String {
        self.shelf.upload_path_json(request)
    }
}

/// `{ chosen, anchor, id, order, shift, toggle }` → `{ chosen, anchor }`
#[wasm_bindgen]
pub fn library_select(request: &str) -> Result<String, JsValue> {
    shelf::select_json(request).map_err(|e| JsValue::from_str(&e))
}

/// `{ kinds, square }` → the kind a picker opens on (JSON string or null)
#[wasm_bindgen]
pub fn library_start_kind(fit: &str) -> String {
    shelf::start_kind_json(fit)
}

/// what opening an address means in an editor (`deck` | `creator`): `{ ok: true, address, workspace }` or `{ ok: false, why }`
#[wasm_bindgen]
pub fn library_open(address: &str, editor: &str) -> String {
    open::open_json(address, editor)
}

/// every kind the Library lists and its answer in an editor: `[{ kind, words, workspace?, why? }]`
#[wasm_bindgen]
pub fn library_open_kinds(editor: &str) -> String {
    open::kinds_json(editor)
}
