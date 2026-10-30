//! A Live outlet on this deck (#186, research/LIVE-LENSES.md, CYBERDECKS.md §8): the scene's `outlet` node shows the
//! cells the conductor sends to this deck's outlet, natively – no browser per outlet (DECISIONS 2026-10-30).
//!
//! Three threads, none on the frame path:
//! - **client** (client.rs): the relay as `output` of the outlet – cells, preloads, the show's slice in; `media.ready`
//!   out; resume / snapshot on a reconnect.
//! - **preloader**: `media.preload` → fetched with the deck token into `<cache>/live/` (media.rs) → `media.ready` (the
//!   conductor's ready light); what is missing is tried again (2, 5, 10, 20, then every 30 s).
//! - **composer**: a cell → its plan (plan.rs: layers → stage nodes) → one plane (compose.rs) → the renderer's
//!   `outlet` pass, which cross-fades it over its `in` (the scene's space node, whose room and spot a 3D set picks –
//!   `SpaceTarget`). A scroll steps its slices every 6 s; surtitles follow the show's slice; new media re-compose.
//!
//! The renderer's side (frame.rs: `Frame`, `Crossfade`) is built with or without the `live` module.
pub mod frame;

#[cfg(feature = "live")]
pub mod client;
#[cfg(feature = "live")]
pub mod compose;
#[cfg(feature = "live")]
pub mod config;
#[cfg(feature = "live")]
pub mod media;
#[cfg(feature = "live")]
pub mod plan;

#[cfg(feature = "live")]
pub use running::*;

/// What the HUD says about the outlet (each part written by its thread).
#[derive(Debug, Clone, Default)]
pub struct Status {
    /// the relay: joining, the client id, offline and why
    pub link: String,
    /// the preload: n / m ready, the first missing one's reason
    pub media: String,
    /// the cell: its layers, fading
    pub cell: String,
    /// what the current cell can't show (layer: reason)
    pub problems: Vec<String>,
    /// the relay's last refusal
    pub problems_relay: Option<String>,
}

impl Status {
    /// the HUD's lines (font8x8: upper case, ASCII dashes)
    pub fn lines(&self, outlet: &str) -> Vec<String> {
        let up = |s: &str| s.replace(['–', '—'], "-").to_uppercase();
        let mut out = vec![up(&format!("LIVE {outlet}  {}", self.link))];
        if !self.media.is_empty() {
            out.push(up(&format!("MEDIA {}", self.media)));
        }
        if !self.cell.is_empty() {
            out.push(up(&format!("CELL {}", self.cell)));
        }
        out.extend(self.problems.iter().chain(&self.problems_relay).map(|p| up(&format!("! {p}"))));
        out
    }
}

/// What the composer thread reacts to.
#[cfg(feature = "live")]
#[derive(Debug, Clone)]
pub enum Input {
    Cell(protocol::Cell),
    /// the show's slice (`state.slice`) – surtitles follow it
    Slice(usize),
    /// panic black
    Black,
    /// the preloader has new files: a cell waiting for them is composed again
    Media,
}

#[cfg(feature = "live")]
mod running {
    use super::compose::{Composed, Composer};
    use super::config::Config;
    use super::frame::{Frame, SpaceTarget};
    use super::media::{Media, RETRY_S};
    use super::plan::{plan, Plan};
    use super::{client, Input, Status};
    use crate::library::Library;
    use protocol::{Cell, CellMode, MediaPreload, Message};
    use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
    use std::sync::mpsc::{channel, Receiver, RecvTimeoutError, Sender, SyncSender, TrySendError};
    use std::sync::{mpsc, Arc, Mutex};
    use std::time::{Duration, Instant};

    /// how long a scroll layer holds a slice (the chapter node's default hold)
    pub const SCROLL_HOLD: Duration = Duration::from_secs(6);

    /// The outlet on this deck: its threads, what they hand the renderer and the space node, the HUD's status.
    pub struct Live {
        pub outlet: String,
        /// the composed cells → the renderer's outlet pass (taken by it; a channel of two – the next cell waits)
        pub frames: Option<Receiver<Frame>>,
        /// a 3D set's room and spot → the scene's space node (main.rs applies it)
        pub space: Receiver<SpaceTarget>,
        pub status: Arc<Mutex<Status>>,
        size: Arc<AtomicU64>,
        stop: Arc<AtomicBool>,
        rejoin: Arc<AtomicBool>,
    }

