//! The deck's flash side (#91 item 5, SHOW.md §4b): a `flash` (the tree resolved to exact versions) goes to deck-show's
//! `ready::Deck`, which decides – taken or refused, the show this outlet plays, what it lacks, the words. This thread
//! only moves bytes, off the frame path:
//! - **the show**: `<cache>/shows/<id>@<v>.json` when it was kept (a version never changes), else
//!   `GET <api>/shows/<id>?v=<n>` with the deck token into that file – kept for the next flash and for a stick;
//! - **each piece** its outlet's cells name: deck-space `piece::available` through what the deck carries (piece.rs `find`);
//! - **the answer**: `flash.report` to the conductor, the HUD's FLASH line and its reasons (deck-outlet's Status).
//!
//! A broken or missing piece is a reason, never a block: the conductor's GO goes on, each cell is compared as it comes.
use super::media::Media;
use super::Status;
use deck_show::ready::{self, Deck};
use deck_show::{Pinned, Show};
use protocol::{Flash, Message};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex};
use std::time::Duration;

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
    pub fn run(mut self, rx: Receiver<Flash>) {
        while !self.stop.load(Ordering::Relaxed) {
            match rx.recv_timeout(Duration::from_millis(500)) {
                Ok(f) => self.one(&f),
                Err(RecvTimeoutError::Timeout) => {}
                Err(RecvTimeoutError::Disconnected) => break,
            }
        }
    }

    fn one(&mut self, flash: &Flash) {
        let report = match self.deck.take(flash) {
            Err(refused) => refused,
            Ok(show) => {
                let doc = self.document(&show);
                let cache = self.media.cache.clone();
                self.deck.check(&show, doc, |p| deck_space::piece::available(p, |place| super::piece::find(&cache, place)))
            }
        };
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
        let _ = self.outbox.send(Message::FlashReport(report));
    }

    /// the show at its exact version: kept, else fetched and kept. Err = why it is not here.
    fn document(&self, p: &Pinned) -> Result<Show, String> {
        let kept: PathBuf = self.media.cache.join(ready::kept_as(p));
        if !kept.exists() {
            let url = format!("{}/shows/{}?v={}", self.api, p.id, p.version);
            let bearer = self.media.token.bearer()?;
            self.media.fetch.get(&url, bearer.as_deref(), &kept)?;
        }
        let text = std::fs::read_to_string(&kept).map_err(|e| format!("{}: {e}", kept.display()))?;
        ready::read(&text).inspect_err(|_| {
            // what is no show is not kept
            let _ = std::fs::remove_file(&kept);
        })
    }
}
