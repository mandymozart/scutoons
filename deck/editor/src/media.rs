//! A medium in an Asset tab (decisions/deck.md [workspaces]): a video, an image, a sound or a 3D set on a stage of
//! its own, with Play. A video plays on the deck's own playback (deck-space `Movie`, deck-media), a sound shows its
//! waveform. The asset's edits ([media]: trim, crop, fades, speed, loudness – deck-core `asset_edit`) are the asset's,
//! applied while playing, and saved onto it (`PUT /api/assets/{id}`); a use's trim cuts inside them: opened from a scene's
//! `video` node, the handles move that node's `start` / `end`, opened from the library they move the asset's trim. Times
//! on the timeline are the source's; the file plays them through its time map. A sound's Play is heard through the
//! engine's preview voice (deck-core `sound_preview`) on a small engine of its own.
use deck_core::asset_edit::{Crop, Edits, Playback};
use deck_core::sound_preview::Clip;
use deck_core::trim::Trim;
use deck_core::units::{Refs, Unit};
use deck_kit::editor::controls::Rect;
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

/// a sound's picture (the peak of each column, 0..1) and the sound its Play hears
#[derive(Debug, Clone, PartialEq)]
pub struct Wave {
    pub peaks: Vec<f32>,
    pub seconds: f64,
    pub clip: Arc<Clip>,
}

/// The sound's way out while a medium is open: an engine with no graph of its own holding the device (the stage rests in
/// an Asset tab), its preview voice playing the sound.
struct Out {
    key: String,
    sounds: deck_engine::sound::Sounds,
    live: bool,
    /// the device has a cue of its own (a live preview is heard)
    cue: bool,
    _handles: deck_native::audio::Handles,
    _run: deck_native::audio::backend::Running,
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

/// an edit dragged on its field
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Field {
    FadeIn,
    FadeOut,
    Speed,
    Gain,
    CropLeft,
    CropTop,
    CropRight,
    CropBottom,
}

impl Field {
    pub const ALL: [Field; 8] = [Field::FadeIn, Field::FadeOut, Field::Speed, Field::Gain, Field::CropLeft, Field::CropTop, Field::CropRight, Field::CropBottom];

    pub fn label(self) -> &'static str {
        match self {
            Field::FadeIn => "fade in",
            Field::FadeOut => "fade out",
            Field::Speed => "speed",
            Field::Gain => "loudness",
            Field::CropLeft => "crop left",
            Field::CropTop => "crop top",
            Field::CropRight => "crop right",
            Field::CropBottom => "crop bottom",
        }
    }

    /// what a picture-less sound has no use for
    pub fn picture(self) -> bool {
        matches!(self, Field::CropLeft | Field::CropTop | Field::CropRight | Field::CropBottom)
    }

    /// its field's range (the core's) and whether it is dragged on a log scale
    pub fn range(self) -> (f64, f64, bool) {
        use deck_core::asset_edit::{CROP_KEEPS, FADE, GAIN_DB, SPEED};
        match self {
            Field::FadeIn | Field::FadeOut => (0.0, FADE, false),
            Field::Speed => (SPEED.0, SPEED.1, true),
            Field::Gain => (GAIN_DB.0, GAIN_DB.1, false),
            _ => (0.0, f64::from(1.0 - CROP_KEEPS), false),
        }
    }

    pub fn get(self, e: &Edits) -> f64 {
        match self {
            Field::FadeIn => e.fade_in,
            Field::FadeOut => e.fade_out,
            Field::Speed => e.speed,
            Field::Gain => e.gain_db,
            Field::CropLeft => f64::from(e.crop.left),
            Field::CropTop => f64::from(e.crop.top),
            Field::CropRight => f64::from(e.crop.right),
            Field::CropBottom => f64::from(e.crop.bottom),
        }
    }

