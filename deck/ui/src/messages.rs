//! The Log: what the app says, in one model for both editors. Two kinds of line: **standing** lines are a state that is
//! true now (a problem: kept by its key, replaced or cleared, never aged out) and **passing** lines are moments (saved,
//! a generation failed, a technical line) in one bounded ring, repeats folded. Every line has a scope – an address path
//! such as `toon/catpope/season/1/chapter/3` – and a scope filter is a prefix of it; an event line carries its type.
//! The editors' status bars show the newest passing line and the counts, their Log panels the lines through a
//! [`Filter`]. Plain data: the adapter stamps `at` (seconds since the Unix epoch) and feeds the technical log in as
//! [`TECH`]. The JSON calls (`*_json`) are the creator's, through space-web.
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

/// the source of the technical log (the `tracing` lines an adapter feeds in)
pub const TECH: &str = "tech";
/// how many passing lines the ring keeps
pub const CAP: usize = 500;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Level {
    Info,
    #[serde(alias = "success")]
    Ok,
    #[serde(alias = "warning")]
    Warn,
    Error,
}

impl Level {
    pub fn word(self) -> &'static str {
        match self {
            Level::Info => "info",
            Level::Ok => "ok",
            Level::Warn => "warn",
            Level::Error => "error",
        }
    }
}

/// an event's type: its icon on the line, its Type pill, its legend entry. Problems carry none.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    Logic,
    Save,
    Publish,
    History,
    Generation,
    Library,
    Show,
    Sync,
    Technical,
}

impl Kind {
    /// the Type pills' order
    pub const ALL: [Kind; 9] = [Kind::Logic, Kind::Save, Kind::Publish, Kind::History, Kind::Generation, Kind::Library, Kind::Show, Kind::Sync, Kind::Technical];

    pub fn word(self) -> &'static str {
        match self {
            Kind::Logic => "logic",
            Kind::Save => "save",
            Kind::Publish => "publish",
            Kind::History => "history",
            Kind::Generation => "generation",
            Kind::Library => "library",
            Kind::Show => "show",
            Kind::Sync => "sync",
            Kind::Technical => "technical",
        }
    }

    /// the pill's and the legend's name
    pub fn label(self) -> &'static str {
        match self {
            Kind::Logic => "Logic",
            Kind::Save => "Saves",
            Kind::Publish => "Publishes",
            Kind::History => "History",
            Kind::Generation => "Generations",
            Kind::Library => "Library",
            Kind::Show => "Show",
            Kind::Sync => "Sync",
            Kind::Technical => "Technical",
        }
    }

    /// what the legend under the list says it means
    pub fn legend(self) -> &'static str {
        match self {
            Kind::Logic => "an address-book event fired in a chapter, experience or plugin",
            Kind::Save => "an autosave of the item",
            Kind::Publish => "made live for readers",
            Kind::History => "a version restored",
            Kind::Generation => "image / video jobs",
            Kind::Library => "uploads, replaces, deletes",
            Kind::Show => "cues, devices, the conductor",
            Kind::Sync => "connection, saving again",
            Kind::Technical => "the app and the core",
        }
    }
}

