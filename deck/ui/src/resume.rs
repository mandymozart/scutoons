//! Reopening resumes like a hibernated computer (the creator's resume.ts, the same rules): what the open tabs and the
//! panels' places don't keep – the workspace in front, the dashboard, each workspace's stage and its view full screen,
//! the workspaces' use – kept with the editor, and how it starts. The very first open (nothing ever done) lands on the
//! dashboard, nothing open yet; someone known with nothing open lands on the dashboard; anything open comes back
//! exactly as it was left, no start page between.
use crate::stage::StageState;
use crate::workspaces::{Use, Workspace};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// a new shape takes a new version: an old one reads as nothing kept
pub const VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Session {
    pub v: u32,
    /// the workspace in front (also when it is open empty)
    #[serde(default)]
    pub workspace: Option<Workspace>,
    /// the dashboard was open
    #[serde(default)]
    pub dashboard: bool,
    /// per workspace: the views on its stage, the view full screen
    #[serde(default)]
    pub stages: BTreeMap<Workspace, StageState>,
    #[serde(default)]
    pub full: BTreeMap<Workspace, String>,
    #[serde(default)]
    pub uses: Use,
    /// when and where it was left (seconds since 1970; the machine's name)
    #[serde(default)]
    pub left_at: Option<u64>,
    #[serde(default)]
    pub machine: Option<String>,
}

impl Default for Session {
    fn default() -> Session {
        Session { v: VERSION, workspace: None, dashboard: false, stages: BTreeMap::new(), full: BTreeMap::new(), uses: Use::default(), left_at: None, machine: None }
    }
}

impl Session {
    /// what was kept, checked: another version or a broken file reads as nothing kept; a workspace the deck does not
    /// have (XR, kept before it became the creator's only) is left out and the rest comes back
    pub fn read(text: Option<&str>) -> Session {
        let Some(mut v) = text.and_then(|t| serde_json::from_str::<serde_json::Value>(t).ok()) else { return Session::default() };
        drop_unknown(&mut v);
        serde_json::from_value::<Session>(v).ok().filter(|s| s.v == VERSION).unwrap_or_default()
    }

    /// nothing ever done here
    pub fn fresh(&self) -> bool {
        self.left_at.is_none() && self.uses.fresh()
    }

    pub fn stage(&self, w: Workspace) -> StageState {
        self.stages.get(&w).cloned().unwrap_or_default()
    }

    pub fn set_stage(&mut self, w: Workspace, s: StageState) {
        if s.views().is_empty() {
            self.stages.remove(&w);
        } else {
            self.stages.insert(w, s);
        }
    }

    pub fn set_full(&mut self, w: Workspace, view: Option<&str>) {
        match view {
            Some(v) => self.full.insert(w, v.to_string()),
            None => self.full.remove(&w),
        };
    }
}

/// a workspace's name the deck knows
fn known(name: &str) -> bool {
    serde_json::from_value::<Workspace>(serde_json::Value::String(name.to_string())).is_ok()
}

/// the workspaces the deck does not know, out of a kept session: as the front one, as keys, in the use
fn drop_unknown(v: &mut serde_json::Value) {
    use serde_json::Value;
    let Some(o) = v.as_object_mut() else { return };
    if o.get("workspace").and_then(Value::as_str).is_some_and(|w| !known(w)) {
        o.insert("workspace".into(), Value::Null);
    }
    let keyed = |m: Option<&mut Value>| {
        if let Some(m) = m.and_then(Value::as_object_mut) {
            m.retain(|k, _| known(k));
        }
    };
    keyed(o.get_mut("stages"));
    keyed(o.get_mut("full"));
    if let Some(u) = o.get_mut("uses").and_then(Value::as_object_mut) {
        keyed(u.get_mut("uses"));
        keyed(u.get_mut("last"));
    }
}

/// how the editor starts
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Start {
    /// the dashboard shows
    pub dashboard: bool,
    /// its welcome: nothing open yet, nothing ever done
    pub welcome: bool,
    /// the workspace in front (none: the front tab's, or none at all)
    pub workspace: Option<Workspace>,
    /// it resumed where it was left: say so once
    pub resumed: bool,
}

/// The start from what was kept and whether a tab comes back in front (`front`: its key)
pub fn start(s: &Session, front: Option<&str>) -> Start {
    if s.fresh() && front.is_none() {
        return Start { dashboard: true, welcome: true, workspace: None, resumed: false };
    }
    let workspace = front.and_then(Workspace::of).or(s.workspace);
    if front.is_none() && s.workspace.is_none() {
        return Start { dashboard: true, welcome: false, workspace: None, resumed: s.left_at.is_some() };
    }
    Start { dashboard: s.dashboard, welcome: false, workspace, resumed: s.left_at.is_some() }
}

