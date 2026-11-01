//! The bridge into today's graph (research/EVENTS.md §8, §10): `GateCv` turns notes into the cv the stage's `seq` node
//! writes – pitch in Hz on the left channel, the gate (0 / 1) on the right – so fm / osc / voices.rs play generated
//! notes without a change. Sample-accurate: a note starts at its frame; it holds for its length; the pitch stays
//! where the last note left it (the voice glides as it does today). Runs on the audio thread: no allocation.
use crate::names::Sym;
use crate::rt::RtEvent;

#[derive(Debug, Clone)]
pub struct GateCv {
    pub note: Sym,
    pub pitch_key: Sym,
    /// the pitch now (Hz)
    pub pitch: f32,
    /// frames of gate left
    gate_left: u32,
}

impl GateCv {
    pub fn new(note: Sym, pitch_key: Sym, pitch: f32) -> GateCv {
        GateCv { note, pitch_key, pitch, gate_left: 0 }
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
                        self.pitch = p;
                    }
                    self.gate_left = e.len.max(1);
                }
                next += 1;
            }
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
        let (note, pitch) = (n.event("music.note").unwrap(), n.key("pitch").unwrap());
        let mut props = Props::default();
        props.set(pitch, 220.0);
        let mut g = GateCv::new(note, pitch, 147.0);
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
}
