//! A sound asset's preview voice (decisions/deck.md [media]): the Asset tab's Play heard. What it plays – the clip from
//! the playhead, looped inside the use's span – and where – the cue while a show is live, else this device's main, only
//! ever audio device channels (never DMX / MIDI / OSC) – is decided here; the engine only carries it to its audio thread.
//! Allocation-free on the audio side: a clip it lets go of is handed back for the frame thread to drop.
use crate::asset_edit::Playback;
use std::sync::Arc;

/// how long the voice fades in on Play and out on Stop (s): no click
pub const FADE: f32 = 0.005;

/// A decoded sound as the voice plays it: stereo, planar, at its own rate (a mono file on both sides).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Clip {
    pub rate: f32,
    pub l: Vec<f32>,
    pub r: Vec<f32>,
}

impl Clip {
    /// interleaved samples of `channels` channels: the first two (or the one) as left and right
    pub fn from_interleaved(rate: u32, channels: u16, samples: &[f32]) -> Clip {
        let ch = usize::from(channels.max(1));
        let l: Vec<f32> = samples.iter().step_by(ch).copied().collect();
        let r = if ch == 1 { l.clone() } else { samples.iter().skip(1).step_by(ch).copied().collect() };
        Clip { rate: rate.max(1) as f32, l, r }
    }

    pub fn frames(&self) -> usize {
        self.l.len().min(self.r.len())
    }

    pub fn seconds(&self) -> f64 {
        self.frames() as f64 / f64::from(self.rate.max(1.0))
    }
}

/// The device channels the voice plays on: the cue's while a show is live (none when the device has no cue of its own –
/// then it is heard nowhere, as a cue preview is), else main's.
pub fn channels<'a>(live: bool, main: &'a [usize], cue: &'a [usize]) -> &'a [usize] {
    if live {
        cue
    } else {
        main
    }
}

/// The audio thread's voice: one clip at a time, latest Play wins.
#[derive(Debug, Clone)]
pub struct Voice {
    rate: f32,
    clip: Option<Arc<Clip>>,
    /// where it reads, in the clip's frames, and the span it loops in
    pos: f64,
    span: (f64, f64),
    /// the asset's edits as this use plays them: speed, gain, fades (`asset_edit`)
    edit: Playback,
    env: f32,
    /// the envelope's change per sample: up after Play, down after Stop
    slope: f32,
}

impl Voice {
    pub fn new(rate: f32) -> Voice {
        Voice { rate: rate.max(1.0), clip: None, pos: 0.0, span: (0.0, 0.0), edit: Playback::whole(0.0), env: 0.0, slope: 0.0 }
    }

    fn per_sample(&self) -> f32 {
        1.0 / (FADE * self.rate)
    }

    /// `clip` from `from` s on, looped inside the playback's span (s on the clip, `asset_edit::Edits::playback`) at its
    /// speed, gain and fades. Returns the clip played before, for the caller to drop off the audio thread.
    pub fn play(&mut self, clip: Arc<Clip>, from: f64, edit: Playback) -> Option<Arc<Clip>> {
        let (rate, frames) = (f64::from(clip.rate), clip.frames() as f64);
        let a = (edit.start.max(0.0) * rate).min(frames);
        let b = (edit.end * rate).clamp(a, frames);
        self.span = (a, b);
        self.edit = edit;
        self.pos = (from * rate).clamp(a, b);
        if self.pos >= b {
            self.pos = a;
        }
        self.slope = self.per_sample();
        if self.clip.is_none() {
            self.env = 0.0;
        }
        self.clip.replace(clip).filter(|old| !Arc::ptr_eq(old, self.clip.as_ref().unwrap()))
    }

    /// Stop: it fades out, then is silent
    pub fn stop(&mut self) {
        if self.clip.is_some() {
            self.slope = -self.per_sample();
        }
    }

    /// PANIC: silent at once; the clip goes back
    pub fn cut(&mut self) -> Option<Arc<Clip>> {
        self.env = 0.0;
        self.slope = 0.0;
        self.clip.take()
    }

    pub fn sounding(&self) -> bool {
        self.clip.is_some()
    }

