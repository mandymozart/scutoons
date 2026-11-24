//! A new deck scene in the Show workspace, or one from the server recast: its id, its stage's name, its cast picked from
//! the open toon's rigs (Build's catalog, rigs.rs). The document, its spots and its checks are the core's (scene_document.rs
//! `new`, `recast`, `check_new`); the stage plays it as the core resolves it (scene_resolve.rs); a save is a new draft
//! through /api/scenes and the creator publishes it. A draft never published joins the scene list beside the rows band,
//! marked as such: a cell here may take it, other decks play it once published (decisions/deck.md 2026-11-08 [scenes]).
use super::dialog;
use super::roles::Typing;
use super::{Hit, Host, Sheet};
use crate::rigs::{self, Catalog};
use deck_kit::base::draw::DrawList;
use deck_kit::base::text::Field;
use deck_kit::base::tokens::Tokens;
use deck_kit::editor::controls::{self, Rect};
use deck_space::scene::Scene;
use deck_space::scene_document::{FillerRef, SceneDocument};
use deck_space::scene_resolve::{core_rigs, no_sets, resolve};
use serde_json::Value;
use std::collections::BTreeMap;
use std::sync::mpsc::{channel, Receiver};
use std::sync::Arc;

/// the dialog's buttons: the foot's left, its right, its primary last
const BUTTONS: [(&str, ComposeHit); 4] = [("Load …", ComposeHit::Load), ("Open", ComposeHit::Build), ("Close", ComposeHit::Close), ("Play", ComposeHit::Play)];
const NO_RIGS: &str = "the toon's rigs come from the server: sign in, or wait for the rigs' list";

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ComposeHit {
    Open,
    Id,
    Name,
    Find,
    /// a rig of the picker joins the cast (or leaves it)
    Pick(usize),
    /// a member's × in the cast
    Drop(usize),
    Load,
    Play,
    /// the scene as the sheet has it, edited in Build
    Build,
    Close,
}

/// What the sheet keeps while it is closed.
#[derive(Default)]
pub struct Composer {
    pub id: String,
    pub name: String,
    pub cast: Vec<String>,
    /// the document read from the server (its other fields stay) and the version a save names as `base`
    pub doc: Option<SceneDocument>,
    pub base: Option<u64>,
    pub query: String,
    pub catalog: Option<Arc<Catalog>>,
    /// the drafts never published of the toon: `scene:<id>`, its stage's name
    pub drafts: Vec<(String, String)>,
    /// it saves itself once it names an id (show/saving.rs); a save carries back the document sent
    pub auto: crate::autosave::Autosave<SceneDocument>,
    asking: Option<Receiver<Result<String, String>>>,
    asked: bool,
}

/// the scenes of `GET /api/scenes` saved but never published whose `toons` name `toon`
pub fn drafts_of(answer: &str, toon: &str) -> Vec<(String, String)> {
    let v: Value = serde_json::from_str(answer).unwrap_or_default();
    v["scenes"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|s| s["published"].is_null() && s["content"]["toons"].as_array().is_some_and(|t| t.iter().any(|x| x == toon)))
        .filter_map(|s| Some((s["key"].as_str()?.to_string(), s["content"]["stage"]["name"].as_str().unwrap_or_default().to_string())))
        .collect()
}

impl Host {
    /// each frame: the toon's drafts asked once, on a thread
    pub(super) fn compose_poll(&mut self) {
        let toon = self.story_toon();
        let c = &mut self.compose;
        if let (false, Some(_)) = (c.asked, &toon) {
            let (tx, rx) = channel();
            let base = self.api.base.clone();
            std::thread::spawn(move || {
                let _ = tx.send(super::api::Api::new(base).scenes());
            });
            (c.asking, c.asked) = (Some(rx), true);
        }
        if let (Some(Ok(answer)), Some(toon)) = (c.asking.as_ref().map(Receiver::try_recv), toon) {
            c.asking = None;
            if let Ok(text) = answer {
                for d in drafts_of(&text, &toon) {
                    if !c.drafts.iter().any(|(a, _)| *a == d.0) {
                        c.drafts.push(d);
                    }
                }
            }
        }
    }

    /// the drafts that the book does not list as published
    pub(super) fn drafts_unlisted(&self) -> Vec<(String, String)> {
        let listed = |a: &str| self.book.as_ref().is_some_and(|b| deck_show::rows::pieces(b, "", super::rows::LOCALE).iter().any(|p| p.address == a));
        self.compose.drafts.iter().filter(|(a, _)| !listed(a)).cloned().collect()
    }

