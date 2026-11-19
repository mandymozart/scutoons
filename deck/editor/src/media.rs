//! A medium in an Asset tab (decisions/deck.md [workspaces]): a video, an image, a sound or a 3D set on a stage of
//! its own, with Play. A video plays on the deck's own playback (deck-space `Movie`, deck-media), a sound shows its
//! waveform. Trimming is the use's, never the file's ([delegated]): opened from a scene's `video` node, the tab moves
//! that node's `start` / `end` (deck-media `Trim`); opened from the library it has no use and the file plays whole.
use deck_kit::editor::controls::Rect;
use deck_media::Trim;
use deck_space::video::{Converter, Movie, Texture};
use std::io::Read;
use std::sync::mpsc::{channel, Receiver};
use std::sync::Arc;
use std::time::Duration;

/// the card the preview's texture is drawn under
pub const CARD: &str = "asset:medium";
/// the most a fetched file may weigh
const MOST: u64 = 1 << 30;
/// the waveform's columns
const COLUMNS: usize = 600;

/// what kind of medium, by its address's first word
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sort {
    Video,
    Image,
    Audio,
    Model,
}

impl Sort {
    pub fn of(key: &str) -> Option<Sort> {
        Some(match key.split_once(':')?.0 {
            "video" => Sort::Video,
            "image" => Sort::Image,
            "audio" => Sort::Audio,
            "model" => Sort::Model,
            _ => return None,
        })
    }

    pub fn word(self) -> &'static str {
        match self {
            Sort::Video => "video",
            Sort::Image => "image",
            Sort::Audio => "audio",
            Sort::Model => "model",
        }
    }
}

/// a library asset's address word: `video`, `image`, `audio`, `model`
pub fn word_of(kind: deck_catalog::library::Kind) -> &'static str {
    use deck_catalog::library::Kind;
    match kind {
        Kind::Video => "video",
        Kind::Image => "image",
        Kind::Audio => "audio",
        _ => "model",
    }
}

/// the use the tab was opened from: a scene's `video` node, whose in and out points the trim moves
#[derive(Debug, Clone, PartialEq)]
pub struct Use {
    pub scene: String,
    pub node: String,
}

/// a sound's picture: the peak of each column, 0..1
#[derive(Debug, Clone, PartialEq)]
pub struct Wave {
    pub peaks: Vec<f32>,
    pub seconds: f64,
}

/// what the loading thread hands back
enum Got {
    Bytes(Vec<u8>),
    Image(Arc<image::RgbaImage>),
    Wave(Wave),
}

pub enum Body {
    Loading,
    Video(Box<Movie>),
    Image(Arc<image::RgbaImage>),
    Audio(Wave),
    /// a 3D set: TODO(#632) its preview on a stage of its own
    Model,
    Failed(String),
}

/// what can be pressed in the view
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Hit {
    /// the timeline: the playhead goes there
    Seek,
    /// the in point's handle, the out point's
    Start,
    End,
}

/// the medium open in the front Asset tab
pub struct Open {
    pub key: String,
    pub sort: Sort,
    /// its file: a path on this machine or on the CDN
    pub path: String,
    pub used: Option<Use>,
    pub trim: Trim,
    pub body: Body,
    rx: Option<Receiver<Result<Got, String>>>,
    /// seconds since the use started (Play runs it; Stop goes back to 0)
    pub clock: f64,
    pub playing: bool,
    tex: Option<Texture>,
    /// the picture is on its texture
    shown: bool,
}

#[derive(Default)]
pub struct Media {
    pub open: Option<Open>,
    conv: Option<Converter>,
    pub hits: Vec<(Rect, Hit)>,
    /// a handle or the playhead held by the pointer
    pub drag: Option<Hit>,
    /// the timeline's place on screen (for a drag)
    pub line: Option<Rect>,
    /// the stretch of the file the timeline shows, s
    pub view: (f64, f64),
}

/// The stretch of a file `length` s long the timeline shows for the span `(a, b)`: the whole file, or – when the span
/// is short against it – the span with room on both sides, so its handles can be taken.
pub fn view((a, b): (f64, f64), length: f64) -> (f64, f64) {
    let lap = b - a;
    if lap * 4.0 >= length {
        return (0.0, length);
    }
    let room = lap.max(0.5) * 1.5;
    ((a - room).max(0.0), (b + room).min(length))
}

