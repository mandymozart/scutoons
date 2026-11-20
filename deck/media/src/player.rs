//! Playback: the clock drives which frame shows. `Player::fill` decodes ahead of the heard time into `Frames` (and seeks
//! to the key frame before it when the heard time jumps); `Frames::frame_for` gives the frame whose time span holds the
//! heard time. Both halves can live on one thread (the browser, tests) or the decoding on its own (a native deck), the
//! queue between them. The player plays a `Span` of the file, looped: each lap decodes from the key frame before the in
//! point and drops the pictures before it, so the in point's own frame shows – and the next lap is decoded ahead across
//! the out point like any other frames (no seek at the wrap). The n-th lap's frames carry n × the lap in their time.
use crate::backend::{DecodeBackend, Packet, Stream};
use crate::demux::{Clip, Track};
use crate::{err, Error, Picture};
use std::collections::VecDeque;
use std::sync::Arc;

/// the most packets a decoder that answers later (the browser's) holds without having given their pictures back: not a
/// whole lap at once, and still the packets a reordering decoder waits for
const IN_FLIGHT: usize = 16;

#[derive(Debug, Clone, PartialEq)]
pub struct Frame {
    /// on the heard clock, s
    pub time: f64,
    /// the sample's place in the track
    pub index: usize,
    /// laps × samples + index: where it lies in the endless stream
    pub pos: i64,
    pub picture: Arc<Picture>,
}

/// Decoded frames waiting for their time, oldest first.
#[derive(Debug, Clone)]
pub struct Frames {
    list: VecDeque<Frame>,
    /// the most a queue holds (memory: a 1080p RGBA frame is 8 MB)
    pub most: usize,
}

impl Frames {
    pub fn new(most: usize) -> Frames {
        Frames { list: VecDeque::with_capacity(most), most: most.max(2) }
    }

    pub fn len(&self) -> usize {
        self.list.len()
    }

    pub fn is_empty(&self) -> bool {
        self.list.is_empty()
    }

    pub fn clear(&mut self) {
        self.list.clear();
    }

    fn push(&mut self, f: Frame) {
        self.list.push_back(f);
    }

    /// The frame for the heard time: the newest one whose time is not after `heard` (the first one before the start).
    /// Older frames leave the queue.
    pub fn frame_for(&mut self, heard: f64) -> Option<&Frame> {
        self.drop_before(heard);
        self.list.front()
    }

    fn drop_before(&mut self, heard: f64) {
        // the same nanosecond's slack as `Player::position`: a frame's time computed on another path still counts as heard
        while self.list.len() >= 2 && self.list[1].time <= heard + 1e-9 {
            self.list.pop_front();
        }
    }

    fn first_pos(&self) -> Option<i64> {
        self.list.front().map(|f| f.pos)
    }

    fn last_time(&self) -> Option<f64> {
        self.list.back().map(|f| f.time)
    }
}

/// The stretch of the file played, looped, and how fast: file seconds `start` to `end`, `speed` file seconds per heard
/// second. An end past the file (or not finite) is the file's end.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Span {
    pub start: f64,
    pub end: f64,
    pub speed: f64,
}

impl Span {
    pub const WHOLE: Span = Span { start: 0.0, end: f64::INFINITY, speed: 1.0 };
}

/// A span on one track: the samples a lap shows and the ones it decodes (from the key frame before the first).
#[derive(Debug, Clone, PartialEq)]
struct Lap {
    span: Span,
    /// heard seconds per lap
    secs: f64,
    /// the samples shown, presentation order: `first` .. `first + count`
    first: usize,
    count: usize,
    /// the samples a lap decodes, in decode order (contiguous from decode place `dk`)
    decode: Vec<u32>,
    dk: usize,
}

