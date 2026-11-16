//! A show cell's piece in Build (#568; owner 2026-11-15 [workspaces]): the cell inspector's "open in Build" hands Build the
//! rig the cell's piece plays – a rig, character or item as the core loads it (deck-space holders.rs `opened`) – or the
//! scene itself to edit (its newest draft, scene.rs in the editor; the list's and the scene sheet's scenes too) – and keeps
//! the link to the cell: the top bar says it, "values → cell" writes Build's knob positions into the cell through the
//! rows' `Edit::Take` (the piece's slots and ranges are deck-show's, values.rs `from_build`). Which far node's holder plays
//! on which outlet of a row is deck-show's (far.rs); here it is asked with the loader's reach and handed to the drawing.
use super::Host;
use deck_show::far::{self, Link};
use deck_show::rows::{self, Edit};
use deck_show::values;
use deck_space::holders::{self, Opened, Reach};
use deck_space::scene_document::SceneDocument;
use std::collections::HashMap;

/// the cell inspector's button, Build's two buttons while it is linked
pub const OPEN: &str = "open in Build";
pub const TO_CELL: &str = "values → cell";
pub const BACK: &str = "back to Show";

/// Build linked to a show cell: the cell, its piece, the rig open (`character:keys`) and its slots' prefix in the piece.
#[derive(Debug, Clone, PartialEq)]
pub struct BuildLink {
    pub row: String,
    pub column: String,
    pub piece: String,
    pub place: String,
    pub prefix: String,
}

/// What Build is asked to play: a rig at hand with the values the cell sets (its rig's slots), one the server's
/// catalog has (rigs.rs opens it), or a scene to edit (scene.rs).
pub enum ToBuild {
    Play(Box<Opened>, Vec<(String, f32)>),
    Server(String),
    Scene(Box<SceneOpen>),
}

/// A scene for Build: the document to edit, the version a save names as `base`, and what this editor keeps of it (its
/// rigs as scene-content gave them).
pub struct SceneOpen {
    pub doc: SceneDocument,
    pub base: Option<u64>,
    pub kept: Option<deck_space::published::Opened>,
    /// what the opening says beside it (the server not reached …)
    pub note: Option<String>,
}

impl Host {
    /// the link's words for the top bar: `row 2 together · left` (and the member, when it is not the piece itself)
    pub fn build_line(&self) -> Option<String> {
        let l = self.build.as_ref()?;
        let n = self.shown().rows.iter().position(|r| r.id == l.row).map_or(0, |i| i + 1);
        let member = if l.place == l.piece { String::new() } else { format!(" · {}", l.place) };
        Some(format!("row {n} {} · {}{member}", l.row, l.column))
    }

    /// the press on "open in Build": what Build plays waits in `to_build` for the editor (main.rs), or why not
    pub(super) fn ask_build(&mut self) {
        match self.open_in_build() {
            Ok(t) => self.to_build = Some(t),
            Err(why) => self.status = why,
        }
    }

