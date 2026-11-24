//! The editor's workspaces Scene · Rigging · Show (XR is the creator's only) on the creator's model (deck-ui `workspaces`, `stage`, `resume`):
//! which one is in front (the front tab's, else one opened empty), the row's use, each workspace's stage and its view
//! full screen, whether the switcher was open – kept in the editor's cache, so reopening resumes like a hibernated
//! computer; the very first open lands on Home, nothing open yet.
use crate::switcher::{self, Card, Press};
use crate::{bar, main_menu, show, App};
use deck_ui::resume::{self, Session};
use deck_ui::stage::{self, Cell, ViewState, ZoneSize};
use deck_ui::workspaces::{self, Click, Workspace};
use std::path::{Path, PathBuf};

/// where the session is kept
pub fn file(cache: &Path) -> PathBuf {
    cache.join("editor-session.json")
}

pub fn load(file: Option<&Path>) -> Session {
    Session::read(file.and_then(|f| std::fs::read_to_string(f).ok()).as_deref())
}

pub fn now_secs() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs())
}

/// this machine's name, as the resumed line says it
fn machine() -> Option<String> {
    std::env::var("COMPUTERNAME").or_else(|_| std::env::var("HOSTNAME")).ok().map(|n| n.to_lowercase()).filter(|n| !n.is_empty())
}

/// the drop zones' reach: a band along each edge, the corners along each edge
pub fn zones(t: &deck_kit::base::tokens::Tokens) -> ZoneSize {
    ZoneSize { band: t.size.rem(1.5), corner: t.size.rem(9.0) }
}

impl App {
    /// the workspace in front: the front tab's, else the one opened empty
    pub(crate) fn deck_ws(&self) -> Option<Workspace> {
        self.tabs.front().and_then(|t| Workspace::of(&t.key)).or(self.empty_ws)
    }

    /// the first frame, after the tabs: how the editor starts (Home, a workspace opened empty, resumed)
    pub(crate) fn start_session(&mut self) {
        let front = self.tabs.front().map(|t| t.key.clone());
        let st = resume::start(&self.session, front.as_deref());
        self.switcher = (st.dashboard && front.is_some()) || self.args.switcher;
        if front.is_none() {
            self.empty_ws = st.workspace;
        }
        if let Some(w) = self.args.empty_ws {
            self.empty_ws = Some(w);
            self.switcher = self.args.switcher;
        }
        if self.args.main_menu {
            self.main_menu = Some(if self.args.views_open { main_menu::Side::Views } else { main_menu::Side::None });
        }
        if st.resumed && self.session_file.is_some() {
            self.note(resume::resumed_words(&self.session, now_secs()));
        }
        // a check run's views on the stage and full screen
        if let Some(w) = self.deck_ws() {
            for (id, cell) in self.args.stage_views.clone() {
                let s = self.session.stage(w).place(&id, cell);
                self.session.set_stage(w, s);
            }
            if let Some(id) = self.args.full_view.clone() {
                self.session.set_full(w, Some(&id));
            }
        }
        self.ws_seen = (None, None);
    }

    /// Each frame: an open item clears the empty workspace, the row counts what came to the front, the workspace's panels
    /// take its stage (a panel dragged off it leaves it), and what changed is kept
    pub(crate) fn follow_session(&mut self) {
        if self.tabs.front.is_some() {
            self.empty_ws = None;
        }
        let ws = self.deck_ws();
        let key = self.tabs.front().map(|t| t.key.clone());
        let mut changed = false;
        if (ws, key.clone()) != self.ws_seen {
            if let Some(w) = ws {
                self.session.uses.entered(self.ws_seen.0, w, key.as_deref());
            }
            self.session.workspace = ws;
            self.ws_seen = (ws, key);
            changed = true;
        }
        let (stage, full) = ws.map_or_else(|| (stage::StageState::default(), None), |w| (self.session.stage(w), self.session.full.get(&w).cloned()));
        let p = self.panels_mut();
        p.stage = stage;
        p.full = full;
        if p.take_left_stage().is_some() {
            let (s, f) = (p.stage.clone(), p.full.clone());
            if let Some(w) = ws {
                self.session.set_stage(w, s);
                self.session.set_full(w, f.as_deref());
            }
            changed = true;
        }
        if self.session.dashboard != self.switcher {
            self.session.dashboard = self.switcher;
            changed = true;
        }
        if changed {
            self.keep_session();
        }
    }