    /// `e` with this field at `v` (the core cleans it into its range)
    pub fn set(self, e: &Edits, v: f64) -> Edits {
        let c = e.crop;
        let (mut fin, mut fout, mut speed, mut gain) = (e.fade_in, e.fade_out, e.speed, e.gain_db);
        let mut crop = [c.left, c.top, c.right, c.bottom];
        match self {
            Field::FadeIn => fin = v,
            Field::FadeOut => fout = v,
            Field::Speed => speed = v,
            Field::Gain => gain = v,
            Field::CropLeft => crop[0] = v as f32,
            Field::CropTop => crop[1] = v as f32,
            Field::CropRight => crop[2] = v as f32,
            Field::CropBottom => crop[3] = v as f32,
        }
        Edits::new(e.trim, Crop::new(crop[0], crop[1], crop[2], crop[3]), fin, fout, speed, gain)
    }

    /// the value at `fraction` (0..1) across its field (the inverse of `crate::fraction`)
    pub fn at(self, fraction: f64) -> f64 {
        let (lo, hi, log) = self.range();
        let f = fraction.clamp(0.0, 1.0);
        if log && lo > 0.0 {
            lo * (hi / lo).powf(f)
        } else {
            lo + f * (hi - lo)
        }
    }

    /// the kind of value it is (whose unit is chosen once for all of its fields) and the unit it is stored in – which
    /// is also its dimension's base (deck-core `units`); none: it has no unit to switch (speed)
    pub fn unit(self) -> Option<(&'static str, Unit)> {
        match self {
            Field::FadeIn | Field::FadeOut => Some((FADE_UNIT, Unit::Seconds)),
            Field::Gain => Some((GAIN_UNIT, Unit::Decibels)),
            Field::CropLeft | Field::CropTop | Field::CropRight | Field::CropBottom => Some((CROP_UNIT, Unit::Percent)),
            Field::Speed => None,
        }
    }

    /// a crop across: a part of the source's width, else of its height
    fn across(self) -> bool {
        matches!(self, Field::CropLeft | Field::CropRight)
    }

    /// the value as read on a field without a unit
    pub fn shown(self, e: &Edits) -> String {
        format!("{:.2} ×", self.get(e))
    }
}

/// the kinds of value whose unit is chosen (a view preference, the editor's settings)
pub const CROP_UNIT: &str = "crop";
pub const FADE_UNIT: &str = "fade";
pub const GAIN_UNIT: &str = "gain";
pub const TRIM_UNIT: &str = "trim";

/// what can be pressed in the view
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Hit {
    /// a unit switch's segment: that kind of value is read in that unit from now on
    Unit(&'static str, Unit),
    /// the timeline: the playhead goes there
    Seek,
    /// the in point's handle, the out point's
    Start,
    End,
    /// an edit's field
    Edit(Field),
}

/// what a moved point became: the use's trim (written onto its scene node) or the asset's (saved onto the asset)
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Point {
    Use(Trim),
    Asset(Trim),
}

/// the asset a medium's file belongs to, as the API answered
type Found = Result<Option<serde_json::Value>, String>;

/// the medium open in the front Asset tab
pub struct Open {
    pub key: String,
    pub sort: Sort,
    /// its file: a path on this machine or on the CDN
    pub path: String,
    pub used: Option<Use>,
    pub trim: Trim,
    /// the asset's edits (in source time) and where the source's time is at this file's 0 (its time map)
    pub edits: Edits,
    pub offset: f64,
    /// the asset's id once the registry answered (none: a file it does not know – its edits cannot be saved)
    pub asset: Option<String>,
    /// what the registry said that is worth a line (no time map measured, not reachable)
    pub note: Option<String>,
    /// the source's size in pixels (a crop's px are the source's, not this file's) and its frame rate, as far as known
    pub source: deck_space::asset_edits::Source,
    found: Option<Receiver<Found>>,
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
    /// a handle, the playhead or an edit's field held by the pointer
    pub drag: Option<Hit>,
    /// the held field's place on screen
    pub field: Option<Rect>,
    /// the asset's edits on their way to the API
    pub saved: Option<Receiver<Result<(), String>>>,
    /// the timeline's place on screen (for a drag)
    pub line: Option<Rect>,
    /// the stretch of the file the timeline shows, s
    pub view: (f64, f64),
    out: Option<Out>,
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
            let clip = Arc::new(Clip::from_interleaved(pcm.rate, pcm.channels, &pcm.samples));
            Ok(Got::Wave(Wave { peaks: peaks(&pcm.samples, ch, COLUMNS), seconds: pcm.seconds(), clip }))
        }
        Sort::Video | Sort::Model => Ok(Got::Bytes(bytes)),
    }
}