    /// `piece` as the cell inspector shows it: a draft is marked
    pub(super) fn marked(&self, piece: &str) -> String {
        if self.drafts_unlisted().iter().any(|(a, _)| a == piece) {
            format!("{piece} · draft")
        } else {
            piece.to_string()
        }
    }

    pub(super) fn compose_press(&mut self, hit: ComposeHit) {
        let c = &self.compose;
        match hit {
            ComposeHit::Open => {
                self.sheet = Some(Sheet::Scene);
                self.flashing = None;
                self.typing = c.id.is_empty().then(|| (Typing::SceneId, Field::with("")));
            }
            ComposeHit::Id => self.typing = Some((Typing::SceneId, Field::with(&c.id))),
            ComposeHit::Name => self.typing = Some((Typing::SceneName, Field::with(&c.name))),
            ComposeHit::Find => self.typing = Some((Typing::SceneCast, Field::with(&c.query))),
            ComposeHit::Pick(i) => {
                if let Some((filler, _)) = self.cast_choices().into_iter().nth(i) {
                    let cast = &mut self.compose.cast;
                    match cast.iter().position(|f| *f == filler) {
                        Some(at) => {
                            cast.remove(at);
                        }
                        None => cast.push(filler),
                    }
                }
            }
            ComposeHit::Drop(i) => {
                if i < self.compose.cast.len() {
                    self.compose.cast.remove(i);
                }
            }
            ComposeHit::Load => self.compose_load(),
            ComposeHit::Play => self.compose_play(),
            ComposeHit::Build => self.compose_build(),
            ComposeHit::Close => {
                self.sheet = None;
                self.typing = None;
            }
        }
    }

    /// Enter in one of the sheet's fields
    pub(super) fn compose_typed(&mut self, what: &Typing, text: &str) {
        if *what == Typing::SceneId && text.trim() != self.compose.id {
            self.compose_leave();
        }
        let c = &mut self.compose;
        match what {
            Typing::SceneId => {
                let id = text.trim().to_string();
                if id != c.id {
                    (c.doc, c.base) = (None, None);
                    c.auto.open(None);
                }
                c.id = id;
                if c.name.is_empty() {
                    self.typing = Some((Typing::SceneName, Field::with("")));
                }
            }
            Typing::SceneName => c.name = text.trim().to_string(),
            Typing::SceneCast => {
                c.query = text.trim().to_string();
                if let Some(i) = self.cast_choices().iter().position(|(f, _)| !self.compose.cast.contains(f)) {
                    self.compose_press(ComposeHit::Pick(i));
                    self.compose.query.clear();
                }
            }
            _ => {}
        }
    }

    /// the field's text: what is being typed, else what is kept
    fn compose_now(&self, what: &Typing) -> String {
        match &self.typing {
            Some((w, f)) if w == what => f.text.clone(),
            _ => match what {
                Typing::SceneId => self.compose.id.clone(),
                Typing::SceneName => self.compose.name.clone(),
                _ => self.compose.query.clone(),
            },
        }
    }

    /// the toon's rigs that stand on a stage, as fillers, with their rig's line, matching the picker's query
    pub(super) fn cast_choices(&self) -> Vec<(String, String)> {
        let Some(c) = &self.compose.catalog else { return Vec::new() };
        let registry = std::fs::read_to_string(crate::toon::kept(&crate::cache_dir())).unwrap_or_default();
        let toon = self.story_toon();
        let q = self.compose_now(&Typing::SceneCast).to_lowercase();
        c.rigs
            .iter()
            .filter(|e| rigs::of_toon(e, toon.as_deref(), &registry))
            .filter_map(|e| Some((FillerRef::of_rig(&e.subject)?, e)))
            .filter(|(f, _)| q.is_empty() || f.to_lowercase().contains(&q))
            .map(|(f, e)| (f, format!("rig:{}{}", e.subject, e.latest.map_or(String::new(), |v| format!(" · v{v}")))))
            .collect()
    }

