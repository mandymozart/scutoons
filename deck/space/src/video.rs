//! A video on the deck's own playback (deck-media): a clip's player and its frame queue. Where there are threads
//! (`native`) the player decodes ahead on one of its own and the frame's thread only picks; else both happen on the
//! frame's thread. The stage's clock gives the heard time, the queue the frame for it. What plays is a `Playback` (an
//! asset's edits and a use's trim, deck-core `asset_edit`): its span looped on the player's laps, at its speed.
use deck_core::asset_edit::{Edits, Playback};
use deck_core::trim::Trim;
use deck_media::{Clip, Frames, Picture, Player, Span};
use std::sync::Arc;

/// how many decoded frames wait at most, and how far ahead of the heard time they reach (s)
const QUEUE: usize = 6;
const AHEAD: f64 = 0.2;

/// How long after its heard time (s) a picture drawn now shows: at the next refresh, one frame interval `dt` later with
/// a FIFO swap chain – a video picks the frame heard then, not the one heard while it is drawn.
pub fn shown_ahead(dt: f32) -> f64 {
    f64::from(dt.clamp(0.0, 0.05))
}

pub struct Movie {
    width: u32,
    height: u32,
    /// the file's length, s
    length: f64,
    /// its frames a second (its frames over its length)
    fps: f64,
    /// what plays: the span, its speed, the fades, the gain, the crop
    play: Playback,
    /// the shown frame's place in the stream; none yet: −1
    shown: i64,
    inner: Inner,
}

enum Inner {
    Here(Box<Player>, Frames),
    #[cfg(all(feature = "native", not(target_arch = "wasm32")))]
    Thread(threaded::Ahead),
}

fn player(bytes: Vec<u8>) -> Result<(Player, u32, u32, f64, f64), String> {
    let clip = Clip::open(bytes.into()).map_err(|e| e.0)?;
    let mut p = Player::open(&clip).map_err(|e| e.0)?;
    p.ahead = AHEAD;
    let (w, h) = (p.track().width, p.track().height);
    let length = p.length();
    let fps = if length > 0.0 { p.track().samples.len() as f64 / length } else { 0.0 };
    Ok((p, w, h, length, fps))
}

impl Movie {
    /// the clip in `bytes`, decoded on the caller's thread
    pub fn open(bytes: Vec<u8>) -> Result<Movie, String> {
        let (p, width, height, length, fps) = player(bytes)?;
        Ok(Movie { width, height, length, fps, play: Playback::whole(length), shown: -1, inner: Inner::Here(Box::new(p), Frames::new(QUEUE)) })
    }

    /// the clip in `bytes`, decoded ahead on a thread of its own
    #[cfg(all(feature = "native", not(target_arch = "wasm32")))]
    pub fn spawn(bytes: Vec<u8>, name: &str) -> Result<Movie, String> {
        let (p, width, height, length, fps) = player(bytes)?;
        Ok(Movie { width, height, length, fps, play: Playback::whole(length), shown: -1, inner: Inner::Thread(threaded::Ahead::start(p, name)) })
    }

    /// width over height
    pub fn aspect(&self) -> f32 {
        self.width as f32 / self.height.max(1) as f32
    }

    /// its size in pixels
    pub fn size(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    /// its frames a second
    pub fn fps(&self) -> f64 {
        self.fps
    }

    /// the file's length, s
    pub fn length(&self) -> f64 {
        self.length
    }

    pub fn playback(&self) -> Playback {
        self.play
    }

    /// what plays from now on (the span from its in point again)
    pub fn set_playback(&mut self, play: Playback) {
        if play == self.play {
            return;
        }
        self.play = play;
        self.shown = -1;
        let span = Span { start: play.start, end: play.end, speed: play.speed };
        match &mut self.inner {
            Inner::Here(p, frames) => {
                p.set_span(span);
                frames.clear();
            }
            #[cfg(all(feature = "native", not(target_arch = "wasm32")))]
            Inner::Thread(a) => a.set_span(span),
        }
    }

    /// a use's in and out points on the file, the asset as it is
    pub fn set_trim(&mut self, trim: Trim) {
        self.set_playback(Edits::NONE.playback(trim, self.length, 0.0));
    }

    /// The picture for `heard` s since the use started (its span looped at its speed: the player decodes the next lap
    /// ahead across the out point) when it is another than the one shown.
    pub fn next_picture(&mut self, heard: f64) -> Option<Arc<Picture>> {
        let (pos, pic) = match &mut self.inner {
            Inner::Here(p, frames) => {
                if let Err(e) = p.fill(heard, frames) {
                    tracing::warn!("video: {e}");
                }
                frames.frame_for(heard).map(|f| (f.pos, f.picture.clone()))?
            }
            #[cfg(all(feature = "native", not(target_arch = "wasm32")))]
            Inner::Thread(a) => a.frame_for(heard)?,
        };
        (pos != self.shown).then(|| {
            self.shown = pos;
            pic
        })
    }
}

#[cfg(feature = "gpu")]
mod gpu;
#[cfg(feature = "gpu")]
pub use gpu::{install_held, upload, upscale_to, Converter, CopyHeld, Texture};

#[cfg(all(feature = "native", not(target_arch = "wasm32")))]
mod threaded {
    use deck_media::{Frames, Picture, Player, Span};
    use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
    use std::sync::{Arc, Mutex};
    use std::time::Duration;