impl Open {
    /// `key` (`video:<path>`, …) asked on a thread, and its asset from the registry at `api` (none: no edits to load);
    /// `used` = the scene node it was opened from, with its trim
    pub fn new(key: &str, cdn: &str, api: Option<crate::show::api::Api>, used: Option<Use>, trim: Trim) -> Option<Open> {
        let sort = Sort::of(key)?;
        let path = key.split_once(':').map_or("", |(_, p)| p).to_string();
        let (tx, rx) = channel();
        let (p, c) = (path.clone(), cdn.to_string());
        std::thread::spawn(move || {
            let _ = tx.send(fetch(&p, &c).and_then(|b| load(sort, b)));
        });
        let found = api.filter(|_| !std::path::Path::new(&path).is_file()).map(|api| {
            let (tx, rx) = channel();
            let p = path.clone();
            std::thread::spawn(move || {
                let _ = tx.send(api.asset_of(&p));
            });
            rx
        });
        let edits = Edits::NONE;
        Some(Open { key: key.to_string(), sort, path, used, trim, edits, offset: 0.0, asset: None, note: None, source: Default::default(), found, body: Body::Loading, rx: Some(rx), clock: 0.0, playing: false, tex: None, shown: false })
    }

    /// the registry's answer: the asset's id, its edits and this file's time map
    pub fn take_asset(&mut self, found: Found) {
        match found {
            Ok(Some(detail)) => {
                let (edits, offset, warn) = deck_space::asset_edits::for_file(&detail, &self.path);
                (self.edits, self.offset, self.note) = (edits, offset, warn);
                self.source = deck_space::asset_edits::source(&detail, &self.path);
                self.asset = detail["asset"]["id"].as_str().map(str::to_string);
                self.apply();
            }
            Ok(None) => self.note = Some("not in the asset registry: its edits cannot be kept".into()),
            Err(why) => self.note = Some(format!("its edits: {why}")),
        }
    }

    /// the file's length, s (an image: none)
    pub fn length(&self) -> Option<f64> {
        match &self.body {
            Body::Video(m) => Some(m.length()),
            Body::Audio(w) => Some(w.seconds),
            _ => None,
        }
    }

    /// the source's length as far as this file shows it, s
    pub fn source_length(&self) -> Option<f64> {
        self.length().map(|l| l + self.offset)
    }

    /// what plays: the asset's edits, the use's trim on top, through the file's time map
    pub fn playback(&self) -> Option<Playback> {
        self.source_length().map(|l| self.edits.playback(self.trim, l, self.offset))
    }

    /// the span played, source s
    pub fn span(&self) -> Option<(f64, f64)> {
        self.source_length().map(|l| self.edits.span(self.trim, l))
    }

    /// where in the source the use is now, s
    pub fn at(&self) -> f64 {
        self.playback().map_or(0.0, |p| p.file_at(self.clock) + self.offset)
    }

    /// the playback onto what plays it
    fn apply(&mut self) {
        let Some(play) = self.playback() else { return };
        if let Body::Video(m) = &mut self.body {
            m.set_playback(play);
        }
    }

