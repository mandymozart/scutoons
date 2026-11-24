//! An entity's "Modules" section (#590, research/MODULE-EXTENSIONS.md §4): its slots as the core lists them (deck-space
//! scene_modules.rs), each holding its module or empty – a dashed slot a module from the library drops on – and a × that
//! takes the module off. A slot the deck does not run yet is listed, muted. What fits where is the core's.
use deck_kit::base::draw::DrawList;
use deck_kit::base::tokens::Tokens;
use deck_kit::editor::controls::{self, Rect};
use deck_space::scene_modules::{On, SlotRow};

/// what a press or a drop lands on
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Part {
    /// slot `i` of the rows: a module dropped here attaches
    Slot(usize),
    /// the module of slot `i` taken off
    Detach(usize),
}

const DASH: f32 = 5.0;

/// The section under `y`; `carry` = where a carried card is (the slot under it lights up). Where each part landed.
pub fn section(d: &mut DrawList, t: &Tokens, area: Rect, y: &mut f32, rows: &[SlotRow], carry: Option<(f32, f32)>) -> Vec<(Rect, Part)> {
    let mut out = Vec::new();
    *y += controls::section(d, t, area.x, *y, area.w, "MODULES", false);
    let h = t.size.row - 4.0;
    let small = t.size.small;
    for (i, s) in rows.iter().enumerate() {
        let r = Rect { x: area.x, y: *y, w: area.w, h };
        let text_y = r.y + (h - t.size.font * 1.25) * 0.5;
        match (&s.module, s.built) {
            (Some(m), _) => {
                d.round_rect(r.x, r.y, r.w, r.h, t.size.radius, t.color.control);
                d.text(r.x + 8.0, text_y, t.size.font, &format!("{} · {m}", s.slot), t.color.text);
                let x = Rect { x: r.x + r.w - h, y: r.y, w: h, h };
                d.text(x.x + h * 0.35, text_y, t.size.font, "×", t.color.muted);
                out.push((r, Part::Slot(i)));
                out.push((x, Part::Detach(i)));
            }
            (None, true) => {
                let hot = carry.is_some_and(|(px, py)| r.has(px, py));
                let line = if hot { t.color.text } else { t.color.muted };
                if hot {
                    d.round_rect(r.x, r.y, r.w, r.h, t.size.radius, t.color.selection);
                }
                controls::dashed(d, r, DASH, if hot { 2.0 } else { 1.0 }, line);
                d.text(r.x + 8.0, text_y, t.size.font, &controls::fit(&format!("{} – drop a module", s.slot), ((r.w - 16.0) / d.advance(t.size.font)) as usize), line);
                out.push((r, Part::Slot(i)));
            }
            (None, false) => {
                d.text(r.x + 8.0, text_y, t.size.font, &format!("{} – not built yet", s.slot), t.color.muted);
            }
        }
        *y += h + 6.0;
        if s.built {
            let words = format!("{} · writes {}", s.name, s.writes.join(", "));
            d.text(area.x + 8.0, *y, small, &controls::fit(&words, ((area.w - 8.0) / d.advance(small)) as usize), t.color.muted);
            *y += small * 1.6;
        }
    }
    out
}

impl crate::App {
    /// the Modules slot under (x, y) on the panels: the camera's (DOP) or the selected member's (the inspector)
    fn slot_under(&self, x: f32, y: f32) -> Option<On> {
        let view = self.scene.as_ref()?;
        self.panel.hits.iter().rev().find(|(r, _)| r.has(x, y)).and_then(|(_, h)| match h {
            crate::Hit::Dop(crate::dop::Field::Slot(_)) => Some(On::Camera),
            crate::Hit::SceneField(crate::scene::Field::Slot(_)) => crate::scene::member_slots(view),
            _ => None,
        })
    }

    /// A module from the library let go at (x, y): on a Modules slot it attaches (the core says whether it fits there),
    /// anywhere else it is refused with where it goes.
    pub(crate) fn module_drop(&mut self, address: &str, x: f32, y: f32) {
        let id = address.strip_prefix("module:").unwrap_or(address).to_string();
        let Some(on) = self.slot_under(x, y) else {
            self.refuse("a module goes on a Modules slot: the camera's in the DOP panel, a member's in its inspector".into(), x, y);
            return;
        };
        let descriptor = self.rigs.module_docs().iter().find(|c| c["id"].as_str() == Some(id.as_str())).and_then(|c| serde_json::from_value::<deck_catalog::Descriptor>(c["descriptor"].clone()).ok());
        let Some(d) = descriptor else {
            self.refuse(format!("module:{id} has no descriptor yet – build it first"), x, y);
            return;
        };
        let Some(view) = self.scene.as_mut() else { return };
        match view.edit.attach(on, &id, &d) {
            Ok(slot) => {
                let whose = match on {
                    On::Camera => "the camera".to_string(),
                    On::Holder(h) => view.edit.doc.holders.get(h).map_or_else(String::new, |h| h.id.clone()),
                };
                self.scene_changed(Ok(()), format!("{id} on {whose}'s {slot} slot"), x, y);
            }
            Err(why) => self.refuse(why, x, y),
        }
    }

    /// A check run's: `--scene-file FILE` opened in Build; `--dop …modules` attaches each slot module of the catalog to
    /// the first entity of its slot (the camera, an item member).
    pub(crate) fn script_slots(&mut self) {
        if self.frames == 5 {
            if let Some(f) = self.args.scene_file.clone() {
                match std::fs::read_to_string(&f).map_err(|e| e.to_string()).and_then(|t| deck_space::scene_document::SceneDocument::parse_json(&t).map_err(|e| format!("{e:#}"))) {
                    Ok(doc) => self.open_scene(crate::show::build::SceneOpen { doc, base: None, kept: None, note: Some(format!("from {}", f.display())), save: None }),
                    Err(e) => tracing::error!("--scene-file {}: {e}", f.display()),
                }
            }
        }
        if self.frames != 70 || !self.args.dop.as_deref().is_some_and(|s| s.contains("modules")) {
            return;
        }
        let (x, y) = self.cursor;
        let docs: Vec<(String, deck_catalog::Descriptor)> = self
            .rigs
            .module_docs()
            .iter()
            .filter_map(|c| Some((c["id"].as_str()?.to_string(), serde_json::from_value(c["descriptor"].clone()).ok()?)))
            .collect();
        let Some(view) = self.scene.as_mut() else { return };
        for (id, d) in docs {
            let Some(slot) = d.slot.as_deref().and_then(deck_catalog::slot::find) else { continue };
            let on = match slot.entity {
                deck_catalog::slot::Entity::Camera => Some(On::Camera),
                e => view.edit.doc.holders.iter().position(|h| h.entity() == e).map(On::Holder),
            };
            if let Some(on) = on {
                if let Err(e) = view.edit.attach(on, &id, &d) {
                    tracing::error!("--dop modules: {id}: {e}");
                }
            }
        }
        self.scene_changed(Ok(()), "the catalog's slot modules attached".into(), x, y);
    }

    /// the module of slot `i` of `on` taken off
    pub(crate) fn slot_detach(&mut self, on: On, i: usize, x: f32, y: f32) {
        let Some(view) = self.scene.as_mut() else { return };
        let Some(row) = view.edit.slot_rows(on).into_iter().nth(i) else { return };
        let done = view.edit.detach(on, row.slot);
        self.scene_changed(done, format!("the {} slot is empty", row.slot), x, y);
    }
}
