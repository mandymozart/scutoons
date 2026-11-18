//! Build for a character without a rig: its empty rig plays nothing (the core's, `Bundle::empty`), and the inspector
//! offers the two ways to start one – tracks from the catalog's modules and instruments (Add, adding.rs), or a new module
//! for the character in the Module workspace. Save then makes `rig:<subject>` a draft like any rig's.
use deck_kit::base::draw::DrawList;
use deck_kit::base::tokens::Tokens;
use deck_kit::editor::controls::{self, Rect};
use deck_space::rig::Bundle;
use serde_json::{json, Value};

/// what a new module starts as: the SDK's worked example
const STARTER: &str = include_str!("../../modules/walk/src/lib.rs");

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Way {
    Modules,
    NewModule,
}

/// the rig open has nothing in it yet
pub fn is_empty(b: &Bundle) -> bool {
    b.rig.tracks.is_empty()
}

/// the character of the rig open, if it is a character's
pub fn character(b: &Bundle) -> Option<&str> {
    crate::toon::character(&b.rig.subject).map(|c| c.split('.').next().unwrap_or(c))
}

/// what the empty state says and the labels of its two ways
pub fn words(b: &Bundle) -> (Vec<String>, [String; 2]) {
    let s = &b.rig.subject;
    let mut lines = vec![format!("{s} has no rig yet – it is silent on every deck."), "Build one from the catalog's modules and instruments, or write a module of its own.".into(), format!("It saves itself as rig:{s}'s draft.")];
    if b.stands_as() == Some(deck_space::cast::Form::Card) {
        lines.push("The stage shows its 2D picture: there is no 3D model for it yet.".into());
    }
    let new = match character(b) {
        Some(c) => format!("New module for {c}"),
        None => "New module".into(),
    };
    (lines, ["Start from modules".into(), new])
}

/// a new module's first draft for `character`: the starter's code, its module named after the character
pub fn new_module(character: &str) -> (String, Value) {
    let code = STARTER.replacen("name = \"walk\"", &format!("name = \"{character}\""), 1);
    (format!("module:{character}"), json!({ "id": character, "name": character, "files": { scutoons_build::files::MAIN_FILE: code } }))
}

/// the empty state in the inspector's `area`: its lines and the two ways, where they landed
pub fn draw(d: &mut DrawList, t: &Tokens, area: Rect, b: &Bundle) -> Vec<(Rect, Way)> {
    let (lines, ways) = words(b);
    let mut y = area.y;
    y += controls::section(d, t, area.x, y, area.w, "NO RIG", true);
    let small = t.size.font * 0.85;
    let room = (area.w / d.advance(small)).max(8.0) as usize;
    for l in &lines {
        for part in wrap(l, room) {
            d.text(area.x, y, small, &part, t.color.muted);
            y += small * 1.5;
        }
    }
    y += t.size.row * 0.25;
    let mut out = Vec::new();
    for (way, label) in [Way::Modules, Way::NewModule].into_iter().zip(ways.iter()) {
        if let Some(r) = controls::pills(d, t, area.x, y, t.size.row - 4.0, &[label.as_str()], usize::MAX).into_iter().next() {
            out.push((r, way));
        }
        y += t.size.row;
    }
    out
}

impl crate::App {
    /// a way chosen: Add at the rig with the catalog's modules, or the character's new module in the Module workspace
    pub(crate) fn start_rig(&mut self, way: Way, x: f32, y: f32) {
        match way {
            Way::Modules => {
                self.sel = crate::inspector::Sel::Rig;
                self.open_add(x, y);
                let refused = self.rigs.modules().1.len();
                let note = match self.rigs.catalog() {
                    None => Some("no catalog from the server yet – the rig picker says why".to_string()),
                    Some(_) if refused > 0 => Some(format!("{refused} modules of the catalog cannot be a track yet (not built, or nothing to send)")),
                    Some(_) => None,
                };
                if let (Some(m), Some(n)) = (self.menu.take(), note) {
                    self.menu = Some(crate::Menu { ta: m.ta.with_note(n), ..m });
                }
            }
            Way::NewModule => {
                let Some(c) = character(&self.stage.bundle).map(str::to_string) else { return self.refuse("only a character's rig gets a module of its own".into(), x, y) };
                let (key, content) = new_module(&c);
                self.module.create(&key, content);
                self.workspace = crate::show::Workspace::Module;
            }
        }
    }
}

/// `text` in lines of at most `room` characters, broken between words
fn wrap(text: &str, room: usize) -> Vec<String> {
    let mut out = vec![String::new()];
    for w in text.split(' ') {
        let line = out.last_mut().expect("one line");
        if !line.is_empty() && line.chars().count() + 1 + w.chars().count() > room {
            out.push(w.to_string());
        } else {
            if !line.is_empty() {
                line.push(' ');
            }
            line.push_str(w);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// the panel model: a character's empty rig says it is silent and offers the two ways by its name
    #[test]
    fn a_character_without_a_rig_offers_two_ways() {
        let b = Bundle::empty("character.yellowbubu");
        assert!(is_empty(&b));
        let (lines, ways) = words(&b);
        assert_eq!(lines[0], "character.yellowbubu has no rig yet – it is silent on every deck.");
        assert_eq!(lines[2], "It saves itself as rig:character.yellowbubu's draft.");
        assert_eq!(lines[3], "The stage shows its 2D picture: there is no 3D model for it yet.");
        assert_eq!(words(&Bundle::empty("item.lamp")).0.len(), 3, "an item's rig stands as nothing");
        assert_eq!(ways, ["Start from modules".to_string(), "New module for yellowbubu".to_string()]);
        assert_eq!(words(&Bundle::empty("item.lamp")).1[1], "New module");
        assert!(!is_empty(&Bundle::core("character.catpope").expect("a core rig").expect("it reads")));
    }

    #[test]
    fn a_new_module_is_the_starter_named_after_the_character() {
        let (key, doc) = new_module("yellowbubu");
        assert_eq!((key.as_str(), doc["id"].as_str(), doc["name"].as_str()), ("module:yellowbubu", Some("yellowbubu"), Some("yellowbubu")));
        let code = doc["files"]["lib.rs"].as_str().expect("its code");
        assert!(code.contains("#[module(name = \"yellowbubu\"") && !code.contains("name = \"walk\""), "the module's own name");
        assert!(scutoons_build::files::check(&serde_json::from_value(doc["files"].clone()).expect("files")).is_empty(), "the build service takes it");
    }

    #[test]
    fn long_lines_break_between_words() {
        assert_eq!(wrap("one two three", 7), ["one two", "three"]);
        assert_eq!(wrap("short", 40), ["short"]);
    }
}
