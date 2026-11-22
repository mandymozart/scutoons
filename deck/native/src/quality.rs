//! The deck's quality settings on the stage (#220, owner 2026-11-01: "a detailed settings page for each quality
//! preset … where I can set the screen resolution … know the max possible on the hardware and then scales of the
//! rendered resolution so I can save power"): the preset file (deck-space `quality::Presets` as TOML – on a deck
//! `/var/lib/deck/quality.toml`, `--quality-file` / `DECK_QUALITY_FILE`), read at the start and again whenever it
//! changes (checked every 2 s), and the settings page in the HUD (F4): one table of the four presets – on, the render
//! sizes under load and its own (a scale of the display, or **P** for a height in pixels), FXAA, bloom, lights, the
//! frame rate cap – the level, the governor and the display mode (its modes from the monitor, the largest named as the
//! hardware's maximum). Every change applies at once and is saved. Pure: the page knows keys, not winit (main.rs maps them).
use deck_space::quality::{step_fps, Level, Presets, Size, MAX_LIGHTS};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

/// how often the file's time is looked at (s)
const CHECK_S: f32 = 2.0;

/// `--quality-file`, else DECK_QUALITY_FILE, else the deck's DECKDATA (/var/lib/deck/quality.toml) when it exists, else
/// next to the cache
pub fn default_path() -> PathBuf {
    if let Some(p) = std::env::var_os("DECK_QUALITY_FILE") {
        return p.into();
    }
    let deck = Path::new("/var/lib/deck");
    if deck.is_dir() {
        return deck.join("quality.toml");
    }
    let cache = crate::cache::default_dir();
    cache.parent().map_or_else(|| cache.clone(), Path::to_path_buf).join("deck-quality.toml")
}

/// The file: its presets now, and whether it changed since.
pub struct QualityFile {
    pub path: PathBuf,
    seen: Option<SystemTime>,
    checked: f32,
}

impl QualityFile {
    pub fn new(path: PathBuf) -> QualityFile {
        QualityFile { path, seen: None, checked: f32::NEG_INFINITY }
    }

    fn mtime(&self) -> Option<SystemTime> {
        std::fs::metadata(&self.path).and_then(|m| m.modified()).ok()
    }

    /// the presets in the file (none = the defaults; a broken file = the defaults and why)
    pub fn load(&mut self) -> (Presets, Option<String>) {
        self.seen = self.mtime();
        match std::fs::read_to_string(&self.path) {
            Ok(text) => match Presets::parse(&text) {
                Ok(p) => (p, None),
                Err(e) => (Presets::default(), Some(format!("{}: {e}", self.path.display()))),
            },
            Err(_) => (Presets::default(), None),
        }
    }

    pub fn save(&mut self, p: &Presets) -> Result<(), String> {
        if let Some(dir) = self.path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        std::fs::write(&self.path, p.to_text()).map_err(|e| format!("{}: {e}", self.path.display()))?;
        // our own write is no change from outside
        self.seen = self.mtime();
        Ok(())
    }

    /// at `now` (s): the file's presets if it changed since the last look (at most every CHECK_S)
    pub fn changed(&mut self, now: f32) -> Option<(Presets, Option<String>)> {
        if now - self.checked < CHECK_S && now >= self.checked {
            return None;
        }
        self.checked = now;
        (self.mtime() != self.seen).then(|| self.load())
    }
}

/// a key on the settings page
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Key {
    Up,
    Down,
    Left,
    Right,
    /// + / − (a toggle flips either way)
    More,
    Less,
    /// Enter / Space: flips a toggle
    Toggle,
    /// P: a render size between a scale and a height in pixels
    Pixels,
    /// R: every preset back to its default (the level, the governor and the display stay)
    Reset,
}

/// the page's rows
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Row {
    Level,
    Auto,
    Display,
    On,
    Min,
    Max,
    Fxaa,
    Bloom,
    Lights,
    Fps,
}

const ROWS: [Row; 10] = [Row::Level, Row::Auto, Row::Display, Row::On, Row::Min, Row::Max, Row::Fxaa, Row::Bloom, Row::Lights, Row::Fps];

/// The settings page: open or not, the cursor (row, the preset's column).
#[derive(Debug, Clone, Default)]
pub struct Page {
    pub open: bool,
    row: usize,
    col: usize,
}

