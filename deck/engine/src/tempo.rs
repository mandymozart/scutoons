//! The scene's `tempo` node as deck-core's controller config (the scene side of the clock: the controller itself is
//! deck-core's `tempo`, #308). The engine publishes it as the first tempo map; a platform's frame thread takes over.
use anyhow::{bail, Result};
use deck_core::tempo::{clamp_bpm, Config, Source};
use deck_space::graph::{Graph, Kind};

/// The scene's `tempo` node (none = the defaults: 120 BPM in 4, internal, the click off).
pub fn tempo_config(graph: &Graph) -> Result<Config> {
    let mut c = Config::default();
    let Some(i) = graph.first(Kind::Tempo) else { return Ok(c) };
    let n = &graph.nodes[i];
    c.bpm = clamp_bpm(n.f32("bpm", 120.0) as f64);
    c.per_bar = n.f32("beats", 4.0).clamp(1.0, 32.0) as u32;
    if let Some(s) = n.str("source") {
        let Some(src) = Source::parse(s) else {
            bail!("node '{}': unknown source '{s}' (internal, midi, link, osc, relay)", n.id)
        };
        c.source = src;
    }
    c.click = n.bool("click", c.click);
    c.lead = n.bool("lead", c.lead);
    c.midi_in = n.str("midi_in").unwrap_or_default().into();
    c.midi_out = n.str("midi_out").map(String::from);
    c.midi_out_lead = n.f32("midi_out_lead", 0.0) as f64 / 1000.0;
    if let Some(o) = n.params.get("osc_port").and_then(|v| v.as_integer()) {
        c.osc = format!("0.0.0.0:{o}");
    }
    Ok(c)
}