    impl Live {
        /// Start the client, the preloader and the composer for `cfg`'s outlet; `size` = the plane's pixels (the window's
        /// render size – `resize` follows it).
        pub fn start(cfg: Config, lib: Library, media: Media, size: (u32, u32), nick: Option<String>) -> Live {
            let status = Arc::new(Mutex::new(Status { link: format!("JOINING {}", cfg.join_url()), ..Default::default() }));
            let stop = Arc::new(AtomicBool::new(false));
            let rejoin = Arc::new(AtomicBool::new(false));
            let size = Arc::new(AtomicU64::new(pack(size)));
            let media = Arc::new(media);
            let (to_compose, compose_rx) = channel::<Input>();
            let (to_preload, preload_rx) = channel::<MediaPreload>();
            let (outbox, outbox_rx) = channel::<Message>();
            let (frames_tx, frames) = mpsc::sync_channel::<Frame>(2);
            let (space_tx, space) = channel::<SpaceTarget>();

            let preloader = Preloader { media: media.clone(), outlet: cfg.outlet.clone(), outbox, compose: to_compose.clone(), status: status.clone(), stop: stop.clone() };
            spawn("live-preload", move || preloader.run(preload_rx));
            let composer = Worker {
                composer: Composer::new(lib, media, size_of(&size)),
                locale: cfg.locale.clone(),
                frames: frames_tx,
                space: space_tx,
                status: status.clone(),
                stop: stop.clone(),
                size: size.clone(),
            };
            spawn("live-compose", move || composer.run(compose_rx));
            let client = client::Client {
                cfg: cfg.clone(),
                nick,
                compose: to_compose,
                preload: to_preload,
                outbox: outbox_rx,
                status: status.clone(),
                stop: stop.clone(),
                rejoin: rejoin.clone(),
            };
            spawn("live-relay", move || client.run());
            Live { outlet: cfg.outlet, frames: Some(frames), space, status, size, stop, rejoin }
        }

        /// the plane's size follows the window's render size (the cell is composed again)
        pub fn resize(&self, w: u32, h: u32) {
            self.size.store(pack((w, h)), Ordering::Relaxed);
        }

        /// drop the relay connection and join again with resume (a network change; the tests)
        pub fn rejoin(&self) {
            self.rejoin.store(true, Ordering::Relaxed);
        }

        pub fn hud_lines(&self) -> Vec<String> {
            self.status.lock().unwrap().lines(&self.outlet)
        }
    }

    impl Drop for Live {
        fn drop(&mut self) {
            self.stop.store(true, Ordering::Relaxed);
        }
    }

    fn spawn(name: &str, f: impl FnOnce() + Send + 'static) {
        std::thread::Builder::new().name(name.into()).spawn(f).expect("a live thread");
    }

    fn pack((w, h): (u32, u32)) -> u64 {
        ((w.max(16) as u64) << 32) | h.max(16) as u64
    }

    fn size_of(a: &AtomicU64) -> (u32, u32) {
        let v = a.load(Ordering::Relaxed);
        ((v >> 32) as u32, v as u32)
    }

    struct Preloader {
        media: Arc<Media>,
        outlet: String,
        outbox: Sender<Message>,
        compose: Sender<Input>,
        status: Arc<Mutex<Status>>,
        stop: Arc<AtomicBool>,
    }

    impl Preloader {
        fn run(self, rx: Receiver<MediaPreload>) {
            let mut job: Option<MediaPreload> = None;
            let mut attempt = 0usize;
            let mut due: Option<Instant> = None;
            while !self.stop.load(Ordering::Relaxed) {
                let wait = due.map_or(Duration::from_millis(200), |d| d.saturating_duration_since(Instant::now()).min(Duration::from_millis(200)));
                match rx.recv_timeout(wait) {
                    Ok(p) => {
                        job = Some(p);
                        attempt = 0;
                        due = Some(Instant::now());
                    }
                    Err(RecvTimeoutError::Timeout) => {}
                    Err(RecvTimeoutError::Disconnected) => break,
                }
                let (Some(p), Some(at)) = (&job, due) else { continue };
                if Instant::now() < at {
                    continue;
                }
                self.status.lock().unwrap().media = format!("LOADING {} ITEMS", p.items.len());
                let ready = self.media.preload(&self.outlet, &p.items);
                let line = match ready.missing.first() {
                    None => format!("{} / {} READY", ready.ready.len(), p.items.len()),
                    Some(id) => {
                        let why = ready.why.as_ref().and_then(|w| w.get(id)).cloned().unwrap_or_default();
                        let again = RETRY_S[attempt.min(RETRY_S.len() - 1)];
                        format!("{} / {} READY - {id}: {why} - AGAIN IN {again} S", ready.ready.len(), p.items.len())
                    }
                };
                self.status.lock().unwrap().media = line;
                due = (!ready.missing.is_empty()).then(|| Instant::now() + Duration::from_secs(RETRY_S[attempt.min(RETRY_S.len() - 1)]));
                attempt += 1;
                let _ = self.compose.send(Input::Media);
                let _ = self.outbox.send(Message::MediaReady(ready));
            }
        }
    }