fn one() -> u32 {
    1
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Message {
    pub level: Level,
    /// where it comes from, in words: a workspace, a part of the app, a problem's place, or [`TECH`]
    #[serde(default)]
    pub source: String,
    pub text: String,
    /// seconds since the Unix epoch, the newest time it was said
    #[serde(default)]
    pub at: f64,
    /// the longer story (a technical line's fields), shown in the Log only
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
    /// how often it was said in a row (the same level, source, text, scope and type)
    #[serde(default = "one")]
    pub times: u32,
    /// the address path it belongs to (`toon/catpope/season/1/chapter/3/…`); empty: the app
    #[serde(default)]
    pub scope: String,
    /// a standing line's key (it replaces the line with the same key); none: a passing line
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub key: Option<String>,
    /// an event's type; a technical line without one counts as [`Kind::Technical`]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<Kind>,
    /// where a press on the line goes (an address or a problem's key)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub jump: Option<String>,
    /// the address-book entry it is about (`event:bell`)
    #[serde(default, rename = "ref", skip_serializing_if = "Option::is_none")]
    pub reference: Option<String>,
    /// a standing warning someone accepted: shown quieter, not counted
    #[serde(default)]
    pub accepted: bool,
}

impl Message {
    pub fn new(level: Level, source: impl Into<String>, text: impl Into<String>, at: f64) -> Message {
        Message { level, source: source.into(), text: text.into(), at, detail: None, times: 1, scope: String::new(), key: None, kind: None, jump: None, reference: None, accepted: false }
    }

    pub fn with_detail(mut self, detail: impl Into<String>) -> Message {
        self.detail = Some(detail.into());
        self
    }

    pub fn with_scope(mut self, scope: impl Into<String>) -> Message {
        self.scope = scope.into();
        self
    }

    pub fn with_kind(mut self, kind: Kind) -> Message {
        self.kind = Some(kind);
        self
    }

    /// a standing line under `key`
    pub fn standing(mut self, key: impl Into<String>) -> Message {
        self.key = Some(key.into());
        self
    }

    pub fn is_standing(&self) -> bool {
        self.key.is_some()
    }

    /// its type: the one it carries, a technical source's [`Kind::Technical`], else none
    pub fn kind(&self) -> Option<Kind> {
        self.kind.or((self.source == TECH).then_some(Kind::Technical))
    }

    /// one line of the copied or saved log: the clock, the level, the source, the text, the detail
    pub fn line(&self) -> String {
        let times = if self.times > 1 { format!(" (×{})", self.times) } else { String::new() };
        let detail = self.detail.as_ref().map(|d| format!(" – {d}")).unwrap_or_default();
        let clock = if self.is_standing() { "standing".to_string() } else { clock(self.at) };
        format!("{clock} {:<5} {}: {}{times}{detail}", self.level.word(), self.source, self.text)
    }

    fn folds_into(&self, last: &Message) -> bool {
        last.level == self.level && last.source == self.source && last.text == self.text && last.detail == self.detail && last.scope == self.scope && last.kind == self.kind
    }
}

/// whether `scope` lies at or under `prefix` (whole segments: `chapter/2` is not under `chapter/20`); an empty prefix is everything
pub fn in_scope(scope: &str, prefix: &str) -> bool {
    let prefix = prefix.trim_end_matches('/');
    prefix.is_empty() || scope.strip_prefix(prefix).is_some_and(|rest| rest.is_empty() || rest.starts_with('/'))
}

/// the deck editor's level pills
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Levels {
    #[default]
    All,
    Errors,
    Warnings,
    Info,
    Technical,
}

impl Levels {
    pub const ALL: [Levels; 5] = [Levels::All, Levels::Errors, Levels::Warnings, Levels::Info, Levels::Technical];

    pub fn word(self) -> &'static str {
        match self {
            Levels::All => "All",
            Levels::Errors => "Errors",
            Levels::Warnings => "Warnings",
            Levels::Info => "Info",
            Levels::Technical => "Technical",
        }
    }

    pub fn admits(self, m: &Message) -> bool {
        match self {
            Levels::All => true,
            Levels::Errors => m.level == Level::Error,
            Levels::Warnings => m.level == Level::Warn,
            Levels::Info => matches!(m.level, Level::Info | Level::Ok),
            Levels::Technical => m.kind() == Some(Kind::Technical),
        }
    }
}

/// the Show pills: everything (problems on top, events below), the standing lines, the passing ones
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Show {
    #[default]
    All,
    Problems,
    Events,
}

impl Show {
    pub const ALL: [Show; 3] = [Show::All, Show::Problems, Show::Events];

    pub fn word(self) -> &'static str {
        match self {
            Show::All => "All",
            Show::Problems => "Problems",
            Show::Events => "Events",
        }
    }
}

/// the Log's filter: levels, Show, the event types (none = every type; problems carry none and pass), the scope prefix,
/// and whether standing info lines (notes) show
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Filter {
    pub level: Levels,
    pub show: Show,
    pub kinds: Vec<Kind>,
    pub scope: String,
    pub notes: bool,
}