    /// what a later start resumes (none in a check run)
    pub(crate) fn keep_session(&mut self) {
        let Some(f) = &self.session_file else { return };
        self.session.left_at = Some(now_secs());
        self.session.machine = machine();
        if let Ok(text) = serde_json::to_string_pretty(&self.session) {
            if let Err(e) = std::fs::write(f, text) {
                tracing::warn!("the session is not kept: {e}");
            }
        }
    }

    /// a workspace of the row or the switcher pressed
    pub(crate) fn workspace_click(&mut self, w: Workspace) {
        self.switcher = false;
        let front = self.tabs.front().map(|t| t.key.clone());
        let open: Vec<String> = self.tabs.list.iter().map(|t| t.key.clone()).collect();
        let open: Vec<&str> = open.iter().map(String::as_str).collect();
        match workspaces::click(&self.session.uses, front.as_deref(), &open, w) {
            Click::Keep => {}
            Click::Front(k) => {
                if let Some(i) = self.tabs.find(&k) {
                    self.activate(i);
                }
            }
            Click::Empty => {
                self.home();
                self.empty_ws = Some(w);
            }
        }
    }

    /// a view put in one of its four states, from the ☰ Views: a Show's sheet shows in the sheet panel
    pub(crate) fn view_to(&mut self, id: &str, state: ViewState) {
        let Some(w) = self.deck_ws() else { return };
        if let Some(&(_, _, _, sheet)) = show::author::SHEETS.iter().find(|s| s.0 == id).filter(|_| self.workspace == show::Workspace::Show) {
            let shown = self.show.sheet == Some(sheet);
            if state == ViewState::Hidden && !shown {
                return;
            }
            if state == ViewState::Hidden {
                self.show.sheet = None;
            } else if !shown {
                self.show.press(show::Hit::Sheet(sheet));
            }
            return self.view_to(show::author::SHEET_PANEL, state);
        }
        let p = self.panels_mut();
        p.set_view(id, state);
        let (s, f) = (p.stage.clone(), p.full.clone());
        self.session.set_stage(w, s);
        self.session.set_full(w, f.as_deref());
        self.panels_moved = true;
        self.keep_session();
    }

    /// a panel let go on a drop zone: it takes that cell of the stage
    pub(crate) fn drop_on_stage(&mut self, id: &str, cell: Cell) {
        let Some(w) = self.deck_ws() else { return };
        let s = self.session.stage(w).place(id, cell);
        self.session.set_stage(w, s);
        if self.session.full.get(&w).map(String::as_str) == Some(id) {
            self.session.set_full(w, None);
        }
        self.keep_session();
    }

    /// the drop zone under the pointer while a panel is in the hand
    pub(crate) fn zone_now(&self) -> Option<Cell> {
        self.panels().floats.moving()?;
        self.deck_ws()?;
        stage::zone_at(self.cursor.0, self.cursor.1, self.window_size(), zones(&self.tokens.tokens))
    }

    /// the switcher's deck cards: the row's order, the one in front framed, each with the item it shows
    pub(crate) fn switch_cards(&self) -> Vec<Card> {
        let front = self.deck_ws();
        self.session
            .uses
            .row()
            .into_iter()
            .map(|w| {
                let shown = self.tabs.front().map(|t| t.key.clone()).filter(|_| front == Some(w));
                let open = |k: &String| self.tabs.list.iter().any(|t| t.key == *k);
                let item = shown.or_else(|| self.session.uses.last.get(&w).filter(|k| open(k)).cloned()).or_else(|| self.tabs.list.iter().find(|t| Workspace::of(&t.key) == Some(w) && !t.key.ends_with(':')).map(|t| t.key.clone()));
                Card { workspace: w, front: front == Some(w), item }
            })
            .collect()
    }

    /// a press on the switcher (or Home): it closes
    pub(crate) fn switcher_press(&mut self, p: Press) {
        self.switcher = false;
        match p {
            Press::Workspace(w) => self.workspace_click(w),
            Press::Recent(i) => self.home_press(crate::home::Press::Recent(i)),
            Press::Creator(i) => {
                if let Some(c) = self.creator.cards.get(i).cloned() {
                    let url = format!("{}{}", crate::creator_cards::CREATOR, c.href);
                    match crate::home::open_in_browser(&url) {
                        Ok(()) if self.autosaves => self.note(format!("{} opens in the creator", c.item)),
                        _ => self.note(format!("{} is in the creator: {url}", c.item)),
                    }
                }
            }
        }
    }