    /// The selected cell's piece for Build, and the link kept.
    pub fn open_in_build(&mut self) -> Result<ToBuild, String> {
        let (Some(row), Some(column)) = self.selected() else { return Err("select a cell first".into()) };
        let piece = rows::cell_inspector(self.shown(), &row, &column).and_then(|i| i.knobs_of).ok_or("the cell plays no piece – nothing to open in Build")?;
        let link = |place: String, prefix: String| BuildLink { row: row.clone(), column: column.clone(), piece: piece.clone(), place, prefix };
        let opened = match deck_space::published::SceneRef::parse(&piece) {
            Some(_) => {
                let open = self.scene_for_build(&piece)?;
                self.build = Some(link(piece.clone(), String::new()));
                self.status = format!("{}: open in Build", self.build_line().unwrap_or_default());
                return Ok(ToBuild::Scene(Box::new(open)));
            }
            None => match holders::opened(&piece, |p| super::values::rig_file(self.knobs.rigs.as_deref(), p)) {
                Ok(o) => o,
                Err(why) if why.ends_with("is not on this deck") => {
                    let place = deck_space::rig::address::Place::parse(&piece).map_err(|e| format!("{piece}: {e}"))?;
                    self.build = Some(link(piece.clone(), String::new()));
                    self.status = format!("{piece}: asking the server for its rig");
                    return Ok(ToBuild::Server(format!("{}.{}", place.kind, place.id)));
                }
                Err(why) => return Err(why),
            },
        };
        // the knobs as the cell sets them, by the rig's own slots (the member's prefix off)
        let set = match self.knobs.get(&piece) {
            Some(Ok(r)) => rows::cell_knobs(self.shown(), &row, &column, &r.knobs, "en").into_iter().filter(|k| k.set).filter_map(|k| Some((k.path.strip_prefix(&opened.prefix)?.to_string(), k.value))).collect(),
            _ => Vec::new(),
        };
        self.build = Some(link(opened.place.clone(), opened.prefix.clone()));
        self.status = format!("{}: open in Build", self.build_line().unwrap_or_default());
        Ok(ToBuild::Play(Box::new(opened), set))
    }

    /// Scene `piece` (`scene:<id>`) for Build: its newest draft from the server (a draft never published too) and the
    /// version to save on; what the cache keeps of it gives its rigs, and stands in for the document when the server is
    /// not reached (a save then asks the server again).
    pub fn scene_for_build(&mut self, piece: &str) -> Result<SceneOpen, String> {
        let r = deck_space::published::SceneRef::parse(piece).ok_or_else(|| format!("{piece} is no scene"))??;
        let dir = self.knobs.cache.join("scenes");
        let kept = [deck_native::published::draft_file(&dir, &r), dir.join(r.cache_name())]
            .iter()
            .find_map(|f| std::fs::read_to_string(f).ok())
            .and_then(|t| deck_space::published::Opened::from_cache(&t).ok());
        let doc = |d: super::api::Draft| SceneDocument::from_value(d.content).map(|doc| (doc, d.version)).map_err(|e| format!("{piece}: {e:#}"));
        let (doc, base, note) = match self.api.scene_draft(&r.id) {
            Ok(Some(d)) => {
                let (doc, base) = doc(d)?;
                (doc, base, None)
            }
            Ok(None) => return Err(format!("{piece} is not on the server")),
            Err(why) => match &kept {
                Some(k) => (k.document.clone(), None, Some(format!("the cached copy – the server: {why}"))),
                None => return Err(format!("{piece}: {why}")),
            },
        };
        Ok(SceneOpen { doc, base, kept, note })
    }

    /// a scene of the list beside the rows band, or the scene sheet's, to Build
    pub(super) fn ask_scene_build(&mut self, piece: &str) {
        match self.scene_for_build(piece) {
            Ok(open) => {
                self.build = None;
                self.status = format!("{piece}: open in Build");
                self.to_build = Some(ToBuild::Scene(Box::new(open)));
            }
            Err(why) => self.status = why,
        }
    }

    /// Build's scene saved as a new draft on `base` → its version and the words for it; the sheet and the list follow
    pub fn save_built_scene(&mut self, doc: &SceneDocument, base: Option<u64>) -> Result<(u64, String), String> {
        let problems = doc.check_new();
        if !problems.is_empty() {
            return Err(format!("not saved: {}", problems.join("; ")));
        }
        let content = serde_json::to_value(doc).map_err(|e| format!("not saved: {e}"))?;
        let version = self.api.save_scene(&doc.id, &content, base, "deck editor").map_err(|e| format!("not saved: {e}"))?;
        self.scene_saved(doc, version);
        let link = crate::rigs::creator_link(&self.api.base, &format!("scene:{}", doc.id));
        Ok((version, crate::rigs::Saved::Done { version, link }.words()))
    }

