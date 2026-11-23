//! The editor's workspaces Scene · Rigging · Show (XR is the creator's only) on the creator's model (deck-ui `workspaces`, `stage`, `resume`):
//! which one is in front (the front tab's, else one opened empty), the row's use, each workspace's stage and its view
//! full screen, the dashboard – kept in the editor's cache, so reopening resumes like a hibernated computer; the very
//! first open lands on the dashboard, nothing open yet.
use crate::dashboard::{self, Card, Press};
use crate::{bar, home, show, App};
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

    /// the first frame, after the tabs: how the editor starts (the dashboard, a workspace opened empty, resumed)
    pub(crate) fn start_session(&mut self) {
        let front = self.tabs.front().map(|t| t.key.clone());
        let st = resume::start(&self.session, front.as_deref());
        self.dashboard = st.dashboard || self.args.main_menu;
        self.welcome = st.welcome;
        if front.is_none() {
            self.empty_ws = st.workspace;
        }
        if let Some(w) = self.args.empty_ws {
            self.empty_ws = Some(w);
            self.dashboard = self.args.main_menu;
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
        if self.session.dashboard != self.dashboard {
            self.session.dashboard = self.dashboard;
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

    /// a workspace of the row or the dashboard pressed
    pub(crate) fn workspace_click(&mut self, w: Workspace) {
        self.dashboard = false;
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

    /// a panel put in one of its four states, from the dashboard's Views
    pub(crate) fn view_to(&mut self, id: &str, state: ViewState) {
        let Some(w) = self.deck_ws() else { return };
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

    /// the dashboard's cards: the front workspace first, then the row's order
    pub(crate) fn dash_cards(&self) -> Vec<Card> {
        let front = self.deck_ws();
        let mut row = self.session.uses.row();
        if let Some(f) = front {
            row.retain(|w| *w != f);
            row.insert(0, f);
        }
        row.into_iter()
            .map(|w| {
                let mut open: Vec<String> = self.tabs.list.iter().filter(|t| Workspace::of(&t.key) == Some(w) && !t.key.ends_with(':')).map(|t| t.name.clone()).collect();
                if let Some(i) = self.tabs.front.filter(|_| front == Some(w)) {
                    let name = self.tabs.list[i].name.clone();
                    open.retain(|n| *n != name);
                    open.insert(0, name);
                }
                Card { workspace: w, front: front == Some(w), open }
            })
            .collect()
    }

    /// a press on the dashboard (it closes, unless the press changed a view)
    pub(crate) fn dashboard_press(&mut self, p: Press) {
        match p {
            Press::Workspace(w) => self.workspace_click(w),
            Press::Home(h) => {
                self.dashboard = false;
                self.home_press(h);
            }
            Press::Menu(a) if self.menu_why_not(a).is_some() => {}
            Press::Menu(a) => {
                self.dashboard = false;
                self.menu_act(a);
            }
            Press::View(id, s) => self.view_to(id, s),
            Press::Load | Press::Card => {}
        }
    }

    /// what the dashboard shows this frame (the views are the panels of the workspace in front)
    pub(crate) fn dash_words(&self) -> (String, String) {
        let (_, _, deck) = self.home_lines();
        let account = deck.first().cloned().unwrap_or_default();
        let cpu = self.cpu.as_ref().and_then(|c| c.reading()).map_or_else(|| "CPU –".to_string(), |c| format!("CPU {:.0} %", c.value * 100.0));
        let gpu = self.gpu_load.reading().map_or_else(|| "GPU –".to_string(), |g| format!("GPU {:.0} %", g.value * 100.0));
        let khz = self.stage.khz().map_or_else(|| "no audio".to_string(), |k| bar::khz(k as f32));
        (account, format!("{cpu} · {khz} · {gpu} · {:.0} fps", self.fps()))
    }

    /// what the dashboard shows this frame, when it shows (nothing open: it is the page; else over the workspace)
    pub(crate) fn dash_data(&self) -> Option<DashData> {
        if !self.dashboard_shows() {
            return None;
        }
        let (recent, library, _) = self.home_lines();
        let why = library.is_empty().then(|| self.book_why().unwrap_or_else(|| "the library comes with a toon: sign in, or open a character of one".into()));
        let menu = bar::MENU.iter().map(|(a, ..)| (*a, self.menu_why_not(*a).is_none())).collect();
        let ws = self.deck_ws();
        let views = if ws.is_some() && self.workspace != show::Workspace::Home { self.panels().views() } else { Vec::new() };
        let (account, load) = self.dash_words();
        Some(DashData { welcome: self.welcome && self.tabs.list.is_empty(), cards: self.dash_cards(), recent, library, why, menu, ws, views, account, load })
    }

    /// the dashboard shows: opened by the menu, or nothing in front and no workspace opened empty
    pub(crate) fn dashboard_shows(&self) -> bool {
        self.dashboard || (self.tabs.front.is_none() && self.empty_ws.is_none())
    }

    /// the line an empty workspace shows on its stage
    pub(crate) fn empty_words(&self) -> Option<String> {
        let w = self.empty_ws.filter(|_| self.tabs.front.is_none() && !self.dashboard)?;
        let how = match w {
            Workspace::Scene => "open a scene from the dashboard (the menu, far right) or start a new one",
            Workspace::Rigging => "open a character, an item or a module from the dashboard",
            Workspace::Show => "open a show from the dashboard or start a new one",
        };
        Some(format!("{}: nothing open yet – {how}.", w.word()))
    }
}

/// the dashboard's content, gathered before the frame is drawn
pub struct DashData {
    welcome: bool,
    cards: Vec<Card>,
    recent: Vec<home::Line>,
    library: Vec<home::Line>,
    why: Option<String>,
    menu: Vec<(bar::Action, bool)>,
    ws: Option<Workspace>,
    views: Vec<(&'static str, &'static str, &'static str, ViewState)>,
    account: String,
    load: String,
}

impl DashData {
    pub fn draw(&self, d: &mut deck_kit::base::draw::DrawList, t: &deck_kit::base::tokens::Tokens, size: (f32, f32), pointer: (f32, f32)) -> Vec<(deck_kit::base::draw::Rect, Press)> {
        let dash = dashboard::Dash { welcome: self.welcome, cards: &self.cards, recent: &self.recent, library: &self.library, library_why: self.why.as_deref(), menu: &self.menu, workspace: self.ws, views: &self.views, account: &self.account, load: &self.load };
        dashboard::draw(d, t, size, bar::bottom(t), &dash, pointer)
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

/// the presses home.rs's lines map to, for the info
pub fn help_of(p: Press) -> (&'static str, String) {
    match p {
        Press::Workspace(w) => (w.word(), format!("{} Click to go there: its last item, else it opens empty.", w.about())),
        Press::Home(home::Press::New(_)) => ("New", "A new scene (in a show's composer), a module (from one of the catalog's) or a show, in a tab of its own.".into()),
        Press::Home(home::Press::Docs) => ("Help", "Opens the creator's docs, or copies the link.".into()),
        Press::Home(_) => ("Open", "Opens it in a tab, or brings its tab to the front.".into()),
        Press::Menu(a) => (bar::MENU.iter().find(|(m, ..)| *m == a).map_or("", |(_, l, _)| *l), "Click to do it.".into()),
        Press::View(_, s) => (s.word(), "The view's state: hidden, a floating panel, on the stage (it takes a cell; drag a panel to a screen edge or corner for one) or full screen.".into()),
        Press::Load => ("Load", String::new()),
        Press::Card => ("Dashboard", "The workspaces, anything to open, the menu and the views; the account, the help, the build and the load at the bottom.".into()),
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
