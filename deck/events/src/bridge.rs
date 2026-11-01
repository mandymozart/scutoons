//! The bridge into today's graph (research/EVENTS.md §8, §10): `GateCv` turns notes into the cv the stage's `seq` node
//! writes – pitch in Hz on the left channel, the gate (0 / 1) on the right – so fm / osc / voices.rs play generated
//! notes without a change. Sample-accurate: a note starts at its frame; it holds for its length. The pitch glides
//! into each note over `glide` × its length (glide.rs, exponential; 0 = a jump) and stays where the last note left
//! it. Runs on the audio thread: no allocation.
use crate::glide::Glide;
use crate::names::Sym;
use crate::rt::RtEvent;

#[derive(Debug, Clone)]
pub struct GateCv {
    pub note: Sym,
    pub pitch_key: Sym,
    /// the prop (0..1) with the fraction of the note's length to glide into its pitch; missing = 0 (a jump)
    pub glide_key: Sym,
    /// the pitch now (Hz)
    pub pitch: f32,
    glide: Glide,
    /// frames since the glide started
    elapsed: u32,
    /// frames of gate left
    gate_left: u32,
}

impl GateCv {
    pub fn new(note: Sym, pitch_key: Sym, glide_key: Sym, pitch: f32) -> GateCv {
        GateCv { note, pitch_key, glide_key, pitch, glide: Glide::hold(pitch), elapsed: 0, gate_left: 0 }
    }

    /// One block: `events` with frames relative to the block's start, sorted (rt.rs `resolve` sorts); the left and
    /// right channels filled. A moment (length 0) opens the gate for one frame (a trigger).
    pub fn render(&mut self, events: &[RtEvent], left: &mut [f32], right: &mut [f32]) {
        let n = left.len().min(right.len());
        let mut next = 0;
        for i in 0..n {
            while next < events.len() && events[next].frame as usize <= i {
                let e = &events[next];
                if e.name == self.note {
                    if let Some(p) = e.props.get(self.pitch_key) {
                        let g = e.props.get(self.glide_key).unwrap_or(0.0).clamp(0.0, 1.0);
                        self.glide.start(self.pitch, p, (g * e.len as f32).round() as u32);
                        self.elapsed = 0;
                    }
                    self.gate_left = e.len.max(1);
                }
                next += 1;
            }
            self.pitch = self.glide.at(self.elapsed);
            self.elapsed = self.elapsed.saturating_add(1);
            left[i] = self.pitch;
            right[i] = if self.gate_left > 0 { 1.0 } else { 0.0 };
            self.gate_left = self.gate_left.saturating_sub(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::Props;
    use crate::names::Names;

    #[test]
    fn notes_become_pitch_and_gate_across_blocks() {
        let mut n = Names::new();
        let (note, pitch, glide) = (n.event("music.note").unwrap(), n.key("pitch").unwrap(), n.key("glide").unwrap());
        let mut props = Props::default();
        props.set(pitch, 220.0);
        let mut g = GateCv::new(note, pitch, glide, 147.0);
        let (mut l, mut r) = (vec![0.0; 8], vec![0.0; 8]);
        g.render(&[RtEvent { frame: 3, name: note, props, len: 6 }], &mut l, &mut r);
        assert_eq!(l, [147.0, 147.0, 147.0, 220.0, 220.0, 220.0, 220.0, 220.0]);
        assert_eq!(r, [0.0, 0.0, 0.0, 1.0, 1.0, 1.0, 1.0, 1.0]);
        // the next block: the gate's last frame, then closed; the pitch stays
        g.render(&[], &mut l, &mut r);
        assert_eq!(r[..3], [1.0, 0.0, 0.0]);
        assert!(l.iter().all(|&p| p == 220.0));
        // a trigger: one frame of gate
        let other = n.event("story.fx").unwrap();
        g.render(&[RtEvent { frame: 0, name: other, props, len: 0 }, RtEvent { frame: 2, name: note, props, len: 0 }], &mut l, &mut r);
        assert_eq!(r[..4], [0.0, 0.0, 1.0, 0.0], "another name is ignored; a moment gates one frame");
    }

    #[test]
    fn a_note_glides_into_its_pitch_over_its_glide_share_of_its_length() {
        let mut n = Names::new();
        let (note, pitch, glide) = (n.event("music.note").unwrap(), n.key("pitch").unwrap(), n.key("glide").unwrap());
        let mut g = GateCv::new(note, pitch, glide, 100.0);
        let (mut l, mut r) = (vec![0.0; 12], vec![0.0; 12]);
        // 400 Hz over 0.5 × 8 = 4 frames, starting at frame 2: the geometric steps 100 → 141 → 200 → 283 → 400
        let mut props = Props::default();
        props.set(pitch, 400.0);
        props.set(glide, 0.5);
        g.render(&[RtEvent { frame: 2, name: note, props, len: 8 }], &mut l, &mut r);
        let want = [100.0, 100.0, 100.0, 141.4214, 200.0, 282.8427, 400.0, 400.0, 400.0, 400.0, 400.0, 400.0];
        for (a, b) in l.iter().zip(want) {
            assert!((a - b).abs() < 1e-2, "{l:?}");
        }
        assert_eq!(r[..4], [0.0, 0.0, 1.0, 1.0]);
        // a glide into a new note starts where the pitch is: this one is cut off after 2 of 4 frames, at 200 Hz
        props.set(pitch, 100.0);
        props.set(glide, 1.0);
        g.render(&[], &mut l[..2], &mut r[..2]);
        let mut g2 = g.clone();
        g2.render(&[RtEvent { frame: 0, name: note, props, len: 4 }], &mut l, &mut r);
        assert!((l[0] - 400.0).abs() < 1e-3 && l[2] < l[1] && (l[4] - 100.0).abs() < 1e-3, "{l:?}");
        // a note without the glide prop jumps
        let mut bare = Props::default();
        bare.set(pitch, 50.0);
        g.render(&[RtEvent { frame: 1, name: note, props: bare, len: 8 }], &mut l, &mut r);
        assert_eq!(l[..3], [400.0, 50.0, 50.0]);
    }
}
