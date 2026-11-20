//! The History panel (history.rs) in Build and Show: the front document's steps newest first – who, when, what changed
//! in words – with Revert and Back to here on each, Undo and Redo in its head; ctrl+z / ctrl+shift+z the same once nothing is
//! unsaved. A revert or a back-to always opens the prompt first (deck-show's prompt, as the flash's); a revert stored
//! opens the document's newest again.
use crate::history::{self, Ask, Done, Plan};
use crate::show::api::Route;
use crate::{log, show, App};
use deck_kit::base::draw::{DrawList, Rect};
use deck_kit::base::tokens::Tokens;
use deck_kit::editor::controls::fit_sans;
use deck_show::choose::{Prompt, Tone};
use deck_ui::messages;
use winit::event::{ElementState, KeyEvent};
use winit::keyboard::{Key, NamedKey};

/// the panel's id in Build and Show
pub const PANEL: &str = "history";
/// the panel's content width
pub const WIDTH: f32 = 560.0;
/// the steps drawn at most, and the places of each
const STEPS: usize = 40;
const PLACES: usize = 3;
/// the places a prompt names
const PROMPT_PLACES: usize = 8;

/// a check run's `--history-ask revert:N | back:N[:confirm]`
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScriptAsk {
    pub back: bool,
    pub row: usize,
    pub confirm: bool,
}

