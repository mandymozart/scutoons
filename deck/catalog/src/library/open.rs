//! What "open" means for everything the Library lists: for each editor, the workspace an address opens in and the tab
//! address it opens as, or the sentence why it does not open there (yet). The one table: the deck editor's Library and
//! the creator's (through space-web) only ask.
use super::Kind;
use serde::Serialize;
use serde_json::{json, Value};

/// the two editors that share the Library
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Editor {
    Deck,
    Creator,
}

impl Editor {
    pub fn of_id(id: &str) -> Option<Editor> {
        match id {
            "deck" => Some(Editor::Deck),
            "creator" => Some(Editor::Creator),
            _ => None,
        }
    }

    fn word(self) -> &'static str {
        match self {
            Editor::Deck => "the deck editor",
            Editor::Creator => "the creator",
        }
    }

    fn other(self) -> Editor {
        match self {
            Editor::Deck => Editor::Creator,
            Editor::Creator => Editor::Deck,
        }
    }
}

/// a workspace something opens in (its id is the editor's own workspace name)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Workspace {
    Toon,
    Chapter,
    Storyboard,
    Character,
    Map,
    Catalog,
    Scene,
    Sound,
    Experience,
    Text,
    /// a file on its own: the creator's asset details, the deck editor's Asset tab
    Asset,
    /// the deck editor's stage: a scene, a character's rig
    Stage,
    /// the deck editor's module code, build and audition
    Module,
    Show,
}

impl Workspace {
    pub fn id(self) -> &'static str {
        match self {
            Workspace::Toon => "toon",
            Workspace::Chapter => "chapter",
            Workspace::Storyboard => "storyboard",
            Workspace::Character => "character",
            Workspace::Map => "map",
            Workspace::Catalog => "catalog",
            Workspace::Scene => "scene",
            Workspace::Sound => "sound",
            Workspace::Experience => "experience",
            Workspace::Text => "text",
            Workspace::Asset => "asset",
            Workspace::Stage => "stage",
            Workspace::Module => "module",
            Workspace::Show => "show",
        }
    }

    /// how a sentence names it
    pub fn word(self) -> &'static str {
        match self {
            Workspace::Toon => "the toon",
            Workspace::Chapter => "the chapter editor",
            Workspace::Storyboard => "the storyboard",
            Workspace::Character => "the character workspace",
            Workspace::Map => "the map",
            Workspace::Catalog => "the catalog",
            Workspace::Scene => "the scene editor",
            Workspace::Sound => "the Sound workspace",
            Workspace::Experience => "the XR workspace",
            Workspace::Text => "the Kanon",
            Workspace::Asset => "the asset details",
            Workspace::Stage => "the stage",
            Workspace::Module => "the module editor",
            Workspace::Show => "the Show",
        }
    }
}

impl Serialize for Workspace {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(self.id())
    }
}

/// every kind of address the Library can list, in the order "Items to open" groups them: its word before `:` and
/// how a list names it
pub const KINDS: [(&str, &str); 31] = [
    ("toon", "toons"),
    ("season", "seasons"),
    ("chapter", "chapters"),
    ("storyboard", "storyboards"),
    ("shot", "shots"),
    ("character", "characters"),
    ("place", "places"),
    ("route", "routes"),
    ("region", "regions"),
    ("scene", "scenes"),
    ("experience", "experiences"),
    ("item", "items"),
    ("recipe", "recipes"),
    ("album", "albums"),
    ("product", "products"),
    ("edition", "editions"),
    ("home-scene", "home scenes"),
    ("airlift-carrier", "airlift carriers"),
    ("rig", "rigs"),
    ("module", "modules"),
    ("instrument", "instruments"),
    ("preset", "presets"),
    ("show", "shows"),
    ("kanon", "Kanon texts"),
    ("generator", "generators"),
    ("person", "people"),
    ("image", "images"),
    ("audio", "audio"),
    ("video", "video"),
    ("model", "3d models"),
    ("animation", "animations"),
];

/// a toon's file without a kind of its own (a PDF, a font): listed apart from `KINDS`, since its address is `file:<path>`
pub const FILE: (&str, &str) = ("file", "files");