    /// The in or out point to `seconds` (source): the use's trim when it was opened from a use, else the asset's; the
    /// other point stays. Err: no file length yet.
    pub fn set_point(&mut self, which: Hit, seconds: f64) -> Result<Point, String> {
        use deck_core::trim::SHORTEST;
        let length = self.source_length().ok_or("not loaded yet")?;
        let s = seconds.clamp(0.0, length);
        let moved = |t: Trim, (a, b): (f64, f64)| match which {
            Hit::Start => Trim::new(s.min(b - SHORTEST).max(0.0), t.end),
            Hit::End if s >= length - 1e-3 => Trim::new(a, 0.0),
            Hit::End => Trim::new(a, s.max(a + SHORTEST)),
            Hit::Seek | Hit::Edit(_) | Hit::Unit(..) => t,
        };
        let point = if self.used.is_some() {
            self.trim = moved(self.trim, self.edits.span(self.trim, length));
            Point::Use(self.trim)
        } else {
            self.edits.trim = moved(self.edits.trim, self.edits.trim.span(length));
            Point::Asset(self.edits.trim)
        };
        self.apply();
        self.clock = 0.0;
        Ok(point)
    }

    /// What a unit of the field `f` (none: the trim's) is read against: the source's pixels across or down (a file on this
    /// machine is its own source), its frame rate (the registry's, else this file's) and the tempo `(bpm, per_bar)`.
    pub fn refs(&self, f: Option<Field>, tempo: Option<(f64, u32)>) -> Refs {
        let local = self.asset.is_none() && std::path::Path::new(&self.path).is_file();
        let own = match &self.body {
            Body::Video(m) => Some((m.size(), Some(m.fps()))),
            Body::Image(i) => Some(((i.width(), i.height()), None)),
            _ => None,
        };
        let pixels = self.source.pixels.or_else(|| own.filter(|_| local).map(|((w, h), _)| (f64::from(w), f64::from(h))));
        let fps = self.source.fps.or_else(|| own.and_then(|(_, fps)| fps)).filter(|v| *v > 0.0);
        let px = pixels.map(|(w, h)| if f.is_some_and(Field::across) { w } else { h });
        match f.and_then(Field::unit).map(|(_, u)| u.dimension()) {
            Some(deck_core::units::Dimension::Length) => Refs { pixels: px, ..Refs::default() },
            Some(deck_core::units::Dimension::Level) => Refs::default(),
            _ => Refs { fps, bpm: tempo.map(|t| t.0), per_bar: tempo.map(|t| t.1), ..Refs::default() },
        }
    }

    /// an edit's field to `v` (cleaned by the core), applied at once
    pub fn set_edit(&mut self, field: Field, v: f64) {
        self.edits = field.set(&self.edits, v);
        self.apply();
    }

    /// the playhead to `seconds` (source, inside the span)
    pub fn seek(&mut self, seconds: f64) {
        if let Some(p) = self.playback() {
            self.clock = p.clock_at(seconds - self.offset);
        }
    }
}

/// an engine with an empty graph on the device as deck.toml has it, and its preview voice
fn start_out(key: &str, volume: f32, live: bool) -> Result<Out, String> {
    use deck_native::audio::{self, backend};
    let scene = deck_space::scene::Scene::parse("").map_err(|e| format!("{e:#}"))?;
    let empty = deck_space::graph::Graph::build(&scene).map_err(|e| format!("{e:#}"))?;
    let mut got = None;
    let mut factory = |rate: f32, channels: usize| {
        let (engine, mut h) = audio::Engine::build_for(&empty, rate, volume, channels)?;
        let sounds = h.core_mut().sound.take().ok_or_else(|| anyhow::anyhow!("the engine's preview voice"))?;
        let cue = h.core_mut().cue.as_ref().is_some_and(deck_engine::cue::Cue::heard);
        got = Some((sounds, cue, h));
        Ok(engine)
    };
    let run = backend::start(&backend::Options::from_deck(&backend::DeckAudio::read()), &mut factory).map_err(|e| format!("no sound device: {e:#}"))?;
    let (sounds, cue, h) = got.ok_or("no sound device")?;
    Ok(Out { key: key.to_string(), sounds, live, cue, _handles: h, _run: run })
}

