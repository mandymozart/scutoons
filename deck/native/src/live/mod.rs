//! A Live outlet on this deck (#186, research/LIVE-LENSES.md, CYBERDECKS.md §8): the scene's `outlet` node shows the
//! cells the conductor sends to this deck's outlet, natively – no browser per outlet (DECISIONS 2026-10-30).
//!
//! Three threads, none on the frame path:
//! - **client** (client.rs): the relay as `output` of the outlet – cells, preloads, the show's slice in (deck-outlet's
//!   route.rs says where each goes); `media.ready` out; resume / snapshot on a reconnect.
//! - **preloader**: `media.preload` → fetched with the deck token into `<cache>/live/` (media.rs) → `media.ready` (the
//!   conductor's ready light); what is missing is tried again (deck-outlet's preload.rs: 2, 5, 10, 20, then every 30 s).
//! - **composer**: a cell → its plan (deck-outlet's plan.rs: layers → stage nodes) → one plane (deck-outlet's compose.rs,
//!   reading through compose.rs here: the library and the preloaded media) → the renderer's `outlet` pass, which
//!   cross-fades it over its `in` (the scene's space node, whose set and spot a 3D set picks – `SpaceTarget`). When it
//!   composes again (a scroll steps its slices every 6 s, surtitles follow the show's slice, new media, a resize) is
//!   deck-outlet's recompose.rs.
//!
//! The decisions and the HUD's words are deck-outlet's (#391); this module is the threads, the channels and the clock
//! (`Instant`, passed in as the time since the thread's start).
//!
//! The renderer's side (deck-outlet's frame.rs: `Frame`, `Crossfade`) is built with or without the `live` module.
#[cfg(feature = "live")]
pub mod client;
#[cfg(feature = "live")]
pub mod compose;
#[cfg(feature = "live")]
pub mod config;
#[cfg(feature = "live")]
pub mod media;
#[cfg(feature = "live")]
pub mod piece;
#[cfg(feature = "live")]
pub mod quic;
#[cfg(feature = "live")]
pub mod socket;

/// What the HUD says about the outlet (each part written by its thread; the wording is deck-outlet's, #391).
pub use deck_outlet::status::Status;

#[cfg(feature = "live")]
pub use running::*;

#[cfg(feature = "live")]
mod running {
    use super::compose::{self, Composer};
    use super::config::Config;
    use super::media::Media;
    use super::{client, Status};
    use crate::library::Library;
    use deck_outlet::frame::{Frame, SpaceTarget};
    use deck_outlet::preload::Preload;
    use deck_outlet::recompose::Recompose;
    use deck_outlet::route::Input;
    use protocol::{MediaPreload, Message};
    use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
    use std::sync::mpsc::{channel, Receiver, RecvTimeoutError, Sender, SyncSender, TrySendError};
    use std::sync::{mpsc, Arc, Mutex};
    use std::time::Instant;

    /// The outlet of a scene with an `outlet` node: deck.toml under the CLI's settings → the relay and outlet; the media
    /// with the deck's resolver (`sets`; `synced` = the sync fetches a set's 3D), HTTP and the token (`DECK_TOKEN`, else what
    /// deck-link's daemon keeps in /run/deck-link – #180 –, else none); then the threads. Err = why the outlet doesn't
    /// play (the HUD says it).
    pub fn start_outlet(cli: super::config::Settings, lib: Library, sets: std::path::PathBuf, synced: bool, size: (u32, u32)) -> Result<Live, String> {
        use super::config::{Settings, DECK_TOML};
        use super::media::{HttpFetch, Linked, LocalSets};
        let settings = Settings::read(std::path::Path::new(DECK_TOML)).merged(cli);
        let cfg = settings
            .config()
            .ok_or("no outlet: set relay + outlet in /var/lib/deck/deck.toml (or --relay / --outlet)")?
            .map_err(|e| format!("{e:#}"))?;
        let media = Media {
            // show media's signed links (#470): the library's API (DECK_API, else production – main.rs)
            api: lib.api.clone(),
            cache: lib.cache.clone(),
            resolve: Box::new(LocalSets { sets, quality: "low".into(), synced }),
            fetch: Box::new(HttpFetch::default()),
            token: Box::new(Linked(deck_link::TokenSource::from_env())),
            local: Mutex::new(Default::default()),
        };
        tracing::info!("live: outlet '{}' of session {} via {}", cfg.outlet, cfg.code, cfg.base);
        Ok(Live::start(cfg, lib, media, size, hostname()))
    }