impl Lap {
    fn new(track: &Track, order: &[u32], length: f64, span: Span) -> Lap {
        let clean = |x: f64, or: f64| if x.is_finite() { x } else { or };
        let start = clean(span.start, 0.0).clamp(0.0, length);
        let mut end = clean(span.end, length).min(length);
        if end <= start {
            end = (start + 1e-3).min(length).max(start);
        }
        let speed = if span.speed.is_finite() && span.speed > 0.0 { span.speed } else { 1.0 };
        let ticks = |s: f64| ((s + 1e-9) * f64::from(track.timescale)).floor() as i64;
        let first = track.at(ticks(start));
        let last = track.at(ticks(end) - 1).max(first);
        let dk = order[track.key_before(first)] as usize;
        let dmax = (first..=last).map(|i| order[i] as usize).max().unwrap_or(dk).max(dk);
        let decode = track.decode[dk..=dmax].to_vec();
        Lap { span: Span { start, end, speed }, secs: ((end - start) / speed).max(1e-6), first, count: last - first + 1, decode, dk }
    }

    fn shown(&self, i: usize) -> bool {
        i >= self.first && i < self.first + self.count
    }
}

/// The decoding half: a clip's video track through a backend.
pub struct Player {
    track: Track,
    data: Arc<[u8]>,
    backend: Box<dyn DecodeBackend>,
    /// a sample's place in decode order, by its place in presentation order
    order: Vec<u32>,
    lap: Lap,
    /// the span changed: the next fill starts over
    moved: bool,
    /// the next packet to submit: laps × the lap's decode list + its place in it
    next: i64,
    /// the stream position (laps × shown samples + the place among them) decoding started from at the last seek
    from: i64,
    /// the decoder was told no more packets follow
    drained: bool,
    /// the clip starts again at its end
    pub looping: bool,
    /// how far ahead of the heard time frames are decoded, s
    pub ahead: f64,
}

impl Player {
    /// `clip`'s video through `backend`. Err: no video track, or a codec the backend lacks.
    pub fn new(clip: &Clip, mut backend: Box<dyn DecodeBackend>) -> Result<Player, Error> {
        let Some(track) = clip.video.clone() else { return err("the clip has no video") };
        if track.samples.is_empty() {
            return err("the clip's video has no frames");
        }
        backend.configure(&Stream { codec: track.codec.clone(), width: track.width, height: track.height, config: track.config.clone() })?;
        let mut order = vec![0u32; track.samples.len()];
        for (d, &shown) in track.decode.iter().enumerate() {
            order[shown as usize] = d as u32;
        }
        let lap = Lap::new(&track, &order, file_length(&track), Span::WHOLE);
        Ok(Player { track, data: clip.data.clone(), backend, order, lap, moved: false, next: 0, from: 0, drained: false, looping: true, ahead: 0.25 })
    }

    /// `clip`'s video through the backend the core picks (`backend::Auto`: hardware where the deck has it, else software)
    pub fn open(clip: &Clip) -> Result<Player, Error> {
        Player::new(clip, Box::new(crate::backend::Auto::default()))
    }

    pub fn track(&self) -> &Track {
        &self.track
    }

    /// the clip's length, s (at least one frame's)
    pub fn length(&self) -> f64 {
        file_length(&self.track)
    }

    /// the span played, as it fits the file
    pub fn span(&self) -> Span {
        self.lap.span
    }

    /// heard seconds per lap of the span
    pub fn lap(&self) -> f64 {
        self.lap.secs
    }

    /// The stretch of the file played from now on (looped), and its speed; the next `fill` starts over.
    pub fn set_span(&mut self, span: Span) {
        let lap = Lap::new(&self.track, &self.order, self.length(), span);
        if lap != self.lap {
            self.lap = lap;
            self.moved = true;
        }
    }

    fn n(&self) -> i64 {
        self.track.samples.len() as i64
    }

    fn count(&self) -> i64 {
        self.lap.count as i64
    }

    /// where `heard` falls in the endless stream of the span's shown samples
    fn position(&self, heard: f64) -> i64 {
        let len = self.lap.secs;
        let (lap, t) = if self.looping { ((heard / len).floor(), heard.rem_euclid(len)) } else { (0.0, heard.clamp(0.0, len)) };
        let file = self.lap.span.start + t * self.lap.span.speed;
        // a nanosecond's slack: a frame's own time, read back as seconds, still lands on that frame
        let ticks = ((file + 1e-9) * f64::from(self.track.timescale)).floor() as i64;
        let i = self.track.at(ticks).clamp(self.lap.first, self.lap.first + self.lap.count - 1);
        lap as i64 * self.count() + (i - self.lap.first) as i64
    }