impl Page {
    /// One key on `p`; `modes` = the display's sizes, largest first; `display_h` = the display's height now (a scale ↔
    /// pixels). Returns whether `p` changed (the caller applies and saves it).
    pub fn key(&mut self, k: Key, p: &mut Presets, modes: &[(u32, u32)], display_h: u32) -> bool {
        let level = Level::ALL[self.col];
        let up = match k {
            Key::Up => {
                self.row = self.row.checked_sub(1).unwrap_or(ROWS.len() - 1);
                return false;
            }
            Key::Down => {
                self.row = (self.row + 1) % ROWS.len();
                return false;
            }
            Key::Left => {
                self.col = self.col.checked_sub(1).unwrap_or(3);
                return false;
            }
            Key::Right => {
                self.col = (self.col + 1) % 4;
                return false;
            }
            Key::Reset => {
                *p = Presets { level: p.level, auto: p.auto, display: p.display.clone(), pixel_ratio: p.pixel_ratio, ..Presets::default() };
                return true;
            }
            Key::More | Key::Toggle | Key::Pixels => true,
            Key::Less => false,
        };
        let pr = p.get_mut(level);
        match (ROWS[self.row], k) {
            (Row::Min | Row::Max, Key::Pixels) => {
                let s = if ROWS[self.row] == Row::Min { &mut pr.min } else { &mut pr.max };
                *s = s.other_unit(display_h);
            }
            (_, Key::Pixels) => return false,
            (Row::Level, _) => {
                let now = p.level.unwrap_or(Level::High) as usize;
                p.level = Some(Level::ALL[if up { (now + 1).min(3) } else { now.saturating_sub(1) }]);
            }
            (Row::Auto, _) => p.auto = !p.auto,
            (Row::Display, _) => {
                // the desktop's (-1), then the modes from the largest (0) down: − a smaller one, + a larger one
                let at = p.display.as_deref().and_then(deck_space::quality::display_mode).and_then(|m| modes.iter().position(|x| *x == m));
                let i = at.map_or(-1, |i| i as isize);
                let j = if up { (i - 1).max(-1) } else { (i + 1).min(modes.len() as isize - 1) };
                p.display = usize::try_from(j).ok().and_then(|j| modes.get(j)).map(|(w, h)| format!("{w}x{h}"));
            }
            (Row::On, _) => pr.on = !pr.on,
            (Row::Fxaa, _) => pr.fxaa = !pr.fxaa,
            (Row::Bloom, _) => pr.bloom = !pr.bloom,
            (Row::Min | Row::Max, Key::Toggle) => return false,
            (Row::Min | Row::Max, _) => {
                let min = ROWS[self.row] == Row::Min;
                let s = if min { &mut pr.min } else { &mut pr.max };
                *s = s.step(up);
                // min never over max
                let (lo, hi) = (pr.min.scale_for(display_h), pr.max.scale_for(display_h));
                if lo > hi {
                    if min {
                        pr.max = pr.min;
                    } else {
                        pr.min = pr.max;
                    }
                }
            }
            (Row::Lights, Key::Toggle) | (Row::Fps, Key::Toggle) => return false,
            (Row::Lights, _) => pr.lights = if up { (pr.lights + 1).min(MAX_LIGHTS) } else { pr.lights.saturating_sub(1) },
            (Row::Fps, _) => {
                pr.fps = step_fps(pr.fps, up);
            }
        }
        true
    }

