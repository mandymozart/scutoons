//! The conductor's roles in the Show workspace (#478 slice 3, the owner's draft 4 "compact", 2026-11-10 [role] [design]):
//! the selected role's inspector – its name, who may enter as toggles, how many are here, its scenes, who defined it – and
//! its remove behind a prompt. Every edit goes through deck-show's `roles::edit` (checked there, refused in its words) and,
//! while this editor runs the conductor, the changed roles go out at once (`Subtree::edited`). The words, what an edit does
//! and what a prompt says are deck-show's; this file lays them out and hands the presses and the typed name in (RULES A1,
//! A2). Drawn at the kit's sizes as they are (rows 24 and 28, text 12 and 11).
use super::flash::prompt;
use super::{Hit, Host};
use deck_kit::controls::{self, Rect};
use deck_kit::draw::DrawList;
use deck_kit::text::Field;
use deck_kit::tokens::Tokens;
use deck_show::roles::{Edit, Inspector};
use winit::keyboard::{Key, NamedKey};

/// What the typed field is for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Typing {
    /// a role's name (its id)
    Label(String),
}

/// What a prompt asks before it is done.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Ask {
    /// remove this role and its cells
    Remove(String),
}

impl Host {
    /// An edit of the shown document's roles: deck-show makes and checks it; the conductor this editor runs takes it and
    /// its session hears the roles at once; a sub conductor's document shown here changes only here.
    pub(super) fn edit_role(&mut self, e: &Edit) {
        match deck_show::roles::edit(self.shown(), e) {
            Ok(done) => {
                self.status = done.said;
                self.role = done.role;
                if self.path.is_empty() {
                    let out = self.sub.edited(done.show);
                    self.send(out);
                } else {
                    self.library.insert(done.show.id.clone(), done.show);
                }
            }
            Err(why) => self.status = why,
        }
    }

    pub(super) fn roles_press(&mut self, hit: Hit) {
        let id = self.role.clone();
        match hit {
            Hit::RoleAdd => self.edit_role(&Edit::Add),
            Hit::RoleName => {
                if let Some(o) = id.as_ref().and_then(|id| self.shown().outlet(id)) {
                    self.typing = Some((Typing::Label(o.id.clone()), Field::with(o.name())));
                }
            }
            Hit::RoleFact(i) => {
                let fact = id.as_ref().and_then(|id| deck_show::roles::toggles(self.shown(), id).into_iter().nth(i));
                if let (Some(id), Some(t)) = (id, fact) {
                    self.edit_role(&Edit::Fact { id, fact: t.fact });
                }
            }
            Hit::RoleRemove => self.asking = id.map(Ask::Remove),
            Hit::AskYes => match self.asking.take() {
                Some(Ask::Remove(id)) => self.edit_role(&Edit::Remove { id }),
                None => {}
            },
            Hit::AskNo => self.asking = None,
            _ => {}
        }
    }

    /// A key while a typed field is open: true = taken. Enter applies it, Escape leaves it as it was.
    pub(super) fn type_field(&mut self, key: &Key) -> bool {
        let Some((what, field)) = self.typing.as_mut() else { return false };
        match key {
            Key::Character(c) => field.insert(c),
            Key::Named(NamedKey::Space) => field.insert(" "),
            Key::Named(NamedKey::Backspace) => field.backspace(),
            Key::Named(NamedKey::Enter) => {
                let (what, text) = (what.clone(), field.text.clone());
                self.typing = None;
                match what {
                    Typing::Label(id) => self.edit_role(&Edit::Label { id, label: text }),
                }
            }
            Key::Named(NamedKey::Escape) => self.typing = None,
            _ => {}
        }
        true
    }

    /// The prompt asked now, in deck-show's words.
    fn ask_prompt(&self) -> Option<deck_show::choose::Prompt> {
        match self.asking.as_ref()? {
            Ask::Remove(id) => {
                let here = self.path.is_empty().then(|| self.sub.here_count(id)).flatten();
                Some(deck_show::roles::remove_prompt(self.shown(), id, here))
            }
        }
    }
}