    /// the frames the thread has, and a span asked for that it has not taken yet (its frames are not shown)
    struct Shared {
        frames: Frames,
        span: Option<Span>,
    }

    /// The decoding thread and what it shares: the heard time it decodes for, the frames it has, its last error.
    pub struct Ahead {
        heard: Arc<AtomicU64>,
        shared: Arc<Mutex<Shared>>,
        stop: Arc<AtomicBool>,
    }

    impl Ahead {
        pub fn start(mut player: Player, name: &str) -> Ahead {
            let heard = Arc::new(AtomicU64::new(0f64.to_bits()));
            let shared = Arc::new(Mutex::new(Shared { frames: Frames::new(super::QUEUE), span: None }));
            let stop = Arc::new(AtomicBool::new(false));
            let (h, both, s) = (heard.clone(), shared.clone(), stop.clone());
            let label = name.to_string();
            std::thread::Builder::new()
                .name(format!("video {name}"))
                .spawn(move || {
                    let mut mine = Frames::new(super::QUEUE);
                    let mut failed = false;
                    while !s.load(Ordering::Relaxed) {
                        if let Some(span) = both.lock().unwrap().span.take() {
                            player.set_span(span);
                            mine.clear();
                        }
                        let t = f64::from_bits(h.load(Ordering::Relaxed));
                        match player.fill(t, &mut mine) {
                            Ok(0) => std::thread::sleep(Duration::from_millis(4)),
                            Ok(_) => {
                                let mut b = both.lock().unwrap();
                                if b.span.is_none() {
                                    b.frames = mine.clone();
                                }
                            }
                            Err(e) if !failed => {
                                failed = true;
                                tracing::warn!("video {label}: {e}");
                            }
                            Err(_) => std::thread::sleep(Duration::from_millis(50)),
                        }
                    }
                })
                .expect("a thread");
            Ahead { heard, shared, stop }
        }

        pub fn frame_for(&mut self, heard: f64) -> Option<(i64, Arc<Picture>)> {
            self.heard.store(heard.to_bits(), Ordering::Relaxed);
            let mut b = self.shared.lock().unwrap();
            b.frames.frame_for(heard).map(|f| (f.pos, f.picture.clone()))
        }

        /// the span from now on: the frames of the last one go at once
        pub fn set_span(&mut self, span: Span) {
            let mut b = self.shared.lock().unwrap();
            b.frames.clear();
            b.span = Some(span);
        }
    }

    impl Drop for Ahead {
        fn drop(&mut self) {
            self.stop.store(true, Ordering::Relaxed);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn clip() -> Vec<u8> {
        std::fs::read(concat!(env!("CARGO_MANIFEST_DIR"), "/../media/tests/fixtures/hap.mov")).unwrap()
    }

    fn print(p: Option<Arc<Picture>>) -> Option<u64> {
        p.map(|p| deck_media::fingerprint(&p.data))
    }

    #[test]
    fn the_clock_picks_the_frame_and_a_frame_shows_once() {
        let mut m = Movie::open(clip()).unwrap();
        assert!((m.aspect() - 64.0 / 48.0).abs() < 1e-6);
        let first = print(m.next_picture(0.0)).expect("the first frame");
        assert!(m.next_picture(0.05).is_none(), "the same frame until the next one's time (12 fps)");
        assert!(m.next_picture(1.0 / 12.0).is_some(), "the second frame, on its own time");
        let later = print(m.next_picture(0.5)).expect("the seventh frame");
        assert_ne!(first, later);
        assert_eq!(print(m.next_picture(1.0)), Some(first), "looped: one second later the first again");
    }

    #[test]
    fn a_trimmed_use_shows_only_its_span_and_loops_back_to_its_in_point() {
        let mut whole = Movie::open(clip()).unwrap();
        let at = |m: &mut Movie, t: f64| {
            m.shown = -1;
            print(m.next_picture(t))
        };
        let (half, seventh) = (at(&mut whole, 0.5), at(&mut whole, 7.0 / 12.0));
        let mut m = Movie::open(clip()).unwrap();
        m.set_trim(Trim::new(0.5, 0.75));
        assert!((m.length() - 1.0).abs() < 1e-3, "the file stays a second long");
        assert_eq!(at(&mut m, 0.0), half, "its first picture is the in point's");
        assert_eq!(at(&mut m, 1.0 / 12.0), seventh);
        assert_eq!(at(&mut m, 0.25), half, "a quarter second later it is back at the in point");
    }

    #[cfg(all(feature = "native", not(target_arch = "wasm32")))]
    #[test]
    fn decoded_ahead_on_its_thread_the_same_frames() {
        let mut here = Movie::open(clip()).unwrap();
        let mut ahead = Movie::spawn(clip(), "test").unwrap();
        // a jump shows what the queue has until the thread caught up; then the same frame as decoded here
        for t in [0.0, 0.25, 0.5, 1.4] {
            let want = print(here.next_picture(t));
            let caught = (0..1000).any(|_| {
                ahead.shown = -1;
                let got = print(ahead.next_picture(t));
                got == want || {
                    std::thread::sleep(std::time::Duration::from_millis(2));
                    false
                }
            });
            assert!(caught, "at {t} s");
        }
    }
}