impl ScriptAsk {
    pub fn parse(v: &str) -> Option<ScriptAsk> {
        let mut p = v.split(':');
        let back = match p.next()? {
            "revert" => false,
            "back" => true,
            _ => return None,
        };
        let row = p.next()?.parse().ok()?;
        Some(ScriptAsk { back, row, confirm: p.next() == Some("confirm") })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Press {
    Undo,
    Redo,
    Revert(usize),
    Back(usize),
    Confirm,
    Cancel,
}

fn button(d: &mut DrawList, t: &Tokens, (x, y): (f32, f32), word: &str, pointer: (f32, f32)) -> Rect {
    let (pad, small) = (t.size.pad, t.size.small);
    let r = Rect { x, y, w: d.sans_width(small, word) + pad, h: t.size.tree_row - 4.0 };
    d.outline(r.x, r.y, r.w, r.h, (t.size.radius, 1.0), if r.has(pointer.0, pointer.1) { t.color.text } else { t.color.line });
    d.sans(r.x + pad * 0.5, r.y + (r.h - small * 1.25) * 0.5, small, word, t.color.text);
    r
}

fn who(row: &history::Row) -> String {
    if row.mine {
        return "you".into();
    }
    row.step.author.clone().unwrap_or_else(|| "someone".into())
}

/// the panel's content: Undo, Redo and the document, then its steps newest first
pub fn draw(d: &mut DrawList, t: &Tokens, a: Rect, h: &history::History, doc: Option<&(Route, Option<u64>)>, pointer: (f32, f32)) -> Vec<(Rect, Press)> {
    let (pad, font, small) = (t.size.pad, t.size.font, t.size.small);
    let mut out = Vec::new();
    let undo = button(d, t, (a.x, a.y), "Undo", pointer);
    let redo = button(d, t, (undo.x + undo.w + 4.0, a.y), "Redo", pointer);
    out.extend([(undo, Press::Undo), (redo, Press::Redo)]);
    let head = doc.map_or_else(|| "nothing open that saves itself".to_string(), |(r, v)| format!("{} · {}", r.key(), v.map_or("not saved yet".into(), |v| format!("v{v}"))));
    let hx = redo.x + redo.w + pad;
    let words = fit_sans(d, small, &head, a.x + a.w - hx);
    let w = d.sans_width(small, &words);
    d.sans(a.x + a.w - w, a.y + (undo.h - small * 1.25) * 0.5, small, &words, t.color.muted);
    let mut y = a.y + undo.h + small * 0.6;
    d.rect(a.x, y - small * 0.3, a.w, 1.0, t.color.line);
    let listed = h.listed.as_ref().filter(|l| doc.is_some_and(|(r, _)| *r == l.route));
    let Some(l) = listed else {
        let said = match (&h.why, doc) {
            (Some(why), Some(_)) => why.clone(),
            (_, Some(_)) => "asking the server …".into(),
            (_, None) => "Open a rig, a scene or a show: its saved steps show here.".into(),
        };
        d.sans(a.x, y + 2.0, small, &fit_sans(d, small, &said, a.w), t.color.muted);
        return out;
    };
    if l.rows.is_empty() {
        d.sans(a.x, y + 2.0, small, "No saved version yet.", t.color.muted);
        return out;
    }
    let (row, now) = (font * 1.45, log::now());
    for (i, r) in l.rows.iter().enumerate().take(STEPS) {
        let s = &r.step;
        // Back to here on every step but the newest; Revert on every step with something before it
        let mut bx = a.x + a.w;
        if i > 0 {
            bx -= d.sans_width(small, "Back to here") + pad;
            out.push((button(d, t, (bx, y - 2.0), "Back to here", pointer), Press::Back(i)));
            bx -= 4.0;
        }
        if s.before.is_some() {
            bx -= d.sans_width(small, "Revert") + pad;
            out.push((button(d, t, (bx, y - 2.0), "Revert", pointer), Press::Revert(i)));
        }
        let room = (bx - pad - a.x).max(0.0);
        let name = fit_sans(d, font, &who(r), room * 0.6);
        let x = a.x + d.sans(a.x, y, font, &name, if r.mine { t.color.bright } else { t.color.text });
        let saves = if s.count > 1 { format!(" · {} saves", s.count) } else { String::new() };
        let label = fit_sans(d, small, &format!(" · {}{saves} · v{}", messages::age(now, s.at_ms as f64 / 1000.0), s.last), (a.x + room - x).max(0.0));
        let lw = d.sans(x, y + (font - small) * 0.6, small, &label, t.color.muted);
        if r.published {
            d.disc(x + lw + 8.0, y + font * 0.62, t.size.rem(0.22), t.color.go);
        }
        y += row;
        match &r.described {
            Some(Ok(desc)) => {
                for line in desc.lines.iter().take(PLACES) {
                    let text = format!("{}: {} → {}", line.place, if line.before.is_empty() { "–" } else { &line.before }, if line.after.is_empty() { "–" } else { &line.after });
                    d.sans(a.x + pad, y, small, &fit_sans(d, small, &text, a.w - pad), t.color.muted);
                    y += small * 1.4;
                }
                let more = desc.lines.len().saturating_sub(PLACES) + desc.more;
                if more > 0 {
                    d.sans(a.x + pad, y, small, &format!("+{more} more"), t.color.muted);
                    y += small * 1.4;
                }
            }
            Some(Err(why)) => {
                d.sans(a.x + pad, y, small, &fit_sans(d, small, why, a.w - pad), t.color.warn);
                y += small * 1.4;
            }
            None if s.before.is_none() => {
                d.sans(a.x + pad, y, small, "the first version", t.color.muted);
                y += small * 1.4;
            }
            None => {}
        }
        y += small * 0.4;
    }
    out
}

/// what a planned revert or back-to asks
pub fn prompt(p: &Plan) -> Prompt {
    let mut lines = Vec::new();
    let title = match p.ask {
        Ask::Revert => format!("Revert the step of v{}?", p.version),
        Ask::BackTo => format!("Go back to v{}?", p.version),
        Ask::Undo => format!("Undo the step of v{}?", p.version),
        Ask::Redo => "Redo the undo?".into(),
    };
    if let Some(b) = &p.back {
        lines.push((format!("{} later save{} undone – they stay in the History", b.later, if b.later == 1 { " is" } else { "s are" }), Tone::Plain));
        match (b.others.is_empty(), p.knows_me) {
            (true, _) => {}
            (false, true) => lines.push((format!("saved since by others: {}", b.others.join(", ")), Tone::Warn)),
            // the member could not be asked: whose saves are theirs is not known
            (false, false) => lines.push((format!("saved since by: {}", b.others.join(", ")), Tone::Plain)),
        }
    }
    if let Some(r) = &p.revert {
        if !r.kept.is_empty() {
            lines.push((format!("changed again since, stays as it is: {}", r.kept.join(", ")), Tone::Warn));
        }
    }
    let shown = &p.described.lines;
    if shown.is_empty() {
        lines.push(("nothing changes on the document".into(), Tone::Muted));
    }
    for l in shown.iter().take(PROMPT_PLACES) {
        lines.push((format!("{}: {} → {}", l.place, if l.before.is_empty() { "–" } else { &l.before }, if l.after.is_empty() { "–" } else { &l.after }), Tone::Plain));
    }
    let more = shown.len().saturating_sub(PROMPT_PLACES) + p.described.more;
    if more > 0 {
        lines.push((format!("+{more} more places"), Tone::Muted));
    }
    lines.push(("stored as a new version: nothing is lost".into(), Tone::Muted));
    let confirm = match p.ask {
        Ask::BackTo => "Back to here",
        Ask::Revert => "Revert",
        Ask::Undo => "Undo",
        Ask::Redo => "Redo",
    };
    Prompt { title, lines, warn: p.knows_me && p.back.as_ref().is_some_and(|b| !b.others.is_empty()), confirm: confirm.into(), can: true }
}

impl App {
    /// the document the front tab edits and saves through the server, and the version it stands on
    pub(crate) fn history_doc(&self) -> Option<(Route, Option<u64>)> {
        match self.workspace {
            show::Workspace::Stage => match &self.scene {
                Some(v) => Some((Route::Scene(v.edit.doc.id.clone()), v.auto.version)),
                None if self.rigs.saves() => Some((Route::Entity(format!("rig:{}", self.rigs.subject)), self.rigs.auto.version)),
                None => None,
            },
            show::Workspace::Show => {
                let id = self.show.shown().id.clone();
                let version = self.show.saves.get(&id).and_then(|a| a.version);
                Some((Route::Show(id), version))
            }
            // a factory module is read-only: its save is a module of your own, with a History of its own
            show::Workspace::Module => self.module.open.as_ref().filter(|o| !o.factory).map(|o| (Route::Entity(o.key()), self.module.auto.version)),
            _ => None,
        }
    }

    /// nothing of the front document unsent or on its way: a revert goes on the version the server has
    fn history_settled(&self) -> bool {
        match self.workspace {
            show::Workspace::Stage => match &self.scene {
                Some(v) => v.auto.settled(),
                None => self.rigs.auto.settled(),
            },
            show::Workspace::Show => self.show.saves.get(&self.show.shown().id).is_none_or(|a| a.settled()),
            show::Workspace::Module => self.module.auto.settled(),
            _ => true,
        }
    }

    fn history_open(&self) -> bool {
        self.panels().floats.view(PANEL).is_some_and(|v| !v.collapsed)
    }

    /// each frame: the open panel's list, the answers, a stored revert's document opened again
    pub(crate) fn follow_history(&mut self) {
        self.history.doc = self.history_doc();
        if self.history_open() {
            let doc = self.history.doc.clone();
            self.history.show(doc.as_ref().map(|(r, _)| r), doc.as_ref().and_then(|(_, v)| *v));
        }
        for d in self.history.poll() {
            self.history_done(d);
        }
    }

    fn history_done(&mut self, d: Done) {
        match d {
            Done::Say(text, true) => {
                let (x, y) = self.cursor;
                self.refuse(text, x, y);
            }
            Done::Say(text, false) => self.note(text),
            Done::Stored(route, _, words) => {
                self.note(words);
                self.history_reopen(&route);
            }
        }
    }

    /// the document's newest opened again after a revert stored it
    fn history_reopen(&mut self, route: &Route) {
        let (x, y) = self.cursor;
        match route {
            Route::Entity(key) => {
                if let Some(subject) = key.strip_prefix("rig:").filter(|s| *s == self.rigs.subject && self.scene.is_none()) {
                    let subject = subject.to_string();
                    if let Err(why) = self.rigs.open(&subject) {
                        self.refuse(why, x, y);
                    }
                }
                if let Some(id) = key.strip_prefix("module:").filter(|id| self.module.open.as_ref().is_some_and(|o| o.id == *id)) {
                    match self.show.api().draft(key) {
                        Ok(Some(d)) => {
                            if let Err(why) = self.module.reopen(d) {
                                self.refuse(why, x, y);
                            }
                        }
                        Ok(None) => {}
                        Err(why) => self.refuse(format!("module:{id}: {why}"), x, y),
                    }
                }
            }
            Route::Scene(id) => {
                if self.scene.as_ref().is_none_or(|v| &v.edit.doc.id != id) {
                    return;
                }
                match self.show.api().scene_draft(id) {
                    Ok(Some(d)) => match deck_space::scene_document::SceneDocument::from_value(d.content) {
                        Ok(doc) => {
                            let kept = self.scene.as_ref().and_then(|v| v.kept());
                            self.open_scene(show::build::SceneOpen { doc, base: d.version, kept, note: None, save: None });
                        }
                        Err(e) => self.refuse(format!("scene:{id}: {e}"), x, y),
                    },
                    Ok(None) => {}
                    Err(why) => self.refuse(why, x, y),
                }
            }
            Route::Show(id) => {
                if &self.show.shown().id == id {
                    self.show.reload();
                }
            }
        }
    }

    /// ctrl+z / ctrl+shift+z (and the panel's Undo, Redo): Build's unsaved rig changes first, then this session's saved steps
    pub(crate) fn history_key(&mut self, redo: bool) {
        if self.playing() {
            return self.note("Play: nothing to undo – what you change now is dropped on Stop".to_string());
        }
        if !redo && !self.undo.is_empty() && self.workspace == show::Workspace::Stage {
            return self.take_back();
        }
        let Some((route, base)) = self.history_doc() else { return };
        if !self.history_settled() {
            self.save_now();
            return self.note(format!("{}: saving first – press again once it is saved", route.key()));
        }
        let Some(base) = base else { return self.note(format!("{}: nothing saved yet", route.key())) };
        if redo {
            if let Some(d) = self.history.redo(&route, base) {
                self.history_done(d);
            }
        } else {
            self.history.undo(&route, base);
        }
    }

    /// a press in the panel or on the prompt (the prompt takes every press while it is open)
    pub(crate) fn press_history(&mut self, x: f32, y: f32) -> bool {
        let hit = self.history_hits.iter().find(|(r, _)| r.has(x, y)).map(|(_, p)| *p);
        if self.history.asking.is_some() {
            match hit {
                Some(Press::Confirm) => self.history.confirm(),
                Some(Press::Cancel) => self.history.cancel(),
                _ => {}
            }
            return true;
        }
        let Some(p) = hit else { return false };
        let doc = self.history_doc();
        let rows = self.history.listed.as_ref().map(|l| l.rows.iter().map(|r| r.step.clone()).collect::<Vec<_>>()).unwrap_or_default();
        match (p, doc) {
            (Press::Undo, _) => self.history_key(false),
            (Press::Redo, _) => self.history_key(true),
            (Press::Revert(_) | Press::Back(_), Some((route, Some(_)))) if !self.history_settled() => {
                self.save_now();
                self.refuse(format!("{}: saving first – press again once it is saved", route.key()), x, y);
            }
            (Press::Revert(i), Some((route, Some(base)))) => {
                if let Some(s) = rows.get(i) {
                    self.history.ask_revert(&route, s, base);
                }
            }
            (Press::Back(i), Some((route, Some(base)))) => {
                if let Some(s) = rows.get(i) {
                    self.history.ask_back(&route, s.last as u64, base);
                }
            }
            _ => {}
        }
        true
    }

    /// a check run's ask on a listed step: the prompt opens (its screenshot), `confirm` stores it a few frames later
    pub(crate) fn script_history(&mut self) {
        let Some(s) = self.args.history_ask.clone() else { return };
        if self.history.busy() {
            return;
        }
        if self.history.asking.is_some() {
            if s.confirm {
                self.args.history_ask = None;
                self.history.confirm();
            }
            return;
        }
        let (Some(l), Some((route, Some(base)))) = (self.history.listed.as_ref(), self.history.doc.clone()) else { return };
        let Some(step) = l.rows.get(s.row).map(|r| r.step.clone()).filter(|_| l.route == route) else { return };
        if s.back {
            self.history.ask_back(&route, step.last as u64, base);
        } else {
            self.history.ask_revert(&route, &step, base);
        }
        if !s.confirm {
            self.args.history_ask = None;
        }
    }

    /// the prompt's keys: Enter confirms, Esc goes back
    pub(crate) fn history_prompt_key(&mut self, k: &KeyEvent) {
        if k.state != ElementState::Pressed {
            return;
        }
        match &k.logical_key {
            Key::Named(NamedKey::Enter) => self.history.confirm(),
            Key::Named(NamedKey::Escape) => self.history.cancel(),
            _ => {}
        }
    }
}

/// the panel in the workspace's panels, the prompt over everything: their places this frame
pub fn draw_all(d: &mut DrawList, t: &Tokens, size: (f32, f32), ws: &mut crate::panels::Panels, h: &history::History, pointer: (f32, f32)) -> Vec<(Rect, Press)> {
    let mut hits = Vec::new();
    if let Some(o) = ws.open(d, t, PANEL, pointer, 0) {
        hits = draw(d, t, o.area, h, h.doc.as_ref(), pointer);
        ws.close(d, o, &mut hits);
    }
    if let Some(p) = &h.asking {
        hits.clear();
        show::flash::prompt(d, t, &prompt(p), Rect { x: 0.0, y: 0.0, w: size.0, h: size.1 }, &mut hits, (Press::Confirm, Press::Cancel));
    }
    hits
}

#[cfg(test)]
mod tests {
    use super::*;
    use deck_core::history::{BackTo, Description, Kind, Line, Revert};

    fn plan(ask: Ask) -> Plan {
        let line = Line { place: "gain".into(), kind: Kind::Changed, before: "2".into(), after: "1".into() };
        Plan { ask, route: Route::Entity("rig:character.catpope".into()), version: 7, base: 9, revert: None, back: None, described: Description { lines: vec![line], more: 0 }, knows_me: true }
    }

    #[test]
    fn a_back_to_prompt_names_the_later_saves_and_other_people() {
        let mut p = plan(Ask::BackTo);
        p.back = Some(BackTo { later: 2, others: vec!["nun@test".into()] });
        let q = prompt(&p);
        assert_eq!((q.title.as_str(), q.confirm.as_str(), q.warn), ("Go back to v7?", "Back to here", true));
        let lines: Vec<&str> = q.lines.iter().map(|(l, _)| l.as_str()).collect();
        assert_eq!(lines, ["2 later saves are undone – they stay in the History", "saved since by others: nun@test", "gain: 2 → 1", "stored as a new version: nothing is lost"]);
        p.knows_me = false;
        let q = prompt(&p);
        assert_eq!((q.lines[1].0.as_str(), q.warn), ("saved since by: nun@test", false), "who is who is not known yet");
    }

    #[test]
    fn a_revert_prompt_names_what_stays() {
        let mut p = plan(Ask::Revert);
        p.revert = Some(Revert { doc: serde_json::json!({}), applied: 1, kept: vec!["name".into()] });
        let q = prompt(&p);
        assert_eq!((q.title.as_str(), q.warn), ("Revert the step of v7?", false));
        assert_eq!(q.lines[0], ("changed again since, stays as it is: name".to_string(), Tone::Warn));
    }
}