fn fit(d: &DrawList, text: &str, px: f32, w: f32) -> String {
    controls::fit(text, (w / d.advance(px)).max(0.0) as usize)
}

/// a label and a quiet value on one tree row
fn line(d: &mut DrawList, t: &Tokens, (x, y, w): (f32, f32, f32), label: &str, value: &str) {
    let font = t.size.font;
    let ty = y + (t.size.tree_row - font * 1.25) * 0.5;
    if !label.is_empty() {
        d.text(x, ty, font, &fit(d, label, font, w * 0.4), t.color.muted);
    }
    let vx = if label.is_empty() { x } else { x + w * 0.42 };
    d.text(vx, ty, font, &fit(d, value, font, x + w - vx), t.color.text);
}

/// The selected role's inspector in `a` (the right side above Load / Flash / Save).
pub fn inspector(d: &mut DrawList, t: &Tokens, host: &mut Host, insp: &Inspector, a: Rect, pointer: (f32, f32)) {
    let (pad, font, row) = (t.size.pad, t.size.font, t.size.tree_row);
    let (x, w) = (a.x, a.w);
    let mut y = a.y;
    y += controls::section(d, t, x, y, w, &insp.title.to_uppercase(), true);
    // the name: a field, typed into after a press
    d.text(x, y + (row - font * 1.25) * 0.5, font, &insp.name.0, t.color.muted);
    let field = Rect { x: x + w * 0.42, y: y + 2.0, w: w * 0.58, h: row - 4.0 };
    d.round_rect(field.x, field.y, field.w, field.h, t.size.radius, t.color.control);
    let typing = host.typing.as_ref().filter(|(what, _)| matches!(what, Typing::Label(_)));
    let text = typing.map_or_else(|| insp.name.1.clone(), |(_, f)| format!("{}|", f.text));
    if typing.is_some() {
        d.outline(field.x, field.y, field.w, field.h, (t.size.radius, 1.0), t.color.accent);
    }
    d.text(field.x + 6.0, field.y + (field.h - font * 1.25) * 0.5, font, &fit(d, &text, font, field.w - 12.0), t.color.text);
    host.hits.push((field, Hit::RoleName));
    y += row;
    // who may enter: the filter in words, then its facts as toggles
    line(d, t, (x, y, w), &insp.enter.0, &insp.enter.1);
    y += row;
    let names: Vec<&str> = insp.toggles.iter().map(|t| t.word.as_str()).collect();
    let on: Vec<bool> = insp.toggles.iter().map(|t| t.on).collect();
    let (cells, used) = controls::chips(d, t, (x, y, w), &names, &on);
    for (i, r) in cells.into_iter().enumerate() {
        host.hits.push((r, Hit::RoleFact(i)));
    }
    y += used + pad;
    for (head, lines) in &insp.sections {
        y += controls::section(d, t, x, y, w, &head.to_uppercase(), false);
        for (label, value) in lines {
            if y + row > a.y + a.h - row - pad {
                break;
            }
            line(d, t, (x, y, w), label, value);
            y += row;
        }
    }
    // remove, at the foot of the inspector
    let r = Rect { x, y: (a.y + a.h - row).max(y + pad), w: (w * 0.5).min(160.0), h: row };
    d.round_rect(r.x, r.y, r.w, r.h, t.size.radius, if r.has(pointer.0, pointer.1) { t.color.danger } else { t.color.control });
    d.text(r.x + pad, r.y + (r.h - font * 1.25) * 0.5, font, &fit(d, &insp.remove, font, r.w - 2.0 * pad), t.color.text);
    host.hits.push((r, Hit::RoleRemove));
}

/// A prompt asked now (a role's remove), over `a`: only its presses count.
pub fn ask(d: &mut DrawList, t: &Tokens, host: &mut Host, a: Rect) {
    let Some(p) = host.ask_prompt() else { return };
    host.hits.clear();
    let mut hits = vec![];
    prompt(d, t, &p, a, &mut hits, (Hit::AskYes, Hit::AskNo));
    host.hits.extend(hits);
}
