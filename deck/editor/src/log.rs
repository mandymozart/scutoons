//! The editor's adapter to the messages (deck-ui `messages`): one inbox every part of the editor posts to, the `tracing`
//! layer that feeds the technical log into it, and the Log panel's drawing. The ring, the counts, the filters and the
//! words are the core's; the status bar is deck-kit's.
use deck_kit::base::draw::{DrawList, Rect};
use deck_kit::base::tokens::Tokens;
use deck_kit::editor::controls::{self, fit_sans};
use deck_ui::messages::{self, Filter, Kind, Level, Levels, Message, Messages};
use std::sync::Mutex;
use std::time::{Instant, SystemTime, UNIX_EPOCH};
use tracing::field::{Field, Visit};
use tracing_subscriber::layer::{Context, Layer};

/// what was posted since the last frame took it
static INBOX: Mutex<Vec<Message>> = Mutex::new(Vec::new());

/// the panel's id in every workspace
pub const PANEL: &str = "log";
/// the panel's content width
pub const WIDTH: f32 = 680.0;
/// the lines the panel draws at most (Copy all and Save take every line)
const SHOWN: usize = 200;
/// how long a refused press's ring stays where it happened
const RING_S: f32 = 0.6;

pub fn now() -> f64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0.0, |d| d.as_secs_f64())
}

/// a message from anywhere in the editor; the next frame shows it
pub fn post(level: Level, source: &str, text: impl Into<String>) {
    let text = text.into().lines().map(str::trim).filter(|l| !l.is_empty()).collect::<Vec<_>>().join(" · ");
    if text.is_empty() {
        return;
    }
    if let Ok(mut inbox) = INBOX.lock() {
        inbox.push(Message::new(level, source, text, now()));
    }
}

/// The technical log: the `tracing` lines at warn and error (and info when asked) go to the inbox as source `tech`.
pub struct Tech {
    info: bool,
}

impl Tech {
    pub fn new(info: bool) -> Tech {
        Tech { info }
    }
}

#[derive(Default)]
struct Fields {
    message: String,
    rest: Vec<String>,
}

impl Visit for Fields {
    fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
        if field.name() == "message" {
            self.message = format!("{value:?}");
        } else {
            self.rest.push(format!("{}={value:?}", field.name()));
        }
    }

    fn record_str(&mut self, field: &Field, value: &str) {
        if field.name() == "message" {
            self.message = value.to_string();
        } else {
            self.rest.push(format!("{}={value}", field.name()));
        }
    }
}

impl<S: tracing::Subscriber> Layer<S> for Tech {
    fn on_event(&self, event: &tracing::Event<'_>, _: Context<'_, S>) {
        let meta = event.metadata();
        let level = match *meta.level() {
            tracing::Level::ERROR => Level::Error,
            tracing::Level::WARN => Level::Warn,
            tracing::Level::INFO if self.info => Level::Info,
            _ => return,
        };
        let mut f = Fields::default();
        event.record(&mut f);
        // the GPU stack's chatter is not the editor's (its `log` lines name their crate in `log.target`)
        let gpu = |s: &str| ["wgpu", "naga"].iter().any(|p| s.starts_with(p));
        if level < Level::Error && (gpu(meta.target()) || f.rest.iter().any(|r| r.strip_prefix("log.target=").is_some_and(gpu))) {
            return;
        }
        // one line in the bar and the Log: what follows the first line goes to the detail
        let (text, more) = f.message.split_once('\n').map_or((f.message.as_str(), ""), |(a, b)| (a, b));
        let detail = std::iter::once(meta.target().to_string()).chain(f.rest).chain((!more.trim().is_empty()).then(|| more.split_whitespace().collect::<Vec<_>>().join(" "))).collect::<Vec<_>>().join(" ");
        if let Ok(mut inbox) = INBOX.lock() {
            inbox.push(Message::new(level, messages::TECH, text.trim_end(), now()).with_detail(detail).with_kind(Kind::Technical));
        }
    }
}

/// a press in the Log panel
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Press {
    Filter(Levels),
    Find,
    Copy,
    Save,
}

/// the messages, the Log's filter and find, the ring where a press was refused
#[derive(Default)]
pub struct Log {
    pub all: Messages,
    pub filter: Filter,
    pub find: String,
    refused: Option<((f32, f32), Instant)>,
    /// the conductor's newest log line taken in (its time and words)
    show_seen: Option<(u64, String)>,
}