    fn hostname() -> Option<String> {
        std::fs::read_to_string("/etc/hostname").map(|h| h.trim().to_owned()).or_else(|_| std::env::var("COMPUTERNAME")).ok().filter(|h| !h.is_empty())
    }

    /// The outlet on this deck: its threads, what they hand the renderer and the space node, the HUD's status.
    pub struct Live {
        pub outlet: String,
        /// the composed cells → the renderer's outlet pass (taken by it; a channel of two – the next cell waits)
        pub frames: Option<Receiver<Frame>>,
        /// a 3D set's file and spot → the scene's space node (main.rs applies it)
        pub space: Receiver<SpaceTarget>,
        /// #91: the outlet's cells → the deck's piece side (piece.rs, on the frame thread)
        pub pieces: Receiver<protocol::Cell>,
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
            let (pieces_tx, pieces) = channel::<protocol::Cell>();

            let preloader = Preloader { media: media.clone(), outlet: cfg.outlet.clone(), outbox, compose: to_compose.clone(), status: status.clone(), stop: stop.clone() };
            spawn("live-preload", move || preloader.run(preload_rx));
            let composer = Worker {
                composer: compose::composer(lib, media, size_of(&size)),
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
                pieces: pieces_tx,
                outbox: outbox_rx,
                status: status.clone(),
                stop: stop.clone(),
                rejoin: rejoin.clone(),
            };
            spawn("live-relay", move || client.run());
            Live { outlet: cfg.outlet, frames: Some(frames), space, pieces, status, size, stop, rejoin }
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
            // the decisions (when to fetch, the retry schedule, the HUD's line) are deck-outlet's `Preload`; the clock is ours
            let start = Instant::now();
            let mut machine = Preload::default();
            while !self.stop.load(Ordering::Relaxed) {
                match rx.recv_timeout(machine.wait(start.elapsed())) {
                    Ok(p) => machine.job(p, start.elapsed()),
                    Err(RecvTimeoutError::Timeout) => {}
                    Err(RecvTimeoutError::Disconnected) => break,
                }
                let Some(p) = machine.due(start.elapsed()) else { continue };
                self.status.lock().unwrap().media = machine.loading_line();
                let ready = self.media.preload(&self.outlet, &p.items);
                self.status.lock().unwrap().media = machine.done(&ready, start.elapsed());
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
            // when to compose again (a cell, black, the slice, media, a resize, the scroll's hold) is deck-outlet's `Recompose`;
            // the thread, the composer and the clock are ours
            let start = Instant::now();
            let mut machine = Recompose::new(&self.locale, size_of(&self.size));
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
                let input = match rx.recv_timeout(machine.wait(start.elapsed())) {
                    Ok(input) => Some(input),
                    Err(RecvTimeoutError::Timeout) => None,
                    Err(RecvTimeoutError::Disconnected) => break,
                };
                let Some(compose) = machine.step(input, size_of(&self.size), start.elapsed(), Self::carried) else { continue };
                self.composer.size = machine.size();
                let t = Instant::now();
                let out = if compose.fresh {
                    self.composer.compose(machine.plan(), machine.slice(), machine.scroll())
                } else {
                    self.composer.scroll_step(machine.plan(), machine.slice(), machine.scroll())
                };
                let done = machine.composed(start.elapsed(), t.elapsed().as_secs_f32() * 1000.0, out.frame.backdrop, &out.problems, &out.space);
                if let Some(s) = done.space {
                    let _ = self.space.send(s);
                }
                {
                    let mut st = self.status.lock().unwrap();
                    st.cell = done.cell;
                    st.problems = out.problems;
                }
                pending = Some(out.frame);
            }
        }
    }
}