    /// The next `n` samples times `scale` added onto `outs` (one channel: left and right summed; two: left, right). Returns
    /// the clip once a Stop's fade has ended.
    pub fn add(&mut self, outs: &mut [&mut [f32]], n: usize, scale: f32) -> Option<Arc<Clip>> {
        let clip = self.clip.as_deref()?;
        let (a, b) = self.span;
        let lap = b - a;
        let step = f64::from(clip.rate) / f64::from(self.rate) * self.edit.speed;
        let heard = 1.0 / (f64::from(clip.rate) * self.edit.speed);
        let last = clip.frames().saturating_sub(1);
        if lap < 1.0 || last == 0 {
            return self.cut();
        }
        let mut ended = false;
        for i in 0..n {
            self.env += self.slope;
            if self.env >= 1.0 {
                self.env = 1.0;
                self.slope = 0.0;
            } else if self.env <= 0.0 && self.slope < 0.0 {
                ended = true;
                break;
            }
            let k = self.pos as usize;
            let t = (self.pos - k as f64) as f32;
            let k1 = if (k + 1) as f64 >= b { a as usize } else { k + 1 };
            let at = |s: &[f32]| s[k.min(last)] + (s[k1.min(last)] - s[k.min(last)]) * t;
            let g = self.env * scale * self.edit.gain * self.edit.fade((self.pos - a) * heard);
            let (l, r) = (at(&clip.l) * g, at(&clip.r) * g);
            match outs {
                [m] => {
                    if let Some(x) = m.get_mut(i) {
                        *x += 0.5 * (l + r);
                    }
                }
                [lo, ro, ..] => {
                    if let Some(x) = lo.get_mut(i) {
                        *x += l;
                    }
                    if let Some(x) = ro.get_mut(i) {
                        *x += r;
                    }
                }
                [] => {}
            }
            self.pos += step;
            if self.pos >= b {
                self.pos -= lap;
            }
        }
        if ended {
            return self.cut();
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// a clip whose every sample says where it is: frame k = k / 1000
    fn ramp(frames: usize, rate: u32) -> Arc<Clip> {
        let s: Vec<f32> = (0..frames).map(|k| k as f32 / 1000.0).collect();
        Arc::new(Clip::from_interleaved(rate, 1, &s))
    }

    fn render(v: &mut Voice, frames: usize) -> Vec<f32> {
        let (mut l, mut r) = (vec![0.0f32; frames], vec![0.0f32; frames]);
        for (cl, cr) in l.chunks_mut(128).zip(r.chunks_mut(128)) {
            let n = cl.len();
            v.add(&mut [cl, cr], n, 1.0);
        }
        l
    }

    /// the clip's span `a`..`b` s as it is
    fn span(a: f64, b: f64) -> Playback {
        Playback { start: a, end: b, ..Playback::whole(b) }
    }

    #[test]
    fn the_assets_speed_gain_and_fades_are_heard() {
        use crate::asset_edit::Edits;
        use crate::trim::Trim;
        let mut v = Voice::new(1000.0);
        let flat = Arc::new(Clip { rate: 1000.0, l: vec![0.5; 4000], r: vec![0.5; 4000] });
        let e = Edits { speed: 2.0, gain_db: -6.0206, fade_in: 0.5, ..Edits::NONE };
        v.play(flat, 0.0, e.playback(Trim::new(0.0, 2.0), 4.0, 0.0));
        let out = render(&mut v, 1500);
        assert!((out[250] - 0.125).abs() < 0.01, "half way up the fade at half the gain: {}", out[250]);
        assert!((out[700] - 0.25).abs() < 1e-3, "past the fade: half the gain");
        assert!(out[1000] < 0.01, "two seconds at twice the speed: a lap is one second, the next fades in again");
        let mut v = Voice::new(1000.0);
        v.play(ramp(2000, 1000), 0.0, Edits { speed: 2.0, ..Edits::NONE }.playback(Trim::WHOLE, 2.0, 0.0));
        assert_eq!((render(&mut v, 100)[50] * 1000.0).round() as usize, 100, "twice the speed: two frames a sample");
    }

    #[test]
    fn a_clip_folds_to_left_and_right() {
        let c = Clip::from_interleaved(44_100, 3, &[1.0, 2.0, 3.0, 4.0, 5.0, 6.0]);
        assert_eq!((c.l.as_slice(), c.r.as_slice(), c.rate), (&[1.0, 4.0][..], &[2.0, 5.0][..], 44_100.0));
        let m = Clip::from_interleaved(8_000, 1, &[0.5, 0.25]);
        assert_eq!(m.l, m.r);
        assert!((m.seconds() - 0.25e-3).abs() < 1e-9);
    }

    #[test]
    fn it_plays_from_the_playhead_inside_the_span_and_loops_there() {
        let mut v = Voice::new(1000.0);
        assert!(render(&mut v, 64).iter().all(|&x| x == 0.0), "silent until Play");
        // 1000 frames at 1 kHz = 1 s; span 0.2..0.3 s, the playhead at 0.25 s
        v.play(ramp(1000, 1000), 0.25, span(0.2, 0.3));
        let out = render(&mut v, 400);
        // past the fade (5 samples), each sample is the frame it reads
        let read: Vec<usize> = out[10..].iter().map(|x| (x * 1000.0).round() as usize).collect();
        assert!(read.iter().all(|&k| (200..300).contains(&k)), "inside the span: {:?}", &read[..20]);
        assert_eq!(read[0], 260, "from the playhead on");
        assert!(read.windows(2).any(|w| w[0] == 299 && w[1] == 200), "looped at the out point");
    }

    #[test]
    fn a_clip_at_another_rate_is_read_at_its_own_speed() {
        let mut v = Voice::new(1000.0);
        v.play(ramp(2000, 2000), 0.0, span(0.0, 1.0));
        let out = render(&mut v, 100);
        assert_eq!((out[50] * 1000.0).round() as usize, 100, "two clip frames per output sample");
    }

    #[test]
    fn stop_fades_out_and_ends_panic_cuts_at_once() {
        let mut v = Voice::new(48_000.0);
        let clip = Arc::new(Clip { rate: 48_000.0, l: vec![0.5; 48_000], r: vec![0.5; 48_000] });
        v.play(clip.clone(), 0.0, span(0.0, 1.0));
        assert!(render(&mut v, 4800).iter().skip(480).all(|&x| (x - 0.5).abs() < 1e-6));
        v.stop();
        let out = render(&mut v, 4800);
        assert!(out[0] > 0.4 && out[1000..].iter().all(|&x| x == 0.0), "a short fade, then nothing");
        assert!(!v.sounding(), "the clip has gone back");
        v.play(clip.clone(), 0.0, span(0.0, 1.0));
        render(&mut v, 4800);
        assert!(v.cut().is_some());
        assert!(render(&mut v, 256).iter().all(|&x| x == 0.0), "PANIC: silent from the next sample");
    }

    #[test]
    fn it_plays_on_the_cue_while_live_else_on_main() {
        let (main, cue) = ([0, 1], [2, 3]);
        assert_eq!(channels(false, &main, &cue), &main);
        assert_eq!(channels(true, &main, &cue), &cue);
        assert!(channels(true, &main, &[]).is_empty(), "live without a cue: heard nowhere");
    }
}