impl Filter {
    pub fn admits(&self, m: &Message) -> bool {
        let shown = match self.show {
            Show::All => true,
            Show::Problems => m.is_standing(),
            Show::Events => !m.is_standing(),
        };
        let typed = m.is_standing() || self.kinds.is_empty() || m.kind().is_some_and(|k| self.kinds.contains(&k));
        let note = !(m.is_standing() && m.level == Level::Info) || self.notes;
        shown && typed && note && self.level.admits(m) && in_scope(&m.scope, &self.scope)
    }

    /// the Problems preset: the standing lines in `scope`
    pub fn problems(scope: impl Into<String>) -> Filter {
        Filter { show: Show::Problems, scope: scope.into(), ..Filter::default() }
    }
}

/// the counts the status bar shows (an accepted line counts none)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize)]
pub struct Counts {
    pub errors: usize,
    pub warnings: usize,
}

/// the standing lines in their order and the ring of passing ones: oldest first inside, newest first out
#[derive(Debug, Clone)]
pub struct Messages {
    standing: Vec<Message>,
    ring: VecDeque<Message>,
    cap: usize,
}

impl Default for Messages {
    fn default() -> Self {
        Messages::new(CAP)
    }
}

impl Messages {
    pub fn new(cap: usize) -> Messages {
        Messages { standing: Vec::new(), ring: VecDeque::with_capacity(cap.min(CAP)), cap: cap.max(1) }
    }

    /// a line in: a standing one replaces the line with its key; a passing one said again right after only counts up and moves to its new time
    pub fn push(&mut self, m: Message) {
        if let Some(key) = &m.key {
            match self.standing.iter_mut().find(|s| s.key.as_ref() == Some(key)) {
                Some(s) => *s = m,
                None => self.standing.push(m),
            }
            return;
        }
        if let Some(last) = self.ring.back_mut() {
            if m.folds_into(last) {
                last.times += 1;
                last.at = m.at;
                return;
            }
        }
        if self.ring.len() == self.cap {
            self.ring.pop_front();
        }
        self.ring.push_back(m);
    }

    /// every standing line whose key starts with `prefix` goes, `lines` stand instead (a poster's whole set, recomputed)
    pub fn stand(&mut self, prefix: &str, lines: impl IntoIterator<Item = Message>) {
        self.standing.retain(|s| !s.key.as_deref().is_some_and(|k| k.starts_with(prefix)));
        for (i, mut m) in lines.into_iter().enumerate() {
            m.key.get_or_insert_with(|| format!("{prefix}{i}"));
            self.push(m);
        }
    }

    /// the standing line with `key` goes
    pub fn clear(&mut self, key: &str) {
        self.standing.retain(|s| s.key.as_deref() != Some(key));
    }

    /// the lines the ring and the standing set hold
    pub fn len(&self) -> usize {
        self.ring.len() + self.standing.len()
    }

    pub fn is_empty(&self) -> bool {
        self.ring.is_empty() && self.standing.is_empty()
    }

    /// the newest passing line (the status bar's)
    pub fn newest(&self) -> Option<&Message> {
        self.ring.back()
    }

    /// the errors and warnings the Log holds (a repeat counts once: it is one line in the Log)
    pub fn counts(&self) -> Counts {
        self.counts_in(&Filter::default())
    }

    /// the errors and warnings through `filter` (the creator's status bar: the Problems preset of the open item)
    pub fn counts_in(&self, filter: &Filter) -> Counts {
        self.standing.iter().chain(self.ring.iter()).filter(|m| !m.accepted && filter.admits(m)).fold(Counts::default(), |mut c, m| {
            match m.level {
                Level::Error => c.errors += 1,
                Level::Warn => c.warnings += 1,
                _ => {}
            }
            c
        })
    }

    /// what the Log shows: the standing lines first (errors before warnings before the rest, accepted last), then the
    /// passing ones newest first – through the filter, with `find` in the source, text, detail or ref (any case)
    pub fn shown(&self, filter: &Filter, find: &str) -> Vec<&Message> {
        let find = find.trim().to_lowercase();
        let found = |m: &&Message| find.is_empty() || [Some(&m.source), Some(&m.text), m.detail.as_ref(), m.reference.as_ref()].into_iter().flatten().any(|s| s.to_lowercase().contains(&find));
        let mut standing: Vec<&Message> = self.standing.iter().filter(|m| filter.admits(m)).filter(found).collect();
        standing.sort_by_key(|m| (m.accepted, std::cmp::Reverse(m.level)));
        standing.into_iter().chain(self.ring.iter().rev().filter(|m| filter.admits(m)).filter(found)).collect()
    }