impl Log {
    /// what was posted since the last frame, into the ring
    pub fn take_inbox(&mut self) {
        let posted = INBOX.lock().map(|mut i| std::mem::take(&mut *i)).unwrap_or_default();
        for m in posted {
            self.all.push(m);
        }
    }

    /// the conductor's log (deck-show): its lines not taken yet, as the show's messages
    pub fn take_show<'a>(&mut self, entries: impl Iterator<Item = &'a deck_show::conductor::Entry>) {
        let lines: Vec<(u64, String)> = entries.map(|e| (e.ms, deck_show::view::line(&e.what))).collect();
        let from = match &self.show_seen {
            Some(seen) => lines.iter().rposition(|l| l == seen).map_or(0, |i| i + 1),
            None => 0,
        };
        for (_, words) in &lines[from..] {
            // the conductor's words for what did not land
            let warn = words.contains("nobody") || words.contains("skipped") || words.contains("off the row");
            self.all.push(Message::new(if warn { Level::Warn } else { Level::Info }, "show", words.clone(), now()).with_kind(Kind::Show).with_scope(messages::SHOW_SCOPE));
        }
        if let Some(last) = lines.last() {
            self.show_seen = Some(last.clone());
        }
    }

    /// a press refused at (x, y): the ring there, the sentence to the bar
    pub fn refuse(&mut self, text: impl Into<String>, at: (f32, f32)) {
        self.all.push(Message::new(Level::Warn, "editor", text, now()));
        self.refused = Some((at, Instant::now()));
    }

    pub fn draw_ring(&mut self, d: &mut DrawList, t: &Tokens) {
        let Some((at, since)) = self.refused else { return };
        let k = since.elapsed().as_secs_f32() / RING_S;
        if k >= 1.0 {
            self.refused = None;
            return;
        }
        deck_kit::editor::status::refused_ring(d, t, at, k);
    }

    /// what Copy all and Save write: what the panel shows, every line
    pub fn text(&self) -> String {
        self.all.text(&self.filter, &self.find)
    }

    /// the log as a file for a bug report: in Downloads, else the editor's cache; where it went
    pub fn save(&self, cache: &std::path::Path) -> std::io::Result<std::path::PathBuf> {
        let home = std::env::var_os("USERPROFILE").or_else(|| std::env::var_os("HOME")).map(std::path::PathBuf::from);
        let dir = home.map(|h| h.join("Downloads")).filter(|d| d.is_dir()).unwrap_or_else(|| cache.join("logs"));
        std::fs::create_dir_all(&dir)?;
        let file = dir.join(format!("scutoons-editor-{}.log", now() as u64));
        std::fs::write(&file, self.text())?;
        Ok(file)
    }
}

pub fn dot(t: &Tokens, level: Level) -> [f32; 4] {
    match level {
        Level::Info => t.color.muted,
        Level::Ok => t.color.go,
        Level::Warn => t.color.warn,
        Level::Error => t.color.danger,
    }
}

