//! What the audio thread gets (research/EVENTS.md §14, §16): flat, fixed-size events at a frame offset inside the
//! window – no names to look up, no allocation, no children. `resolve` turns a window's events into them (on the
//! sequencer's side, never on the audio thread). A late event (a trigger, a delayed edge's) plays at offset 0.
use crate::event::{Event, Props};
use crate::generator::Span;
use crate::names::Sym;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RtEvent {
    /// frames after the window's start
    pub frame: u32,
    pub name: Sym,
    pub props: Props,
    /// its length in frames (0 = a moment)
    pub len: u32,
}

/// `events` as real-time events in `span`, sorted by frame; events starting at or after the span's end are left for a
/// later window. No allocation while `out` has room (reserve it) and the events have no children – a compound event
/// is flattened into a temporary list (the sequencer's side, never the audio thread's).
pub fn resolve(events: &[Event], span: &Span, out: &mut Vec<RtEvent>) {
    for e in events {
        if e.children.is_empty() {
            push(e, span, out);
        } else {
            let mut flat = Vec::new();
            e.flatten(span.tempo, span.from_frame, &mut flat);
            for f in &flat {
                push(f, span, out);
            }
        }
    }
    // unstable: no buffer; same input, same order on every deck
    out.sort_unstable_by_key(|r| r.frame);
}

fn push(e: &Event, span: &Span, out: &mut Vec<RtEvent>) {
    let at = e.frame(span.tempo, span.from_frame);
    if at >= span.to_frame {
        return;
    }
    let frame = (at - span.from_frame).max(0.0).round() as u32;
    out.push(RtEvent { frame, name: e.name, props: e.props, len: e.dur_frames(span.tempo).round() as u32 });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::Time;
    use crate::names::Names;
    use deck_core::tempo::TempoMap;

    #[test]
    fn events_land_at_their_frame_offsets() {
        let mut n = Names::new();
        let (note, fx) = (n.event("music.note").unwrap(), n.event("story.fx").unwrap());
        // 120 bpm, 48 kHz: a beat = 24 000 frames; the window = beats 1 .. 1.5
        let t = TempoMap::new(120.0, 4, 48_000.0);
        let span = Span::frames(&t, 24_000.0, 36_000.0);
        let events = vec![
            Event::beats(note, 1.25).lasting(Time::Beats(0.25)),
            Event::now(fx),
            Event::beats(note, 0.5),
            Event::beats(note, 1.5),
        ];
        let mut out = Vec::new();
        resolve(&events, &span, &mut out);
        let got: Vec<(u32, Sym, u32)> = out.iter().map(|r| (r.frame, r.name, r.len)).collect();
        assert_eq!(got.len(), 3, "beat 1.5 is the next window's");
        assert_eq!(got[2], (6_000, note, 6_000));
        assert!(got[..2].contains(&(0, fx, 0)) && got[..2].contains(&(0, note, 0)), "a trigger and a late note at 0");
    }
}
