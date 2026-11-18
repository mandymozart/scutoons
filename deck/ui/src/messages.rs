//! The messages: what the app says, its state words' changes and the technical log, in one bounded ring. The editor's
//! status bar shows the newest and the counts, its Log panel all of them; a deck with a screen reads the same ring. Plain
//! data: the adapter stamps `at` (seconds since the Unix epoch) and feeds the technical log in as [`TECH`].
use std::collections::VecDeque;

/// the source of the technical log (the `tracing` lines an adapter feeds in)
pub const TECH: &str = "tech";
/// how many messages the ring keeps
pub const CAP: usize = 500;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Level {
    Info,
    Ok,
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

#[derive(Debug, Clone, PartialEq)]
pub struct Message {
    pub level: Level,
    /// where it comes from: a workspace, a part of the app, or [`TECH`]
    pub source: String,
    pub text: String,
    /// seconds since the Unix epoch, the newest time it was said
    pub at: f64,
    /// the longer story (a technical line's fields), shown in the Log only
    pub detail: Option<String>,
    /// how often it was said in a row (the same level, source and text)
    pub times: u32,
}

impl Message {
    pub fn new(level: Level, source: impl Into<String>, text: impl Into<String>, at: f64) -> Message {
        Message { level, source: source.into(), text: text.into(), at, detail: None, times: 1 }
    }

    pub fn with_detail(mut self, detail: impl Into<String>) -> Message {
        self.detail = Some(detail.into());
        self
    }

    /// one line of the copied or saved log: the clock, the level, the source, the text, the detail
    pub fn line(&self) -> String {
        let times = if self.times > 1 { format!(" (×{})", self.times) } else { String::new() };
        let detail = self.detail.as_ref().map(|d| format!(" – {d}")).unwrap_or_default();
        format!("{} {:<5} {}: {}{times}{detail}", clock(self.at), self.level.word(), self.source, self.text)
    }
}

/// the Log's filters
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Filter {
    #[default]
    All,
    Errors,
    Warnings,
    Info,
    Technical,
}

impl Filter {
    pub const ALL: [Filter; 5] = [Filter::All, Filter::Errors, Filter::Warnings, Filter::Info, Filter::Technical];

    pub fn word(self) -> &'static str {
        match self {
            Filter::All => "All",
            Filter::Errors => "Errors",
            Filter::Warnings => "Warnings",
            Filter::Info => "Info",
            Filter::Technical => "Technical",
        }
    }

    pub fn admits(self, m: &Message) -> bool {
        match self {
            Filter::All => true,
            Filter::Errors => m.level == Level::Error,
            Filter::Warnings => m.level == Level::Warn,
            Filter::Info => matches!(m.level, Level::Info | Level::Ok),
            Filter::Technical => m.source == TECH,
        }
    }
}

/// the counts the status bar shows
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Counts {
    pub errors: usize,
    pub warnings: usize,
}

/// the ring: oldest first inside, newest first out
#[derive(Debug, Clone)]
pub struct Messages {
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
        Messages { ring: VecDeque::with_capacity(cap.min(CAP)), cap: cap.max(1) }
    }

    /// a message in; the same one again right after only counts up and moves to its new time
    pub fn push(&mut self, m: Message) {
        if let Some(last) = self.ring.back_mut() {
            if last.level == m.level && last.source == m.source && last.text == m.text && last.detail == m.detail {
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

    pub fn len(&self) -> usize {
        self.ring.len()
    }

    pub fn is_empty(&self) -> bool {
        self.ring.is_empty()
    }

    pub fn newest(&self) -> Option<&Message> {
        self.ring.back()
    }

    /// the errors and warnings the ring holds (a repeat counts once: it is one line in the Log)
    pub fn counts(&self) -> Counts {
        self.ring.iter().fold(Counts::default(), |mut c, m| {
            match m.level {
                Level::Error => c.errors += 1,
                Level::Warn => c.warnings += 1,
                _ => {}
            }
            c
        })
    }

    /// what the Log shows: newest first, through the filter, with `find` in its source, text or detail (any case)
    pub fn shown(&self, filter: Filter, find: &str) -> Vec<&Message> {
        let find = find.trim().to_lowercase();
        self.ring
            .iter()
            .rev()
            .filter(|m| filter.admits(m))
            .filter(|m| find.is_empty() || [Some(&m.source), Some(&m.text), m.detail.as_ref()].into_iter().flatten().any(|s| s.to_lowercase().contains(&find)))
            .collect()
    }

    /// what Copy all and Save write: the lines shown, newest first, one per line
    pub fn text(&self, filter: Filter, find: &str) -> String {
        self.shown(filter, find).iter().map(|m| m.line() + "\n").collect()
    }
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
        let texts = |f: Filter, find: &str| all.shown(f, find).iter().map(|m| m.text.clone()).collect::<Vec<_>>();
        assert_eq!(texts(Filter::All, ""), ["loaded rehearsal@3", "sign-in: connection refused", "refused: no port there"]);
        assert_eq!(texts(Filter::Errors, ""), ["sign-in: connection refused"]);
        assert_eq!(texts(Filter::Warnings, ""), ["refused: no port there"]);
        assert_eq!(texts(Filter::Info, ""), ["loaded rehearsal@3"]);
        assert_eq!(texts(Filter::Technical, ""), ["sign-in: connection refused"]);
        assert_eq!(texts(Filter::All, "REFUSED"), ["sign-in: connection refused", "refused: no port there"]);
        assert_eq!(texts(Filter::All, "pairing"), ["sign-in: connection refused"], "the detail is searched too");
    }

    #[test]
    fn copy_all_writes_one_line_per_message_with_its_clock_level_and_source() {
        let mut all = Messages::default();
        all.push(m(Level::Warn, "build", "pick the shot it ends on", 3_600.0 * 13.0 + 62.0));
        all.push(m(Level::Warn, "build", "pick the shot it ends on", 3_600.0 * 13.0 + 65.0));
        all.push(m(Level::Error, TECH, "autosave: disk full", 3_600.0 * 13.0 + 70.0).with_detail("deck_editor::autosave"));
        assert_eq!(all.text(Filter::All, ""), "13:01:10 error tech: autosave: disk full – deck_editor::autosave\n13:01:05 warn  build: pick the shot it ends on (×2)\n");
    }

    #[test]
    fn the_age_is_short() {
        assert_eq!([age(10.0, 8.0), age(30.0, 18.0), age(100.0, 40.0), age(4000.0, 100.0), age(90_000.0, 0.0), age(0.0, 5.0)], ["now", "12 s", "1 min", "1 h", "1 d", "now"]);
    }
}
