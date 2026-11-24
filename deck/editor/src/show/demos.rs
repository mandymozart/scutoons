//! The demos: shows with their rigs, an address book and the story events beside them (deck/demos; on a deck
//! /usr/share/deck/demos), so each plays without a server where it can. The Demos sheet in Show lists them with what each
//! first row says to try, a press opens one; `--demo NAME` opens one at the start.
use super::{values, Hit, Host};
use deck_kit::base::draw::DrawList;
use deck_kit::base::tokens::Tokens;
use deck_kit::editor::controls::{self, Rect};
use deck_show::{ClockChoice, Subtree};
use std::path::{Path, PathBuf};
use std::sync::Arc;

const SUFFIX: &str = ".show.json";

/// one show of the folder: its name (the file's, the show's id), the file, what its first row says to try
#[derive(Debug, Clone, PartialEq)]
pub struct Demo {
    pub name: String,
    pub file: PathBuf,
    pub hint: String,
}

/// `DECK_DEMOS`, else the deck's folder, else the source tree's
pub fn dir() -> Option<PathBuf> {
    if let Some(d) = std::env::var_os("DECK_DEMOS") {
        return Some(d.into());
    }
    [PathBuf::from("/usr/share/deck/demos"), PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../demos"))].into_iter().find(|d| d.is_dir())
}

/// the folder's shows by name; one that does not open says why in its hint
pub fn list(dir: &Path) -> Vec<Demo> {
    let Ok(entries) = std::fs::read_dir(dir) else { return Vec::new() };
    let mut out: Vec<Demo> = entries
        .filter_map(|e| {
            let file = e.ok()?.path();
            let name = file.file_name()?.to_str()?.strip_suffix(SUFFIX)?.to_string();
            let hint = match std::fs::read_to_string(&file).map_err(|e| e.to_string()).and_then(|t| deck_show::document::open(&t)) {
                Ok(s) => s.rows.first().map(|r| r.id.clone()).unwrap_or_default(),
                Err(e) => format!("does not open: {e}"),
            };
            Some(Demo { name, file, hint })
        })
        .collect();
    out.sort_by(|a, b| a.name.cmp(&b.name));
    out
}

impl Host {
    /// the Demos sheet opens: the folder read again
    pub(super) fn demos_open(&mut self) {
        self.demos = dir().map(|d| list(&d)).unwrap_or_default();
    }

    /// Opens demo `name` in place of the show: the conductor starts over on it, its rigs answer the cells' knobs, its book
    /// and story events replace the API's.
    pub fn open_demo(&mut self, name: &str) -> Result<(), String> {
        let dir = dir().ok_or("no demos folder (DECK_DEMOS, /usr/share/deck/demos)")?;
        let file = dir.join(format!("{name}{SUFFIX}"));
        let show = std::fs::read_to_string(&file).map_err(|e| format!("{}: {e}", file.display())).and_then(|t| deck_show::document::open(&t))?;
        let hint = show.rows.first().map(|r| r.id.clone()).unwrap_or_default();
        self.sub = Subtree::new(show, ClockChoice::Parent);
        self.library.clear();
        self.path.clear();
        (self.row, self.role, self.address, self.seat, self.sheet, self.context) = (0, None, None, None, None, None);
        self.cells.clear();
        self.knobs = values::Loader::new(self.api.base.clone());
        self.knobs.rigs = Some(dir.join("rigs")).filter(|r| r.is_dir());
        let book = dir.join("book.json");
        if let Ok(b) = std::fs::read_to_string(&book).map_err(|e| e.to_string()).and_then(|t| deck_space::rig::book::Book::parse(&t)) {
            self.toon = Some(b.toon().to_string());
            self.book = Some(Arc::new(b));
            self.demo_book = Some(book);
        }
        let story = dir.join("story-events.json");
        if story.is_file() {
            self.story_from_file(&story);
        }
        self.status = format!("demo {name} – {hint}");
        Ok(())
    }

    pub(super) fn demo_press(&mut self, i: usize) {
        let Some(name) = self.demos.get(i).map(|d| d.name.clone()) else { return };
        if let Err(e) = self.open_demo(&name) {
            self.refuse(e);
        }
    }
}

pub fn draw(d: &mut DrawList, t: &Tokens, host: &mut Host, a: Rect, pointer: (f32, f32)) {
    let (pad, font, small) = (t.size.pad, t.size.font, t.size.small);
    let mut y = a.y + pad;
    d.text(a.x + pad, y, small, "DEMOS – A PRESS OPENS ONE IN PLACE OF THE SHOW; ITS FIRST ROW SAYS WHAT TO TRY", t.color.muted);
    y += small * 1.8;
    d.rect(a.x, y - 4.0, a.w, 1.0, t.color.line);
    if host.demos.is_empty() {
        d.text(a.x + pad, y + 4.0, font, "No demos here: the deck's are in /usr/share/deck/demos, or set DECK_DEMOS to a folder.", t.color.muted);
        return;
    }
    let lh = font * 1.5 + small * 1.5 + pad * 0.5;
    let fit = |d: &DrawList, s: &str, px: f32| controls::fit(s, ((a.w - 2.0 * pad) / d.advance(px)).max(0.0) as usize);
    for (i, demo) in host.demos.clone().iter().enumerate() {
        if y + lh > a.y + a.h {
            break;
        }
        let whole = Rect { x: a.x, y, w: a.w, h: lh };
        if whole.has(pointer.0, pointer.1) {
            controls::list_row(d, t, whole, "hover");
        }
        d.text(a.x + pad, y + 4.0, font, &fit(d, &demo.name, font), t.color.text);
        d.text(a.x + pad, y + 4.0 + font * 1.5, small, &fit(d, &demo.hint, small), t.color.muted);
        d.rect(a.x, y + lh - 1.0, a.w, 1.0, t.color.line);
        host.hits.push((whole, Hit::Demo(i)));
        y += lh;
    }
}

#[cfg(test)]
mod tests {
    use super::{dir, list};
    use crate::show::Host;
    use deck_show::document::Cell;

    /// every demo opens, its pieces' knobs come from its rigs or the core's without a server, the story's questions are there
    #[test]
    fn every_demo_opens_and_its_rigs_answer() {
        let demos = list(&dir().expect("the source tree's demos"));
        assert!(demos.len() >= 4);
        for demo in &demos {
            assert!(demo.hint.starts_with("try: "), "{demo:?}");
            let mut host = Host::new(None, None, None, "http://127.0.0.1:9".into());
            host.open_demo(&demo.name).unwrap();
            assert_eq!(host.shown().id, demo.name);
            assert!(host.book.is_some() && matches!(host.story, Some(Ok(_))), "{}", demo.name);
            let pieces: Vec<String> = host.shown().rows.iter().flat_map(|r| r.cells.values()).filter_map(|c| match c {
                Cell::Play { piece: Some(p), .. } | Cell::Values { piece: p, .. } => Some(p.clone()),
                _ => None,
            }).collect();
            assert!(!pieces.is_empty(), "{}", demo.name);
            for piece in &pieces {
                assert!(matches!(host.knobs.now(piece), Some(Ok(_))), "{}: {piece}: {:?}", demo.name, host.knobs.now(piece).map(|a| a.err()));
            }
        }
    }

    #[test]
    fn the_sheet_lists_the_demos_and_a_press_opens_one() {
        let mut host = Host::new(None, None, None, "http://127.0.0.1:9".into());
        host.press(crate::show::Hit::Sheet(crate::show::Sheet::Demos));
        let i = host.demos.iter().position(|d| d.name == "far-nodes").expect("far-nodes is listed");
        host.press(crate::show::Hit::Demo(i));
        assert_eq!(host.shown().id, "far-nodes");
        assert_eq!(host.sheet, None);
        assert!(host.status.contains("Open in a tab"), "{}", host.status);
    }
}