/// how long ago, in few words: just now · 5 min · 3 h · 2 days
pub fn ago(then: u64, now: u64) -> String {
    let m = now.saturating_sub(then) / 60;
    match m {
        0 => "just now".into(),
        1..=59 => format!("{m} min ago"),
        60..=1439 => format!("{} h ago", m / 60),
        _ if m < 2880 => "yesterday".into(),
        _ => format!("{} days ago", m / 1440),
    }
}

/// the status line a resumed start says once
pub fn resumed_words(s: &Session, now: u64) -> String {
    let when = s.left_at.map(|t| format!(" {}", ago(t, now))).unwrap_or_default();
    let place = s.machine.as_deref().filter(|m| !m.is_empty()).map(|m| format!(" on {m}")).unwrap_or_default();
    format!("Resumed – exactly as you left it{when}{place}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_very_first_open_is_the_dashboard_nothing_open_yet() {
        let st = start(&Session::default(), None);
        assert_eq!(st, Start { dashboard: true, welcome: true, workspace: None, resumed: false });
        assert!(Session::read(None).fresh());
        assert!(Session::read(Some("{\"v\":7}")).fresh(), "another version: nothing kept");
        assert!(Session::read(Some("not json")).fresh());
    }

    #[test]
    fn someone_known_with_nothing_open_lands_on_the_dashboard() {
        let mut s = Session { left_at: Some(100), ..Session::default() };
        s.uses.entered(None, Workspace::Scene, Some("scene:nave"));
        let st = start(&s, None);
        assert!(st.dashboard && !st.welcome && st.resumed);
    }

    #[test]
    fn an_open_tab_resumes_exactly_with_its_workspace_and_stage() {
        let mut s = Session { left_at: Some(100), machine: Some("studio-pc".into()), dashboard: false, workspace: Some(Workspace::Rigging), ..Session::default() };
        s.set_stage(Workspace::Rigging, StageState::default().add("mixer"));
        s.set_full(Workspace::Show, Some("rows"));
        let json = serde_json::to_string(&s).unwrap();
        let back = Session::read(Some(&json));
        assert_eq!(back, s);
        let st = start(&back, Some("character:catpope"));
        assert_eq!(st, Start { dashboard: false, welcome: false, workspace: Some(Workspace::Rigging), resumed: true });
        assert!(back.stage(Workspace::Rigging).has("mixer"));
        assert_eq!(back.full.get(&Workspace::Show).map(String::as_str), Some("rows"));
        let words = resumed_words(&back, 100 + 9 * 3600);
        assert_eq!(words, "Resumed – exactly as you left it 9 h ago on studio-pc");
    }

    #[test]
    fn an_empty_workspace_left_in_front_comes_back_empty() {
        let s = Session { left_at: Some(1), workspace: Some(Workspace::Show), ..Session::default() };
        let st = start(&s, None);
        assert_eq!((st.dashboard, st.workspace), (false, Some(Workspace::Show)));
    }

    #[test]
    fn a_session_kept_with_xr_loads_without_it() {
        let old = r#"{"v":1,"workspace":"xr","dashboard":false,"stages":{"xr":{"layout":"one","slots":["layers"]},"show":{"layout":"one","slots":["rows"]}},"full":{"xr":"layers"},"uses":{"uses":{"xr":4,"scene":2},"last":{"xr":"xr:poster","scene":"scene:nave"}},"left_at":5}"#;
        let s = Session::read(Some(old));
        assert_eq!(s.workspace, None, "XR in front: none – the dashboard");
        assert!(s.stage(Workspace::Show).has("rows") && s.full.is_empty());
        assert_eq!((s.uses.uses.get(&Workspace::Scene), s.uses.uses.len()), (Some(&2), 1));
        assert_eq!(s.uses.last.get(&Workspace::Scene).map(String::as_str), Some("scene:nave"));
        assert_eq!(s.left_at, Some(5), "the rest comes back");
        assert!(start(&s, None).dashboard);
    }

    #[test]
    fn ago_in_few_words() {
        assert_eq!(ago(0, 30), "just now");
        assert_eq!(ago(0, 300), "5 min ago");
        assert_eq!(ago(0, 3 * 3600), "3 h ago");
        assert_eq!(ago(0, 30 * 3600), "yesterday");
        assert_eq!(ago(0, 5 * 86400), "5 days ago");
    }
}
