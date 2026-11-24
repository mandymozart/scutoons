//! The small prompt a new scene starts with: its name and its id, the id suggested from the name until it is typed by hand.
//! The id's rule and the suggestion are the core's (scene_document.rs `id_problem`, `suggest_id`); a taken id is the editor's
//! scene list. Enter creates, Esc and Cancel open nothing.
use deck_kit::base::draw::DrawList;
use deck_kit::base::text::Field;
use deck_kit::base::tokens::Tokens;
use deck_kit::editor::controls::Rect;
use deck_space::scene_document::{id_problem, suggest_id};
use winit::keyboard::{Key, NamedKey};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Focus {
    Name,
    Id,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Part {
    Name,
    Id,
    Cancel,
    Create,
}

/// What a key or a press did to the prompt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    Nothing,
    /// the scene's id and name, both fine
    Create(String, String),
    Cancel,
}

pub struct ScenePrompt {
    name: Field,
    id: Field,
    focus: Focus,
    /// the id was typed by hand: the name no longer suggests it
    own_id: bool,
    /// the scene ids already there, bare (`nave`)
    taken: Vec<String>,
    hits: Vec<(Rect, Part)>,
}

impl ScenePrompt {
    pub fn new(taken: Vec<String>) -> ScenePrompt {
        let mut name = Field::with("");
        name.focused = true;
        ScenePrompt { name, id: Field::with(""), focus: Focus::Name, own_id: false, taken, hits: Vec::new() }
    }

    /// why the scene cannot be made yet (none = it can)
    pub fn problem(&self) -> Option<String> {
        if self.name.text.trim().is_empty() {
            return Some("the scene needs a name".into());
        }
        let id = self.id.text.trim();
        if id.is_empty() {
            return Some("the id is empty: the name gives nothing to suggest one from, type it".into());
        }
        if let Some(why) = id_problem(id) {
            return Some(why);
        }
        self.taken.iter().any(|t| t == id).then(|| format!("scene:{id} is taken: pick another id"))
    }

    fn field(&mut self) -> &mut Field {
        match self.focus {
            Focus::Name => &mut self.name,
            Focus::Id => &mut self.id,
        }
    }

    fn focus_on(&mut self, to: Focus) {
        self.focus = to;
        self.name.focused = to == Focus::Name;
        self.id.focused = to == Focus::Id;
    }

    fn create(&self) -> Outcome {
        match self.problem() {
            None => Outcome::Create(self.id.text.trim().to_string(), self.name.text.trim().to_string()),
            Some(_) => Outcome::Nothing,
        }
    }

    /// a key pressed: `text` is what it types
    pub fn key(&mut self, key: Key<&str>, text: Option<&str>, ctrl: bool, shift: bool) -> Outcome {
        match key {
            Key::Named(NamedKey::Escape) => return Outcome::Cancel,
            Key::Named(NamedKey::Enter) => return self.create(),
            Key::Named(NamedKey::Tab) => self.focus_on(if self.focus == Focus::Name { Focus::Id } else { Focus::Name }),
            Key::Named(k) => {
                let f = self.field();
                match k {
                    NamedKey::Backspace => f.backspace(),
                    NamedKey::Delete => f.delete(),
                    NamedKey::ArrowLeft => f.left(ctrl, shift),
                    NamedKey::ArrowRight => f.right(ctrl, shift),
                    NamedKey::Home => f.home(shift),
                    NamedKey::End => f.end(shift),
                    _ => return Outcome::Nothing,
                }
            }
            Key::Character(c) if ctrl && c.eq_ignore_ascii_case("a") => self.field().select_all(),
            _ => {
                let Some(t) = text.filter(|t| !ctrl && !t.chars().any(char::is_control)) else { return Outcome::Nothing };
                self.field().insert(t);
            }
        }
        self.follow();
        Outcome::Nothing
    }

    /// the id follows the name until it was typed by hand
    fn follow(&mut self) {
        match self.focus {
            Focus::Id => self.own_id = !self.id.text.is_empty(),
            Focus::Name if !self.own_id => self.id = Field::with(&suggest_id(&self.name.text)),
            Focus::Name => {}
        }
        if self.focus == Focus::Id && self.id.text.is_empty() {
            self.id = Field::with(&suggest_id(&self.name.text));
            self.id.focused = true;
        }
    }

    pub fn press(&mut self, x: f32, y: f32) -> Outcome {
        let Some(part) = self.hits.iter().find(|(r, _)| r.has(x, y)).map(|(_, p)| *p) else { return Outcome::Nothing };
        match part {
            Part::Name => self.focus_on(Focus::Name),
            Part::Id => self.focus_on(Focus::Id),
            Part::Cancel => return Outcome::Cancel,
            Part::Create => return self.create(),
        }
        Outcome::Nothing
    }