/// a medium's bytes: a file here, a URL, else the path on the CDN
fn fetch(path: &str, cdn: &str) -> Result<Vec<u8>, String> {
    let local = std::path::Path::new(path);
    if local.is_file() {
        return std::fs::read(local).map_err(|e| format!("{path}: {e}"));
    }
    let url = if path.starts_with("http://") || path.starts_with("https://") {
        path.to_string()
    } else if !cdn.is_empty() {
        format!("{}/{}", cdn.trim_end_matches('/'), path.trim_start_matches('/'))
    } else {
        return Err(format!("{path}: not on this machine, and the CDN is not known yet (sign in)"));
    };
    let r = ureq::AgentBuilder::new().timeout(Duration::from_secs(30)).build().get(&url).call().map_err(|e| format!("{url}: {e}"))?;
    let mut bytes = Vec::new();
    r.into_reader().take(MOST).read_to_end(&mut bytes).map_err(|e| format!("{url}: {e}"))?;
    Ok(bytes)
}

/// the peaks of `columns` columns of interleaved samples
pub fn peaks(samples: &[f32], channels: usize, columns: usize) -> Vec<f32> {
    let frames = samples.len() / channels.max(1);
    if frames == 0 || columns == 0 {
        return Vec::new();
    }
    (0..columns)
        .map(|c| {
            let (a, b) = (c * frames / columns, ((c + 1) * frames / columns).max(c * frames / columns + 1).min(frames));
            samples[a * channels..b * channels].iter().fold(0.0f32, |p, x| p.max(x.abs())).min(1.0)
        })
        .collect()
}

fn load(sort: Sort, bytes: Vec<u8>) -> Result<Got, String> {
    match sort {
        Sort::Image => image::load_from_memory(&bytes).map(|i| Got::Image(Arc::new(i.to_rgba8()))).map_err(|e| format!("the image: {e}")),
        Sort::Audio => {
            let pcm = deck_media::audio::decode(bytes).map_err(|e| e.0)?.ok_or("the file has no sound")?;
            let ch = usize::from(pcm.channels.max(1));
            Ok(Got::Wave(Wave { peaks: peaks(&pcm.samples, ch, COLUMNS), seconds: pcm.seconds() }))
        }
        Sort::Video | Sort::Model => Ok(Got::Bytes(bytes)),
    }
}

impl Open {
    /// `key` (`video:<path>`, …) asked on a thread; `used` = the scene node it was opened from, with its trim
    pub fn new(key: &str, cdn: &str, used: Option<Use>, trim: Trim) -> Option<Open> {
        let sort = Sort::of(key)?;
        let path = key.split_once(':').map_or("", |(_, p)| p).to_string();
        let (tx, rx) = channel();
        let (p, c) = (path.clone(), cdn.to_string());
        std::thread::spawn(move || {
            let _ = tx.send(fetch(&p, &c).and_then(|b| load(sort, b)));
        });
        Some(Open { key: key.to_string(), sort, path, used, trim, body: Body::Loading, rx: Some(rx), clock: 0.0, playing: false, tex: None, shown: false })
    }

    /// the file's length, s (an image: none)
    pub fn length(&self) -> Option<f64> {
        match &self.body {
            Body::Video(m) => Some(m.length()),
            Body::Audio(w) => Some(w.seconds),
            _ => None,
        }
    }

    /// where on the file the use is now, s
    pub fn at(&self) -> f64 {
        self.length().map_or(0.0, |l| self.trim.at(self.clock, l))
    }

    /// the in or out point to `seconds` on the file; the other stays. Err: no file length yet, or no use to keep it.
    pub fn set_point(&mut self, which: Hit, seconds: f64) -> Result<Trim, String> {
        let length = self.length().ok_or("not loaded yet")?;
        if self.used.is_none() {
            return Err("trimming is a use's: open the video from a scene's video node to trim it there".into());
        }
        let s = seconds.clamp(0.0, length);
        let (a, b) = self.trim.span(length);
        self.trim = match which {
            Hit::Start => Trim::new(s.min(b - deck_media::trim::SHORTEST).max(0.0), self.trim.end),
            Hit::End if s >= length - 1e-3 => Trim::new(a, 0.0),
            Hit::End => Trim::new(a, s.max(a + deck_media::trim::SHORTEST)),
            Hit::Seek => return Ok(self.trim),
        };
        if let Body::Video(m) = &mut self.body {
            m.set_trim(self.trim);
        }
        self.clock = 0.0;
        Ok(self.trim)
    }

    /// the playhead to `seconds` on the file (inside the span)
    pub fn seek(&mut self, seconds: f64) {
        if let Some(l) = self.length() {
            let (a, b) = self.trim.span(l);
            self.clock = seconds.clamp(a, b) - a;
        }
    }
}

