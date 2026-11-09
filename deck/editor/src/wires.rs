//! Wiring on the being (#431, W1 of research/MODULE-WIRING.md; owner 2026-11-09: on the being as it is, the Wire side of
//! the one stage). Nothing is decided here (RULES A1): the ports, their addresses and every cable's home are the core's
//! wiring, a gesture is `Bundle::connected` / `cut` on the document that holds the cable, whether it may stand is
//! `rig::validate`, its sentence is shown as it comes. The ports a dragged cable may reach are the core's answer too: each
//! is tried. The draft is a TOML file (the rig, the instruments that changed) until the editor can sign in and save.
use anyhow::{Context, Result};
use deck_space::rig::bundle::Played;
use deck_space::rig::edit::Op;
use deck_space::rig::validate::Request;
use deck_space::rig::wiring::Home;
use deck_space::rig::{Bundle, Instrument, Rig};
use deck_space::space::being::Level;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// one cable into an input
#[derive(Debug, Clone, PartialEq)]
pub struct Cable {
    /// the other end's address, its node's id and its port (`""` address: a node of the scene's own)
    pub from: String,
    pub home: Home,
}

/// one port a cable can end at
#[derive(Debug, Clone, PartialEq)]
pub struct Port {
    /// the flattened node's id: the graph's, so the being's point
    pub node: String,
    pub name: String,
    pub input: bool,
    pub ty: &'static str,
    /// the cable's name for it (`voice:voice.out`)
    pub address: String,
    /// the track it belongs to
    pub track: String,
    /// the cables feeding an input
    pub cables: Vec<Cable>,
}

/// every port of the played rig's tracks
pub fn ports(played: &Played) -> Result<Vec<Port>> {
    let w = played.wiring()?;
    let home = |k: usize| -> Home { played.homes.get(k).cloned().unwrap_or(Home::Bound) };
    let mut out = Vec::new();
    for (i, n) in w.nodes.iter().enumerate() {
        for (input, list) in [(true, &n.inputs), (false, &n.outputs)] {
            for p in list {
                let Some(address) = n.address(&p.name) else { continue };
                let cables = if input {
                    w.links.iter().enumerate().filter(|(_, l)| l.to == i && l.input == p.name).map(|(k, l)| Cable { from: w.nodes[l.from].address(&l.out).unwrap_or_default(), home: home(k) }).collect()
                } else {
                    Vec::new()
                };
                out.push(Port { node: n.id.clone(), name: p.name.clone(), input, ty: p.ty, address, track: n.track.clone(), cables });
            }
        }
    }
    Ok(out)
}

/// what a port is seen as from outside its track (#457): an output nothing inside takes, or the instrument's own; a free
/// audio input
fn is_outlet(bundle: &Bundle, ports: &[Port], p: &Port) -> bool {
    if p.input {
        return p.ty == "audio" && p.cables.is_empty();
    }
    let own = bundle.rig.tracks.iter().find(|t| t.id == p.track).and_then(|t| bundle.instruments.iter().find(|i| i.id == t.instrument)).and_then(|i| i.out.as_deref());
    let local = p.address.split_once(':').map_or(p.address.as_str(), |(_, l)| l);
    let is_out = own.is_some_and(|o| o == local || format!("{o}.out") == local);
    is_out || !ports.iter().any(|q| q.track == p.track && q.cables.iter().any(|c| c.from == p.address))
}

/// One level of the tree on the stage (#457): the path's last step, or the rig. At the rig each track is one body with only
/// its outlets; inside a track all its nodes and ports. The answer: what the being shows, and which ports are at the level.
pub fn level(bundle: &Bundle, ports: &[Port], node_ids: &[String], core_node: usize, track: Option<&str>) -> (Level, Vec<bool>) {
    let index = |id: &str| node_ids.iter().position(|n| n == id);
    let mut shown: Vec<Option<usize>> = vec![None; node_ids.len()];
    let mut visible = vec![false; ports.len()];
    let mut outlets = Vec::new();
    match track {
        None => {
            shown[core_node] = Some(core_node);
            let mut first: Vec<(String, usize)> = Vec::new();
            for p in ports {
                let Some(n) = index(&p.node) else { continue };
                let rep = match first.iter().find(|(t, _)| *t == p.track) {
                    Some((_, r)) => *r,
                    None => {
                        first.push((p.track.clone(), n));
                        n
                    }
                };
                shown[n] = Some(rep);
            }
        }
        Some(track) => {
            for p in ports.iter().filter(|p| p.track == track) {
                if let Some(n) = index(&p.node) {
                    shown[n] = Some(n);
                }
            }
        }
    }
    for (i, p) in ports.iter().enumerate() {
        let here = match track {
            None => is_outlet(bundle, ports, p),
            Some(t) => p.track == t,
        };
        if let (true, Some(n)) = (here, index(&p.node)) {
            visible[i] = true;
            outlets.push((n, p.name.clone(), p.input));
        }
    }
    (Level { shown, outlets }, visible)
}

