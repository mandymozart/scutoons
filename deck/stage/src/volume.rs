//! The deck's hardware volume keys (#253): AudioVolumeUp / Down / Mute change the **PipeWire default sink's** volume
//! through `wpctl` (so Chromium on F2 and everything else follow, and WirePlumber keeps the level), and the level shows
//! in the HUD for about a second. No window, no GPU here: the command lines, the parsing of `wpctl get-volume` and the
//! overlay's timing; main.rs only wires the keys and draws the text.
use std::process::Command;
use std::sync::mpsc::{channel, Receiver, Sender};

/// the sink every command targets
const SINK: &str = "@DEFAULT_AUDIO_SINK@";
/// one key press moves the volume by this much
pub const STEP: &str = "5%";
/// the volume never goes above 100 % (wpctl's `-l 1.0`)
pub const LIMIT: &str = "1.0";
/// how long the overlay stays after the last key (seconds)
pub const SHOW_SECS: f32 = 1.2;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Key {
    Up,
    Down,
    Mute,
}

/// what `wpctl get-volume` says
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Level {
    /// 0.0 – 1.0 (wpctl's scale, 1.0 = 100 %)
    pub volume: f32,
    pub muted: bool,
}

/// the arguments of the `wpctl` call a key makes
pub fn set_args(key: Key) -> Vec<String> {
    let s = |v: &[&str]| v.iter().map(|x| x.to_string()).collect();
    match key {
        Key::Up => s(&["set-volume", SINK, &format!("{STEP}+"), "-l", LIMIT]),
        Key::Down => s(&["set-volume", SINK, &format!("{STEP}-"), "-l", LIMIT]),
        Key::Mute => s(&["set-mute", SINK, "toggle"]),
    }
}

/// the arguments that read the level back
pub fn get_args() -> Vec<String> {
    vec!["get-volume".into(), SINK.into()]
}

/// "Volume: 0.45" / "Volume: 0.45 [MUTED]" → the level
pub fn parse(out: &str) -> Option<Level> {
    let rest = out.lines().find_map(|l| l.trim().strip_prefix("Volume:"))?;
    let volume = rest.split_whitespace().next()?.parse::<f32>().ok()?;
    if !volume.is_finite() || volume < 0.0 {
        return None;
    }
    Some(Level { volume, muted: rest.contains("[MUTED]") })
}

/// the HUD text: `VOL ########.... 80%` or `VOL MUTED 80%`
pub fn text(l: Level) -> String {
    let pct = (l.volume * 100.0).round() as u32;
    if l.muted {
        return format!("VOL MUTED {pct}%");
    }
    let on = ((l.volume.min(1.0) * 10.0).round() as usize).min(10);
    format!("VOL {}{} {pct}%", "#".repeat(on), ".".repeat(10 - on))
}

/// the overlay: the last level and when it appeared
#[derive(Default)]
pub struct Overlay {
    shown: Option<(f32, Level)>,
}

impl Overlay {
    pub fn show(&mut self, now: f32, level: Level) {
        self.shown = Some((now, level));
    }

    /// the text while it is on (a clock that went back counts as expired)
    pub fn text(&self, now: f32) -> Option<String> {
        self.shown.filter(|(t, _)| now >= *t && now - t < SHOW_SECS).map(|(_, l)| text(l))
    }

    pub fn active(&self, now: f32) -> bool {
        self.text(now).is_some()
    }
}

/// runs the wpctl calls off the frame thread; the levels come back through `poll`
pub struct Volume {
    tx: Sender<Level>,
    rx: Receiver<Level>,
}

impl Default for Volume {
    fn default() -> Self {
        let (tx, rx) = channel();
        Volume { tx, rx }
    }
}

impl Volume {
    /// a key press: set, read back, report (a deck without wpctl logs it once per press and shows nothing)
    pub fn press(&self, key: Key) {
        let tx = self.tx.clone();
        std::thread::spawn(move || {
            match Command::new("wpctl").args(set_args(key)).status() {
                Ok(s) if s.success() => {}
                Ok(s) => return tracing::warn!("volume: wpctl {key:?} -> {s}"),
                Err(e) => return tracing::warn!("volume: wpctl {key:?}: {e}"),
            }
            match Command::new("wpctl").args(get_args()).output() {
                Ok(o) => match parse(&String::from_utf8_lossy(&o.stdout)) {
                    Some(l) => {
                        let _ = tx.send(l);
                    }
                    None => tracing::warn!("volume: no level in {:?}", String::from_utf8_lossy(&o.stdout)),
                },
                Err(e) => tracing::warn!("volume: wpctl get-volume: {e}"),
            }
        });
    }

    /// the newest level that came back, if any
    pub fn poll(&self) -> Option<Level> {
        self.rx.try_iter().last()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command_lines() {
        assert_eq!(set_args(Key::Up), ["set-volume", "@DEFAULT_AUDIO_SINK@", "5%+", "-l", "1.0"]);
        assert_eq!(set_args(Key::Down), ["set-volume", "@DEFAULT_AUDIO_SINK@", "5%-", "-l", "1.0"]);
        assert_eq!(set_args(Key::Mute), ["set-mute", "@DEFAULT_AUDIO_SINK@", "toggle"]);
        assert_eq!(get_args(), ["get-volume", "@DEFAULT_AUDIO_SINK@"]);
    }

    #[test]
    fn parses_wpctl_output() {
        assert_eq!(parse("Volume: 0.45\n"), Some(Level { volume: 0.45, muted: false }));
        assert_eq!(parse("Volume: 0.30 [MUTED]\n"), Some(Level { volume: 0.30, muted: true }));
        assert_eq!(parse("Volume: 1.00"), Some(Level { volume: 1.0, muted: false }));
        assert_eq!(parse(""), None);
        assert_eq!(parse("Volume: abc"), None);
        assert_eq!(parse("Volume: -1"), None);
    }

    #[test]
    fn text_shows_level_and_mute() {
        assert_eq!(text(Level { volume: 0.5, muted: false }), "VOL #####..... 50%");
        assert_eq!(text(Level { volume: 0.0, muted: false }), "VOL .......... 0%");
        assert_eq!(text(Level { volume: 1.0, muted: false }), "VOL ########## 100%");
        assert_eq!(text(Level { volume: 0.4, muted: true }), "VOL MUTED 40%");
    }

    #[test]
    fn overlay_lasts_about_a_second() {
        let mut o = Overlay::default();
        assert!(!o.active(0.0));
        o.show(10.0, Level { volume: 0.5, muted: false });
        assert!(o.active(10.0));
        assert!(o.active(10.0 + SHOW_SECS - 0.01));
        assert!(!o.active(10.0 + SHOW_SECS + 0.01));
        // a new press restarts it
        o.show(11.0, Level { volume: 0.55, muted: false });
        assert!(o.text(11.5).is_some_and(|t| t.ends_with("55%")));
        assert!(o.active(12.1));
        assert!(!o.active(12.3));
    }
}
