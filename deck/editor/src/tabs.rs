//! The editor's tabs (decisions/deck.md [workspaces]): the items open – a scene (the map), an asset on its own stage (a
//! character with its rig, an item, an emotion, a module, a medium), a show – each at most once. Only the front tab runs;
//! the others keep what they had, frozen (`kept`, the editor's own state of the item). None open is Home. The tabs, the
//! front one and the recent items are remembered, so a later start reopens them; a small device keeps fewer tabs, the
//! least recently used going to Recent.
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// what a tab holds, by its address's first word
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Scene,
    Character,
    Item,
    Emotion,
    Module,
    Media,
    Show,
}

/// the three kinds of view a tab opens in
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum View {
    /// the map: holders, camera, shots, parallax, events
    Scene,
    /// one entity on an isolated stage
    Asset,
    Show,
}

impl Kind {
    /// `scene:nave`, `character:catpope`, `module:analog`, `show:duet` …
    pub fn of(key: &str) -> Option<Kind> {
        let (word, _) = key.split_once(':')?;
        Some(match word {
            "scene" => Kind::Scene,
            "character" => Kind::Character,
            "item" => Kind::Item,
            "emotion" => Kind::Emotion,
            "module" => Kind::Module,
            "video" | "audio" | "image" | "model" | "media" => Kind::Media,
            "show" => Kind::Show,
            _ => return None,
        })
    }

    pub fn icon(self) -> &'static str {
        match self {
            Kind::Scene => "map",
            Kind::Character => "user",
            Kind::Item => "package",
            Kind::Emotion => "smile",
            Kind::Module => "code",
            Kind::Media => "video",
            Kind::Show => "list",
        }
    }

    pub fn view(self) -> View {
        match self {
            Kind::Scene => View::Scene,
            Kind::Show => View::Show,
            _ => View::Asset,
        }
    }

    pub fn word(self) -> &'static str {
        match self {
            Kind::Scene => "scene",
            Kind::Character => "character",
            Kind::Item => "item",
            Kind::Emotion => "emotion",
            Kind::Module => "module",
            Kind::Media => "medium",
            Kind::Show => "show",
        }
    }
}

impl View {
    pub fn word(self) -> &'static str {
        match self {
            View::Scene => "Scene",
            View::Asset => "Asset",
            View::Show => "Show",
        }
    }
}

/// the name a tab shows for an address: what follows its kind (`character:catpope` → catpope), a medium's file name
/// (`video:live/x/loop.mov` → loop.mov)
pub fn name_of(key: &str) -> String {
    let id = key.split_once(':').map_or(key, |(_, id)| id);
    match Kind::of(key) {
        Some(Kind::Media) => id.rsplit(['/', '\\']).next().unwrap_or(id).to_string(),
        _ => id.to_string(),
    }
}

/// one open item
pub struct Tab<K> {
    /// its address, the identity: one tab per address
    pub key: String,
    pub kind: Kind,
    pub name: String,
    /// what the editor kept of it while it is behind: none for the front one, and for one not loaded yet (a later
    /// start's, a module's replaced in place) – it opens from its address then
    pub kept: Option<K>,
    /// it has changes the server has not got (said when it went behind; the front one's is read live)
    pub dirty: bool,
}

/// a recent item: its address and name
pub type Recent = (String, String);

/// the recent items Home lists
pub const RECENT: usize = 12;

pub struct Tabs<K> {
    pub list: Vec<Tab<K>>,
    pub front: Option<usize>,
    pub recent: Vec<Recent>,
    /// the most tabs at once (`cap`)
    pub cap: usize,
    /// the addresses by their last time in front, the oldest first
    used: Vec<String>,
}

/// what a later start reopens
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct Saved {
    pub tabs: Vec<Recent>,
    pub front: Option<usize>,
    pub recent: Vec<Recent>,
}

/// the tabs a device keeps at once: `DECK_EDITOR_TABS`, else few on a small machine (a lean deck, ≤ 4 GiB), else many
pub fn cap_for(memory_mib: Option<u64>, asked: Option<&str>) -> usize {
    if let Some(n) = asked.and_then(|a| a.trim().parse::<usize>().ok()) {
        return n.max(1);
    }
    match memory_mib {
        Some(m) if m <= 4096 => 4,
        _ => 16,
    }
}