    /// Build saved scene `doc` as version `version`: the sheet and the list follow, the knobs are asked again
    pub fn scene_saved(&mut self, doc: &SceneDocument, version: u64) {
        let key = format!("scene:{}", doc.id);
        if self.compose.id == doc.id {
            (self.compose.doc, self.compose.base) = (Some(doc.clone()), Some(version));
            self.compose.name.clone_from(&doc.stage.name);
            self.compose.cast = doc.cast().into_iter().map(String::from).collect();
        }
        if !self.compose.drafts.iter().any(|(a, _)| *a == key) {
            self.compose.drafts.push((key.clone(), doc.stage.name.clone()));
        }
        self.knobs.forget(&key);
    }

    /// "values → cell": Build's knob positions (its rig's slots) into the linked cell, as the piece names them. Whether it
    /// was written; the status says what.
    pub fn values_to_cell(&mut self, positions: &[(String, f32)]) -> bool {
        let Some(l) = self.build.clone() else {
            self.status = "Build is not linked to a cell – open one from Show's cell inspector".into();
            return false;
        };
        let read = match self.knobs.get(&l.piece) {
            Some(Ok(r)) => r,
            Some(Err(why)) => {
                self.status = why;
                return false;
            }
            None => {
                self.status = values::LOADING.into();
                return false;
            }
        };
        let (taken, left) = values::from_build(positions, &l.prefix, &read.knobs);
        if taken.is_empty() {
            self.status = format!("none of Build's knobs turns a knob of {}", l.piece);
            return false;
        }
        let done = self.edit_rows_with(&Edit::Take { row: l.row, column: l.column, positions: taken }, Some(&read.knobs));
        if done && left > 0 {
            self.status = format!("{} ({left} of Build's knobs are not the piece's)", self.status);
        }
        done
    }

    /// The far nodes of row `row`'s pieces and where their holders play (deck-show far.rs); a piece's reach is asked of the
    /// loader, one not known yet waits.
    pub fn row_links(&mut self, row: &str) -> Vec<Link> {
        self.row_links_with(row, None)
    }

    /// the same, with `own` standing in for one piece's reach (Build's rig as it is edited now)
    fn row_links_with(&mut self, row: &str, own: Option<(&str, &Reach)>) -> Vec<Link> {
        let show = self.shown().clone();
        let Some(i) = show.rows.iter().position(|r| r.id == row) else { return Vec::new() };
        let mut got: HashMap<String, Reach> = HashMap::new();
        for (_, p) in far::on_row(&show, i) {
            if let Some(Ok(r)) = self.knobs.get(&p) {
                got.insert(p, r.reach.clone());
            }
        }
        if let Some((piece, reach)) = own {
            got.insert(piece.to_string(), reach.clone());
        }
        far::links(&show, row, |p| got.get(p).cloned())
    }

