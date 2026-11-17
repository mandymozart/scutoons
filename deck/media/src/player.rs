//! Playback: the clock drives which frame shows. `Player::fill` decodes ahead of the heard time into `Frames` (and seeks
//! to the key frame before it when the heard time jumps); `Frames::frame_for` gives the frame whose time span holds the
//! heard time. Both halves can live on one thread (the browser, tests) or the decoding on its own (a native deck), the
//! queue between them. Looping clips run on: the n-th lap's frames carry n × the clip's length in their time.
use crate::backend::{DecodeBackend, Packet, Stream};
use crate::demux::{Clip, Track};
use crate::{err, Error, Picture};
use std::collections::VecDeque;
use std::sync::Arc;

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
        while self.list.len() >= 2 && self.list[1].time <= heard {
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

/// The decoding half: a clip's video track through a backend.
pub struct Player {
    track: Track,
    data: Arc<[u8]>,
    backend: Box<dyn DecodeBackend>,
    /// the next sample to submit, as a stream position (laps × samples + index)
    next: i64,
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
        Ok(Player { track, data: clip.data.clone(), backend, next: 0, looping: true, ahead: 0.25 })
    }

    pub fn track(&self) -> &Track {
        &self.track
    }

    /// the clip's length, s (at least one frame's)
    pub fn length(&self) -> f64 {
        let last = self.track.samples.last().map_or(0, |s| s.pts);
        self.track.length().max(self.track.seconds(last) + 1e-3)
    }

    fn n(&self) -> i64 {
        self.track.samples.len() as i64
    }

    /// where `heard` falls in the endless stream
    fn position(&self, heard: f64) -> i64 {
        let len = self.length();
        let (lap, t) = if self.looping { ((heard / len).floor(), heard.rem_euclid(len)) } else { (0.0, heard.clamp(0.0, len)) };
        // a nanosecond's slack: a frame's own time, read back as seconds, still lands on that frame
        let ticks = ((t + 1e-9) * f64::from(self.track.timescale)).floor() as i64;
        lap as i64 * self.n() + self.track.at(ticks) as i64
    }

    fn split(&self, pos: i64) -> (i64, usize) {
        (pos.div_euclid(self.n()), pos.rem_euclid(self.n()) as usize)
    }

    /// Decodes until `frames` holds the frame for `heard` and those up to `ahead` after it (or is full); a heard time
    /// before the queue or past the next key frame seeks. Returns how many frames were decoded.
    pub fn fill(&mut self, heard: f64, frames: &mut Frames) -> Result<usize, Error> {
        let want = self.position(heard.max(0.0));
        let covered = frames.first_pos().is_some_and(|first| first <= want) && want < self.next;
        let (lap, i) = self.split(want);
        let key = self.track.key_before(i) as i64 + lap * self.n();
        if !covered && (want < self.next || key > self.next || frames.first_pos().is_some_and(|f| f > want)) {
            self.backend.reset();
            frames.clear();
            self.next = key;
        }
        frames.drop_before(heard);
        let end = if self.looping { i64::MAX } else { self.n() };
        let mut decoded = 0;
        // a decoder that gives nothing back stops after a lap
        let mut budget = self.n() + frames.most as i64;
        while self.next < end && frames.len() < frames.most && budget > 0 {
            budget -= 1;
            if frames.last_time().is_some_and(|t| t >= heard + self.ahead) && frames.first_pos().is_some_and(|f| f <= want) {
                break;
            }
            let (_, i) = self.split(self.next);
            let s = self.track.samples[i];
            self.backend.submit(Packet { data: &self.data[s.at..s.at + s.len], id: self.next, key: s.key })?;
            self.next += 1;
            while let Some(d) = self.backend.poll() {
                let (dl, di) = self.split(d.id);
                let time = dl as f64 * self.length() + self.track.seconds(self.track.samples[di].pts);
                frames.push(Frame { time, index: di, pos: d.id, picture: Arc::new(d.picture) });
                decoded += 1;
            }
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