/// the machine's memory where it is cheap to read (Linux: the decks), in MiB
pub fn memory_mib() -> Option<u64> {
    let text = std::fs::read_to_string("/proc/meminfo").ok()?;
    let kb: u64 = text.lines().find_map(|l| l.strip_prefix("MemTotal:"))?.trim().trim_end_matches("kB").trim().parse().ok()?;
    Some(kb / 1024)
}

impl<K> Tabs<K> {
    pub fn new(cap: usize) -> Tabs<K> {
        Tabs { list: Vec::new(), front: None, recent: Vec::new(), cap: cap.max(1), used: Vec::new() }
    }

    pub fn find(&self, key: &str) -> Option<usize> {
        self.list.iter().position(|t| t.key == key)
    }

    pub fn front(&self) -> Option<&Tab<K>> {
        self.front.and_then(|i| self.list.get(i))
    }

    pub fn front_mut(&mut self) -> Option<&mut Tab<K>> {
        self.front.and_then(|i| self.list.get_mut(i))
    }

    fn touch(&mut self, key: &str) {
        self.used.retain(|k| k != key);
        self.used.push(key.to_string());
    }

    /// `i` comes to the front (its `kept` is the caller's to take)
    pub fn set_front(&mut self, i: usize) {
        if let Some(key) = self.list.get(i).map(|t| t.key.clone()) {
            self.front = Some(i);
            self.touch(&key);
        }
    }

    /// a new tab at the end, in front; over the cap the least recently used one behind goes to Recent (returned, with
    /// what it kept)
    pub fn add(&mut self, key: &str, name: &str) -> Vec<Tab<K>> {
        if let Some(i) = self.find(key) {
            self.set_front(i);
            return Vec::new();
        }
        let kind = Kind::of(key).unwrap_or(Kind::Media);
        self.list.push(Tab { key: key.to_string(), kind, name: name.to_string(), kept: None, dirty: false });
        self.set_front(self.list.len() - 1);
        let mut gone = Vec::new();
        while self.list.len() > self.cap {
            let front = self.front().map(|t| t.key.clone());
            let Some(oldest) = self.used.iter().find(|k| Some(*k) != front.as_ref()).cloned() else { break };
            let Some(i) = self.find(&oldest) else { break };
            gone.extend(self.remove(i));
        }
        gone
    }

    fn remove(&mut self, i: usize) -> Option<Tab<K>> {
        if i >= self.list.len() {
            return None;
        }
        let tab = self.list.remove(i);
        self.used.retain(|k| *k != tab.key);
        self.remember(&tab.key, &tab.name);
        self.front = match self.front {
            Some(f) if f > i => Some(f - 1),
            Some(f) if f == i => None,
            f => f,
        };
        Some(tab)
    }

    /// `i` closed; when it was in front, its right neighbour comes forward (else the left one); the last one closed
    /// leaves Home (front None). Returns it, and whether the front changed.
    pub fn close(&mut self, i: usize) -> Option<(Tab<K>, bool)> {
        let was_front = self.front == Some(i);
        let tab = self.remove(i)?;
        if was_front && !self.list.is_empty() {
            self.set_front(i.min(self.list.len() - 1));
        }
        Some((tab, was_front))
    }

    /// every tab but `keep` closed
    pub fn close_others(&mut self, keep: usize) -> Vec<Tab<K>> {
        let Some(key) = self.list.get(keep).map(|t| t.key.clone()) else { return Vec::new() };
        let mut gone = Vec::new();
        while let Some(i) = self.list.iter().position(|t| t.key != key) {
            gone.extend(self.remove(i));
        }
        self.set_front(0);
        gone
    }

    pub fn close_all(&mut self) -> Vec<Tab<K>> {
        let mut gone = Vec::new();
        while !self.list.is_empty() {
            gone.extend(self.remove(self.list.len() - 1));
        }
        self.front = None;
        gone
    }