impl Media {
    /// The preview voice follows the open sound: playing → from its playhead inside the use's span, on the cue while
    /// `live`, else main; anything else → it stops. Ok(Some(note)): live without a cue of its own (heard nowhere).
    pub fn sound(&mut self, volume: f32, live: bool) -> Result<Option<String>, String> {
        let playing = self.open.as_ref().filter(|o| o.playing).and_then(|o| match &o.body {
            Body::Audio(w) => Some((o.key.clone(), w.clip.clone(), o.at() - o.offset, o.playback()?)),
            _ => None,
        });
        let Some((key, clip, at, play)) = playing else {
            if let Some(out) = self.out.as_mut() {
                out.sounds.stop();
            }
            return Ok(None);
        };
        if self.out.as_ref().is_some_and(|o| o.key != key) {
            self.out = None;
        }
        if self.out.is_none() {
            self.out = Some(start_out(&key, volume, live)?);
        }
        let out = self.out.as_mut().expect("started");
        out.live = live;
        if !out.sounds.play(clip, at, play, live) {
            return Err("the sound device did not take it".into());
        }
        Ok((live && !out.cue).then(|| "live: this device has no cue of its own – the sound is heard nowhere".into()))
    }

    /// whether the voice plays with a live flag other than `live` (the show went live or ended)
    pub fn sound_stale(&self, live: bool) -> bool {
        self.out.as_ref().is_some_and(|o| o.live != live) && self.open.as_ref().is_some_and(|o| o.playing)
    }

    /// PANIC: the voice silent at once, the device let go, the playhead back at its in point
    pub fn panic(&mut self) {
        if let Some(out) = self.out.as_mut() {
            out.sounds.panic();
        }
        self.out = None;
        if let Some(o) = self.open.as_mut() {
            o.playing = false;
            o.clock = 0.0;
        }
    }

    /// the medium goes: its device is let go
    pub fn silence(&mut self) {
        if let Some(out) = self.out.as_mut() {
            out.sounds.stop();
        }
        self.out = None;
    }