    struct Worker {
        composer: Composer,
        locale: String,
        frames: SyncSender<Frame>,
        space: Sender<SpaceTarget>,
        status: Arc<Mutex<Status>>,
        stop: Arc<AtomicBool>,
        size: Arc<AtomicU64>,
    }

    impl Worker {
        fn carried(m: &str) -> bool {
            crate::modules::built(m) == Some(true)
        }

        fn run(mut self, rx: Receiver<Input>) {
            let mut cell: Option<Cell> = None;
            let mut planned = Plan::default();
            let mut slice: Option<usize> = None;
            let mut scroll = 0usize;
            let mut next_step: Option<Instant> = None;
            let mut space_sent: Option<SpaceTarget> = None;
            let mut waiting_for_media = false;
            let mut pending: Option<Frame> = None;
            while !self.stop.load(Ordering::Relaxed) {
                // a channel of two: the renderer takes one per frame; full = it isn't drawing (the browser on top) –
                // the newest plane waits its turn, an older waiting one is dropped
                if let Some(f) = pending.take() {
                    match self.frames.try_send(f) {
                        Ok(()) => {}
                        Err(TrySendError::Full(f)) => pending = Some(f),
                        Err(TrySendError::Disconnected(_)) => break,
                    }
                }
                let wait = next_step.map_or(Duration::from_millis(200), |t| t.saturating_duration_since(Instant::now()).min(Duration::from_millis(200)));
                let mut fresh = false;
                let mut step = false;
                match rx.recv_timeout(wait) {
                    Ok(Input::Cell(c)) => {
                        planned = plan(&c, &self.locale, Self::carried);
                        cell = Some(c);
                        scroll = 0;
                        fresh = true;
                    }
                    Ok(Input::Black) => {
                        let black = Cell { mode: CellMode::Black, layers: None, of: None, send: None };
                        planned = plan(&black, &self.locale, Self::carried);
                        cell = Some(black);
                        fresh = true;
                    }
                    Ok(Input::Slice(s)) => {
                        let changed = slice != Some(s);
                        slice = Some(s);
                        step = changed && planned.follows_the_slice();
                    }
                    Ok(Input::Media) => step = waiting_for_media,
                    Err(RecvTimeoutError::Timeout) => {}
                    Err(RecvTimeoutError::Disconnected) => break,
                }
                let size = size_of(&self.size);
                if size != self.composer.size && cell.is_some() {
                    self.composer.size = size;
                    step = true;
                }
                if next_step.is_some_and(|t| Instant::now() >= t) {
                    scroll += 1;
                    step = true;
                }
                if !(fresh || step) || cell.is_none() {
                    continue;
                }
                let t = Instant::now();
                let out: Composed = if fresh { self.composer.compose(&planned, slice, scroll) } else { self.composer.scroll_step(&planned, slice, scroll) };
                next_step = planned.scrolls().then(|| Instant::now() + SCROLL_HOLD);
                waiting_for_media = out.problems.iter().any(|p| p.contains("not preloaded"));
                if out.space != space_sent {
                    if let Some(s) = &out.space {
                        let _ = self.space.send(s.clone());
                    }
                    space_sent = out.space.clone();
                }
                {
                    let mut st = self.status.lock().unwrap();
                    st.cell = format!(
                        "{} LAYERS{}{} {:.0} MS",
                        planned.layers.len(),
                        if out.frame.backdrop { " + SPACE" } else { "" },
                        if planned.scrolls() { format!(" SCROLL {scroll}") } else { String::new() },
                        t.elapsed().as_secs_f32() * 1000.0
                    );
                    st.problems = out.problems;
                }
                pending = Some(out.frame);
            }
        }
    }
}