    /// the document the sheet stands for: the one read (its other fields kept) or a new one, recast by the core
    pub fn compose_document(&self) -> SceneDocument {
        let c = &self.compose;
        let mut doc = c.doc.clone().unwrap_or_else(|| SceneDocument::new(&c.id, &c.name, self.story_toon().into_iter().collect()));
        doc.id = c.id.clone();
        doc.stage.name = c.name.trim().to_string();
        doc.recast(&c.cast);
        doc
    }

    /// The scene as the stage plays it: the core resolves it with the catalog's rigs (else the deck's core rigs). Or the
    /// core's problems.
    pub fn compose_scene(&self) -> Result<(Scene, Vec<String>), Vec<String>> {
        let doc = self.compose_document();
        let problems = doc.check_new();
        if !problems.is_empty() {
            return Err(problems);
        }
        let catalog = self.compose.catalog.clone();
        let rig = move |subject: &str, version: Option<&str>| catalog.as_ref().and_then(|c| c.rig(subject).and_then(|e| c.bundle(e.content.clone()).ok())).or_else(|| core_rigs(subject, version));
        resolve(&doc, &BTreeMap::new(), false, &rig, &no_sets).map(|r| (r.scene, r.notes))
    }

    fn compose_play(&mut self) {
        match self.compose_scene() {
            Ok((scene, notes)) => {
                let n = self.compose.cast.len();
                self.status = format!("scene:{} plays: {n} on its stage{}", self.compose.id, notes.iter().map(|x| format!(" – {x}")).collect::<String>());
                self.to_stage = Some(scene);
                self.sheet = None;
                self.typing = None;
            }
            Err(p) => self.refuse(format!("not played: {}", p.join("; "))),
        }
    }

    /// the sheet's scene to Build, on the version it was read or saved at
    fn compose_build(&mut self) {
        let doc = self.compose_document();
        let problems = doc.check_new();
        if !problems.is_empty() {
            self.refuse(format!("not opened: {}", problems.join("; ")));
            return;
        }
        self.status = format!("scene:{}: open in a tab", doc.id);
        self.to_build = Some(super::build::ToBuild::Scene(Box::new(super::build::SceneOpen { doc, base: self.compose.base, kept: None, note: None, save: None })));
        self.build = None;
        self.sheet = None;
        self.typing = None;
    }

    /// the scene named by the id from the server: its name and cast fill the sheet, a save goes on its version. A local
    /// copy not sent yet opens instead; one in conflict gives way to the server's newest.
    fn compose_load(&mut self) {
        let id = self.compose.id.clone();
        if id.is_empty() {
            self.refuse("type the scene's id first".into());
            return;
        }
        if let Some((doc, auto)) = self.take_unsent(&id) {
            let c = &mut self.compose;
            (c.name, c.cast) = (doc.stage.name.clone(), doc.cast().into_iter().map(String::from).collect());
            (c.doc, c.base, c.auto) = (Some(doc), auto.version, auto);
        }
        let kept = self.api.outbox.get("scene", &id);
        if kept.as_ref().is_some_and(|e| !e.conflict) {
            self.status = format!("scene:{id}: unsent – kept locally, it goes up first");
            return;
        }
        match self.api.scene_draft(&id).and_then(|d| d.map(|d| SceneDocument::from_value(d.content).map(|doc| (doc, d.version)).map_err(|e| format!("scene:{id}: {e:#}"))).transpose()) {
            Ok(Some((doc, version))) => {
                if kept.is_some() {
                    self.api.outbox.remove("scene", &id);
                }
                let c = &mut self.compose;
                c.name = doc.stage.name.clone();
                c.cast = doc.cast().into_iter().map(String::from).collect();
                (c.doc, c.base) = (Some(doc), version);
                c.auto.open(version);
                self.status = format!("scene:{id} loaded: {} on its stage", c.cast.len());
            }
            Ok(None) => self.status = format!("scene:{id} is not on the server: a new one"),
            Err(e) => self.refuse(format!("not loaded: {e}")),
        }
    }
}