    /// the prompt over the whole editor, a card in the middle
    pub fn draw(&mut self, d: &mut DrawList, t: &Tokens, room: (f32, f32)) {
        let (pad, font, small, row) = (t.size.pad, t.size.font, t.size.small, t.size.tree_row);
        self.hits.clear();
        // on the overlay level: its ground is the editor's (main.rs, the still or the reduced ground)
        let w = (font * 32.0).min(room.0 - 2.0 * pad);
        let h = small * 1.8 + row * 2.0 + row + pad * 3.0 + (t.size.row - 4.0);
        let (x, y) = ((room.0 - w) * 0.5, ((room.1 - h) * 0.4).max(pad));
        d.round_rect(x, y, w, h, t.size.radius, t.color.panel());
        d.outline(x, y, w, h, (t.size.radius, 1.0), t.color.line);
        let (x, w) = (x + pad, w - 2.0 * pad);
        let mut y = y + pad;
        d.text(x, y, small, "NEW SCENE", t.color.muted);
        y += small * 1.8;
        for (label, f, part, on) in [("name", &self.name, Part::Name, self.focus == Focus::Name), ("id", &self.id, Part::Id, self.focus == Focus::Id)] {
            d.text(x, y + (row - font * 1.25) * 0.5, font, label, t.color.muted);
            let r = Rect { x: x + w * 0.2, y: y + 2.0, w: w * 0.8, h: row - 4.0 };
            d.round_rect(r.x, r.y, r.w, r.h, t.size.radius, t.color.control);
            if on {
                d.outline(r.x, r.y, r.w, r.h, (t.size.radius, 1.0), t.color.text);
            }
            let shown = if on { format!("{}|", f.text) } else { f.text.clone() };
            let shown = if shown.is_empty() && !on && part == Part::Id { "suggested from the name".to_string() } else { shown };
            let ink = if f.text.is_empty() && !on { t.color.muted } else { t.color.text };
            d.text(r.x + 6.0, r.y + (r.h - font * 1.25) * 0.5, font, &deck_kit::editor::controls::fit(&shown, ((r.w - 12.0) / d.advance(font)).max(0.0) as usize), ink);
            self.hits.push((r, part));
            y += row;
        }
        let said = self.problem().filter(|_| !self.name.text.is_empty() || !self.id.text.is_empty());
        let fit = |d: &DrawList, s: &str| deck_kit::editor::controls::fit(s, (w / d.advance(small)).max(0.0) as usize);
        match &said {
            Some(p) => d.text(x, y + 2.0, small, &fit(d, p), t.color.warn),
            None => d.text(x, y + 2.0, small, "Enter creates it, Esc cancels", t.color.muted),
        };
        y += row + pad;
        let bh = t.size.row - 4.0;
        let bw = d.advance(font * 1.3) * 6.0 + 2.0 * pad;
        let ok = said.is_none() && !self.name.text.trim().is_empty();
        let create = Rect { x: x + w - bw, y, w: bw, h: bh };
        let cancel = Rect { x: create.x - bw - pad * 0.5, y, w: bw, h: bh };
        crate::show::live::button(d, t, cancel, "Cancel", t.color.control, t.color.text);
        crate::show::live::button(d, t, create, "Create", if ok { t.color.pressed } else { t.color.control }, if ok { t.color.pressed_text } else { t.color.muted });
        self.hits.push((cancel, Part::Cancel));
        self.hits.push((create, Part::Create));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn type_in(p: &mut ScenePrompt, s: &str) {
        for c in s.chars() {
            let t = c.to_string();
            p.key(Key::Character(t.as_str()), Some(&t), false, false);
        }
    }

    impl ScenePrompt {
        fn name(&self) -> &str {
            &self.name.text
        }

        fn id(&self) -> &str {
            &self.id.text
        }
    }

    fn prompt() -> ScenePrompt {
        ScenePrompt::new(vec!["nave".into()])
    }

    #[test]
    fn the_id_is_suggested_from_the_name_until_it_is_typed() {
        let mut p = prompt();
        type_in(&mut p, "Night Nave");
        assert_eq!((p.name(), p.id()), ("Night Nave", "night-nave"));
        p.key(Key::Named(NamedKey::Tab), None, false, false);
        type_in(&mut p, "x");
        assert_eq!(p.id(), "night-navex", "typed by hand now");
        p.key(Key::Named(NamedKey::Tab), None, false, false);
        type_in(&mut p, "!");
        assert_eq!(p.id(), "night-navex", "the name no longer moves it");
    }

    #[test]
    fn a_bad_empty_or_taken_id_is_refused_with_a_reason() {
        let mut p = prompt();
        assert_eq!(p.key(Key::Named(NamedKey::Enter), None, false, false), Outcome::Nothing);
        assert_eq!(p.problem().as_deref(), Some("the scene needs a name"));
        type_in(&mut p, "Nave");
        assert_eq!(p.id(), "nave");
        assert_eq!(p.problem().as_deref(), Some("scene:nave is taken: pick another id"));
        assert_eq!(p.key(Key::Named(NamedKey::Enter), None, false, false), Outcome::Nothing, "a taken id creates nothing");
        p.key(Key::Named(NamedKey::Tab), None, false, false);
        type_in(&mut p, "A");
        assert!(p.problem().unwrap().contains("no scene id"), "{:?}", p.problem());
        p.key(Key::Named(NamedKey::Backspace), None, false, false);
        p.key(Key::Named(NamedKey::Backspace), None, false, false);
        type_in(&mut p, "-two");
        assert_eq!(p.problem(), None);
        assert_eq!(p.key(Key::Named(NamedKey::Enter), None, false, false), Outcome::Create("nav-two".into(), "Nave".into()));
    }

    #[test]
    fn a_name_with_nothing_to_suggest_from_asks_for_an_id() {
        let mut p = prompt();
        type_in(&mut p, "???");
        assert!(p.problem().unwrap().starts_with("the id is empty"));
    }

    #[test]
    fn escape_and_cancel_open_nothing() {
        let mut p = prompt();
        type_in(&mut p, "Duet");
        assert_eq!(p.key(Key::Named(NamedKey::Escape), None, false, false), Outcome::Cancel);
        p.hits.push((Rect { x: 0.0, y: 0.0, w: 10.0, h: 10.0 }, Part::Cancel));
        assert_eq!(p.press(5.0, 5.0), Outcome::Cancel);
    }
}