    /// Build's far wires while it is linked: the linked cell's far nodes as Build's node ids (the member's prefix off).
    /// `reach` is Build's rig as edited: it stands in for the cell's piece when the piece is that rig itself.
    pub fn build_wires(&mut self, reach: &Reach) -> Vec<Link> {
        let Some(l) = self.build.clone() else { return Vec::new() };
        let own = l.prefix.is_empty().then_some((l.piece.as_str(), reach));
        self.row_links_with(&l.row, own)
            .into_iter()
            .filter(|x| x.from == l.column)
            .filter_map(|mut x| {
                x.node = x.node.strip_prefix(&l.prefix)?.to_string();
                Some(x)
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::super::rows::tests::{host_with, press};
    use super::super::values::KnobHit;
    use super::super::Hit;
    use super::ToBuild;
    use deck_show::far::Holder;

    const SHOW: &str = include_str!("../../../show/fixtures/far-outlets.show.json");

    fn host() -> super::Host {
        let mut host = host_with(SHOW);
        host.knobs.rigs = Some(concat!(env!("CARGO_MANIFEST_DIR"), "/../show/fixtures/rigs").into());
        for p in ["character:keys", "character:bells", "item:incense_burner"] {
            assert!(matches!(host.knobs.now(p), Some(Ok(_))), "{p}: {:?}", host.knobs.now(p).map(|a| a.err()));
        }
        host
    }

    fn select(host: &mut super::Host, row: usize, column: &str) {
        host.row = row;
        let c = deck_show::view::plan(host.shown()).columns.iter().position(|c| c.id == column).unwrap();
        press(host, false, |h| *h == Hit::Cell(row, c));
    }

    /// the cell inspector's "open in Build": Build gets the cell's rig, the link names the cell; "values → cell" writes
    /// Build's knobs into it through the core (a knob out of its range is refused)
    #[test]
    fn a_cell_opens_in_build_and_builds_values_go_back_into_it() {
        let mut host = host();
        select(&mut host, 1, "left");
        press(&mut host, false, |h| *h == Hit::Knob(KnobHit::Build));
        let Some(ToBuild::Play(opened, set)) = host.to_build.take() else { panic!("Build gets a rig: {}", host.status) };
        assert_eq!(opened.bundle.rig.subject, "character.keys");
        assert_eq!(set, [("tone_osc.freq".to_string(), 440.0)], "Build starts from the cell's values");
        assert_eq!(host.build_line().as_deref(), Some("row 2 together · left"));
        assert_eq!(host.status, "row 2 together · left: open in Build");
        // Build turned the keys: their gain and pitch go back, a knob the piece lacks is counted
        let positions = [("tone_osc.gain".to_string(), 0.65), ("tone_osc.freq".to_string(), 523.0), ("far_in.x".to_string(), 0.0)];
        assert!(host.values_to_cell(&positions), "{}", host.status);
        assert!(host.status.starts_with("row 2 together, left: took 2 knobs") && host.status.ends_with("(1 of Build's knobs are not the piece's)"), "{}", host.status);
        let Some(deck_show::Cell::Values { values, .. }) = host.shown().row("together").unwrap().cells.get("left") else { panic!() };
        assert_eq!(values.get("tone:osc.freq").and_then(|v| v.as_f64()), Some(523.0));
        assert!(!host.values_to_cell(&[("tone_osc.freq".to_string(), 9000.0)]), "out of range");
        assert!(host.status.contains("goes from"), "{}", host.status);
        // a cell without a piece opens nothing; a rig the deck lacks is the server's
        select(&mut host, 2, "left");
        assert!(host.open_in_build().is_err());
        host.build = None;
        assert!(!host.values_to_cell(&positions));
        assert!(host.status.starts_with("Build is not linked"));
    }

    /// the scene sheet's "Build" hands Build the sheet's document on its version; one the core refuses stays in the sheet
    #[test]
    fn the_scene_sheet_opens_its_scene_in_build() {
        use super::super::new_scene::ComposeHit;
        let mut host = host();
        host.toon = Some("keys".into());
        host.compose_press(ComposeHit::Build);
        assert!(host.status.starts_with("not opened: "), "{}", host.status);
        assert!(host.to_build.is_none());
        (host.compose.id, host.compose.name, host.compose.cast, host.compose.base) = ("duet".into(), "Duet".into(), vec!["character:catpope".into()], Some(5));
        host.compose_press(ComposeHit::Build);
        let Some(ToBuild::Scene(open)) = host.to_build.take() else { panic!("{}", host.status) };
        assert_eq!((open.doc.id.as_str(), open.doc.cast(), open.base), ("duet", vec!["character:catpope"], Some(5)));
        assert_eq!(host.status, "scene:duet: open in Build");
    }

    /// Against a running API (`DECK_TEST_API`, the test user's token in `SCUTOONS_TOKEN`; skipped without): a scene made in
    /// the sheet and saved as a draft opens in Build from the server, a member's value set by the core, saved as the
    /// scene's next draft; back in Show the sheet has that version and a save on the old one is refused.
    #[test]
    fn against_a_server_a_scene_opens_in_build_and_saves_its_draft() {
        use super::super::new_scene::ComposeHit;
        let Ok(api) = std::env::var("DECK_TEST_API") else { return };
        let mut host = host();
        host.api = super::super::api::Api::new(api);
        host.toon = Some("catpope".into());
        let id = format!("build-{}", std::process::id());
        (host.compose.id, host.compose.name, host.compose.cast) = (id.clone(), "Built".into(), vec!["character:catpope".into(), "item:incense_burner".into()]);
        host.compose_press(ComposeHit::Save);
        let first = host.compose.base.unwrap_or_else(|| panic!("{}", host.status));
        let open = host.scene_for_build(&format!("scene:{id}")).expect("the draft from the server");
        assert_eq!((open.base, open.doc.cast().len()), (Some(first), 2));
        let rigs = deck_space::scene_resolve::core_rigs;
        let (mut edit, _) = deck_space::scene_edit::SceneEdit::open(open.doc, open.base, &rigs, &deck_space::scene_resolve::no_sets).expect("it plays");
        let k = edit.members[0].knobs[0].clone();
        let v = if k.value == k.min { k.max } else { k.min };
        edit.set(0, &k.path, v).unwrap();
        let (version, words) = host.save_built_scene(&edit.doc, edit.base).expect("saved");
        assert!(version > first && words.starts_with("saved as draft v"), "{words}");
        assert_eq!(host.compose.base, Some(version), "Show's sheet follows");
        assert_eq!(host.compose.doc.as_ref().map(|d| d.holders[0].values.len()), Some(1));
        let back = host.scene_for_build(&format!("scene:{id}")).unwrap();
        assert_eq!(back.doc.holders[0].values, edit.doc.holders[0].values, "the value is the server's");
        assert!(host.save_built_scene(&edit.doc, Some(first)).unwrap_err().contains("someone saved a newer version"));
        // never published: its knobs come from its draft, kept apart from the published copies
        host.knobs = super::super::values::Loader::new(host.api.base.clone());
        host.knobs.cache = std::env::temp_dir().join(format!("deck-editor-{id}"));
        let read = host.knobs.now(&format!("scene:{id}")).expect("asked").expect("the draft's knobs");
        assert!(read.knobs.iter().any(|k| k.path.starts_with("catpope_")), "{:?}", read.knobs.iter().map(|k| &k.path).collect::<Vec<_>>());
        assert!(host.knobs.cache.join("scenes/drafts").join(format!("{id}.json")).exists());
        let _ = std::fs::remove_dir_all(&host.knobs.cache);
    }

    /// the far nodes: in Build the linked cell's, towards the outlet its holder plays on; in Show, the row's pairs
    #[test]
    fn far_wires_follow_the_core_in_build_and_in_show() {
        let mut host = host();
        select(&mut host, 1, "left");
        host.ask_build();
        let keys = host.knobs.now("character:keys").and_then(Result::ok).expect("the keys").reach.clone();
        let wires = host.build_wires(&keys);
        assert_eq!(wires.iter().map(|w| (w.node.as_str(), &w.holder)).collect::<Vec<_>>(), [("far_in", &Holder::Outlet("right".into()))]);
        // a far node added in Build to the keys, listening to the bells, is wired to the outlet the bells play on
        let mut edited = keys.clone();
        edited.far.push(deck_space::holders::FarNode { node: "tone_far".into(), address: deck_space::rig::address::Address::parse("character:bells/far:in.events").expect("an address") });
        let wires = host.build_wires(&edited);
        assert!(wires.iter().any(|w| w.node == "tone_far" && w.holder == Holder::Outlet("right".into())), "{wires:?}");
        let row = host.row_links("together");
        assert_eq!(row.iter().map(|l| (l.from.as_str(), &l.holder)).collect::<Vec<_>>(), [("left", &Holder::Outlet("right".into())), ("right", &Holder::Outlet("left".into()))]);
        assert_eq!(host.row_links("keys-alone").iter().map(|l| &l.holder).collect::<Vec<_>>(), [&Holder::Away]);
    }
}