/// The dialog's body in `a` (D3c): the id and the stage's name, the cast with a × each, what the core says is missing,
/// the toon's rigs to tick; Load … at the foot's left, Open, Close and Play at its right.
pub fn draw(d: &mut DrawList, t: &Tokens, host: &mut Host, a: Rect, pointer: (f32, f32)) {
    let (pad, font, small, row) = (t.size.pad, t.size.font, t.size.small, t.size.tree_row);
    let fit = |d: &DrawList, s: &str, px: f32, w: f32| controls::fit_sans(d, px, s, w);
    let col = t.size.rem(5.0);
    let mut y = a.y;
    for (what, label, hint, hit) in [(Typing::SceneId, "id", "scene:", ComposeHit::Id), (Typing::SceneName, "stage", "", ComposeHit::Name)] {
        let typing = host.typing.as_ref().filter(|(w, _)| *w == what).map(|(_, f)| f.text.clone());
        let now = host.compose_now(&what);
        let shown = if what == Typing::SceneId && typing.is_none() && !now.is_empty() { format!("scene:{now}") } else { now };
        let r = dialog::labelled(d, t, (a.x, y, a.w, col), (label, hint), (&shown, typing.as_deref()), pointer);
        host.hits.push((r, Hit::Compose(hit)));
        y += t.size.row + pad * 0.3;
    }
    y += pad * 0.5;
    controls::label(d, t, a.x, y, "cast", t.color.muted);
    y += t.size.label * 1.25 + pad * 0.5;
    let doc = host.compose_document();
    if doc.holders.is_empty() {
        d.sans(a.x, y, font, "nobody yet", t.color.muted);
        y += row;
    }
    for h in &doc.holders {
        let ty = y + (row - font * 1.25) * 0.5;
        d.sans(a.x, ty, font, &fit(d, h.filler.as_deref().unwrap_or(&h.id), font, a.w * 0.5), t.color.text);
        d.sans(a.x + a.w * 0.52, ty + 1.0, small, &fit(d, &format!("{} · {}", h.id, h.spot.as_deref().unwrap_or("the scene's spot")), small, a.w * 0.4), t.color.muted);
        if let Some(at) = h.filler.as_ref().and_then(|f| host.compose.cast.iter().position(|c| c == f)) {
            let r = Rect { x: a.x + a.w - row, y: y + 2.0, w: row - 4.0, h: row - 4.0 };
            controls::icon_button(d, t, r, "close", t.color.muted, pointer);
            host.hits.push((r, Hit::Compose(ComposeHit::Drop(at))));
        }
        y += row;
    }
    // what the core says is missing, once something is typed or cast
    let begun = !host.compose.id.is_empty() || !host.compose.name.is_empty() || !host.compose.cast.is_empty();
    let problems = if begun { doc.check_new() } else { Vec::new() };
    for p in problems.iter().filter(|p| !p.starts_with("the cast is empty")) {
        d.sans(a.x, y, small, &fit(d, p, small, a.w), t.color.warn);
        y += small * 1.6;
    }
    y += pad;
    let toon = host.story_toon().unwrap_or_else(|| "no toon open".into());
    controls::label(d, t, a.x, y, &format!("from {toon}'s rigs"), t.color.muted);
    y += t.size.label * 1.25 + pad * 0.5;
    let query = host.compose_now(&Typing::SceneCast);
    let typing = host.typing.as_ref().filter(|(w, _)| *w == Typing::SceneCast).map(|(_, f)| f.text.clone());
    let find = dialog::input(d, t, Rect { x: a.x, y, w: a.w, h: t.size.row - 4.0 }, (&query, typing.as_deref()), ("Find", true), pointer);
    host.hits.push((find, Hit::Compose(ComposeHit::Find)));
    y += t.size.row + pad * 0.3;
    let foot = a.y + a.h - t.size.row - pad;
    if host.compose.catalog.is_none() {
        d.sans(a.x, y + 4.0, font, &fit(d, NO_RIGS, font, a.w), t.color.warn);
    } else {
        let cast = host.compose.cast.clone();
        let lines: Vec<dialog::Tick> = host.cast_choices().into_iter().map(|(f, _)| dialog::Tick { on: cast.contains(&f), label: f, detail: String::new(), warns: false, indent: false, tick: true }).collect();
        if lines.is_empty() {
            d.sans(a.x, y + 4.0, font, deck_show::rows::NONE_FOUND, t.color.muted);
        }
        for (i, r) in dialog::ticks(d, t, Rect { x: a.x, y, w: a.w, h: foot - y - pad }, &lines, pointer).into_iter().enumerate() {
            host.hits.push((r, Hit::Compose(ComposeHit::Pick(i))));
        }
    }
    let right = [(BUTTONS[1].0, Hit::Compose(BUTTONS[1].1)), (BUTTONS[2].0, Hit::Compose(BUTTONS[2].1))];
    let f = dialog::Foot { left: &[(BUTTONS[0].0, Hit::Compose(BUTTONS[0].1))], right: &right, primary: Some((BUTTONS[3].0, Hit::Compose(BUTTONS[3].1), true)) };
    let hits = dialog::foot(d, t, Rect { x: a.x, y: foot, w: a.w, h: t.size.row }, &f, pointer);
    host.hits.extend(hits);
}