/// where a kind opens in `editor`, if it has a workspace there
fn workspace(kind: &str, editor: Editor) -> Option<Workspace> {
    use Workspace as W;
    match editor {
        Editor::Creator => Some(match kind {
            "toon" | "season" => W::Toon,
            "chapter" => W::Chapter,
            "storyboard" | "shot" | "generator" => W::Storyboard,
            "character" => W::Character,
            "place" | "route" | "region" => W::Map,
            "scene" => W::Scene,
            "experience" => W::Experience,
            "item" | "recipe" | "album" | "product" | "edition" | "home-scene" | "airlift-carrier" => W::Catalog,
            "rig" | "module" => W::Sound,
            "kanon" => W::Text,
            "image" | "audio" | "video" | "model" | "animation" | "file" => W::Asset,
            _ => return None,
        }),
        Editor::Deck => Some(match kind {
            "scene" | "character" | "shot" => W::Stage,
            "module" => W::Module,
            "show" => W::Show,
            "image" | "audio" | "video" | "model" | "animation" => W::Asset,
            _ => return None,
        }),
    }
}

fn plural(kind: &str) -> Option<&'static str> {
    KINDS.iter().chain([&FILE]).find(|(k, _)| *k == kind).map(|(_, p)| *p)
}

/// whether `kind` has a workspace in `editor`, else the sentence why not
pub fn opens(kind: &str, editor: Editor) -> Result<Workspace, String> {
    let Some(words) = plural(kind) else { return Err(format!("{kind}: nothing the Library opens")) };
    workspace(kind, editor).ok_or_else(|| match workspace(kind, editor.other()) {
        Some(there) => format!("{words} open in {} of {}; {} has no workspace for them yet", there.word(), editor.other().word(), editor.word()),
        None => format!("{words} have no workspace yet"),
    })
}

/// what opening something means: the tab it opens as and the workspace that edits it
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Opening {
    pub address: String,
    pub workspace: Workspace,
}

/// what opening `address` (`character:catpope`, `place:harbour`, `chapter:catpope:3`, `image:<path>` …) means in `editor`
pub fn open(address: &str, editor: Editor) -> Result<Opening, String> {
    let (kind, id) = address.split_once(':').filter(|(k, id)| !k.is_empty() && !id.is_empty()).ok_or_else(|| format!("{address}: not an address (kind:id)"))?;
    opens(kind, editor).map(|workspace| Opening { address: format!("{kind}:{id}"), workspace })
}

/// a Library entry opened: a toon's file by its kind (an address without it is its path), anything else by its address
pub fn open_entry(kind: Kind, address: &str, editor: Editor) -> Result<Opening, String> {
    if kind.is_file() && !address.starts_with(&format!("{}:", kind.id())) {
        open(&format!("{}:{address}", kind.id()), editor)
    } else {
        open(address, editor)
    }
}

/// `open`'s answer for the wasm bridge: `{ ok: true, address, workspace }` or `{ ok: false, why }`
pub fn open_json(address: &str, editor: &str) -> String {
    let answer = match Editor::of_id(editor) {
        Some(e) => open(address, e),
        None => Err(format!("{editor}: no such editor")),
    };
    answer_json(answer).to_string()
}

/// every kind and its answer in `editor`, in the listing order: `[{ kind, words, workspace?, opens?, why? }]` (`opens`: how a sentence names the workspace)
pub fn kinds_json(editor: &str) -> String {
    let Some(e) = Editor::of_id(editor) else { return "[]".into() };
    let list: Vec<Value> = KINDS
        .iter()
        .chain([&FILE])
        .map(|(kind, words)| match opens(kind, e) {
            Ok(w) => json!({ "kind": kind, "words": words, "workspace": w, "opens": w.word() }),
            Err(why) => json!({ "kind": kind, "words": words, "why": why }),
        })
        .collect();
    Value::Array(list).to_string()
}

