//! The cue while a show is live (decisions/deck.md 2026-11-18 [workspaces], #630): the Session panel's cue mix – the show
//! against the preview on the phones (deck-engine cue.rs) – and the cell inspector's pre-listen: the selected cell's piece
//! on the cue before its row fires, as the published copy a deck plays. The editor takes the asks (live_tabs.rs) and plays
//! them on the stage's cue.
use super::{Hit, Host};
use deck_kit::base::draw::DrawList;
use deck_kit::base::tokens::Tokens;
use deck_kit::editor::controls::{self, Rect};
use deck_show::rows;
use deck_space::published::{Opened, SceneRef};
use deck_space::scene::Scene;
use deck_space::scene_resolve::{core_rigs, resolve};
use std::collections::BTreeMap;

/// the cell inspector's button, and while a cell is on the cue
pub const PRELISTEN: &str = "pre-listen";
pub const STOP: &str = "stop pre-listen";

/// What the stage's cue is asked to do.
pub enum Ask {
    /// this scene on the cue, its knobs as the cell sets them (`node.port` of the scene)
    Play { label: String, scene: Box<Scene>, set: Vec<(String, f32)> },
    Stop,
}

/// a drag on the cue mix: where it started and from what
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MixDrag {
    x0: f32,
    v0: f32,
}

/// the cue mix in words: how much of the show and of the preview the phones get
pub fn mix_words(mix: f32) -> String {
    let p = (mix.clamp(0.0, 1.0) * 100.0).round() as u32;
    format!("show {} · preview {p}", 100 - p)
}

impl Host {
    /// the inspector's pre-listen: the selected cell's piece onto the cue, or the one there off
    pub(super) fn ask_prelisten(&mut self) {
        if self.prelistening.is_some() {
            self.to_prelisten = Some(Ask::Stop);
            return;
        }
        match self.prelisten_scene() {
            Ok(a) => self.to_prelisten = Some(a),
            Err(why) => self.refuse(why),
        }
    }

    /// The selected cell's piece as the stage plays it: a published scene from the copy decks play (this editor's cache),
    /// a rig, character or item as the core loads it, with the values the cell sets.
    fn prelisten_scene(&mut self) -> Result<Ask, String> {
        let (Some(row), Some(column)) = self.selected() else { return Err("select a cell first".into()) };
        let piece = rows::cell_inspector(self.shown(), &row, &column).and_then(|i| i.knobs_of).ok_or("the cell plays no piece – nothing to pre-listen")?;
        let label = format!("{piece} · {column}");
        if let Some(r) = SceneRef::parse(&piece) {
            let r = r?;
            let kept = std::fs::read_to_string(self.knobs.cache.join("scenes").join(r.cache_name())).map_err(|_| format!("{piece} is not on this deck yet – flash it first"))?;
            let o = Opened::from_cache(&kept)?;
            let rig = |subject: &str, version: Option<&str>| o.rig(subject).or_else(|| core_rigs(subject, version));
            let set = |location: &str| o.set(location);
            let scene = resolve(&o.document, &BTreeMap::new(), false, &rig, &set).map_err(|p| format!("{piece}: {}", p.join("; ")))?.scene;
            return Ok(Ask::Play { label, scene: Box::new(scene), set: Vec::new() });
        }
        let opened = deck_space::holders::opened(&piece, |p| super::values::rig_file(self.knobs.rigs.as_deref(), p))?;
        let set = match self.knobs.get(&piece) {
            Some(Ok(r)) => rows::cell_knobs(self.shown(), &row, &column, &r.knobs, "en").into_iter().filter(|k| k.set).filter_map(|k| Some((k.path.strip_prefix(&opened.prefix)?.to_string(), k.value))).collect(),
            _ => Vec::new(),
        };
        let scene = crate::stage::bundle_scene(opened.bundle).map_err(|e| format!("{piece}: {e:#}"))?;
        Ok(Ask::Play { label, scene: Box::new(scene), set })
    }

    /// a press on the cue mix starts its drag
    pub(super) fn mix_press(&mut self) {
        self.mix_drag = Some(MixDrag { x0: self.cursor.0, v0: self.cue_mix });
    }

    /// the pointer moved: the cue mix follows (a bar's width of travel is the whole range)
    pub(super) fn mix_moved(&mut self, x: f32, width: f32) {
        if let Some(d) = self.mix_drag {
            self.cue_mix = (d.v0 + (x - d.x0) / width.max(1.0)).clamp(0.0, 1.0);
        }
    }

    pub(super) fn mix_release(&mut self) {
        self.mix_drag = None;
    }
}

/// The Session panel's cue rail: the mix, what is on the cue or why nothing is heard; the height it took.
pub fn rail(d: &mut DrawList, t: &Tokens, host: &mut Host, x: f32, y: f32, w: f32) -> f32 {
    let (small, h) = (t.size.small, t.size.tree_row);
    let mut at = y;
    d.text(x, at + 4.0, small, "CUE", t.color.muted);
    at += h;
    let r = Rect { x, y: at, w, h };
    let bar = controls::scrub(d, t, r, "cue mix", &mix_words(host.cue_mix), host.cue_mix, host.mix_drag.is_some());
    host.mix_width = bar.w;
    host.hits.push((bar, Hit::CueMix));
    at += h;
    let (line, ink) = match (&host.prelistening, host.cue_note.as_deref()) {
        (_, Some(note)) => (note.to_string(), t.color.warn),
        (Some(p), None) => (format!("on the cue: {p}"), t.color.go),
        (None, None) => ("select a cell: pre-listen puts it on the cue".to_string(), t.color.muted),
    };
    d.text(x, at, small, &controls::fit(&line, (w / d.advance(small)).max(0.0) as usize), ink);
    at + h - y
}

#[cfg(test)]
mod tests {
    use super::super::rows::tests::host_with;
    use super::*;

    const DOC: &str = r#"{ "id": "sermon", "version": 1, "toon": "catpope", "outlets": [{ "id": "left" }], "rows": [] }"#;

    #[test]
    fn the_mix_reads_as_show_against_preview() {
        assert_eq!(mix_words(1.0), "show 0 · preview 100");
        assert_eq!(mix_words(0.25), "show 75 · preview 25");
        assert_eq!(mix_words(-3.0), "show 100 · preview 0");
    }

    #[test]
    fn the_mix_follows_a_drag_within_its_range() {
        let mut host = host_with(DOC);
        host.cue_mix = 0.5;
        host.cursor = (100.0, 0.0);
        host.mix_press();
        host.mix_moved(150.0, 200.0);
        assert!((host.cue_mix - 0.75).abs() < 1e-6, "{}", host.cue_mix);
        host.mix_moved(900.0, 200.0);
        assert_eq!(host.cue_mix, 1.0);
        host.mix_release();
        host.mix_moved(0.0, 200.0);
        assert_eq!(host.cue_mix, 1.0, "let go: it stays");
    }

    #[test]
    fn pre_listen_wants_a_cell_and_a_second_press_stops_it() {
        let mut host = host_with(DOC);
        host.ask_prelisten();
        assert!(host.to_prelisten.is_none() && host.bad, "no cell selected: {}", host.status);
        host.prelistening = Some("character:keys · left".into());
        host.ask_prelisten();
        assert!(matches!(host.to_prelisten, Some(Ask::Stop)));
    }
}