/// what a place of the scene dialog means (the Info view and the hover, H14)
pub fn help(hit: ComposeHit) -> Option<(String, String)> {
    let (name, what) = match hit {
        ComposeHit::Id => ("Id", "The scene's id: scene:<id>. It saves itself as a draft once it has one; the creator publishes it."),
        ComposeHit::Name => ("Stage", "The name of the scene's stage."),
        ComposeHit::Find => ("Find", "Finds a rig of the open toon; Enter takes the first found into the cast."),
        ComposeHit::Pick(_) => ("Cast", "A press adds the rig to the cast or takes it off."),
        ComposeHit::Load => ("Load …", "Reads the scene of this id from the server to recast it: its name and cast fill the dialog, a save goes on its version."),
        ComposeHit::Play => ("Play", "The stage plays the scene as the core resolves it."),
        ComposeHit::Build => ("Open", "Opens the scene in a tab of its own."),
        _ => return None,
    };
    Some((name.into(), what.into()))
}

#[cfg(test)]
mod tests {
    use super::super::rows::tests::{host_with, press, typed};
    use super::super::{Hit, Sheet};
    use super::{drafts_of, ComposeHit};
    use crate::rigs::Catalog;
    use std::sync::Arc;

    const VOTE: &str = include_str!("../../../show/fixtures/vote-cell.show.json");
    const RIGS: &str = r#"{ "entries": [
        { "id": "character.catpope", "latest": 14, "draft": false, "toons": ["*"], "content": { "subject": "character.catpope" } },
        { "id": "item.incense_burner", "latest": 2, "draft": false, "toons": ["*"], "content": { "subject": "item.incense_burner" } },
        { "id": "location.dom", "latest": 1, "draft": false, "toons": ["*"], "content": { "subject": "location.dom" } }
    ] }"#;
    const NONE: &str = r#"{ "entries": [] }"#;
    const REGISTRY: &str = r#"{ "characters": [{ "id": "catpope", "own_toon": "catpope" }] }"#;
    static KEPT: std::sync::Once = std::sync::Once::new();

    fn host() -> super::Host {
        KEPT.call_once(|| {
            let kept = crate::toon::kept(&crate::cache_dir());
            std::fs::create_dir_all(kept.parent().unwrap()).unwrap();
            std::fs::write(kept, REGISTRY).unwrap();
        });
        let mut host = host_with(VOTE);
        host.book = Some(Arc::new(deck_space::rig::book::Book::parse(super::super::rows::tests::BOOK).unwrap()));
        host.toon = Some("catpope".into());
        host.compose.catalog = Some(Arc::new(Catalog::parse(RIGS, NONE, NONE, NONE).unwrap()));
        host
    }

    fn compose(host: &mut super::Host, hit: ComposeHit) {
        press(host, |h| *h == Hit::Compose(hit));
    }

    /// the panel model: "new scene" opens the sheet, id and name typed, two of the toon's rigs picked (a location's
    /// rig is no cast member), one taken off and back; the core's document plays and what is missing is said
    #[test]
    fn a_new_scene_is_named_and_cast_from_the_toons_rigs() {
        let mut host = host();
        // the ☰ menu's New … · Scene in the Show
        host.press(Hit::Compose(ComposeHit::Open));
        assert_eq!(host.sheet, Some(Sheet::Scene));
        assert!(host.compose_scene().unwrap_err().iter().any(|p| p.contains("no scene id")));
        typed(&mut host, "duet");
        typed(&mut host, "Duet");
        assert_eq!((host.compose.id.as_str(), host.compose.name.as_str()), ("duet", "Duet"));
        assert_eq!(host.cast_choices().iter().map(|(f, _)| f.as_str()).collect::<Vec<_>>(), ["character:catpope", "item:incense_burner"]);
        compose(&mut host, ComposeHit::Pick(0));
        compose(&mut host, ComposeHit::Find);
        typed(&mut host, "incense");
        assert_eq!(host.compose.cast, ["character:catpope", "item:incense_burner"], "Enter takes the first found");
        compose(&mut host, ComposeHit::Drop(1));
        assert_eq!(host.compose.cast, ["character:catpope"]);
        compose(&mut host, ComposeHit::Pick(1));
        let doc = host.compose_document();
        assert_eq!(doc.holders.iter().map(|h| h.spot.as_deref().unwrap()).collect::<Vec<_>>(), ["spot_left", "spot_right"]);
        assert_eq!(doc.toons, ["catpope"]);
        compose(&mut host, ComposeHit::Play);
        assert!(host.status.starts_with("scene:duet plays: 2 on its stage"), "{}", host.status);
        assert_eq!(host.sheet, None, "the stage shows it");
        let scene = host.to_stage.take().expect("the stage gets it");
        deck_space::graph::Graph::build(&scene).expect("it builds");
    }

    /// a save without a server is refused in words and nothing joins the list; the toon's never published drafts do,
    /// marked as drafts, and a cell takes one
    #[test]
    fn a_draft_joins_shows_list_marked_and_a_cell_takes_it() {
        let mut host = host();
        host.compose.id = "duet".into();
        host.compose.name = "Duet".into();
        host.compose.cast = vec!["character:catpope".into()];
        host.compose_save_waiting();
        assert!(host.status.starts_with("not saved: "), "{}", host.status);
        assert!(host.compose.drafts.is_empty());
        let answer = r#"{ "scenes": [
            { "key": "scene:duet", "published": null, "content": { "id": "duet", "toons": ["catpope"], "stage": { "name": "Duet" } } },
            { "key": "scene:other", "published": null, "content": { "id": "other", "toons": ["feilchen"], "stage": { "name": "Other" } } },
            { "key": "scene:nave", "published": 7, "content": { "id": "nave", "toons": ["catpope"], "stage": { "name": "Nave" } } }
        ] }"#;
        host.compose.drafts = drafts_of(answer, "catpope");
        assert_eq!(host.compose.drafts, [("scene:duet".to_string(), "Duet".to_string())]);
        let listed = host.scene_list();
        let duet = listed.iter().position(|p| p.address == "scene:duet").expect("listed");
        assert_eq!(listed[duet].detail, format!("scene:duet · {}", deck_show::rows::DRAFT));
        let mut lib = crate::library::Library::new(std::env::temp_dir().join("deck-editor-new-scene-library"));
        super::super::scenes::tests::drag(&mut host, &mut lib, &listed[duet].address, "opening", "middle");
        assert_eq!(host.status, "row 1 opening, middle: scene:duet");
        assert_eq!(host.marked("scene:duet"), "scene:duet · draft");
        assert_eq!(host.marked("scene:nave"), "scene:nave");
    }

    /// Against a running API (`DECK_TEST_API`, the test user's token in `SCUTOONS_TOKEN`; skipped without): a new scene of
    /// two saved as a draft, loaded back with its cast, saved again on its version; a save on the old one is refused.
    #[test]
    fn against_a_server_save_and_load() {
        let Ok(api) = std::env::var("DECK_TEST_API") else { return };
        let mut host = host();
        host.api = super::super::api::Api::new(api);
        let id = format!("duet-{}", std::process::id());
        host.compose.id = id.clone();
        host.compose.name = "Duet".into();
        host.compose.cast = vec!["character:catpope".into(), "item:incense_burner".into()];
        host.compose_save_waiting();
        assert!(host.status.starts_with("saved as draft v"), "{}", host.status);
        assert!(host.status.ends_with(&format!("/edit/scene:{id}")), "{}", host.status);
        assert!(host.compose.drafts.iter().any(|(a, _)| *a == format!("scene:{id}")));
        let first = host.compose.base;
        host.compose = super::Composer { id: id.clone(), catalog: host.compose.catalog.clone(), ..Default::default() };
        host.compose_press(ComposeHit::Load);
        assert_eq!((host.compose.name.as_str(), host.compose.cast.len(), host.compose.base), ("Duet", 2, first), "{}", host.status);
        host.compose.cast.pop();
        host.compose_save_waiting();
        assert!(host.status.starts_with("saved as draft v"), "{}", host.status);
        host.compose.base = first;
        host.compose.cast.push("item:incense_burner".into());
        host.compose_save_waiting();
        assert!(host.status.contains("someone saved a newer version"), "{}", host.status);
    }
}