    /// the switcher opened or closed (the row's ⌄, the strip's +); opened, the creator's cards are asked again
    pub(crate) fn open_switcher(&mut self) {
        self.main_menu = None;
        let over = self.tabs.front.is_some() || self.empty_ws.is_some();
        self.switcher = over && !self.switcher;
        if self.switcher || !over {
            self.creator.ask(&self.args.api, self.auth.token());
        }
    }

    /// a press while the ☰ menu is open (none: outside it – it closes); a view's state keeps it open
    pub(crate) fn main_menu_press(&mut self, p: Option<main_menu::Press>) {
        use main_menu::{Line, Press, Side};
        let side = self.main_menu.unwrap_or_default();
        let toggle = |s: Side| Some(if side == s { Side::None } else { s });
        match p {
            None => self.main_menu = None,
            Some(Press::Ground) => {}
            Some(Press::Line(Line::Views)) if self.deck_ws().is_some() => self.main_menu = toggle(Side::Views),
            Some(Press::Line(Line::Views)) => {}
            Some(Press::Line(Line::Do(bar::Action::New))) => self.main_menu = toggle(Side::New),
            Some(Press::View(id, s)) => self.view_to(id, s),
            Some(Press::New(n)) => {
                self.main_menu = None;
                self.home_press(crate::home::Press::New(n));
            }
            Some(Press::Line(Line::Do(a))) if self.menu_why_not(a).is_some() => {}
            Some(Press::Line(l)) => {
                self.main_menu = None;
                match l {
                    Line::SaveWorkspace => {
                        self.remember_panels();
                        self.keep_session();
                        self.note("the workspace is kept as it is now".to_string());
                    }
                    Line::Search => self.search(),
                    Line::Docs => self.home_press(crate::home::Press::Docs),
                    Line::Do(a) => self.menu_act(a),
                    Line::Views => {}
                }
            }
        }
    }

    /// Search: the Library's Find, its panel open, typing
    pub(crate) fn search(&mut self) {
        self.panels_mut().floats.reopen(crate::library::PANEL);
        self.panels_moved = true;
        let q = self.library.browser.filter.query.clone();
        self.library.browser.typing = Some(deck_kit::base::text::Field::with(&q));
    }

    /// what the switcher shows this frame, when it shows (nothing open: Home; else over the workspace)
    pub(crate) fn switch_data(&self) -> Option<SwitchData> {
        if !self.switcher_shows() {
            return None;
        }
        let (recent, _, _) = self.home_lines();
        let recent = recent.into_iter().map(|l| switcher::Recent { workspace: Workspace::of(&l.key), key: l.key, name: l.name }).collect();
        let creator = self.creator.cards.iter().map(crate::creator_cards::Card::shown).collect();
        Some(SwitchData { home: !self.switcher, recent, cards: self.switch_cards(), creator })
    }

    /// the switcher shows: opened (the row's ⌄, the strip's +), or Home – nothing in front, no workspace opened empty
    pub(crate) fn switcher_shows(&self) -> bool {
        self.switcher || (self.tabs.front.is_none() && self.empty_ws.is_none())
    }

    /// the ☰ menu's foot: the load in words
    pub(crate) fn load_words(&self) -> String {
        let cpu = self.cpu.as_ref().and_then(|c| c.reading()).map_or_else(|| "CPU –".to_string(), |c| format!("CPU {:.0} %", c.value * 100.0));
        let gpu = self.gpu_load.reading().map_or_else(|| "GPU –".to_string(), |g| format!("GPU {:.0} %", g.value * 100.0));
        let khz = self.stage.khz().map_or_else(|| "no audio".to_string(), |k| bar::khz(k as f32));
        format!("{cpu} · {khz} · {gpu} · {:.0} fps", self.fps())
    }

