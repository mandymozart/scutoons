//! The focus tone: a spore focused or selected sounds a short tone pitched by its signal, so a run of Tab
//! presses is heard as well as seen. The pitches are one major pentatonic over two octaves, a type to a note, so any
//! two of them sound well together. The engine plays it (deck-core `ui_tone`); the editor's settings switch it.
use deck_kit::base::graph::Node;
use deck_kit::base::signal::Signal;

/// the switch's default: on
pub const ON: bool = true;

/// semitones from A4 (440 Hz)
fn semitones(s: Signal) -> i32 {
    match s {
        Signal::Far => -14,    // G3
        Signal::Audio => -12,  // A3
        Signal::Scene => -9,   // C4
        Signal::Picture => -7, // D4
        Signal::Control => -5, // E4
        Signal::Clock => -2,   // G4
        Signal::Vote => 0,     // A4
        Signal::Event => 3,    // C5
        Signal::Address => 5,  // D5
        Signal::Job => 7,      // E5
        Signal::Asset => 10,   // G5
    }
}

/// the tone of a signal, Hz
pub fn pitch(s: Signal) -> f32 {
    440.0 * (semitones(s) as f32 / 12.0).exp2()
}

/// the tone of a node, Hz; one without pins (a kept cell, a folded pill) sounds as a piece
pub fn tone(n: &Node) -> f32 {
    pitch(n.signal().unwrap_or(Signal::Scene))
}

#[cfg(test)]
mod tests {
    use super::*;
    use deck_kit::base::graph::Pin;

    #[test]
    fn every_signal_has_its_own_pitch_in_a_comfortable_range() {
        let mut all: Vec<f32> = Signal::ALL.iter().map(|&s| pitch(s)).collect();
        assert!(all.iter().all(|&f| (180.0..1000.0).contains(&f)), "{all:?}");
        all.sort_by(f32::total_cmp);
        assert!(all.windows(2).all(|w| w[1] / w[0] > 1.1), "at least a whole tone apart: {all:?}");
        assert!((pitch(Signal::Vote) - 440.0).abs() < 1e-3);
    }

    #[test]
    fn a_node_sounds_by_its_outlet_else_its_inlet() {
        let n = |ins: Vec<Pin>, outs: Vec<Pin>| Node { ins, outs, ..Node::default() };
        assert_eq!(tone(&n(vec![Pin::new("a", Signal::Event)], vec![Pin::new("b", Signal::Clock)])), pitch(Signal::Clock));
        assert_eq!(tone(&n(vec![Pin::new("a", Signal::Event)], vec![])), pitch(Signal::Event));
        assert_eq!(tone(&Node::default()), pitch(Signal::Scene));
    }
}