fn answer_json(answer: Result<Opening, String>) -> Value {
    match answer {
        Ok(o) => json!({ "ok": true, "address": o.address, "workspace": o.workspace }),
        Err(why) => json!({ "ok": false, "why": why }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(address: &str, editor: Editor) -> Result<(String, &'static str), String> {
        open(address, editor).map(|o| (o.address, o.workspace.id()))
    }

    #[test]
    fn every_kind_opens_in_its_creator_workspace_or_says_why_not() {
        let c = Editor::Creator;
        assert_eq!(at("character:catpope", c), Ok(("character:catpope".into(), "character")));
        assert_eq!(at("place:harbour", c), Ok(("place:harbour".into(), "map")));
        assert_eq!(at("chapter:catpope:3", c), Ok(("chapter:catpope:3".into(), "chapter")));
        assert_eq!(at("season:catpope:1", c), Ok(("season:catpope:1".into(), "toon")));
        assert_eq!(at("experience:poster", c), Ok(("experience:poster".into(), "experience")));
        assert_eq!(at("item:camera", c), Ok(("item:camera".into(), "catalog")));
        assert_eq!(at("rig:character.catpope", c), Ok(("rig:character.catpope".into(), "sound")));
        assert_eq!(at("image:catpope/s1/c1/x.webp", c), Ok(("image:catpope/s1/c1/x.webp".into(), "asset")));
        assert_eq!(at("instrument:organ", c), Err("instruments have no workspace yet".into()));
        assert_eq!(at("show:duet", c), Err("shows open in the Show of the deck editor; the creator has no workspace for them yet".into()));
        assert_eq!(at("nonsense", c), Err("nonsense: not an address (kind:id)".into()));
        assert_eq!(at("toast:x", c), Err("toast: nothing the Library opens".into()));
    }

    #[test]
    fn the_deck_editor_opens_what_it_always_did() {
        let d = Editor::Deck;
        let e = |kind, address: &str| open_entry(kind, address, d).map(|o| (o.address, o.workspace.id()));
        assert_eq!(e(Kind::Scene, "scene:nave"), Ok(("scene:nave".into(), "stage")));
        assert_eq!(e(Kind::Character, "character:catpope"), Ok(("character:catpope".into(), "stage")));
        assert_eq!(e(Kind::Module, "module:walk"), Ok(("module:walk".into(), "module")));
        assert_eq!(e(Kind::Video, "cat/s1/intro.mp4"), Ok(("video:cat/s1/intro.mp4".into(), "asset")), "a file by its kind");
        assert_eq!(e(Kind::Model, "cat/torso.glb"), Ok(("model:cat/torso.glb".into(), "asset")));
        assert_eq!(e(Kind::Place, "place:harbour"), Err("places open in the map of the creator; the deck editor has no workspace for them yet".into()));
        assert_eq!(e(Kind::File, "cat/notes.pdf"), Err("files open in the asset details of the creator; the deck editor has no workspace for them yet".into()));
        assert_eq!(e(Kind::Instrument, "instrument:organ"), Err("instruments have no workspace yet".into()));
        assert_eq!(e(Kind::Image, "image:cat/a.webp"), Ok(("image:cat/a.webp".into(), "asset")), "the server's file address");
        assert_eq!(e(Kind::Show, "show:duet"), Ok(("show:duet".into(), "show")));
        assert_eq!(at("shot:catpope:3:sb-25-1", d), Ok(("shot:catpope:3:sb-25-1".into(), "stage")), "a shot opens on the stage");
        assert_eq!(at("shot:catpope:3:sb-25-1", Editor::Creator), Ok(("shot:catpope:3:sb-25-1".into(), "storyboard")));
        assert_eq!(e(Kind::Toon, "toon:catpope"), Err("toons open in the toon of the creator; the deck editor has no workspace for them yet".into()));
        assert_eq!(e(Kind::Generator, "generator:g1"), Err("generators open in the storyboard of the creator; the deck editor has no workspace for them yet".into()));
        assert_eq!(e(Kind::Person, "person:p1"), Err("people have no workspace yet".into()));
    }

    #[test]
    fn every_library_kind_has_an_answer_in_both_editors() {
        for k in Kind::ALL {
            let word = k.id();
            assert!(plural(word).is_some(), "{word} is listed");
            for e in [Editor::Deck, Editor::Creator] {
                let _ = opens(word, e);
            }
        }
        for (kind, _) in KINDS {
            assert!(opens(kind, Editor::Creator).is_ok() || opens(kind, Editor::Deck).is_ok() || ["instrument", "preset", "person"].contains(&kind), "{kind} opens somewhere or is a known gap");
        }
    }

    #[test]
    fn the_bridge_answers_in_json() {
        let ok: Value = serde_json::from_str(&open_json("place:harbour", "creator")).unwrap();
        assert_eq!(ok, json!({ "ok": true, "address": "place:harbour", "workspace": "map" }));
        let no: Value = serde_json::from_str(&open_json("place:harbour", "phone")).unwrap();
        assert_eq!(no, json!({ "ok": false, "why": "phone: no such editor" }));
        let kinds: Vec<Value> = serde_json::from_str(&kinds_json("creator")).unwrap();
        assert_eq!(kinds.len(), KINDS.len() + 1);
        assert_eq!(kinds[5], json!({ "kind": "character", "words": "characters", "workspace": "character", "opens": "the character workspace" }));
        assert!(kinds.iter().any(|k| k["kind"] == "instrument" && k["why"] == "instruments have no workspace yet"));
        assert_eq!(kinds_json("phone"), "[]");
    }
}