    /// what Copy all and Save write: the lines shown, one per line
    pub fn text(&self, filter: &Filter, find: &str) -> String {
        self.shown(filter, find).iter().map(|m| m.line() + "\n").collect()
    }

    /// a line in from JSON (a [`Message`]; `times` and the rest may be left out)
    pub fn push_json(&mut self, message: &str) -> Result<(), String> {
        self.push(serde_json::from_str(message).map_err(|e| e.to_string())?);
        Ok(())
    }

    /// a poster's standing set from JSON (an array of [`Message`]s)
    pub fn stand_json(&mut self, prefix: &str, lines: &str) -> Result<(), String> {
        let lines: Vec<Message> = serde_json::from_str(lines).map_err(|e| e.to_string())?;
        self.stand(prefix, lines);
        Ok(())
    }

    /// `{ ...Filter, find, now, max }` → `{ problems, events, counts, notes, newest }`: the lines as [`Message`]s with
    /// their `age` and `clock`, the Problems group's and the Events group's apart (`max` caps the events, default 200)
    pub fn view_json(&self, request: &str) -> Result<String, String> {
        let r: ViewRequest = serde_json::from_str(request).map_err(|e| e.to_string())?;
        let shown = self.shown(&r.filter, &r.find);
        let out = |m: &Message| Shown { age: if m.is_standing() { String::new() } else { age(r.now, m.at) }, clock: clock(m.at), message: Message { kind: m.kind(), ..m.clone() } };
        let (problems, events): (Vec<&Message>, Vec<&Message>) = shown.into_iter().partition(|m| m.is_standing());
        let notes = self.standing.iter().filter(|m| m.level == Level::Info && Filter { notes: true, ..r.filter.clone() }.admits(m)).count();
        let view = View {
            problems: problems.into_iter().map(out).collect(),
            events: events.into_iter().take(r.max.unwrap_or(200)).map(out).collect(),
            counts: self.counts_in(&r.filter),
            notes,
            newest: self.newest().map(out),
        };
        serde_json::to_string(&view).map_err(|e| e.to_string())
    }

    /// `{ ...Filter }` → its errors and warnings as JSON
    pub fn counts_json(&self, filter: &str) -> Result<String, String> {
        let f: Filter = serde_json::from_str(filter).map_err(|e| e.to_string())?;
        serde_json::to_string(&self.counts_in(&f)).map_err(|e| e.to_string())
    }

    /// `{ ...Filter, find }` → Copy all's text
    pub fn text_json(&self, request: &str) -> Result<String, String> {
        let r: ViewRequest = serde_json::from_str(request).map_err(|e| e.to_string())?;
        Ok(self.text(&r.filter, &r.find))
    }
}

#[derive(Deserialize)]
struct ViewRequest {
    #[serde(flatten)]
    filter: Filter,
    #[serde(default)]
    find: String,
    #[serde(default)]
    now: f64,
    #[serde(default)]
    max: Option<usize>,
}

#[derive(Serialize)]
struct Shown {
    #[serde(flatten)]
    message: Message,
    age: String,
    clock: String,
}

#[derive(Serialize)]
struct View {
    problems: Vec<Shown>,
    events: Vec<Shown>,
    counts: Counts,
    notes: usize,
    newest: Option<Shown>,
}

/// the scope pills, in their order
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Scope {
    Selection,
    Item,
    Chapter,
    Season,
    Toon,
    Show,
    All,
}

impl Scope {
    pub const ALL: [Scope; 7] = [Scope::Selection, Scope::Item, Scope::Chapter, Scope::Season, Scope::Toon, Scope::Show, Scope::All];

    pub fn word(self) -> &'static str {
        match self {
            Scope::Selection => "Selection",
            Scope::Item => "Item",
            Scope::Chapter => "Chapter",
            Scope::Season => "Season",
            Scope::Toon => "Toon",
            Scope::Show => "Show",
            Scope::All => "All",
        }
    }
}