/// the Log panel's content: the filters, the find field, Copy all and Save, then the lines newest first. `typing` is
/// the find field's text while it is typed in.
pub fn draw(d: &mut DrawList, t: &Tokens, a: Rect, log: &Log, typing: Option<&str>, pointer: (f32, f32)) -> Vec<(Rect, Press)> {
    let (pad, font, small) = (t.size.pad, t.size.font, t.size.small);
    let mut out = Vec::new();
    // the head: the filters left, Copy all and Save right, the find field between
    let names = Levels::ALL.map(Levels::word);
    let on = Levels::ALL.map(|f| f == log.filter.level);
    let bw = |d: &DrawList, s: &str| d.sans_width(small, s) + pad;
    let (copy_w, save_w) = (bw(d, "Copy all"), bw(d, "Save"));
    let filters_w = names.iter().map(|n| controls::badge_width(d, t, n) + pad * 0.5).sum::<f32>();
    let (rects, head) = controls::chips(d, t, (a.x, a.y, filters_w), &names, &on);
    out.extend(rects.into_iter().zip(Levels::ALL).map(|(r, f)| (r, Press::Filter(f))));
    let h = t.size.tree_row - 4.0;
    let mut x = a.x + a.w;
    for (word, w, press) in [("Save", save_w, Press::Save), ("Copy all", copy_w, Press::Copy)] {
        x -= w;
        let r = Rect { x, y: a.y, w, h };
        d.outline(r.x, r.y, r.w, r.h, (t.size.radius, 1.0), if r.has(pointer.0, pointer.1) { t.color.text } else { t.color.line });
        d.sans(r.x + pad * 0.5, r.y + (h - small * 1.25) * 0.5, small, word, t.color.text);
        out.push((r, press));
        x -= 4.0;
    }
    let find = Rect { x: a.x + filters_w + pad, y: a.y, w: (x - pad - (a.x + filters_w + pad)).max(0.0), h };
    if find.w > 40.0 {
        d.outline(find.x, find.y, find.w, find.h, (t.size.radius, 1.0), t.color.line);
        let (words, color) = match typing {
            Some(text) => (text.to_string(), t.color.text),
            None if !log.find.is_empty() => (log.find.clone(), t.color.text),
            None => ("Find".to_string(), t.color.muted),
        };
        if typing.is_none() {
            d.sans(find.x + pad * 0.5, find.y + (h - small * 1.25) * 0.5, small, &fit_sans(d, small, &words, find.w - pad), color);
        }
        out.push((find, Press::Find));
    }
    // the lines, newest first
    let find_now = typing.unwrap_or(&log.find);
    let shown = log.all.shown(&log.filter, find_now);
    let mut y = a.y + head.max(h) + small * 0.6;
    d.rect(a.x, y - small * 0.3, a.w, 1.0, t.color.line);
    if shown.is_empty() {
        d.sans(a.x, y + 2.0, small, if log.all.is_empty() { "Nothing said yet." } else { "Nothing matches." }, t.color.muted);
        return out;
    }
    let row = font * 1.45;
    let clock_w = d.advance(small) * 8.0 + pad * 0.5;
    for m in shown.iter().take(SHOWN) {
        d.text(a.x, y + (font - small) * 0.6, small, &messages::clock(m.at), t.color.muted);
        let r = t.size.rem(0.22);
        d.disc(a.x + clock_w + r, y + font * 0.62, r, dot(t, m.level));
        let mut x = a.x + clock_w + r * 2.0 + 6.0;
        let src = format!("{} ", m.source);
        x += d.sans(x, y + (font - small) * 0.6, small, &src, t.color.muted);
        let times = if m.times > 1 { format!(" ×{}", m.times) } else { String::new() };
        let words = format!("{}{times}", m.text);
        let tw = d.sans(x, y, font, &fit_sans(d, font, &words, a.x + a.w - x), if m.level >= Level::Warn { t.color.bright } else { t.color.text });
        if let Some(detail) = &m.detail {
            let dx = x + tw + pad * 0.5;
            if a.x + a.w - dx > 40.0 {
                d.sans(dx, y + (font - small) * 0.6, small, &fit_sans(d, small, detail, a.x + a.w - dx), t.color.muted);
            }
        }
        y += row;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_warning_and_an_error_from_tracing_reach_the_log_as_technical_lines() {
        use tracing_subscriber::layer::SubscriberExt;
        let subscriber = tracing_subscriber::registry().with(Tech::new(false));
        tracing::subscriber::with_default(subscriber, || {
            tracing::info!("not asked for");
            tracing::warn!(file = "x.toml", "autosave: the draft did not write");
            tracing::error!("sync: refused");
        });
        let mut log = Log::default();
        log.take_inbox();
        let tech: Vec<&Message> = log.all.shown(&Filter { level: Levels::Technical, ..Filter::default() }, "").into_iter().filter(|m| m.text != "not asked for").collect();
        assert!(tech.iter().any(|m| m.level == Level::Error && m.text == "sync: refused"));
        let w = tech.iter().find(|m| m.level == Level::Warn).expect("the warning");
        assert_eq!(w.text, "autosave: the draft did not write");
        assert!(w.detail.as_deref().is_some_and(|d| d.contains("file=x.toml")), "{:?}", w.detail);
        assert!(log.all.shown(&Filter::default(), "not asked for").is_empty(), "info only when asked");
    }

    #[test]
    fn the_conductors_lines_come_in_once() {
        use deck_show::conductor::{Entry, Logged};
        let mut entries = vec![Entry { ms: 10, what: Logged::NoNextRow }, Entry { ms: 20, what: Logged::Stopped { column: "left".into() } }];
        let mut log = Log::default();
        log.take_show(entries.iter());
        log.take_show(entries.iter());
        assert_eq!(log.all.len(), 2);
        entries.push(Entry { ms: 30, what: Logged::NoRow { event: "door".into() } });
        log.take_show(entries.iter());
        assert_eq!(log.all.shown(&Filter::default(), "").len(), 3);
    }
}
