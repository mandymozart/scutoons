//! The deck's flash side (#91 item 5, #471, SHOW.md §4b): a `flash` (the tree resolved to exact versions) goes to deck-show's
//! `ready::Deck`, which decides – taken or refused (left out, "please don't flash" without the operator's warning accepted),
//! the show this outlet plays, what it lacks, the own version kept when a flash replaced it, the notice and the switch, the
//! words. This thread only moves bytes, off the frame path:
//! - **the show**: `<cache>/shows/<id>@<v>.json` when it was kept (a version never changes), else
//!   `GET <api>/shows/<id>?v=<n>` with the deck token into that file – kept for the next flash, for a stick and for the
//!   way back to the deck's own version;
//! - **each piece** its outlet's cells name: deck-space `piece::available` through what the deck carries (piece.rs `find`);
//! - **the answer**: `flash.report` to the conductor (also when the deck joins: its mark, what it runs), the HUD's FLASH
//!   line, its reasons and the notice (deck-outlet's Status).
//!
//! A broken or missing piece is a reason, never a block: the conductor's GO goes on, each cell is compared as it comes.
//!
//! #481: the steps are deck-show's `Flashed` (the same as the browser deck's), and what it says to the person at the
//! deck – a suggestion, the notice, "please don't flash", a toast – is painted here by deck-show's `Face` (the component
//! library's panel on the CPU, off the frame path); the frame thread shows the picture (`Panel`) and hands keys and
//! taps back (`In::Key`, `In::Tap`).
use super::media::Media;
use super::Status;
use deck_show::dialog::Shown;
use deck_show::face::{Face, Pixels};
use deck_show::flashed::Flashed;
use deck_show::ready;
use deck_space::fetched::Fetched;
use protocol::{Flash, FlashReport, Message};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// What the flash side hears.
pub enum In {
    /// the relay's welcome: this deck's client id (the flash's lists name it so); the deck says what it is
    Joined(String),
    Flash(Box<Flash>),
    /// #471: the operator switches to the deck's own version (`true`) or back to the flashed one
    Switch(bool),
    /// #481: a key of the person at the deck, by the name the web gives it (deck-show `Face::key`)
    Key(String),
    /// #481: a tap or a click at (x, y) of the screen, in its pixels
    Tap(f32, f32),
}

/// #481: the deck's UI for the person at it, between this thread (it paints) and the frame thread (it shows).
#[derive(Default)]
pub struct Panel {
    /// a new picture until the frame thread took it; none inside = nothing shows any more
    pub next: Option<Option<Pixels>>,
    /// a dialog shows: the keys are its own
    pub dialog: bool,
    /// where what shows begins, in the screen's pixels from its top: a tap below it may be the panel's
    pub top: Option<f32>,
}

pub struct Flasher {
    pub flash: Flashed,
    /// a native deck plays what it carries (`Fetched::carried_only`)
    pub fetched: Fetched,
    pub face: Face,
    /// `https://api.scutoons.com/api`
    pub api: String,
    pub media: Arc<Media>,
    pub outbox: Sender<Message>,
    pub status: Arc<Mutex<Status>>,
    pub panel: Arc<Mutex<Panel>>,
    /// the screen's pixels, packed (mod.rs `pack`)
    pub size: Arc<AtomicU64>,
    pub stop: Arc<AtomicBool>,
}

impl Flasher {
    pub fn run(mut self, rx: Receiver<In>) {
        let started = Instant::now();
        self.show_notice();
        while !self.stop.load(Ordering::Relaxed) {
            // while something shows (a countdown runs, a toast goes by itself) the turns are short
            let wait = if self.flash.shown().is_some() { 50 } else { 500 };
            match rx.recv_timeout(Duration::from_millis(wait)) {
                Ok(In::Joined(id)) => self.flash.joined(&id),
                Ok(In::Flash(f)) => self.flash.flash(&f, &self.api, "", &mut self.fetched),
                Ok(In::Switch(own)) => self.flash.switch(own, &mut self.fetched),
                Ok(In::Key(name)) => {
                    self.face.key(&name, &mut self.flash, &mut self.fetched);
                }
                Ok(In::Tap(x, y)) => {
                    self.face.tap(x, y, &mut self.flash, &mut self.fetched);
                }
                Err(RecvTimeoutError::Timeout) => {}
                Err(RecvTimeoutError::Disconnected) => break,
            }
            self.turn(started.elapsed().as_secs_f32());
        }
    }

    /// one turn of the flash side: the show it asks for, its pieces, its reports, its picture
    fn turn(&mut self, now: f32) {
        if let Some((key, url)) = self.flash.ask() {
            let answer = self.document(&key, &url);
            self.flash.document(&key, answer.as_ref().map(|text| (200, text.as_str())).map_err(String::clone));
        }
        let cache = self.media.cache.clone();
        self.flash.turn(now, &mut self.fetched, |place| super::piece::find(&cache, place));
        for report in self.flash.reports() {
            self.say(report);
        }
        self.paint();
    }

    /// what the flash side says to the person at the deck, painted when it changed, for the frame thread
    fn paint(&mut self) {
        let size = self.size.load(Ordering::Relaxed);
        let shown = self.flash.shown();
        let next = self.face.paint(shown.as_ref(), ((size >> 32) as f32, size as u32 as f32), deck_outlet::text::mono());
        let mut panel = self.panel.lock().unwrap();
        if next.is_some() {
            panel.next = next;
        }
        (panel.dialog, panel.top) = (matches!(shown, Some(Shown::Dialog(_))), self.face.top());
    }

    /// the report to the HUD and the conductor
    fn say(&mut self, report: FlashReport) {
        let line = ready::line(&report);
        tracing::info!("flash: {line}");
        for l in report.lacks.iter().flatten() {
            tracing::warn!("flash: {l}");
        }
        {
            let mut st = self.status.lock().unwrap();
            st.flash = Some(line);
            st.flash_lacks = report.lacks.clone().unwrap_or_default();
        }
        self.show_notice();
        let _ = self.outbox.send(Message::FlashReport(report));
    }

    /// "please don't flash" and the notice after a flash replaced the deck's own version (deck-show's words)
    fn show_notice(&self) {
        self.status.lock().unwrap().flash_notice = self.flash.notice();
    }

    /// the show at its exact version as text: kept under `key` in the cache, else fetched from `url` and kept. Err =
    /// why it is not here.
    fn document(&self, key: &str, url: &str) -> Result<String, String> {
        let kept = self.media.cache.join(key);
        if !kept.exists() {
            let bearer = self.media.token.bearer()?;
            self.media.fetch.api_file(url, bearer.as_deref(), &kept)?;
        }
        let text = std::fs::read_to_string(&kept).map_err(|e| format!("{}: {e}", kept.display()))?;
        if ready::read(&text).is_err() {
            // what is no show is not kept
            let _ = std::fs::remove_file(&kept);
        }
        Ok(text)
    }
}