    /// the tab in front became another item in place (a show saved under a new id, a module picked in an empty tab)
    pub fn rekey(&mut self, i: usize, key: &str, name: &str) {
        let Some(old) = self.list.get(i).map(|t| t.key.clone()) else { return };
        if old == key {
            if let Some(t) = self.list.get_mut(i) {
                t.name = name.to_string();
            }
            return;
        }
        if let Some(j) = self.find(key) {
            // already open elsewhere: this one goes, that one comes forward
            self.remove(i);
            let j = if j > i { j - 1 } else { j };
            self.set_front(j);
            return;
        }
        let t = &mut self.list[i];
        (t.key, t.name, t.kind) = (key.to_string(), name.to_string(), Kind::of(key).unwrap_or(t.kind));
        for k in &mut self.used {
            if *k == old {
                *k = key.to_string();
            }
        }
    }

    /// `key` heads Recent
    pub fn remember(&mut self, key: &str, name: &str) {
        if key.ends_with(':') {
            return;
        }
        self.recent.retain(|(k, _)| k != key);
        self.recent.insert(0, (key.to_string(), name.to_string()));
        self.recent.truncate(RECENT);
    }

    pub fn saved(&self) -> Saved {
        let tabs: Vec<Recent> = self.list.iter().filter(|t| !t.key.ends_with(':')).map(|t| (t.key.clone(), t.name.clone())).collect();
        let front = self.front().and_then(|f| tabs.iter().position(|(k, _)| *k == f.key));
        Saved { tabs, front, recent: self.recent.clone() }
    }

    /// a later start: the tabs as they were, none loaded yet (each opens from its address when it comes to the front)
    pub fn from_saved(s: Saved, cap: usize) -> Tabs<K> {
        let mut tabs = Tabs::new(cap);
        tabs.recent = s.recent;
        for (key, name) in s.tabs.iter().filter(|(k, _)| Kind::of(k).is_some()) {
            tabs.add(key, name);
        }
        let recent = std::mem::take(&mut tabs.recent);
        tabs.recent = recent.into_iter().filter(|(k, _)| !s.tabs.iter().any(|(t, _)| t == k)).collect();
        // Home in front last time: Home again, the tabs behind it
        tabs.front = None;
        if let Some(f) = s.front.filter(|f| *f < tabs.list.len()) {
            tabs.set_front(f);
        }
        tabs
    }
}

/// where the tabs are remembered
pub fn file(cache: &Path) -> PathBuf {
    cache.join("editor-tabs.json")
}

pub fn load(file: &Path) -> Saved {
    std::fs::read_to_string(file).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default()
}