/// `a` joined to `b` (dragged either way): the changed bundle, or the core's sentence
pub fn connect(bundle: &Bundle, a: &Port, b: &Port) -> Result<Bundle, String> {
    let (from, to) = if a.input && !b.input { (b, a) } else { (a, b) };
    checked(bundle.connected(&from.address, &to.address)?)
}

/// a track's gain set; the changed bundle
pub fn set_gain(bundle: &Bundle, track: &str, gain: f32) -> Result<Bundle, String> {
    checked(bundle.track_set(&Op::SetTrack { id: track.to_string(), instrument: None, preset: None, gain: Some(gain), pan: None, mute: None })?)
}

/// a track's pan set (−1 … 1); the changed bundle
pub fn set_pan(bundle: &Bundle, track: &str, pan: f32) -> Result<Bundle, String> {
    checked(bundle.track_set(&Op::SetTrack { id: track.to_string(), instrument: None, preset: None, gain: None, pan: Some(pan), mute: None })?)
}

/// a track muted or not; the changed bundle
pub fn set_mute(bundle: &Bundle, track: &str, mute: bool) -> Result<Bundle, String> {
    checked(bundle.track_set(&Op::SetTrack { id: track.to_string(), instrument: None, preset: None, gain: None, pan: None, mute: Some(mute) })?)
}

/// the last cable into input `port` taken off: the bundle without it, and the address it came from
pub fn pulled(bundle: &Bundle, port: &Port) -> Result<(Bundle, String), String> {
    let cable = port.cables.last().ok_or_else(|| format!("nothing feeds {}", port.address))?;
    Ok((bundle.cut(&cable.from, &port.address, &cable.home)?, cable.from.clone()))
}

/// the cable into `port` cut
pub fn cut(bundle: &Bundle, port: &Port) -> Result<Bundle, String> {
    checked(pulled(bundle, port)?.0)
}

/// the cable into `port` moved to the input `to`
pub fn moved(bundle: &Bundle, ports: &[Port], port: &Port, to: &Port) -> Result<Bundle, String> {
    let (without, from) = pulled(bundle, port)?;
    let source = ports.iter().find(|p| p.address == from).ok_or_else(|| format!("{from} is not a port"))?;
    connect(&without, source, to)
}

/// which ports a cable from `start` may reach: each tried on the core (the dragged one itself: no)
pub fn reachable(bundle: &Bundle, ports: &[Port], start: &Port) -> Vec<bool> {
    ports.iter().map(|q| q.address != start.address && q.input != start.input && connect(bundle, start, q).is_ok()).collect()
}

/// which inputs the cable into `port` may be moved to
pub fn reachable_moved(bundle: &Bundle, ports: &[Port], port: &Port) -> Vec<bool> {
    let Ok((without, from)) = pulled(bundle, port) else { return vec![false; ports.len()] };
    let Some(source) = ports.iter().find(|p| p.address == from) else { return vec![false; ports.len()] };
    ports.iter().map(|q| q.input && q.address != port.address && connect(&without, source, q).is_ok()).collect()
}

/// the bundle as the core's validate judges it
fn checked(bundle: Bundle) -> Result<Bundle, String> {
    let values = |v: Result<serde_json::Value, serde_json::Error>| v.map_err(|e| e.to_string());
    let list = |v: serde_json::Value| v.as_array().cloned().unwrap_or_default();
    let request = Request {
        kind: "rigs".into(),
        id: bundle.rig.subject.clone(),
        document: values(serde_json::to_value(&bundle.rig))?,
        instruments: list(values(serde_json::to_value(&bundle.instruments))?),
        presets: list(values(serde_json::to_value(&bundle.presets))?),
        modules: list(values(serde_json::to_value(&bundle.modules))?),
    };
    let problems = request.problems()?;
    if !problems.is_empty() {
        return Err(problems.join("; "));
    }
    Ok(bundle)
}

