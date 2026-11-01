//! The tempo grid as events: one `name` event every `every` beats (from `offset`), its index as a prop – a bell on the
//! beat (#210), a flash on the bar, the clock a pattern follows.
use super::steps;
use crate::event::{Event, Time};
use crate::generator::{Generator, Inbox, Outbox, Ports, Span};
use crate::names::Sym;

#[derive(Debug, Clone)]
pub struct Pulse {
    pub name: Sym,
    pub index: Sym,
    /// beats between pulses
    pub every: f64,
    pub offset: f64,
}

impl Generator for Pulse {
    fn ports(&self) -> Ports {
        Ports { ins: &[], outs: &["out"] }
    }

    fn query(&mut self, span: &Span, _: &Inbox, out: &mut Outbox) {
        if self.every <= 0.0 {
            return;
        }
        for (k, beat) in steps(span.from - self.offset, span.to - self.offset, 1.0 / self.every) {
            out.push(0, Event::new(self.name, Time::Beats(beat + self.offset)).with(self.index, k as f32));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::names::Names;
    use deck_core::tempo::TempoMap;

    #[test]
    fn a_pulse_on_every_bar_from_an_offset() {
        let mut n = Names::new();
        let mut p = Pulse { name: n.event("music.bar").unwrap(), index: n.key("index").unwrap(), every: 4.0, offset: 1.0 };
        let t = TempoMap::new(120.0, 4, 48_000.0);
        // beats 0 .. 12
        let span = Span::frames(&t, 0.0, 12.0 * 24_000.0);
        let mut out = Outbox::new(1, 64);
        p.query(&span, &Inbox { ports: &[] }, &mut out);
        let got: Vec<(Time, f32)> = out.port(0).iter().map(|e| (e.start, e.props.get(p.index).unwrap())).collect();
        assert_eq!(got, [(Time::Beats(1.0), 0.0), (Time::Beats(5.0), 1.0), (Time::Beats(9.0), 2.0)]);
    }
}