pub fn save(file: &Path, s: &Saved) {
    if let Ok(text) = serde_json::to_string_pretty(s) {
        if let Err(e) = std::fs::write(file, text) {
            tracing::warn!("the tabs are not remembered: {e}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tabs(keys: &[&str]) -> Tabs<()> {
        let mut t = Tabs::new(16);
        for k in keys {
            t.add(k, &name_of(k));
        }
        t
    }

    #[test]
    fn an_address_says_its_kind_and_view() {
        assert_eq!(Kind::of("scene:nave"), Some(Kind::Scene));
        assert_eq!(Kind::of("character:catpope").map(Kind::view), Some(View::Asset));
        assert_eq!(Kind::of("module:analog").map(Kind::view), Some(View::Asset), "a module is an asset");
        assert_eq!(Kind::of("video:intro").map(Kind::view), Some(View::Asset));
        assert_eq!(Kind::of("show:duet").map(Kind::view), Some(View::Show));
        assert_eq!(Kind::of("catpope"), None);
        assert_eq!(name_of("character:catpope"), "catpope");
        assert_eq!(name_of("video:live/x/loop.mov"), "loop.mov");
        assert_eq!(name_of("image:C:\\media\\a.png"), "a.png");
    }

    #[test]
    fn one_tab_per_item_opening_an_open_one_brings_it_forward() {
        let mut t = tabs(&["scene:nave", "character:catpope", "module:analog"]);
        assert_eq!(t.front, Some(2));
        t.add("scene:nave", "nave");
        assert_eq!((t.list.len(), t.front), (3, Some(0)));
    }

    #[test]
    fn closing_the_front_brings_its_neighbour_and_the_last_brings_home() {
        let mut t = tabs(&["scene:nave", "character:catpope", "module:analog"]);
        t.set_front(1);
        let (gone, was_front) = t.close(1).unwrap();
        assert!(was_front && gone.key == "character:catpope");
        assert_eq!(t.front().map(|f| f.key.as_str()), Some("module:analog"), "the right neighbour");
        t.close(1);
        assert_eq!(t.front().map(|f| f.key.as_str()), Some("scene:nave"));
        t.close(0);
        assert_eq!(t.front, None, "nothing open: Home");
        assert_eq!(t.recent.first().map(|r| r.0.as_str()), Some("scene:nave"), "closed tabs go to Recent, newest first");
        assert_eq!(t.recent.len(), 3);
    }

    #[test]
    fn closing_one_behind_keeps_the_front_on_its_item() {
        let mut t = tabs(&["scene:nave", "character:catpope", "module:analog"]);
        let (_, was_front) = t.close(0).unwrap();
        assert!(!was_front);
        assert_eq!(t.front().map(|f| f.key.as_str()), Some("module:analog"));
    }

    #[test]
    fn close_others_and_close_all() {
        let mut t = tabs(&["scene:nave", "character:catpope", "module:analog", "show:duet"]);
        assert_eq!(t.close_others(1).len(), 3);
        assert_eq!((t.list.len(), t.front), (1, Some(0)));
        assert_eq!(t.close_all().len(), 1);
        assert!(t.list.is_empty() && t.front.is_none());
    }

    #[test]
    fn a_small_device_moves_the_least_recently_used_tab_to_recent() {
        let mut t: Tabs<()> = Tabs::new(cap_for(Some(2048), None));
        for k in ["scene:a", "scene:b", "scene:c", "scene:d"] {
            t.add(k, &name_of(k));
        }
        t.set_front(0);
        let gone = t.add("scene:e", "e");
        assert_eq!(gone.iter().map(|g| g.key.as_str()).collect::<Vec<_>>(), ["scene:b"], "b was used longest ago, a was just in front");
        assert_eq!(t.list.len(), 4);
        assert_eq!(t.front().map(|f| f.key.as_str()), Some("scene:e"));
        assert_eq!(t.recent[0].0, "scene:b", "never lost: in Recent");
        assert_eq!(cap_for(Some(16_000), None), 16);
        assert_eq!(cap_for(None, Some("2")), 2, "DECK_EDITOR_TABS wins");
    }

    #[test]
    fn a_later_start_reopens_the_last_tabs_with_the_same_front() {
        let mut t = tabs(&["scene:nave", "character:catpope", "show:duet"]);
        t.set_front(1);
        t.remember("module:analog", "analog");
        let s = t.saved();
        let json = serde_json::to_string(&s).unwrap();
        let back: Tabs<()> = Tabs::from_saved(serde_json::from_str(&json).unwrap(), 16);
        assert_eq!(back.list.iter().map(|t| t.key.as_str()).collect::<Vec<_>>(), ["scene:nave", "character:catpope", "show:duet"]);
        assert_eq!(back.front, Some(1));
        assert!(back.list.iter().all(|t| t.kept.is_none()), "none loaded until it comes forward");
        assert_eq!(back.recent, [("module:analog".to_string(), "analog".to_string())]);
        let empty: Tabs<()> = Tabs::from_saved(Saved::default(), 16);
        assert_eq!(empty.front, None, "all closed last time: Home");
    }

    #[test]
    fn a_tab_renamed_in_place_onto_an_open_item_gives_way_to_it() {
        let mut t = tabs(&["module:analog", "module:"]);
        t.rekey(1, "module:walk", "walk");
        assert_eq!(t.list[1].key, "module:walk");
        t.add("module:", "new module");
        t.rekey(2, "module:analog", "analog");
        assert_eq!(t.list.len(), 2);
        assert_eq!(t.front().map(|f| f.key.as_str()), Some("module:analog"));
        assert!(t.saved().tabs.iter().all(|(k, _)| !k.ends_with(':')), "an empty tab is not remembered");
    }
}