    /// Each frame: what loaded is taken, a playing clock runs, the picture for it goes onto the texture. Returns the
    /// texture when there is one, and whether it is new (the card is made again).
    pub fn frame(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, dt: f32) -> Option<(&wgpu::TextureView, bool)> {
        let o = self.open.as_mut()?;
        if let Some(got) = o.rx.as_ref().and_then(|rx| rx.try_recv().ok()) {
            o.rx = None;
            o.body = match got {
                Ok(Got::Bytes(b)) if o.sort == Sort::Video => match Movie::spawn(b, &o.key) {
                    Ok(m) => Body::Video(Box::new(m)),
                    Err(e) => Body::Failed(e),
                },
                Ok(Got::Bytes(_)) => Body::Model,
                Ok(Got::Image(i)) => Body::Image(i),
                Ok(Got::Wave(w)) => Body::Audio(w),
                Err(e) => Body::Failed(e),
            };
            o.apply();
        }
        if let Some(found) = o.found.as_ref().and_then(|rx| rx.try_recv().ok()) {
            o.found = None;
            o.take_asset(found);
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
        let mut o = Open::new("audio:nowhere.wav", "", None, used.then(|| Use { scene: "scene:nave".into(), node: "v".into() }), Trim::WHOLE).unwrap();
        o.body = Body::Audio(Wave { peaks: Vec::new(), seconds: 10.0, clip: Arc::new(Clip::default()) });
        o
    }

    #[test]
    fn the_trim_moves_the_uses_points_and_the_file_stays() {
        let mut o = opened(true);
        assert_eq!(o.set_point(Hit::Start, 2.0), Ok(Point::Use(Trim::new(2.0, 0.0))));
        assert_eq!(o.set_point(Hit::End, 5.0), Ok(Point::Use(Trim::new(2.0, 5.0))));
        assert_eq!(o.set_point(Hit::Start, 9.0), Ok(Point::Use(Trim::new(5.0 - deck_core::trim::SHORTEST, 5.0))), "the in point stays before the out point");
        assert_eq!(o.set_point(Hit::End, 10.0), Ok(Point::Use(Trim::new(5.0 - deck_core::trim::SHORTEST, 0.0))), "the file's end is no out point");
        assert_eq!(o.length(), Some(10.0));
        o.seek(7.0);
        assert!((o.at() - 7.0).abs() < 1e-9);
        assert!(o.edits.none(), "the asset is untouched");
    }

    #[test]
    fn without_a_use_the_handles_move_the_assets_trim_and_a_use_cuts_inside_it() {
        let mut o = opened(false);
        assert_eq!(o.set_point(Hit::Start, 2.0), Ok(Point::Asset(Trim::new(2.0, 0.0))));
        assert_eq!(o.set_point(Hit::End, 6.0), Ok(Point::Asset(Trim::new(2.0, 6.0))));
        assert_eq!(o.trim, Trim::WHOLE, "no use: its trim stays whole");
        assert_eq!(o.span(), Some((2.0, 6.0)));
        o.used = Some(Use { scene: "scene:nave".into(), node: "v".into() });
        o.set_point(Hit::Start, 1.0).unwrap();
        assert_eq!(o.span(), Some((2.0, 6.0)), "a use's in point before the asset's: the asset's");
        o.set_point(Hit::End, 4.0).unwrap();
        assert_eq!(o.span(), Some((2.0, 4.0)));
    }

    #[test]
    fn a_crop_is_read_against_the_sources_pixels_and_the_trim_against_its_frames_and_the_tempo() {
        let mut o = opened(false);
        assert_eq!(o.refs(Some(Field::CropLeft), None).pixels, None, "not known yet: no pixels");
        o.source = deck_space::asset_edits::Source { pixels: Some((1920.0, 1080.0)), fps: Some(25.0) };
        assert_eq!(o.refs(Some(Field::CropLeft), None).pixels, Some(1920.0), "across: the source's width");
        assert_eq!(o.refs(Some(Field::CropBottom), None).pixels, Some(1080.0), "down: its height");
        let trim = o.refs(None, Some((120.0, 4)));
        assert_eq!((trim.fps, trim.bpm, trim.pixels), (Some(25.0), Some(120.0), None));
        assert_eq!(Field::Speed.unit(), None, "speed has no unit to switch");
        assert_eq!(Field::CropTop.unit(), Some((CROP_UNIT, Unit::Percent)), "stored as parts: the base of a length");
    }

    #[test]
    fn an_edits_field_sets_its_value_through_the_core() {
        let mut o = opened(false);
        o.set_edit(Field::Speed, Field::Speed.at(1.0));
        assert_eq!(o.edits.speed, deck_core::asset_edit::SPEED.1, "the far end of the field: the fastest");
        o.set_edit(Field::Speed, Field::Speed.at(0.5));
        assert!((o.edits.speed - 1.0).abs() < 1e-9, "log: the middle of 0.25–4 is 1");
        o.set_edit(Field::CropLeft, 0.9);
        o.set_edit(Field::CropRight, 0.9);
        assert!((o.edits.crop.rect()[2] - deck_core::asset_edit::CROP_KEEPS).abs() < 1e-6, "a crop always leaves a part");
        assert_eq!(Field::Speed.shown(&deck_core::asset_edit::Edits { speed: 1.5, ..o.edits }), "1.50 ×");
        let play = o.playback().unwrap();
        assert!((play.lap() - 10.0).abs() < 1e-9, "at speed 1 a lap is the file");
    }
}