    /// a stream position as (lap, sample)
    fn split(&self, pos: i64) -> (i64, usize) {
        (pos.div_euclid(self.count()), self.lap.first + pos.rem_euclid(self.count()) as usize)
    }

    /// where sample `i` of lap `lap` lies among the packets submitted
    fn cursor(&self, lap: i64, i: usize) -> i64 {
        lap * self.lap.decode.len() as i64 + (self.order[i] as usize - self.lap.dk) as i64
    }

    /// the pictures the backend has ready, into `frames`; those before the in point (decoded to reach it) are dropped
    fn collect(&mut self, frames: &mut Frames) -> usize {
        let mut decoded = 0;
        while let Some(d) = self.backend.poll() {
            let (lap, i) = (d.id.div_euclid(self.n()), d.id.rem_euclid(self.n()) as usize);
            if !self.lap.shown(i) {
                continue;
            }
            let into = ((self.track.seconds(self.track.samples[i].pts) - self.lap.span.start) / self.lap.span.speed).max(0.0);
            let pos = lap * self.count() + (i - self.lap.first) as i64;
            frames.push(Frame { time: lap as f64 * self.lap.secs + into, index: i, pos, picture: Arc::new(d.picture) });
            decoded += 1;
        }
        decoded
    }

    /// Decodes until `frames` holds the frame for `heard` and those up to `ahead` after it (or is full); a heard time
    /// before what decoding started from, before the queue, or past the next key frame seeks. Packets go to the backend
    /// in decode order, pictures come back in presentation order (a hardware decoder some packets later). Returns how
    /// many frames were decoded.
    pub fn fill(&mut self, heard: f64, frames: &mut Frames) -> Result<usize, Error> {
        self.backend.check()?;
        let want = self.position(heard.max(0.0));
        let (lap, i) = self.split(want);
        let key = self.track.key_before(i);
        let behind = self.moved || want < self.from || frames.first_pos().is_some_and(|f| f > want);
        if behind || self.cursor(lap, key) > self.next {
            self.backend.reset();
            frames.clear();
            self.next = self.cursor(lap, key);
            self.from = lap * self.count() + (key.max(self.lap.first) - self.lap.first) as i64;
            self.drained = false;
            self.moved = false;
        }
        frames.drop_before(heard);
        let per_lap = self.lap.decode.len() as i64;
        let end = if self.looping { i64::MAX } else { per_lap };
        let mut decoded = self.collect(frames);
        // a decoder that gives nothing back stops after a lap
        let mut budget = per_lap + frames.most as i64;
        while self.next < end && frames.len() < frames.most && self.backend.in_flight() < IN_FLIGHT && budget > 0 {
            budget -= 1;
            if frames.last_time().is_some_and(|t| t >= heard + self.ahead) && frames.first_pos().is_some_and(|f| f <= want) {
                break;
            }
            let (lap, k) = (self.next.div_euclid(per_lap), self.next.rem_euclid(per_lap) as usize);
            let shown = self.lap.decode[k] as usize;
            let s = self.track.samples[shown];
            self.backend.submit(Packet { data: &self.data[s.at..s.at + s.len], id: lap * self.n() + shown as i64, key: s.key })?;
            self.next += 1;
            decoded += self.collect(frames);
            frames.drop_before(heard);
        }
        if self.next >= end && !self.drained {
            self.backend.drain();
            self.drained = true;
            decoded += self.collect(frames);
            frames.drop_before(heard);
        }
        Ok(decoded)
    }

    /// Both halves on the caller's thread: decode as needed, then the frame for `heard`.
    pub fn frame_for<'a>(&mut self, heard: f64, frames: &'a mut Frames) -> Result<Option<&'a Frame>, Error> {
        self.fill(heard, frames)?;
        Ok(frames.frame_for(heard))
    }
}

