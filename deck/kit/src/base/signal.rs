//! The one typed grammar of connections: what a port carries, its pin's shape and its colour. Every view that shows a
//! port or a wire – the node views, the rows band, the stage's tubes – draws from this table, so a type looks the same
//! wherever it shows.
use serde::{Deserialize, Serialize};

/// what a port carries
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Signal {
    /// something happened (a story event, a row fired)
    Event,
    /// a value that changes (a knob, a level)
    Control,
    /// the beat, the bar, the show's time
    Clock,
    Audio,
    Picture,
    /// a place a message is sent to
    Address,
    /// the audience's answer
    Vote,
    /// a piece that plays
    Scene,
    /// from outside this subtree: only events reach it
    Far,
    /// a run that starts, progresses and ends: a generation, a night run, a publish
    Job,
    /// a reference to a Library asset of any kind – a picture that arrived, an approved text
    Asset,
}

/// a pin's outline
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Shape {
    /// points along the flow
    Triangle,
    Dot,
    Square,
    Diamond,
    /// a hollow dot
    Ring,
}

impl Signal {
    pub const ALL: [Signal; 11] = [
        Signal::Event,
        Signal::Control,
        Signal::Clock,
        Signal::Audio,
        Signal::Picture,
        Signal::Address,
        Signal::Vote,
        Signal::Scene,
        Signal::Far,
        Signal::Job,
        Signal::Asset,
    ];

    pub fn shape(self) -> Shape {
        match self {
            Signal::Event | Signal::Job => Shape::Triangle,
            Signal::Control | Signal::Clock | Signal::Vote => Shape::Dot,
            Signal::Audio | Signal::Picture | Signal::Scene | Signal::Asset => Shape::Square,
            Signal::Address => Shape::Diamond,
            Signal::Far => Shape::Ring,
        }
    }

    pub fn word(self) -> &'static str {
        match self {
            Signal::Event => "event",
            Signal::Control => "control",
            Signal::Clock => "clock",
            Signal::Audio => "audio",
            Signal::Picture => "picture",
            Signal::Address => "address",
            Signal::Vote => "vote",
            Signal::Scene => "scene",
            Signal::Far => "far",
            Signal::Job => "job",
            Signal::Asset => "asset",
        }
    }

    /// whether an outlet of this signal may be wired into an inlet of `into`: the same signal, an answer setting a value,
    /// or a job's change of state making something happen
    pub fn feeds(self, into: Signal) -> bool {
        self == into || matches!((self, into), (Signal::Vote, Signal::Control) | (Signal::Job, Signal::Event))
    }

    /// a wire's width relative to the others: an event's is heavier, it is what makes things happen
    pub fn weight(self) -> f32 {
        match self {
            Signal::Event => 1.4,
            _ => 1.0,
        }
    }
}

/// a colour per signal; day and night hold the same roles, each readable on its own panels (the kit's `[day|night.signals]`)
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Signals {
    pub event: [f32; 4],
    pub control: [f32; 4],
    pub clock: [f32; 4],
    pub audio: [f32; 4],
    pub picture: [f32; 4],
    pub address: [f32; 4],
    pub vote: [f32; 4],
    pub scene: [f32; 4],
    pub far: [f32; 4],
    pub job: [f32; 4],
    pub asset: [f32; 4],
}

use crate::base::tokens::Colors;

impl Signals {
    pub fn day() -> Signals {
        Colors::day().signals
    }

    pub fn night() -> Signals {
        Colors::night().signals
    }

    pub fn of(&self, s: Signal) -> [f32; 4] {
        match s {
            Signal::Event => self.event,
            Signal::Control => self.control,
            Signal::Clock => self.clock,
            Signal::Audio => self.audio,
            Signal::Picture => self.picture,
            Signal::Address => self.address,
            Signal::Vote => self.vote,
            Signal::Scene => self.scene,
            Signal::Far => self.far,
            Signal::Job => self.job,
            Signal::Asset => self.asset,
        }
    }
}

impl Default for Signals {
    fn default() -> Signals {
        Signals::day()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::base::tokens::contrast;

    #[test]
    fn every_signal_has_a_look_of_its_own() {
        for set in [Signals::day(), Signals::night()] {
            for (i, a) in Signal::ALL.iter().enumerate() {
                for b in &Signal::ALL[i + 1..] {
                    assert!(a.shape() != b.shape() || set.of(*a) != set.of(*b), "{a:?} {b:?}");
                }
            }
        }
    }

    #[test]
    fn a_signal_reads_on_its_own_panels() {
        for (set, ground) in [(Signals::day(), Colors::day().surface), (Signals::night(), Colors::night().surface)] {
            for s in Signal::ALL {
                assert!(contrast(set.of(s), ground) >= 2.5, "{s:?} {}", contrast(set.of(s), ground));
            }
        }
    }

    #[test]
    fn a_signal_feeds_its_own_kind_and_an_answer_feeds_a_value() {
        for s in Signal::ALL {
            assert!(s.feeds(s));
        }
        assert!(Signal::Vote.feeds(Signal::Control) && Signal::Job.feeds(Signal::Event));
        assert!(!Signal::Control.feeds(Signal::Vote) && !Signal::Event.feeds(Signal::Control) && !Signal::Audio.feeds(Signal::Picture));
        assert!(!Signal::Event.feeds(Signal::Job) && !Signal::Asset.feeds(Signal::Picture));
    }

    #[test]
    fn its_word_is_the_one_a_file_names() {
        #[derive(Serialize)]
        struct One {
            s: Signal,
        }
        for s in Signal::ALL {
            assert_eq!(toml::to_string(&One { s }).unwrap().trim(), format!("s = \"{}\"", s.word()));
        }
    }
}