    /// the views the ☰ lists for the workspace in front: its panels, and in a Show its sheets in the sheet panel's place
    pub(crate) fn menu_views(&self) -> Vec<(&'static str, &'static str, &'static str, ViewState)> {
        if self.deck_ws().is_none() || self.workspace == show::Workspace::Home {
            return Vec::new();
        }
        let mut views = self.panels().views();
        if self.workspace == show::Workspace::Show {
            let sheet = self.panels().view_state(show::author::SHEET_PANEL);
            views.retain(|v| v.0 != show::author::SHEET_PANEL);
            for &(id, name, icon, s) in &show::author::SHEETS {
                views.push((id, name, icon, if self.show.sheet == Some(s) { sheet } else { ViewState::Hidden }));
            }
        }
        views
    }

    /// the line an empty workspace shows on its stage
    pub(crate) fn empty_words(&self) -> Option<String> {
        let w = self.empty_ws.filter(|_| self.tabs.front.is_none() && !self.switcher)?;
        let how = match w {
            Workspace::Scene => "open a scene (☰ Open) or start a new one (☰ New)",
            Workspace::Rigging => "open a character, an item or a module (☰ Open)",
            Workspace::Show => "open a show (☰ Open) or start a new one (☰ New)",
        };
        Some(format!("{}: nothing open yet – {how}.", w.word()))
    }
}

/// the switcher's content, gathered before the frame is drawn
pub struct SwitchData {
    /// nothing open: Home under the bar; else the switcher over the workspace
    pub home: bool,
    recent: Vec<switcher::Recent>,
    cards: Vec<Card>,
    creator: Vec<switcher::CreatorCard>,
}

impl SwitchData {
    pub fn draw(&self, d: &mut deck_kit::base::draw::DrawList, t: &deck_kit::base::tokens::Tokens, size: (f32, f32), pointer: (f32, f32)) -> Vec<(deck_kit::base::draw::Rect, Press)> {
        let top = if self.home { bar::bottom(t) } else { 0.0 };
        switcher::draw(d, t, size, top, &switcher::Switch { recent: &self.recent, cards: &self.cards, creator: &self.creator }, pointer)
    }
}

/// a check run's `--stage ID:CELL,…`: views on the stage (cells: whole left right top-left top-right bottom-left bottom-right)
pub fn parse_stage(text: &str) -> Vec<(String, Cell)> {
    text.split(',')
        .filter_map(|p| {
            let (id, cell) = p.split_once(':')?;
            let cell = match cell {
                "whole" => Cell::Whole,
                "left" => Cell::Left,
                "right" => Cell::Right,
                "top-left" => Cell::TopLeft,
                "top-right" => Cell::TopRight,
                "bottom-left" => Cell::BottomLeft,
                "bottom-right" => Cell::BottomRight,
                _ => return None,
            };
            Some((id.to_string(), cell))
        })
        .collect()
}

/// a check run's `--empty show`: a workspace opened empty
pub fn parse_ws(text: &str) -> Option<Workspace> {
    workspaces::ALL.into_iter().find(|w| w.word().eq_ignore_ascii_case(text))
}

/// what a place of the switcher does, for the info
pub fn help_of(p: Press) -> (&'static str, String) {
    match p {
        Press::Workspace(w) => (w.word(), format!("{} Click to go there: its last item, else it opens empty.", w.about())),
        Press::Recent(_) => ("Open", "Opens it in a tab, or brings its tab to the front.".into()),
        Press::Creator(_) => ("Creator", "A workspace of the creator: opens the creator there in the browser.".into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_check_run_names_its_views_on_the_stage_and_an_empty_workspace() {
        assert_eq!(parse_stage("mixer:left,info:top-right,x:nowhere"), vec![("mixer".to_string(), Cell::Left), ("info".to_string(), Cell::TopRight)]);
        assert_eq!(parse_ws("xr"), None, "XR is the creator's workspace, not the deck's");
        assert_eq!(parse_ws("Rigging"), Some(Workspace::Rigging));
        assert_eq!(parse_ws("perform"), None, "no workspace beyond the four");
    }

    #[test]
    fn the_session_file_comes_back() {
        let f = std::env::temp_dir().join(format!("deck-editor-session-{}.json", std::process::id()));
        let mut s = Session { workspace: Some(Workspace::Show), dashboard: true, ..Session::default() };
        s.set_stage(Workspace::Show, stage::StageState::default().add("rows"));
        std::fs::write(&f, serde_json::to_string(&s).unwrap()).unwrap();
        let back = load(Some(&f));
        std::fs::remove_file(&f).ok();
        assert_eq!(back, s);
        assert!(load(Some(Path::new("no/such/file.json"))).fresh());
    }
}