fn file_length(track: &Track) -> f64 {
    let last = track.samples.last().map_or(0, |s| s.pts);
    track.length().max(track.seconds(last) + 1e-3)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Software;

    fn player(name: &str) -> Player {
        let bytes = std::fs::read(format!("{}/tests/fixtures/{name}", env!("CARGO_MANIFEST_DIR"))).unwrap();
        Player::new(&Clip::open(bytes.into()).unwrap(), Box::new(Software::default())).unwrap()
    }

    /// (sample index, frame time) of the frame for each heard time
    fn walk(p: &mut Player, times: &[f64]) -> Vec<(usize, f64)> {
        let mut q = Frames::new(4);
        times.iter().map(|&t| p.frame_for(t, &mut q).unwrap().map(|f| (f.index, (f.time * 1e3).round() / 1e3)).unwrap()).collect()
    }

    #[test]
    fn the_heard_time_picks_the_frame_forward_looped_and_back() {
        for name in ["hap.mov", if crate::AV1 { "av1.mp4" } else { "hap_q.mov" }, if crate::AV1 { "av1.webm" } else { "hap.mov" }] {
            let mut p = player(name);
            let got = walk(&mut p, &[0.0, 0.04, 0.09, 0.5, 0.99, 1.0, 1.55, 0.2, 13.0]);
            assert_eq!(got, [(0, 0.0), (0, 0.0), (1, 0.083), (6, 0.5), (11, 0.917), (0, 1.0), (6, 1.5), (2, 0.167), (0, 13.0)], "{name}");
        }
    }

    /// what a decoder that answers later has: packets taken, pictures on their way, pictures out, a failure
    #[derive(Default)]
    struct Mail {
        taken: usize,
        coming: Vec<crate::Decoded>,
        out: std::collections::VecDeque<crate::Decoded>,
        failed: Option<String>,
    }

    /// a decoder that answers later (WebCodecs): its pictures come out only after the caller returned (`arrive`)
    struct Later {
        sw: Software,
        mail: Arc<std::sync::Mutex<Mail>>,
    }

    impl DecodeBackend for Later {
        fn caps(&self) -> crate::Caps {
            self.sw.caps()
        }
        fn configure(&mut self, stream: &Stream) -> Result<(), Error> {
            self.sw.configure(stream)
        }
        fn submit(&mut self, packet: Packet<'_>) -> Result<(), Error> {
            self.sw.submit(packet)?;
            let mut m = self.mail.lock().unwrap();
            m.taken += 1;
            m.coming.extend(std::iter::from_fn(|| self.sw.poll()));
            Ok(())
        }
        fn poll(&mut self) -> Option<crate::Decoded> {
            self.mail.lock().unwrap().out.pop_front()
        }
        fn reset(&mut self) {
            self.sw.reset();
            let mut m = self.mail.lock().unwrap();
            m.coming.clear();
            m.out.clear();
        }
        fn check(&mut self) -> Result<(), Error> {
            self.mail.lock().unwrap().failed.take().map_or(Ok(()), |e| Err(Error(e)))
        }
        fn in_flight(&self) -> usize {
            self.mail.lock().unwrap().coming.len()
        }
    }

    #[test]
    fn a_decoder_that_answers_later_gets_packets_in_measure_shows_the_same_frames_and_says_when_it_failed() {
        let bytes = std::fs::read(format!("{}/tests/fixtures/hap.mov", env!("CARGO_MANIFEST_DIR"))).unwrap();
        let clip = Clip::open(bytes.into()).unwrap();
        let mail = Arc::new(std::sync::Mutex::new(Mail::default()));
        let mut p = Player::new(&clip, Box::new(Later { sw: Software::default(), mail: mail.clone() })).unwrap();
        let arrive = || {
            let mut m = mail.lock().unwrap();
            let got = std::mem::take(&mut m.coming);
            m.out.extend(got);
        };
        let mut q = Frames::new(4);
        assert_eq!(p.fill(0.0, &mut q).unwrap(), 0, "nothing back yet");
        assert_eq!(mail.lock().unwrap().taken, IN_FLIGHT, "not a lap and more at once: the most in flight");
        let mut got = vec![];
        for t in [0.0, 0.04, 0.09, 0.5, 0.99, 1.0, 1.55, 0.2] {
            // one frame asks, the page's event loop runs, the next frame has it
            p.fill(t, &mut q).unwrap();
            arrive();
            got.push(p.frame_for(t, &mut q).unwrap().map(|f| f.index));
        }
        assert_eq!(got, [0, 0, 1, 6, 11, 0, 6, 2].map(Some));
        mail.lock().unwrap().failed = Some("Decoding error.".into());
        assert_eq!(p.fill(0.3, &mut q).unwrap_err().0, "Decoding error.", "a failure between two frames comes out on the next");
    }

    /// the software backend, counting its resets (a seek)
    struct Counting(Software, Arc<std::sync::atomic::AtomicU32>);

    impl DecodeBackend for Counting {
        fn caps(&self) -> crate::Caps {
            self.0.caps()
        }
        fn configure(&mut self, stream: &Stream) -> Result<(), Error> {
            self.0.configure(stream)
        }
        fn submit(&mut self, packet: Packet<'_>) -> Result<(), Error> {
            self.0.submit(packet)
        }
        fn poll(&mut self) -> Option<crate::Decoded> {
            self.0.poll()
        }
        fn reset(&mut self) {
            self.1.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            self.0.reset();
        }
    }

    #[test]
    fn a_span_starts_on_its_in_points_frame_and_wraps_decoded_ahead_without_a_seek() {
        let bytes = std::fs::read(format!("{}/tests/fixtures/{}", env!("CARGO_MANIFEST_DIR"), if crate::AV1 { "av1.mp4" } else { "hap.mov" })).unwrap();
        let resets = Arc::new(std::sync::atomic::AtomicU32::new(0));
        let clip = Clip::open(bytes.into()).unwrap();
        let mut p = Player::new(&clip, Box::new(Counting(Software::default(), resets.clone()))).unwrap();
        // 12 fps, key frames every 6: an in point at frame 3 decodes from frame 0 and drops 0–2
        p.set_span(Span { start: 0.25, end: 0.75, speed: 1.0 });
        assert!((p.lap() - 0.5).abs() < 1e-9);
        let mut q = Frames::new(6);
        let mut got = vec![];
        for k in 0..30 {
            let t = f64::from(k) / 12.0;
            got.push(p.frame_for(t, &mut q).unwrap().map(|f| f.index).unwrap());
        }
        assert_eq!(&got[..12], [3, 4, 5, 6, 7, 8, 3, 4, 5, 6, 7, 8], "frames 3–8, looped");
        assert_eq!(resets.load(std::sync::atomic::Ordering::Relaxed), 1, "one start, no seek at the wraps");
        assert!(got.chunks(6).all(|lap| lap == [3, 4, 5, 6, 7, 8]));
    }

    #[test]
    fn a_span_at_another_speed_runs_its_frames_faster_or_slower() {
        let mut p = player("hap.mov");
        p.set_span(Span { start: 0.0, end: 1.0, speed: 2.0 });
        assert!((p.lap() - 0.5).abs() < 1e-9, "twice as fast: half a second a lap");
        assert_eq!(walk(&mut p, &[0.0, 0.05, 0.25, 0.5]), [(0, 0.0), (1, 0.042), (6, 0.25), (0, 0.5)]);
        p.set_span(Span { start: 0.5, end: f64::INFINITY, speed: 0.5 });
        assert!((p.lap() - 1.0).abs() < 1e-9, "half as fast from the middle: a second a lap");
        assert_eq!(walk(&mut p, &[0.0, 1.0 / 6.0, 0.99]), [(6, 0.0), (7, 0.167), (11, 0.833)]);
    }

    #[test]
    fn decoding_runs_ahead_of_the_heard_time_and_stops_at_the_end_unlooped() {
        let mut p = player("hap.mov");
        let mut q = Frames::new(8);
        assert_eq!(p.fill(0.0, &mut q).unwrap(), 4, "a quarter second ahead at 12 fps: frames 0–3");
        assert_eq!(p.fill(0.0, &mut q).unwrap(), 0, "already there");
        p.looping = false;
        assert_eq!(p.frame_for(5.0, &mut q).unwrap().map(|f| f.index), Some(11), "past the end: the last frame holds");
    }
}
