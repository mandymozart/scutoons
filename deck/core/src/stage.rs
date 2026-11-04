//! deck/native's modules (#185): each cargo feature of deck-native and what it registers – node kinds and tempo sources.
//! The stage's registry (deck/native/src/modules.rs) checks a scene against the features it was built with; the
//! `deck-scene-features` tool lists the features a set of scenes needs, so a deck build carries exactly those. Data
//! only (no stage code here), so the tool builds without wgpu; the stage's tests keep this table and its kinds in step.
use std::collections::BTreeSet;

/// A node's params from the scene (`[[node]]` keys besides id / kind).
pub type Params = toml::Table;

/// A module of the stage: its cargo feature (`core` = always built), what it is, its node kinds and tempo sources.
#[derive(Debug, Clone, Copy)]
pub struct StageModule {
    pub feature: &'static str,
    pub what: &'static str,
    pub sources: &'static [&'static str],
}

pub const CORE: &str = "core";

const fn module(feature: &'static str, what: &'static str, sources: &'static [&'static str]) -> StageModule {
    StageModule { feature, what, sources }
}

impl StageModule {
    /// the node kinds this module carries: the catalog's kinds with this `feature` (deck-catalog, #315)
    pub fn kinds(&self) -> impl Iterator<Item = &'static str> + '_ {
        deck_catalog::builtin_kinds().iter().filter(move |d| d.feature == self.feature).map(|d| deck_catalog::static_str(&d.name))
    }
}

/// Every module of the stage, built or not. A new kind = its entry in the catalog (deck-catalog, with this `feature`) + its
/// variant in deck-space's graph.rs.
pub const STAGE_MODULES: &[StageModule] = &[
    module(
        CORE,
        "the graph, control, the basic audio nodes, the render passes, the HUD, the internal tempo clock",
        &["internal", "relay"],
    ),
    module("space", "the 3D space: a set's glTF and the deck's being (#188)", &[]),
    module("events", "generators that talk in events (#236, deck-events): `walk` – a seeded melody, `events` – a patch of generators, as seq's cv", &[]),
    module("modules", "rig modules (#280, deck-module): compiled wasm instruments run in wasmtime – the scene's `module:<name>` kinds, whose ports come from the module's descriptor", &[]),
    module("sync", "the scenes' sets (3D) and portraits from the API into the deck's cache, offline from it (#195)", &[]),
    module("chapter", "a chapter from scutoons.com: its slices and its music, cached (library API)", &[]),
    module("live", "a Live outlet: the relay's cells for this deck's outlet, drawn natively; show media preloaded with the deck token (#186)", &[]),
    module("midi", "MIDI clock in and out (the tempo clock's `midi` source, `midi_out`)", &["midi"]),
    module("osc", "the OSC remote for the tempo clock (`/tempo/*`)", &["osc"]),
    module("outputs", "the parameters as outputs: MIDI out, DMX fixtures, the clock-synced strobe – the scene's [outputs] (#211)", &[]),
    module("link", "Ableton Link as a tempo source (needs cmake + libclang to build)", &["link"]),
    module("pipewire", "the PipeWire audio backend (Linux; without it: cpal)", &[]),
];

/// the module (feature) that carries `kind`; None = no stage kind
pub fn module_of_kind(kind: &str) -> Option<&'static StageModule> {
    let feature = &deck_catalog::builtin(kind).filter(|d| !d.feature.is_empty())?.feature;
    STAGE_MODULES.iter().find(|m| m.feature == feature.as_ref())
}

/// What one node of a scene needs: the module of its kind, and for the `tempo` node the modules of its source and
/// of the MIDI clock out. Err = an unknown kind or source.
pub fn node_needs(id: &str, kind: &str, params: &Params) -> Result<Vec<&'static str>, String> {
    // a rig module's kind is dynamic (graph.rs registry, modreg.rs): any `module:<name>` needs the host
    if kind.starts_with("module:") {
        return Ok(vec!["modules"]);
    }
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

/// The modules the browser's engine (live-core: deck-engine in wasm) carries: the core, the event hosts, the space node
/// (the being: its inputs are control values the page reads back) and the MIDI clock (Web MIDI feeds the core's clock).
/// Not yet: `chapter` (decoded audio – deck-media, plan 2.9), `modules` (the browser's module backend, P9b), the rest
/// are devices a browser has no port for.
pub const BROWSER_MODULES: &[&str] = &[CORE, "events", "space", "midi"];

/// whether a build that carries the modules `carries` plays the node (an unknown kind: no build plays it)
pub fn carries_node(id: &str, kind: &str, params: &Params, carries: impl Fn(&str) -> bool) -> bool {
    node_needs(id, kind, params).is_ok_and(|needs| needs.iter().all(|f| carries(f)))
}

/// What of a scene a build plays (#360) – one rule for every target: each node's `stays` (`carries_node`), each link's
/// (both its ends' nodes stay).
#[derive(Debug, Clone, PartialEq)]
pub struct Part {
    pub nodes: Vec<bool>,
    pub links: Vec<bool>,
}

