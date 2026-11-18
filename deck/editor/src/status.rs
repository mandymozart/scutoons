//! The status bar and the Log panel in every workspace: the messages taken in each frame (the inbox, Show's said line,
//! the conductor's log), the state words as the bar's chips, the bar and the panel drawn, their presses.
use crate::{log, panels, show, sign_in, App, Editing, ValueEdit};
use deck_kit::base::draw::{DrawList, Rect};
use deck_kit::base::tokens::Tokens;
use deck_kit::editor::status::{self as bar, Chip, Newest, Status};
use deck_kit::base::text;
use deck_ui::messages::{self, Level, Message};

/// a state word for the bar: its text, the level its dot says (none: quiet)
pub type Word = (String, Option<Level>);

impl App {
    /// this frame's messages into the ring: what was posted, what Show said, the conductor's new lines
    pub(crate) fn take_messages(&mut self) {
        self.log.take_inbox();
        let said = std::mem::take(&mut self.show.status);
        let bad = std::mem::take(&mut self.show.bad);
        match (self.show.refused.take(), said.is_empty()) {
            (Some(at), false) => self.log.refuse(said, at),
            (_, false) => self.log.all.push(Message::new(if bad { Level::Warn } else { Level::Info }, "show", said, log::now())),
            _ => {}
        }
        self.log.take_show(self.show.sub.log());
    }

    /// the app's state as the bar's quiet chips: what saves, what is linked, the relay, the problems, the glitches, the account
    pub(crate) fn state_words(&self) -> Vec<Word> {
        let mut words: Vec<Word> = Vec::new();
        match self.workspace {
            show::Workspace::Build => {
                if let Some(v) = &self.scene {
                    words.push((v.title(), None));
                }
                if let Some(l) = self.show.build_line() {
                    words.push((l, None));
                }
                let n = self.undo.len();
                if n > 0 {
                    words.push((format!("{n} change{}", if n == 1 { "" } else { "s" }), None));
                }
            }
            show::Workspace::Module => {
                if !self.stage.audio.is_empty() {
                    words.push((self.stage.audio.clone(), None));
                }
            }
            show::Workspace::Show | show::Workspace::Perform => {
                words.push((self.show.link_line(), None));
                let problems = self.show.problems();
                if !problems.is_empty() {
                    let errors = problems.iter().any(|p| p.error);
                    words.push((format!("{} problem{}", problems.len(), if problems.len() == 1 { "" } else { "s" }), Some(if errors { Level::Error } else { Level::Warn })));
                }
            }
        }
        if let Some(p) = self.save_pill() {
            words.push((p, None));
        }
        let glitches = self.stage.glitches();
        if glitches > 0 {
            words.push((format!("{glitches} glitch{}", if glitches == 1 { "" } else { "es" }), Some(Level::Warn)));
        }
        words.push(match self.auth.state() {
            sign_in::Signed::In => ("signed in".into(), None),
            sign_in::Signed::Signing => ("signing in".into(), None),
            sign_in::Signed::Out => ("signed out".into(), Some(Level::Warn)),
        });
        words
    }

    /// a press on the bar's newest message, its counts or its icon: the Log panel opens (the icon closes it again)
    pub(crate) fn press_status(&mut self, x: f32, y: f32) -> bool {
        let Some(i) = self.opens_log.iter().position(|r| r.has(x, y)) else { return false };
        let p = self.panels_mut();
        if p.floats.closed(log::PANEL) {
            p.floats.reopen(log::PANEL);
        } else if p.floats.collapsed(log::PANEL) {
            p.floats.set_collapsed(log::PANEL, false);
        } else if i == 0 {
            p.floats.close(log::PANEL);
        }
        self.panels_moved = true;
        true
    }

    /// a press inside the Log panel
    pub(crate) fn press_log(&mut self, x: f32, y: f32) -> bool {
        let Some((at, p)) = self.log_hits.iter().find(|(r, _)| r.has(x, y)).copied() else { return false };
        match p {
            log::Press::Filter(f) => self.log.filter = f,
            log::Press::Find => {
                let mut field = text::Field::with(&self.log.find);
                field.select_all();
                field.focused = true;
                self.edit = Some(ValueEdit { what: Editing::LogFind, at, field });
            }
            log::Press::Copy => {
                let all = self.log.text();
                match arboard::Clipboard::new().and_then(|mut c| c.set_text(all)) {
                    Ok(()) => log::post(Level::Ok, "log", format!("{} lines copied", self.log.all.shown(self.log.filter, &self.log.find).len())),
                    Err(e) => self.refuse(format!("not copied: {e}"), x, y),
                }
            }
            log::Press::Save => match self.log.save(&crate::cache_dir()) {
                Ok(file) => log::post(Level::Ok, "log", format!("saved {}", file.display())),
                Err(e) => self.refuse(format!("not saved: {e}"), x, y),
            },
        }
        true
    }
}

/// the Log panel in the workspace's panels and the bar along the bottom; the panel's places and the bar's that open it
pub fn draw(d: &mut DrawList, t: &Tokens, size: (f32, f32), all: &log::Log, ws: &mut panels::Panels, (typing, words): (Option<&str>, &[Word]), pointer: (f32, f32)) -> (Vec<(Rect, log::Press)>, Vec<Rect>) {
    let mut hits = Vec::new();
    if let Some(o) = ws.open(d, t, log::PANEL, pointer, 0) {
        hits = log::draw(d, t, o.area, all, typing, pointer);
        ws.close(d, o, &mut hits);
    }
    let now = log::now();
    let age = all.all.newest().map(|m| messages::age(now, m.at)).unwrap_or_default();
    let newest = all.all.newest().map(|m| Newest { dot: log::dot(t, m.level), text: &m.text, age: &age });
    let chips: Vec<Chip> = words.iter().map(|(text, level)| Chip { text, dot: level.map(|l| log::dot(t, l)) }).collect();
    let counts = all.all.counts();
    let log_open = ws.floats.view(log::PANEL).is_some_and(|v| !v.collapsed);
    let placed = bar::draw(d, t, size, &Status { newest, chips: &chips, errors: counts.errors, warnings: counts.warnings, log_open }, pointer);
    (hits, placed.opens_log)
}
