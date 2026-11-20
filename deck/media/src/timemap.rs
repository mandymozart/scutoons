//! A derivative's time against its source's: where the source's moment lies in the derivative's file, and the
//! derivative's frame rate. An asset's edits are in source time; each derivative plays them through its map, so the in
//! point shows the same picture whichever derivative the deck chose. Measured by pictures (`measure`): the
//! derivative's first frames against the source's, which holds for any encoder, a cut start or another frame rate.
use crate::demux::{Clip, Track};
use crate::player::{Frames, Player};
use crate::{err, Error, Picture, Software};

/// how far into the source a derivative's start is looked for, s
pub const WINDOW: f64 = 5.0;
/// how much of the derivative is compared, s
const COMPARED: f64 = 1.0;
/// a thumbnail's side, in cells
const CELLS: usize = 32;

/// `offset`: the source's time at the derivative's 0 (a derivative cut later starts later in the source); `fps`: the
/// derivative's frames per second (0: not known).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TimeMap {
    pub offset: f64,
    pub fps: f64,
}

impl TimeMap {
    /// a derivative with the source's own times – also what a derivative without a measured map is taken for
    pub const SAME: TimeMap = TimeMap { offset: 0.0, fps: 0.0 };

    /// the stored numbers (not finite = 0; a negative rate = not known)
    pub fn new(offset: f64, fps: f64) -> TimeMap {
        let clean = |x: f64| if x.is_finite() { x } else { 0.0 };
        TimeMap { offset: clean(offset), fps: clean(fps).max(0.0) }
    }

    /// source time → the derivative's file time
    pub fn to_file(self, source: f64) -> f64 {
        source - self.offset
    }

    /// the derivative's file time → source time
    pub fn to_source(self, file: f64) -> f64 {
        file + self.offset
    }
}

/// a track's frames per second: its samples over its length
pub fn fps(track: &Track) -> f64 {
    let length = track.length();
    if length <= 0.0 {
        return 0.0;
    }
    track.samples.len() as f64 / length
}

/// A picture small enough to compare across sizes and codecs: the mean colour of `CELLS` × `CELLS` cells, 0..1, each
/// channel less its mean over the picture (a codec's level or range shift is no difference).
pub fn thumbnail(p: &Picture) -> Vec<f32> {
    let rgba = p.to_rgba();
    let (w, h) = (rgba.width.max(1) as usize, rgba.height.max(1) as usize);
    let mut sum = vec![0f32; CELLS * CELLS * 3];
    let mut n = vec![0u32; CELLS * CELLS];
    for y in 0..h {
        for x in 0..w {
            let px = &rgba.data[(y * w + x) * 4..(y * w + x) * 4 + 3];
            let c = (y * CELLS / h) * CELLS + x * CELLS / w;
            for k in 0..3 {
                sum[c * 3 + k] += f32::from(px[k]) / 255.0;
            }
            n[c] += 1;
        }
    }
    let mut cells: Vec<f32> = sum.iter().enumerate().map(|(i, s)| if n[i / 3] > 0 { s / n[i / 3] as f32 } else { 0.0 }).collect();
    for k in 0..3 {
        let mean = cells.iter().skip(k).step_by(3).sum::<f32>() / (CELLS * CELLS) as f32;
        cells.iter_mut().skip(k).step_by(3).for_each(|v| *v -= mean);
    }
    cells
}

/// how unlike two thumbnails are: the mean absolute difference, 0..1
pub fn unlike(a: &[f32], b: &[f32]) -> f32 {
    a.iter().zip(b).map(|(x, y)| (x - y).abs()).sum::<f32>() / a.len().max(1) as f32
}

/// a clip's frames from its start up to `until` s, as (time, thumbnail)
pub fn thumbnails(clip: &Clip, until: f64) -> Result<Vec<(f64, Vec<f32>)>, Error> {
    let Some(track) = clip.video.clone() else { return err("the clip has no video") };
    let mut player = Player::new(clip, Box::new(Software::default()))?;
    player.looping = false;
    let mut queue = Frames::new(4);
    let mut out = Vec::new();
    for s in &track.samples {
        let t = track.seconds(s.pts);
        if t > until {
            break;
        }
        if let Some(f) = player.frame_for(t, &mut queue)? {
            out.push((t, thumbnail(&f.picture)));
        }
    }
    Ok(out)
}

/// the thumbnail shown at `t` among `frames` (the last one not after it)
fn shown_at(frames: &[(f64, Vec<f32>)], t: f64) -> Option<&[f32]> {
    let i = frames.partition_point(|f| f.0 <= t + 1e-9);
    frames.get(i.checked_sub(1)?).map(|f| f.1.as_slice())
}

/// The map of `derivative` against `source`, by pictures: the derivative's first second against each of the source's
/// frames in its first `WINDOW` s as a start; the start where they differ least wins (the earliest of equals: a still
/// opening measures 0). Decoded in software. Err: either has no video or does not decode here.
pub fn measure(source: &Clip, derivative: &Clip) -> Result<TimeMap, Error> {
    let theirs = thumbnails(derivative, COMPARED)?;
    let ours = thumbnails(source, WINDOW + COMPARED)?;
    let Some(t0) = theirs.first().map(|f| f.0) else { return err("the derivative has no frames") };
    let mut best: Option<(f32, f64)> = None;
    for (start, _) in &ours {
        if *start > WINDOW {
            break;
        }
        let offset = start - t0;
        let scores: Vec<f32> = theirs.iter().filter_map(|(t, th)| shown_at(&ours, t + offset).map(|o| unlike(th, o))).collect();
        if scores.is_empty() {
            continue;
        }
        let score = scores.iter().sum::<f32>() / scores.len() as f32;
        if best.is_none_or(|(b, _)| score < b - 1e-6) {
            best = Some((score, offset));
        }
    }
    let (_, offset) = best.ok_or_else(|| Error("the source has no frames".into()))?;
    let fps = derivative.video.as_ref().map_or(0.0, fps);
    Ok(TimeMap::new((offset * 1e6).round() / 1e6, fps))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_map_moves_source_time_into_the_file_and_back() {
        let m = TimeMap::new(0.5, 24.0);
        assert_eq!(m.to_file(1.25), 0.75);
        assert_eq!(m.to_source(0.75), 1.25);
        assert_eq!(TimeMap::new(f64::NAN, -3.0), TimeMap::SAME);
    }

    #[test]
    fn thumbnails_compare_pictures_of_any_size() {
        let grey = |w: u32, h: u32, v: u8| Picture::new(w, h, crate::Format::Rgba8, [v, v, v, 255].repeat((w * h) as usize));
        let (a, b) = (thumbnail(&grey(64, 48, 128)), thumbnail(&grey(32, 24, 128)));
        assert_eq!(a.len(), CELLS * CELLS * 3);
        assert!(unlike(&a, &b) < 1e-6, "the same picture, half the size");
        assert!(unlike(&a, &thumbnail(&grey(64, 48, 0))) > 0.4);
    }
}