/// the draft: the rig, and the instruments that differ from the rig's own
#[derive(Serialize, Deserialize)]
struct Draft {
    rig: Rig,
    #[serde(default, rename = "instrument")]
    instruments: Vec<Instrument>,
}

/// the draft over `base`, if the file is there and still valid
pub fn load_draft(path: &Path, base: &Bundle) -> Bundle {
    let Ok(text) = std::fs::read_to_string(path) else { return base.clone() };
    let read = toml::from_str::<Draft>(&text).map_err(|e| e.to_string()).and_then(|d| {
        let mut b = Bundle { rig: d.rig, ..base.clone() };
        for i in d.instruments {
            if let Some(at) = b.instruments.iter().position(|x| x.id == i.id) {
                b.instruments[at] = i;
            }
        }
        checked(b)
    });
    match read {
        Ok(b) => {
            tracing::info!("draft: {}", path.display());
            b
        }
        Err(e) => {
            tracing::warn!("draft {}: {e}, left out", path.display());
            base.clone()
        }
    }
}

pub fn save_draft(path: &Path, base: &Bundle, bundle: &Bundle) -> Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).with_context(|| format!("making {}", dir.display()))?;
    }
    let instruments = bundle.instruments.iter().filter(|i| !base.instruments.contains(i)).cloned().collect();
    let text = toml::to_string(&Draft { rig: bundle.rig.clone(), instruments }).context("the draft as TOML")?;
    std::fs::write(path, text).with_context(|| format!("writing {}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn catpope() -> (Bundle, Vec<Port>) {
        let b = Bundle::core("character.catpope").expect("the core rig").expect("it reads");
        let p = again(&b);
        (b, p)
    }

    fn again(b: &Bundle) -> Vec<Port> {
        ports(&b.play(None, None).expect("it plays")).expect("it wires")
    }

    fn port<'a>(all: &'a [Port], address: &str) -> &'a Port {
        all.iter().find(|p| p.address == address).unwrap_or_else(|| panic!("no port {address}"))
    }

    fn draft_file(name: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!("deck-editor-{name}-{}", std::process::id())).join("rig.toml")
    }

    #[test]
    fn the_ports_are_the_tracks_by_their_addresses() {
        let (_, p) = catpope();
        assert!(!port(&p, "voice:voice.out").input);
        assert_eq!(port(&p, "voice:voice.cv").cables[0].from, "voice:melody.cv");
        assert!(p.iter().all(|x| x.address.contains(':')), "the scene's own nodes have no address");
    }

    #[test]
    fn a_dragged_cable_lands_and_plays() {
        let (b, p) = catpope();
        let next = connect(&b, port(&p, "voice:voice.out"), port(&p, "drone:mix.in3")).expect("it may stand");
        assert_eq!(next.rig.routes.len(), b.rig.routes.len() + 1);
        assert_eq!(port(&again(&next), "drone:mix.in3").cables[0].from, "voice:voice.out");
    }

    #[test]
    fn dragged_from_the_input_it_is_the_same_cable() {
        let (b, p) = catpope();
        let a = connect(&b, port(&p, "voice:voice.out"), port(&p, "drone:mix.in3")).expect("forwards");
        let c = connect(&b, port(&p, "drone:mix.in3"), port(&p, "voice:voice.out")).expect("backwards");
        assert_eq!(a.rig, c.rig);
    }

    #[test]
    fn a_wrong_type_is_refused_with_the_cores_sentence() {
        let (b, p) = catpope();
        let e = connect(&b, port(&p, "voice:voice.out"), port(&p, "drone:drone.freq")).expect_err("audio into a control input");
        assert!(e.contains("cannot feed"), "{e}");
    }

    #[test]
    fn a_cable_inside_a_track_can_be_drawn() {
        let (b, p) = catpope();
        let next = connect(&b, port(&p, "drone:drone.out"), port(&p, "drone:fifth.cv")).expect("the instrument holds it");
        assert_eq!(port(&again(&next), "drone:fifth.cv").cables[0].from, "drone:drone.out");
    }

    #[test]
    fn a_cable_is_cut_and_gone() {
        let (b, p) = catpope();
        let next = cut(&b, port(&p, "voice:voice.cv")).expect("the instrument's link");
        assert!(port(&again(&next), "voice:voice.cv").cables.is_empty());
    }

    #[test]
    fn a_cable_is_moved_to_another_input() {
        let (b, p) = catpope();
        let next = moved(&b, &p, port(&p, "voice:voice.cv"), port(&p, "drone:mix.in3")).expect("moved");
        let q = again(&next);
        assert!(port(&q, "voice:voice.cv").cables.is_empty());
        assert_eq!(port(&q, "drone:mix.in3").cables[0].from, "voice:melody.cv");
    }

    #[test]
    fn a_dragged_cable_can_reach_something_and_not_a_control_input() {
        let (b, p) = catpope();
        let r = reachable(&b, &p, port(&p, "voice:melody.cv"));
        let can: Vec<&str> = p.iter().zip(&r).filter(|(_, ok)| **ok).map(|(q, _)| q.address.as_str()).collect();
        assert!(can.contains(&"drone:mix.in3"), "{can:?}");
        assert!(!can.contains(&"voice:voice.freq"), "{can:?}");
    }

    #[test]
    fn a_reload_shows_the_same_cables_rig_and_instrument() {
        let (b, p) = catpope();
        let a = connect(&b, port(&p, "voice:voice.out"), port(&p, "drone:mix.in3")).expect("a route");
        let next = cut(&a, port(&p, "voice:voice.cv")).expect("an instrument's link");
        let file = draft_file("draft");
        save_draft(&file, &b, &next).expect("written");
        let read = load_draft(&file, &b);
        assert_eq!((&read.rig, &read.instruments), (&next.rig, &next.instruments));
        std::fs::remove_file(&file).ok();
    }

    fn node_ids(p: &[Port]) -> Vec<String> {
        let mut ids: Vec<String> = p.iter().map(|q| q.node.clone()).collect();
        ids.dedup();
        ids.push("being".to_string());
        ids
    }

    #[test]
    fn from_outside_a_track_is_one_body_with_only_its_outlets() {
        let (b, p) = catpope();
        let ids = node_ids(&p);
        let core = ids.len() - 1;
        let (l, visible) = level(&b, &p, &ids, core, None);
        let seen: Vec<&str> = p.iter().zip(&visible).filter(|(_, v)| **v).map(|(q, _)| q.address.as_str()).collect();
        assert!(seen.contains(&"voice:voice.out"), "{seen:?}");
        assert!(seen.contains(&"drone:mix.in3"), "a free audio input: {seen:?}");
        assert!(!seen.contains(&"voice:voice.cv"), "fed inside the track: {seen:?}");
        assert!(!seen.contains(&"voice:melody.cv"), "taken inside the track: {seen:?}");
        assert!(!seen.contains(&"drone:drone.out"), "taken inside the track: {seen:?}");
        let reps: std::collections::BTreeSet<usize> = l.shown.iter().flatten().copied().collect();
        assert_eq!(reps.len(), 3, "two tracks and the being: {:?}", l.shown);
    }

    #[test]
    fn inside_a_track_is_all_of_it_and_nothing_else() {
        let (b, p) = catpope();
        let ids = node_ids(&p);
        let (l, visible) = level(&b, &p, &ids, ids.len() - 1, Some("voice"));
        assert!(p.iter().zip(&visible).all(|(q, v)| *v == (q.track == "voice")));
        assert_eq!(l.shown.iter().flatten().count(), 2, "melody and voice: {:?}", l.shown);
    }

    #[test]
    fn a_broken_draft_is_left_out() {
        let (b, _) = catpope();
        let file = draft_file("broken");
        std::fs::create_dir_all(file.parent().expect("a folder")).expect("made");
        std::fs::write(&file, "not toml [").expect("written");
        assert_eq!(load_draft(&file, &b).rig, b.rig);
        std::fs::remove_file(&file).ok();
    }
}
