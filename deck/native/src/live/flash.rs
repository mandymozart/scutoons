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
use super::media::Media;
use super::Status;
use deck_show::ready::{self, Deck};
use deck_show::{Pinned, Show};
use protocol::{Flash, FlashReport, Message};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// What the flash side hears.
pub enum In {
    /// the relay's welcome: this deck's client id (the flash's lists name it so); the deck says what it is
    Joined(String),
    Flash(Box<Flash>),
    /// #471: the operator switches to the deck's own version (`true`) or back to the flashed one
    Switch(bool),
}

pub struct Flasher {
    pub deck: Deck,
    /// `https://api.scutoons.com/api`
    pub api: String,
    pub media: Arc<Media>,
    pub outbox: Sender<Message>,
    pub status: Arc<Mutex<Status>>,
    pub stop: Arc<AtomicBool>,
}

impl Flasher {
    pub fn run(mut self, rx: Receiver<In>) {
        self.show_notice();
        while !self.stop.load(Ordering::Relaxed) {
            match rx.recv_timeout(Duration::from_millis(500)) {
                Ok(In::Joined(id)) => {
                    self.deck.set_id(&id);
                    let r = self.deck.about();
                    self.say(r);
                }
                Ok(In::Flash(f)) => self.one(&f),
                Ok(In::Switch(own)) => match self.deck.switch(own) {
                    Ok(show) => {
                        let r = self.check(&show);
                        self.say(r);
                    }
                    Err(why) => {
                        tracing::warn!("flash: {why}");
                        self.status.lock().unwrap().flash_notice.push(why);
                    }
                },
                Err(RecvTimeoutError::Timeout) => {}
                Err(RecvTimeoutError::Disconnected) => break,
            }
        }
    }

    fn one(&mut self, flash: &Flash) {
        let report = match self.deck.take(flash) {
            Err(refused) => *refused,
            Ok(show) => self.check(&show),
        };
        self.say(report);
    }

    fn check(&mut self, show: &Pinned) -> FlashReport {
        let cache = self.media.cache.clone();
        let has = |p: &str| deck_space::piece::available(p, |place| super::piece::find(&cache, place));
        // #475: the flash said what this outlet plays – no document is read (a deck whose member has no `live` cannot)
        if let Some(report) = self.deck.check_told(show, has) {
            return report;
        }
        let doc = self.document(show);
        self.deck.check(show, doc, has)
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
        let lines: Vec<String> = self.deck.keep_line().into_iter().chain(self.deck.notice()).collect();
        self.status.lock().unwrap().flash_notice = lines;
    }

    /// the show at its exact version: kept, else fetched and kept. Err = why it is not here.
    fn document(&self, p: &Pinned) -> Result<Show, String> {
        let kept: PathBuf = self.media.cache.join(ready::kept_as(p));
        if !kept.exists() {
            let url = format!("{}/shows/{}?v={}", self.api, p.id, p.version);
            let bearer = self.media.token.bearer()?;
            self.media.fetch.api_file(&url, bearer.as_deref(), &kept)?;
        }
        let text = std::fs::read_to_string(&kept).map_err(|e| format!("{}: {e}", kept.display()))?;
        ready::read(&text).inspect_err(|_| {
            // what is no show is not kept
            let _ = std::fs::remove_file(&kept);
        })
    }
}
