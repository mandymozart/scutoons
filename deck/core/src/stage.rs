//! deck/stage's modules (#185): each cargo feature of deck-stage and what it registers – node kinds and tempo sources.
//! The stage's registry (deck/stage/src/modules.rs) checks a scene against the features it was built with; the
//! `deck-scene-features` tool lists the features a set of scenes needs, so a deck build carries exactly those. Data
//! only (no stage code here), so the tool builds without wgpu; the stage's tests keep this table and its kinds in step.
use crate::module::Params;
use std::collections::BTreeSet;

/// A module of the stage: its cargo feature (`core` = always built), what it is, its node kinds and tempo sources.
#[derive(Debug, Clone, Copy)]
pub struct StageModule {
    pub feature: &'static str,
    pub what: &'static str,
    pub kinds: &'static [&'static str],
    pub sources: &'static [&'static str],
}

pub const CORE: &str = "core";

const fn module(feature: &'static str, what: &'static str, kinds: &'static [&'static str], sources: &'static [&'static str]) -> StageModule {
    StageModule { feature, what, kinds, sources }
}

/// Every module of the stage, built or not. A new kind = a line here + its variant in deck/stage's graph.rs.
pub const STAGE_MODULES: &[StageModule] = &[
    module(
        CORE,
        "the graph, control, the basic audio nodes, the render passes, the HUD, the internal tempo clock",
        &["test_pattern", "shader", "screen", "overlay", "lfo", "tempo", "click", "osc", "noise", "fm", "seq", "mixer", "fft", "audio_out"],
        &["internal", "relay"],
    ),
    module("space", "the 3D space: a set's glTF and the deck's being (#188)", &["space"], &[]),
    module("sync", "the scenes' sets (3D) and portraits from the API into the deck's cache, offline from it (#195)", &[], &[]),
    module("chapter", "a chapter from scutoons.com: its slices and its music, cached (library API)", &["chapter", "chapter_music"], &[]),
    module("live", "a Live outlet: the relay's cells for this deck's outlet, drawn natively; show media preloaded with the deck token (#186)", &["outlet"], &[]),
    module("midi", "MIDI clock in and out (the tempo clock's `midi` source, `midi_out`)", &[], &["midi"]),
    module("osc", "the OSC remote for the tempo clock (`/tempo/*`)", &[], &["osc"]),
    module("outputs", "the parameters as outputs: MIDI out, DMX fixtures, the clock-synced strobe – the scene's [outputs] (#211)", &[], &[]),
    module("link", "Ableton Link as a tempo source (needs cmake + libclang to build)", &[], &["link"]),
    module("pipewire", "the PipeWire audio backend (Linux; without it: cpal)", &[], &[]),
];

/// the module (feature) that carries `kind`; None = no stage kind
pub fn module_of_kind(kind: &str) -> Option<&'static StageModule> {
    STAGE_MODULES.iter().find(|m| m.kinds.contains(&kind))
}

/// What one node of a scene needs: the module of its kind, and for the `tempo` node the modules of its source and
/// of the MIDI clock out. Err = an unknown kind or source.
pub fn node_needs(id: &str, kind: &str, params: &Params) -> Result<Vec<&'static str>, String> {
    let Some(m) = module_of_kind(kind) else { return Err(format!("node '{id}': unknown kind '{kind}'")) };
    let mut out = vec![m.feature];
    if kind == "tempo" {
        if let Some(src) = params.get("source").and_then(|v| v.as_str()) {
            let Some(m) = STAGE_MODULES.iter().find(|m| m.sources.iter().any(|s| s.eq_ignore_ascii_case(src))) else {
                return Err(format!("node '{id}': unknown source '{src}' (internal, midi, link, osc, relay)"));
            };
            out.push(m.feature);
        }
        if params.contains_key("midi_out") {
            out.push("midi");
        }
    }
    Ok(out)
}

/// The modules (features) the nodes need besides `core`, sorted – what `deck-scene-features` prints.
pub fn features<'a>(nodes: impl IntoIterator<Item = (&'a str, &'a str, &'a Params)>) -> Result<Vec<&'static str>, String> {
    let mut set = BTreeSet::new();
    for (id, kind, params) in nodes {
        set.extend(node_needs(id, kind, params)?);
    }
    set.remove(CORE);
    Ok(set.into_iter().collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::SceneDef;

    fn needs(text: &str) -> Result<Vec<&'static str>, String> {
        let s = SceneDef::parse(text)?;
        features(s.nodes.iter().map(|n| (n.id.as_str(), n.kind.as_str(), &n.params)))
    }

    #[test]
    fn kinds_and_sources_are_unique() {
        let kinds: Vec<&str> = STAGE_MODULES.iter().flat_map(|m| m.kinds.iter().copied()).collect();
        assert_eq!(kinds.iter().collect::<BTreeSet<_>>().len(), kinds.len());
        let sources: Vec<&str> = STAGE_MODULES.iter().flat_map(|m| m.sources.iter().copied()).collect();
        assert_eq!(sources.iter().collect::<BTreeSet<_>>().len(), sources.len());
    }

    #[test]
    fn a_scene_needs_its_kinds_and_its_clock() {
        assert_eq!(needs("[[node]]\nid=\"a\"\nkind=\"lfo\"\n").unwrap(), Vec::<&str>::new());
        assert_eq!(needs("[[node]]\nid=\"s\"\nkind=\"space\"\n[[node]]\nid=\"m\"\nkind=\"chapter_music\"\n").unwrap(), ["chapter", "space"]);
        assert_eq!(needs("[[node]]\nid=\"t\"\nkind=\"tempo\"\nsource=\"Link\"\nmidi_out=\"UCX\"\n").unwrap(), ["link", "midi"]);
        assert_eq!(needs("[[node]]\nid=\"t\"\nkind=\"tempo\"\nsource=\"internal\"\n").unwrap(), Vec::<&str>::new());
    }

    #[test]
    fn unknown_kinds_and_sources_are_named() {
        assert_eq!(needs("[[node]]\nid=\"x\"\nkind=\"teleport\"\n").unwrap_err(), "node 'x': unknown kind 'teleport'");
        assert!(needs("[[node]]\nid=\"t\"\nkind=\"tempo\"\nsource=\"sundial\"\n").unwrap_err().contains("unknown source 'sundial'"));
    }
}