/// the scope of every live show's lines
pub const SHOW_SCOPE: &str = "show";

/// `scope` cut after its `word/<id>` pair (`chapter` in `toon/a/season/1/chapter/3/groups/0` → `toon/a/season/1/chapter/3`)
fn ancestor(scope: &str, word: &str) -> Option<String> {
    let parts: Vec<&str> = scope.split('/').collect();
    parts.chunks(2).position(|pair| pair.len() == 2 && pair[0] == word).map(|i| parts[..i * 2 + 2].join("/"))
}

/// The scope pills for the open item and the selection: each with its prefix, or none where the item has no such place
/// (a character has no chapter). Selection follows the selection; Chapter, Season and Toon are the item's ancestors.
pub fn scopes(item: Option<&str>, selection: Option<&str>) -> Vec<(Scope, Option<String>)> {
    let item = item.filter(|s| !s.is_empty());
    Scope::ALL
        .into_iter()
        .map(|s| {
            let prefix = match s {
                Scope::Selection => selection.filter(|x| !x.is_empty()).map(str::to_string),
                Scope::Item => item.map(str::to_string),
                Scope::Chapter => item.and_then(|i| ancestor(i, "chapter")),
                Scope::Season => item.and_then(|i| ancestor(i, "season")),
                Scope::Toon => item.and_then(|i| ancestor(i, "toon")),
                Scope::Show => Some(SHOW_SCOPE.to_string()),
                Scope::All => Some(String::new()),
            };
            (s, prefix)
        })
        .collect()
}

/// `{ item, selection }` → `[{ scope, words, prefix }]` (prefix null: not offered)
pub fn scopes_json(request: &str) -> Result<String, String> {
    #[derive(Deserialize, Default)]
    #[serde(default)]
    struct R {
        item: Option<String>,
        selection: Option<String>,
    }
    let r: R = serde_json::from_str(request).map_err(|e| e.to_string())?;
    let out: Vec<serde_json::Value> = scopes(r.item.as_deref(), r.selection.as_deref()).into_iter().map(|(s, prefix)| serde_json::json!({ "scope": s, "words": s.word(), "prefix": prefix })).collect();
    serde_json::to_string(&out).map_err(|e| e.to_string())
}

/// the Type pills and the legend: `[{ kind, label, legend }]`; the Show pills: `shows: [{ show, words }]`
pub fn kinds_json() -> String {
    let kinds: Vec<serde_json::Value> = Kind::ALL.into_iter().map(|k| serde_json::json!({ "kind": k, "label": k.label(), "legend": k.legend() })).collect();
    let shows: Vec<serde_json::Value> = Show::ALL.into_iter().map(|s| serde_json::json!({ "show": s, "words": s.word() })).collect();
    serde_json::json!({ "kinds": kinds, "shows": shows }).to_string()
}

/// how long ago, short: "now", "12 s", "4 min", "2 h", "3 d"
pub fn age(now: f64, at: f64) -> String {
    let s = (now - at).max(0.0);
    match s {
        s if s < 5.0 => "now".into(),
        s if s < 60.0 => format!("{} s", s as u64),
        s if s < 3600.0 => format!("{} min", (s / 60.0) as u64),
        s if s < 86_400.0 => format!("{} h", (s / 3600.0) as u64),
        s => format!("{} d", (s / 86_400.0) as u64),
    }
}