impl Media {
    /// Each frame: what loaded is taken, a playing clock runs, the picture for it goes onto the texture. Returns the
    /// texture when there is one, and whether it is new (the card is made again).
    pub fn frame(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, dt: f32) -> Option<(&wgpu::TextureView, bool)> {
        let o = self.open.as_mut()?;
        if let Some(got) = o.rx.as_ref().and_then(|rx| rx.try_recv().ok()) {
            o.rx = None;
            o.body = match got {
                Ok(Got::Bytes(b)) if o.sort == Sort::Video => match Movie::spawn(b, &o.key) {
                    Ok(mut m) => {
                        m.set_trim(o.trim);
                        Body::Video(Box::new(m))
                    }
                    Err(e) => Body::Failed(e),
                },
                Ok(Got::Bytes(_)) => Body::Model,
                Ok(Got::Image(i)) => Body::Image(i),
                Ok(Got::Wave(w)) => Body::Audio(w),
                Err(e) => Body::Failed(e),
            };
        }
        if o.playing {
            o.clock += f64::from(dt);
        }
        let picture = match &mut o.body {
            Body::Video(m) => m.next_picture(o.clock),
            Body::Image(i) if !o.shown => Some(Arc::new(deck_media::Picture::new(i.width(), i.height(), deck_media::Format::Rgba8, i.as_raw().clone()))),
            _ => None,
        };
        let mut fresh = false;
        if let Some(p) = picture {
            let conv = self.conv.get_or_insert_with(|| Converter::new(device));
            fresh = deck_space::video::upload(device, queue, conv, &p, false, None, &mut o.tex);
            o.shown = true;
        }
        o.tex.as_ref().map(|t| (&t.view, fresh))
    }

    /// the picture's width over height
    pub fn aspect(&self) -> Option<f32> {
        match &self.open.as_ref()?.body {
            Body::Video(m) => Some(m.aspect()),
            Body::Image(i) => Some(i.width() as f32 / i.height().max(1) as f32),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_medium_is_known_by_its_first_word() {
        assert_eq!(Sort::of("video:live/x/loop.mov"), Some(Sort::Video));
        assert_eq!(Sort::of("image:a.png"), Some(Sort::Image));
        assert_eq!(Sort::of("module:fixture"), None);
    }

    #[test]
    fn a_short_span_of_a_long_file_is_shown_with_room_around_it() {
        assert_eq!(view((1.0, 4.0), 10.0), (0.0, 10.0), "a span of a third: the whole file");
        assert_eq!(view((100.0, 102.0), 474.0), (97.0, 105.0));
        assert_eq!(view((0.0, 1.0), 474.0), (0.0, 2.5));
    }

    #[test]
    fn a_waveform_holds_each_columns_peak() {
        let samples = [0.1, -0.2, 0.5, 0.0, -0.9, 0.3, 0.2, 0.1];
        assert_eq!(peaks(&samples, 2, 2), [0.5, 0.9]);
        assert_eq!(peaks(&samples, 1, 4), [0.2, 0.5, 0.9, 0.2]);
        assert!(peaks(&[], 2, 4).is_empty());
    }

    fn opened(used: bool) -> Open {
        let mut o = Open::new("audio:nowhere.wav", "", used.then(|| Use { scene: "scene:nave".into(), node: "v".into() }), Trim::WHOLE).unwrap();
        o.body = Body::Audio(Wave { peaks: Vec::new(), seconds: 10.0 });
        o
    }

    #[test]
    fn the_trim_moves_the_uses_points_and_the_file_stays() {
        let mut o = opened(true);
        assert_eq!(o.set_point(Hit::Start, 2.0), Ok(Trim::new(2.0, 0.0)));
        assert_eq!(o.set_point(Hit::End, 5.0), Ok(Trim::new(2.0, 5.0)));
        assert_eq!(o.set_point(Hit::Start, 9.0), Ok(Trim::new(5.0 - deck_media::trim::SHORTEST, 5.0)), "the in point stays before the out point");
        assert_eq!(o.set_point(Hit::End, 10.0), Ok(Trim::new(5.0 - deck_media::trim::SHORTEST, 0.0)), "the file's end is no out point");
        assert_eq!(o.length(), Some(10.0));
        o.seek(7.0);
        assert!((o.at() - 7.0).abs() < 1e-9);
    }

    #[test]
    fn without_a_use_there_is_nothing_to_trim() {
        let mut o = opened(false);
        assert!(o.set_point(Hit::Start, 2.0).is_err());
        assert_eq!(o.trim, Trim::WHOLE);
    }
}