    /// The page as HUD lines (≤ 13 × 39): the title and keys, the level / governor / now, the display and the hardware's
    /// largest mode, the table (the cursor's cell in brackets), the status. Returns the lines and the cursor's line.
    pub fn lines(&self, p: &Presets, now: &str, modes: &[(u32, u32)], status: &str) -> (Vec<String>, usize) {
        let mut out = vec!["QUALITY  F4 CLOSE  +/- P=PX R=RESET".to_string()];
        let mark = |row: Row, text: String| if ROWS[self.row] == row { format!("[{text}]") } else { text };
        let level = p.level.map_or("GPU".to_string(), |l| l.name().to_uppercase());
        out.push(format!("{} {}  NOW {now}", mark(Row::Level, format!("LEVEL {level}")), mark(Row::Auto, format!("AUTO {}", if p.auto { "ON" } else { "OFF" }))));
        let max = modes.first().map_or("?".into(), |(w, h)| format!("{w}X{h}"));
        out.push(format!("{} MAX {max}", mark(Row::Display, format!("DISPLAY {}", p.display.as_deref().map_or("DESKTOP".into(), |d| d.to_uppercase())))));
        out.push(format!("         {}", Level::ALL.map(|l| format!("{:<6}", l.label())).concat()).trim_end().to_string());
        let cell = |row: Row, l: Level| -> String {
            let pr = p.get(l);
            let on_off = |b: bool| if b { "ON" } else { "OFF" }.to_string();
            match row {
                Row::On => on_off(pr.on),
                Row::Min => size(pr.min),
                Row::Max => size(pr.max),
                Row::Fxaa => on_off(pr.fxaa),
                Row::Bloom => on_off(pr.bloom),
                Row::Lights => pr.lights.to_string(),
                Row::Fps => pr.fps.to_string(),
                _ => String::new(),
            }
        };
        let mut cursor = 0;
        for (i, row) in ROWS.iter().enumerate().skip(3) {
            let name = format!("{row:?}").to_uppercase();
            let mut line = format!("{name:<8}");
            for (c, l) in Level::ALL.into_iter().enumerate() {
                let v = cell(*row, l);
                let v = if self.row == i && self.col == c { format!("[{v}]") } else { format!(" {v} ") };
                line.push_str(&format!("{v:<6}"));
            }
            if self.row == i {
                cursor = out.len();
            }
            out.push(line.trim_end().to_string());
        }
        if self.row < 3 {
            cursor = if self.row == 2 { 2 } else { 1 };
        }
        out.push(status.chars().take(39).collect());
        (out, cursor)
    }
}

/// a render size in ≤ 4 characters: .50 · 1.25 · 1080
fn size(s: Size) -> String {
    match s {
        Size::Scale(x) if x < 1.0 => format!("{x:.2}").trim_start_matches('0').to_string(),
        Size::Scale(x) => format!("{x:.2}"),
        Size::Height(h) => h.to_string(),
    }
}

/// the display's sizes from its modes, largest first, each once
pub fn sizes(mut modes: Vec<(u32, u32)>) -> Vec<(u32, u32)> {
    modes.sort_by_key(|&(w, h)| std::cmp::Reverse((w as u64 * h as u64, w)));
    modes.dedup();
    modes
}

#[cfg(test)]
mod tests {
    use super::*;

    const MODES: [(u32, u32); 3] = [(3840, 2160), (1920, 1080), (1280, 720)];

    #[test]
    fn the_page_edits_every_knob() {
        let mut page = Page::default();
        let mut p = Presets::default();
        let key = |page: &mut Page, p: &mut Presets, k| page.key(k, p, &MODES, 2160);
        // the level row: + from the GPU's (none = high) to ultra
        assert!(key(&mut page, &mut p, Key::More));
        assert_eq!(p.level, Some(Level::Ultra));
        key(&mut page, &mut p, Key::Down);
        key(&mut page, &mut p, Key::Toggle);
        assert!(!p.auto);
        // the display: the desktop → the largest mode down, + back up to the desktop
        key(&mut page, &mut p, Key::Down);
        key(&mut page, &mut p, Key::Less);
        assert_eq!(p.display.as_deref(), Some("3840x2160"));
        key(&mut page, &mut p, Key::Less);
        assert_eq!(p.display.as_deref(), Some("1920x1080"));
        key(&mut page, &mut p, Key::More);
        key(&mut page, &mut p, Key::More);
        assert_eq!(p.display, None);
        // ultra off
        key(&mut page, &mut p, Key::Down);
        key(&mut page, &mut p, Key::Left);
        key(&mut page, &mut p, Key::Toggle);
        assert!(!p.ultra.on);
        // high's own size: 1.0 → a height on this 4K display (2160) → down to 1800
        key(&mut page, &mut p, Key::Left);
        key(&mut page, &mut p, Key::Down);
        key(&mut page, &mut p, Key::Down);
        assert!(key(&mut page, &mut p, Key::Pixels));
        assert_eq!(p.high.max, Size::Height(2160));
        key(&mut page, &mut p, Key::Less);
        assert_eq!(p.high.max, Size::Height(1800));
        // min may not pass max: high's min (0.9 = 1944 px) is pulled down with it
        assert_eq!(p.high.min, Size::Height(1800));
        // fps down: 60 → 48; lights never past 8
        for _ in 0..3 {
            key(&mut page, &mut p, Key::Down);
        }
        key(&mut page, &mut p, Key::More);
        assert_eq!(p.high.lights, 8);
        key(&mut page, &mut p, Key::Down);
        key(&mut page, &mut p, Key::Less);
        assert_eq!(p.high.fps, 48);
        // reset: the presets, not the level
        assert!(key(&mut page, &mut p, Key::Reset));
        assert_eq!((p.high, p.level, p.auto), (Presets::default().high, Some(Level::Ultra), false));
    }

