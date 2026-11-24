//! The links that open the deck and the installed creator (agents/research/redesign/lap2/DECK-LINK.md): their grammar,
//! the deck's cards and its device row are `server/schema/tables/deck-link.json`, built in here; the creator builds and
//! reads the same file (shared/api deck-link.ts), and both run `deck-link.vectors.json`. A link is
//! `<scheme>://<workspace>[/<address>]`: the workspace the core's id (`stage` · `rigging` · `show`, and where an item
//! opens: `module`, `asset`), the address an A14 address, percent-encoded.
use crate::workspaces::Workspace;
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::sync::OnceLock;

const TABLE: &str = include_str!("../../../server/schema/tables/deck-link.json");

#[derive(Deserialize)]
struct Scheme {
    scheme: String,
    app: String,
}

#[derive(Deserialize)]
struct Table {
    scheme: String,
    app: String,
    cards: Vec<String>,
    creator: Scheme,
}

fn table() -> &'static Table {
    static T: OnceLock<Table> = OnceLock::new();
    T.get_or_init(|| serde_json::from_str(TABLE).expect("deck-link.json is the deck's built-in table"))
}

/// what the deck's device row names as its `app`
pub fn deck_app() -> &'static str {
    &table().app
}

/// the deck's workspace row in its order: the cards the creator's switcher shows (`stage` · `rigging` · `show`)
pub fn cards() -> &'static [String] {
    &table().cards
}

/// a deck workspace's id in a link and in the device row
pub fn id_of(w: Workspace) -> &'static str {
    match w {
        Workspace::Scene => "stage",
        Workspace::Rigging => "rigging",
        Workspace::Show => "show",
    }
}

/// the deck workspace a link's id comes up on: the cards' own, and the core's ids for where an item opens
/// (`module`, `asset` – an entity on its own stage); none: not a workspace of the deck
pub fn workspace_of(id: &str) -> Option<Workspace> {
    match id {
        "stage" => Some(Workspace::Scene),
        "rigging" | "module" | "asset" => Some(Workspace::Rigging),
        "show" => Some(Workspace::Show),
        _ => None,
    }
}