/// The carried part of a scene (`nodes` as id, kind, params; `links` as from, to – `node` or `node.port`) for a build
/// that carries the modules `carries`: the native stage's tests and benches (its cargo features), the browser's engine
/// (`BROWSER_MODULES`), the headless host.
pub fn carried_part<'a>(
    nodes: impl IntoIterator<Item = (&'a str, &'a str, &'a Params)>,
    links: impl IntoIterator<Item = (&'a str, &'a str)>,
    carries: impl Fn(&str) -> bool,
) -> Part {
    let mut gone = BTreeSet::new();
    let nodes: Vec<bool> = nodes
        .into_iter()
        .map(|(id, kind, params)| {
            let stays = carries_node(id, kind, params, &carries);
            if !stays {
                gone.insert(id);
            }
            stays
        })
        .collect();
    fn node_of(r: &str) -> &str {
        r.split_once('.').map_or(r, |(n, _)| n)
    }
    let links = links.into_iter().map(|(from, to)| !gone.contains(node_of(from)) && !gone.contains(node_of(to))).collect();
    Part { nodes, links }
}

/// A scene file's `[[node]]` tables as (id, kind, params) – the only part of a scene text this check reads (the links and
/// the rest are the deck's own scene model, deck-space's `Scene`).
pub fn scene_nodes(text: &str) -> Result<Vec<(String, String, Params)>, String> {
    let table: Params = text.parse().map_err(|e: toml::de::Error| e.to_string())?;
    let Some(nodes) = table.get("node") else { return Ok(Vec::new()) };
    let nodes = nodes.as_array().ok_or("`node` is not an array of tables")?;
    nodes
        .iter()
        .enumerate()
        .map(|(i, n)| {
            let mut params = n.as_table().ok_or_else(|| format!("node #{}: not a table", i + 1))?.clone();
            let mut text_of = |key: &str| match params.remove(key) {
                Some(toml::Value::String(s)) => Ok(s),
                _ => Err(format!("node #{}: `{key}` is missing or not text", i + 1)),
            };
            let (id, kind) = (text_of("id")?, text_of("kind")?);
            Ok((id, kind, params))
        })
        .collect()
}

/// The modules (features) a scene text needs besides `core`, sorted (`features` over its `[[node]]` tables).
pub fn scene_features(text: &str) -> Result<Vec<&'static str>, String> {
    let nodes = scene_nodes(text)?;
    features(nodes.iter().map(|(id, kind, params)| (id.as_str(), kind.as_str(), params)))
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

    fn needs(text: &str) -> Result<Vec<&'static str>, String> {
        scene_features(text)
    }

    #[test]
    fn kinds_and_sources_are_unique() {
        let kinds: Vec<&str> = STAGE_MODULES.iter().flat_map(|m| m.kinds()).collect();
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
    fn a_rig_module_needs_the_modules_feature() {
        assert_eq!(needs("[[node]]
id=\"w\"
kind=\"module:walk\"
[[node]]
id=\"a\"
kind=\"lfo\"
").unwrap(), ["modules"]);
    }

    #[test]
    fn the_carried_part_drops_a_node_with_its_links() {
        let nodes = scene_nodes(
            "[[node]]\nid=\"m\"\nkind=\"chapter_music\"\n[[node]]\nid=\"o\"\nkind=\"osc\"\n[[node]]\nid=\"x\"\nkind=\"teleport\"\n\
             [[node]]\nid=\"out\"\nkind=\"audio_out\"\n",
        )
        .unwrap();
        let links = [("m", "out.in"), ("o.out", "out.in")];
        let part = carried_part(nodes.iter().map(|(id, kind, params)| (id.as_str(), kind.as_str(), params)), links, |f| f == CORE);
        assert_eq!(part, Part { nodes: vec![false, true, false, true], links: vec![false, true] });
        let browser = |f: &str| BROWSER_MODULES.contains(&f);
        let t = |src: &str| toml::from_str::<Params>(&format!("source = \"{src}\"")).unwrap();
        assert!(carries_node("t", "tempo", &t("midi"), browser) && !carries_node("t", "tempo", &t("link"), browser));
    }

    #[test]
    fn a_node_needs_an_id_and_a_kind() {
        assert!(scene_nodes("[[node]]\nkind=\"lfo\"\n").unwrap_err().contains("`id` is missing"));
        assert!(scene_nodes("[[node]]\nid=\"a\"\nkind=3\n").unwrap_err().contains("`kind` is missing"));
        assert!(scene_nodes("[[node]\n").is_err(), "not TOML");
        assert!(scene_nodes("[[link]]\nfrom=\"a\"\nto=\"b\"\n").unwrap().is_empty());
    }

    #[test]
    fn unknown_kinds_and_sources_are_named() {
        assert_eq!(needs("[[node]]\nid=\"x\"\nkind=\"teleport\"\n").unwrap_err(), "node 'x': unknown kind 'teleport'");
        assert!(needs("[[node]]\nid=\"t\"\nkind=\"tempo\"\nsource=\"sundial\"\n").unwrap_err().contains("unknown source 'sundial'"));
    }
}