    #[test]
    fn the_page_fits_the_hud_and_marks_the_cursor() {
        let mut page = Page::default();
        let mut p = Presets::default();
        p.medium.max = Size::Height(1080);
        for _ in 0..4 {
            page.key(Key::Down, &mut p, &MODES, 2160);
        }
        page.key(Key::Right, &mut p, &MODES, 2160);
        let (lines, cursor) = page.lines(&p, "HIGH 0.90", &MODES, "SAVED");
        assert!(lines.len() <= crate::hud::LINES, "{lines:#?}");
        for l in &lines {
            assert!(l.chars().count() <= crate::hud::COLS, "{l}");
        }
        assert!(lines[2].contains("MAX 3840X2160"));
        assert!(lines[cursor].starts_with("MIN") && lines[cursor].contains("[.70]"), "{}", lines[cursor]);
        assert!(lines.iter().any(|l| l.starts_with("MAX") && l.contains("1080")));
    }

    #[test]
    fn the_page_reads_the_cores_lists() {
        use deck_space::quality::{FPS, SCALES};
        let (lines, _) = Page::default().lines(&Presets::default(), "", &MODES, "");
        let labels: Vec<&str> = Level::ALL.iter().map(|l| l.label()).collect();
        assert_eq!(lines[3].split_whitespace().collect::<Vec<_>>(), labels, "the header is the core's level labels");
        // + walks the core's lists: every fps cap and every scale, once each
        let (mut page, mut p) = (Page::default(), Presets::default());
        let walk: Vec<u32> = std::iter::successors(Some(FPS[0]), |f| Some(step_fps(*f, true)).filter(|n| n != f)).collect();
        assert_eq!(walk, FPS);
        p.low.min = Size::Scale(SCALES[0]);
        p.low.max = Size::Scale(*SCALES.last().unwrap());
        for _ in 0..4 {
            page.key(Key::Down, &mut p, &MODES, 2160);
        }
        page.key(Key::More, &mut p, &MODES, 2160);
        assert_eq!(p.low.min, Size::Scale(SCALES[1]), "the page's + is the core's step");
    }

    #[test]
    fn the_file_is_read_saved_and_watched() {
        let dir = std::env::temp_dir().join(format!("deck-quality-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let mut f = QualityFile::new(dir.join("quality.toml"));
        assert_eq!(f.load(), (Presets::default(), None), "no file: the defaults");
        let p = Presets { level: Some(Level::Low), ..Presets::default() };
        f.save(&p).unwrap();
        assert_eq!(f.load().0, p);
        assert!(f.changed(0.0).is_none(), "our own write");
        // Windows' file times tick every ~16 ms
        std::thread::sleep(std::time::Duration::from_millis(60));
        std::fs::write(&f.path, "level = \"ultra\"\n[low]\nfps = 30\n").unwrap();
        assert!(f.changed(1.0).is_none(), "not looked at again within 2 s");
        let (q, problem) = f.changed(2.5).expect("changed from outside");
        assert_eq!((q.level, q.low.fps, problem), (Some(Level::Ultra), 30, None));
        std::thread::sleep(std::time::Duration::from_millis(60));
        std::fs::write(&f.path, "level = 3").unwrap();
        let (q, problem) = f.changed(5.0).unwrap();
        assert!(q == Presets::default() && problem.is_some(), "a broken file: the defaults and why");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_sizes_largest_first() {
        assert_eq!(sizes(vec![(1280, 720), (3840, 2160), (1920, 1080), (3840, 2160)]), MODES);
    }
}