fn is_word(s: &str) -> bool {
    let mut c = s.chars();
    c.next().is_some_and(|f| f.is_ascii_lowercase()) && c.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

fn is_address(s: &str) -> bool {
    s.split_once(':').is_some_and(|(kind, rest)| is_word(kind) && !rest.is_empty())
}

/// every byte outside A-Z a-z 0-9 - . _ ~ : / as UTF-8 percent-encoding, upper-case hex
fn encode(address: &str) -> String {
    let mut out = String::with_capacity(address.len());
    for b in address.bytes() {
        if b.is_ascii_alphanumeric() || b"-._~:/".contains(&b) {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

/// percent-decoding to UTF-8; none when a `%` is not followed by two hex digits or the bytes are not UTF-8
fn decode(text: &str) -> Option<String> {
    let b = text.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' {
            let hex = std::str::from_utf8(b.get(i + 1..i + 3)?).ok()?;
            out.push(u8::from_str_radix(hex, 16).ok().filter(|_| hex.bytes().all(|c| c.is_ascii_hexdigit()))?);
            i += 3;
        } else {
            out.push(b[i]);
            i += 1;
        }
    }
    String::from_utf8(out).ok()
}

fn link_of(scheme: &str, workspace: &str, address: Option<&str>) -> Option<String> {
    if !is_word(workspace) {
        return None;
    }
    match address {
        None => Some(format!("{scheme}://{workspace}")),
        Some(a) if is_address(a) => Some(format!("{scheme}://{workspace}/{}", encode(a))),
        Some(_) => None,
    }
}

/// the link opening the deck on `workspace` and, given, the item at `address`; none when either is not one
pub fn deck_link(workspace: &str, address: Option<&str>) -> Option<String> {
    link_of(&table().scheme, workspace, address)
}

/// the link opening the installed creator on its `workspace` and, given, the item at `address`
pub fn creator_link(workspace: &str, address: Option<&str>) -> Option<String> {
    link_of(&table().creator.scheme, workspace, address)
}

/// a link of either app, read
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppLink {
    /// `deck` or `creator` (deck-link.json `app`)
    pub app: String,
    pub workspace: String,
    pub address: Option<String>,
}

impl AppLink {
    /// a link of the deck's own scheme
    pub fn is_deck(&self) -> bool {
        self.app == table().app
    }
}

/// a `scutoons-deck://` or `scutoons-creator://` link → its app, workspace and address; anything else: none
pub fn read(link: &str) -> Option<AppLink> {
    let (scheme, rest) = link.split_once("://")?;
    let mut sc = scheme.chars();
    if !sc.next().is_some_and(|c| c.is_ascii_alphabetic()) || !sc.all(|c| c.is_ascii_alphanumeric() || "+.-".contains(c)) {
        return None;
    }
    if rest.contains(['?', '#']) {
        return None;
    }
    let t = table();
    let scheme = scheme.to_ascii_lowercase();
    let app = if scheme == t.scheme { &t.app } else if scheme == t.creator.scheme { &t.creator.app } else { return None };
    let (workspace, address) = rest.split_once('/').map_or((rest, ""), |(w, a)| (w, a));
    if !is_word(workspace) {
        return None;
    }
    let address = if address.is_empty() { None } else { Some(decode(address).filter(|a| is_address(a))?) };
    Some(AppLink { app: app.clone(), workspace: workspace.to_string(), address })
}

/// what the deck's device row says: the workspace in front and each workspace's last item
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DeckState {
    pub workspace: Option<String>,
    pub last: BTreeMap<String, String>,
}

/// the deck's device row state (PUT /api/me/session `state`): its app, the workspace in front, each one's last item
pub fn deck_state(front: Option<Workspace>, last: &BTreeMap<Workspace, String>) -> Value {
    let last: BTreeMap<&str, &str> = last.iter().filter(|(_, a)| is_address(a)).map(|(w, a)| (id_of(*w), a.as_str())).collect();
    json!({ "app": deck_app(), "v": 1, "workspace": front.map(id_of), "last": last })
}

/// the deck's device row state, checked (not the deck's: none); a workspace or an item that is not one is left out
pub fn read_deck_state(raw: &Value) -> Option<DeckState> {
    let o = raw.as_object()?;
    if o.get("app").and_then(Value::as_str) != Some(deck_app()) {
        return None;
    }
    let workspace = o.get("workspace").and_then(Value::as_str).filter(|w| is_word(w)).map(str::to_string);
    let last = o
        .get("last")
        .and_then(Value::as_object)
        .map(|m| m.iter().filter_map(|(w, a)| a.as_str().filter(|a| is_word(w) && is_address(a)).map(|a| (w.clone(), a.to_string()))).collect())
        .unwrap_or_default();
    Some(DeckState { workspace, last })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workspaces::ALL;

    fn vectors() -> Value {
        serde_json::from_str(include_str!("../../../server/schema/tables/deck-link.vectors.json")).unwrap()
    }

    fn opt(v: &Value) -> Option<&str> {
        v.as_str()
    }

    #[test]
    fn the_links_are_built_and_read_back_as_the_vectors_say() {
        let v = vectors();
        for c in v["links"].as_array().unwrap() {
            let (w, a, l) = (c["workspace"].as_str().unwrap(), opt(&c["address"]), c["link"].as_str().unwrap());
            assert_eq!(deck_link(w, a).as_deref(), Some(l), "{c}");
            assert_eq!(read(l), Some(AppLink { app: "deck".into(), workspace: w.into(), address: a.map(str::to_string) }), "{l}");
        }
        for c in v["creator_links"].as_array().unwrap() {
            let (w, a, l) = (c["workspace"].as_str().unwrap(), opt(&c["address"]), c["link"].as_str().unwrap());
            assert_eq!(creator_link(w, a).as_deref(), Some(l), "{c}");
            assert_eq!(read(l), Some(AppLink { app: "creator".into(), workspace: w.into(), address: a.map(str::to_string) }), "{l}");
        }
        for c in v["refused"].as_array().unwrap() {
            assert_eq!(deck_link(c["workspace"].as_str().unwrap(), opt(&c["address"])), None, "{c}");
        }
    }

    #[test]
    fn a_link_reads_as_the_vectors_say() {
        for c in vectors()["read"].as_array().unwrap() {
            let link = c["link"].as_str().unwrap();
            let want = c["read"].as_object().map(|r| AppLink { app: r["app"].as_str().unwrap().into(), workspace: r["workspace"].as_str().unwrap().into(), address: opt(&r["address"]).map(str::to_string) });
            assert_eq!(read(link), want, "{link}");
        }
    }

    #[test]
    fn the_device_row_reads_as_the_vectors_say_and_the_decks_own_reads_back() {
        for c in vectors()["states"].as_array().unwrap() {
            let want = c["read"].as_object().map(|r| DeckState {
                workspace: opt(&r["workspace"]).map(str::to_string),
                last: r["last"].as_object().unwrap().iter().map(|(k, v)| (k.clone(), v.as_str().unwrap().to_string())).collect(),
            });
            assert_eq!(read_deck_state(&c["state"]), want, "{c}");
        }
        let last = BTreeMap::from([(Workspace::Scene, "scene:avatar-home".to_string()), (Workspace::Show, "show:new-show".to_string()), (Workspace::Rigging, "rigging:".to_string())]);
        let state = deck_state(Some(Workspace::Scene), &last);
        assert_eq!(state["app"], "deck");
        assert_eq!(state["v"], 1);
        let back = read_deck_state(&state).unwrap();
        assert_eq!(back.workspace.as_deref(), Some("stage"));
        assert_eq!(back.last, BTreeMap::from([("stage".to_string(), "scene:avatar-home".to_string()), ("show".to_string(), "show:new-show".to_string())]), "an empty item is left out");
        assert_eq!(read_deck_state(&deck_state(None, &BTreeMap::new())), Some(DeckState::default()));
    }

    #[test]
    fn the_decks_row_is_the_tables_cards_in_their_order() {
        assert_eq!(ALL.iter().map(|w| id_of(*w)).collect::<Vec<_>>(), cards(), "deck-link.json `cards` is the deck's row");
        for w in ALL {
            assert_eq!(workspace_of(id_of(w)), Some(w));
        }
        assert_eq!(workspace_of("module"), Some(Workspace::Rigging), "the core's id where a module opens");
        assert_eq!(workspace_of("asset"), Some(Workspace::Rigging));
        assert_eq!(workspace_of("xr"), None, "XR is the creator's");
    }
}