/// the time of day in UTC, HH:MM:SS
pub fn clock(at: f64) -> String {
    let s = at.max(0.0) as u64 % 86_400;
    format!("{:02}:{:02}:{:02}", s / 3600, s / 60 % 60, s % 60)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn m(level: Level, source: &str, text: &str, at: f64) -> Message {
        Message::new(level, source, text, at)
    }

    fn texts(all: &Messages, f: &Filter, find: &str) -> Vec<String> {
        all.shown(f, find).iter().map(|m| m.text.clone()).collect()
    }

    fn levels(level: Levels) -> Filter {
        Filter { level, ..Filter::default() }
    }

    #[test]
    fn the_counts_are_the_errors_and_warnings_the_ring_holds() {
        let mut all = Messages::new(4);
        assert_eq!(all.counts(), Counts::default());
        all.push(m(Level::Warn, "build", "pick the shot it ends on", 1.0));
        all.push(m(Level::Error, TECH, "autosave: disk full", 2.0));
        all.push(m(Level::Info, "show", "loaded", 3.0));
        all.push(m(Level::Ok, "module", "built", 4.0));
        assert_eq!(all.counts(), Counts { errors: 1, warnings: 1 });
        // a repeat is one line: it counts once
        all.push(m(Level::Ok, "module", "built", 5.0));
        assert_eq!((all.len(), all.newest().map(|n| (n.times, n.at))), (4, Some((2, 5.0))));
        // the ring drops the oldest: the warning goes, its count with it
        all.push(m(Level::Error, TECH, "sync: refused", 6.0));
        assert_eq!(all.counts(), Counts { errors: 2, warnings: 0 });
        assert_eq!(all.len(), 4);
    }

    #[test]
    fn the_log_is_newest_first_through_its_filter_and_find() {
        let mut all = Messages::default();
        all.push(m(Level::Warn, "build", "refused: no port there", 1.0));
        all.push(m(Level::Error, TECH, "sign-in: connection refused", 2.0).with_detail("deck_link::pairing"));
        all.push(m(Level::Info, "show", "loaded rehearsal@3", 3.0));
        assert_eq!(texts(&all, &levels(Levels::All), ""), ["loaded rehearsal@3", "sign-in: connection refused", "refused: no port there"]);
        assert_eq!(texts(&all, &levels(Levels::Errors), ""), ["sign-in: connection refused"]);
        assert_eq!(texts(&all, &levels(Levels::Warnings), ""), ["refused: no port there"]);
        assert_eq!(texts(&all, &levels(Levels::Info), ""), ["loaded rehearsal@3"]);
        assert_eq!(texts(&all, &levels(Levels::Technical), ""), ["sign-in: connection refused"]);
        assert_eq!(texts(&all, &levels(Levels::All), "REFUSED"), ["sign-in: connection refused", "refused: no port there"]);
        assert_eq!(texts(&all, &levels(Levels::All), "pairing"), ["sign-in: connection refused"], "the detail is searched too");
    }

    #[test]
    fn copy_all_writes_one_line_per_message_with_its_clock_level_and_source() {
        let mut all = Messages::default();
        all.push(m(Level::Warn, "build", "pick the shot it ends on", 3_600.0 * 13.0 + 62.0));
        all.push(m(Level::Warn, "build", "pick the shot it ends on", 3_600.0 * 13.0 + 65.0));
        all.push(m(Level::Error, TECH, "autosave: disk full", 3_600.0 * 13.0 + 70.0).with_detail("deck_editor::autosave"));
        all.push(m(Level::Error, "Group 2 · Slice 3", "no image", 0.0).standing("chapter#a"));
        assert_eq!(all.text(&Filter::default(), ""), "standing error Group 2 · Slice 3: no image\n13:01:10 error tech: autosave: disk full – deck_editor::autosave\n13:01:05 warn  build: pick the shot it ends on (×2)\n");
    }

    #[test]
    fn a_scope_is_a_prefix_of_whole_segments() {
        assert!(in_scope("toon/a/season/1/chapter/2/groups/1", "toon/a/season/1/chapter/2"));
        assert!(in_scope("toon/a/season/1/chapter/2", "toon/a/season/1/chapter/2/"));
        assert!(!in_scope("toon/a/season/1/chapter/20", "toon/a/season/1/chapter/2"));
        assert!(in_scope("", ""));
        assert!(!in_scope("", "show"), "an app line is only under All");
    }

    #[test]
    fn standing_lines_stand_by_key_and_come_first_errors_on_top_accepted_last() {
        let mut all = Messages::new(3);
        all.push(m(Level::Ok, "Chapter 2", "Saved", 1.0).with_kind(Kind::Save));
        all.stand(
            "chapter:a:2#",
            [
                m(Level::Warn, "Chapter", "description is short", 0.0).standing("chapter:a:2#desc"),
                m(Level::Warn, "Season 1", "no cover", 0.0).standing("chapter:a:2#cover").tap(|m| m.accepted = true),
                m(Level::Error, "Group 2 · Slice 3", "no image", 0.0).standing("chapter:a:2#img"),
            ],
        );
        assert_eq!(texts(&all, &Filter::default(), ""), ["no image", "description is short", "no cover", "Saved"]);
        assert_eq!(all.counts(), Counts { errors: 1, warnings: 1 }, "the accepted warning counts none");
        // the ring dropping old lines never drops a standing one
        for i in 0..5 {
            all.push(m(Level::Info, "x", &format!("{i}"), 2.0 + i as f64));
        }
        assert_eq!(all.shown(&Filter::problems(""), "").len(), 3);
        // the same key replaces; a recomputed set replaces the poster's whole set
        all.push(m(Level::Error, "Group 2 · Slice 3", "no image, no text", 0.0).standing("chapter:a:2#img"));
        assert_eq!(texts(&all, &Filter::problems(""), "")[0], "no image, no text");
        all.stand("chapter:a:2#", [m(Level::Warn, "Chapter", "description is short", 0.0)]);
        assert_eq!(texts(&all, &Filter::problems(""), ""), ["description is short"]);
        assert_eq!(all.shown(&Filter::problems(""), "")[0].key.as_deref(), Some("chapter:a:2#0"), "a line without a key gets one under the prefix");
        all.clear("chapter:a:2#0");
        assert!(all.shown(&Filter::problems(""), "").is_empty());
    }

    trait Tap: Sized {
        fn tap(self, f: impl FnOnce(&mut Self)) -> Self;
    }
    impl Tap for Message {
        fn tap(mut self, f: impl FnOnce(&mut Self)) -> Self {
            f(&mut self);
            self
        }
    }

    #[test]
    fn show_types_scope_and_notes_filter_the_lines() {
        let ch = "toon/a/season/1/chapter/2";
        let mut all = Messages::default();
        all.push(m(Level::Error, "Slice 2.3", "no image", 0.0).standing("p#1").with_scope(format!("{ch}/groups/1/slices/2")));
        all.push(m(Level::Info, "Slice 2.3", "a note", 0.0).standing("p#2").with_scope(format!("{ch}/groups/1/slices/2")));
        all.push(m(Level::Warn, "Season 1", "no cover", 0.0).standing("p#3").with_scope("toon/a/season/1"));
        all.push(m(Level::Ok, "Chapter 2", "Saved", 1.0).with_kind(Kind::Save).with_scope(ch));
        all.push(m(Level::Info, "Chapter 2 · event:bell", "bell fired", 2.0).with_kind(Kind::Logic).with_scope(format!("{ch}/groups/1")));
        all.push(m(Level::Info, "Show", "cue 4 fired", 3.0).with_kind(Kind::Show).with_scope(SHOW_SCOPE));
        all.push(m(Level::Warn, TECH, "slow frame", 4.0));
        let f = |show: Show, kinds: &[Kind], scope: &str| Filter { show, kinds: kinds.to_vec(), scope: scope.into(), ..Filter::default() };
        assert_eq!(texts(&all, &f(Show::All, &[], ""), ""), ["no image", "no cover", "slow frame", "cue 4 fired", "bell fired", "Saved"], "notes hidden by default");
        assert_eq!(texts(&all, &Filter { notes: true, ..f(Show::Problems, &[], ch) }, ""), ["no image", "a note"]);
        assert_eq!(texts(&all, &f(Show::All, &[], ch), ""), ["no image", "bell fired", "Saved"], "the item's scope: the season's problem and the app's line stay out");
        assert_eq!(texts(&all, &f(Show::Events, &[], ch), ""), ["bell fired", "Saved"]);
        assert_eq!(texts(&all, &f(Show::All, &[Kind::Logic], ch), ""), ["no image", "bell fired"], "problems carry no type and stay");
        assert_eq!(texts(&all, &f(Show::Events, &[Kind::Save, Kind::Technical], ""), ""), ["slow frame", "Saved"], "a tech source is Technical");
        assert_eq!(texts(&all, &f(Show::All, &[], SHOW_SCOPE), ""), ["cue 4 fired"]);
        assert_eq!(all.counts_in(&Filter::problems(ch)), Counts { errors: 1, warnings: 0 }, "the item's counts");
        assert_eq!(all.counts_in(&Filter::problems("toon/a")), Counts { errors: 1, warnings: 1 }, "the toon's counts");
    }

    #[test]
    fn the_scope_pills_are_the_items_ancestors() {
        let pills = scopes(Some("toon/a/season/1/chapter/3"), Some("toon/a/season/1/chapter/3/groups/0/slices/2"));
        let prefixes: Vec<(&str, Option<&str>)> = pills.iter().map(|(s, p)| (s.word(), p.as_deref())).collect();
        assert_eq!(
            prefixes,
            [
                ("Selection", Some("toon/a/season/1/chapter/3/groups/0/slices/2")),
                ("Item", Some("toon/a/season/1/chapter/3")),
                ("Chapter", Some("toon/a/season/1/chapter/3")),
                ("Season", Some("toon/a/season/1")),
                ("Toon", Some("toon/a")),
                ("Show", Some("show")),
                ("All", Some("")),
            ]
        );
        let character = scopes(Some("character/catpope"), None);
        assert_eq!(character.iter().filter(|(_, p)| p.is_none()).map(|(s, _)| *s).collect::<Vec<_>>(), [Scope::Selection, Scope::Chapter, Scope::Season, Scope::Toon]);
    }

    #[test]
    fn the_creator_speaks_json() {
        let mut all = Messages::default();
        all.push_json(r#"{"level":"success","source":"Chapter 2","text":"Saved","at":100,"scope":"toon/a/season/1/chapter/2","kind":"save"}"#).unwrap();
        all.stand_json("chapter:a:2#", r#"[{"level":"warning","source":"Chapter","text":"short","key":"chapter:a:2#d","scope":"toon/a/season/1/chapter/2","jump":"chapter:a:2#d"}]"#).unwrap();
        let v: serde_json::Value = serde_json::from_str(&all.view_json(r#"{"show":"all","kinds":[],"scope":"toon/a/season/1/chapter/2","find":"","now":112}"#).unwrap()).unwrap();
        assert_eq!(v["problems"][0]["level"], "warn");
        assert_eq!(v["problems"][0]["jump"], "chapter:a:2#d");
        assert_eq!(v["events"][0]["kind"], "save");
        assert_eq!(v["events"][0]["age"], "12 s");
        assert_eq!(v["newest"]["text"], "Saved");
        assert_eq!(v["counts"], serde_json::json!({ "errors": 0, "warnings": 1 }));
        all.push(m(Level::Warn, TECH, "slow frame", 110.0));
        let v: serde_json::Value = serde_json::from_str(&all.view_json(r#"{"now":112}"#).unwrap()).unwrap();
        assert_eq!((v["events"][0]["kind"].as_str(), v["events"][0]["age"].as_str()), (Some("technical"), Some("now")), "a tech source shows as Technical");
        assert_eq!(all.counts_json(r#"{"show":"problems","scope":"toon/b"}"#).unwrap(), r#"{"errors":0,"warnings":0}"#);
        assert!(all.push_json("{}").is_err());
        let k: serde_json::Value = serde_json::from_str(&kinds_json()).unwrap();
        assert_eq!((k["kinds"][0]["kind"].as_str(), k["kinds"][1]["label"].as_str(), k["kinds"].as_array().map(Vec::len)), (Some("logic"), Some("Saves"), Some(9)));
        let s: serde_json::Value = serde_json::from_str(&scopes_json(r#"{"item":"character/x"}"#).unwrap()).unwrap();
        assert_eq!((s[1]["prefix"].as_str(), s[2]["prefix"].is_null()), (Some("character/x"), true));
    }

    #[test]
    fn the_age_is_short() {
        assert_eq!([age(10.0, 8.0), age(30.0, 18.0), age(100.0, 40.0), age(4000.0, 100.0), age(90_000.0, 0.0), age(0.0, 5.0)], ["now", "12 s", "1 min", "1 h", "1 d", "now"]);
    }
}
