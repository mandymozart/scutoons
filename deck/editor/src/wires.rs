//! The first dragged cable (#431, W1 of research/MODULE-WIRING.md; owner 2026-11-09: on the being as it is, the Wire side
//! of the one stage). Nothing is decided here (RULES A1): the ports and their addresses are the core's wiring, a gesture
//! is `rig::edit::Op` on the rig document, whether it may stand is `rig::validate`, its sentence is shown as it comes.
//! The draft is a TOML file of the rig document (`Doc::to_toml`) until the editor can sign in and save to the API.
use anyhow::{Context, Result};
use deck_space::rig::bundle::Played;
use deck_space::rig::edit::{Doc, Op};
use deck_space::rig::validate::Request;
use deck_space::rig::{Bundle, Rig};
use std::path::Path;

/// one port a route can end at
#[derive(Debug, Clone, PartialEq)]
pub struct Port {
    /// the flattened node's id: the graph's, so the being's point
    pub node: String,
    pub name: String,
    pub input: bool,
    pub ty: &'static str,
    /// the route's name for it (`voice:voice.out`)
    pub address: String,
    /// an input a cable already feeds
    pub linked: bool,
}

/// every port of the played rig's tracks
pub fn ports(played: &Played) -> Result<Vec<Port>> {
    let w = played.wiring()?;
    let mut out = Vec::new();
    for (i, n) in w.nodes.iter().enumerate() {
        for (input, list) in [(true, &n.inputs), (false, &n.outputs)] {
            for p in list {
                let Some(address) = n.address(&p.name) else { continue };
                let linked = input && w.links.iter().any(|l| l.to == i && l.input == p.name);
                out.push(Port { node: n.id.clone(), name: p.name.clone(), input, ty: p.ty, address, linked });
            }
        }
    }
    Ok(out)
}

/// `a` joined to `b` (dragged either way): the rig with the new route, or the core's sentence
pub fn connect(bundle: &Bundle, a: &Port, b: &Port) -> Result<Bundle, String> {
    let (from, to) = if a.input && !b.input { (b, a) } else { (a, b) };
    let mut doc = Doc::Rigs(bundle.rig.clone());
    doc.apply(&Op::Connect { from: from.address.clone(), to: to.address.clone(), scale: 1.0, offset: 0.0 }, &bundle.library())?;
    let Doc::Rigs(rig) = doc else { unreachable!("a rig stays a rig") };
    checked(bundle, rig)
}

/// the rig as the core's validate judges it, beside the bundle's instruments, presets and modules
fn checked(bundle: &Bundle, rig: Rig) -> Result<Bundle, String> {
    let values = |v: Result<serde_json::Value, serde_json::Error>| v.map_err(|e| e.to_string());
    let list = |v: serde_json::Value| v.as_array().cloned().unwrap_or_default();
    let request = Request {
        kind: "rigs".into(),
        id: rig.subject.clone(),
        document: values(serde_json::to_value(&rig))?,
        instruments: list(values(serde_json::to_value(&bundle.instruments))?),
        presets: list(values(serde_json::to_value(&bundle.presets))?),
        modules: list(values(serde_json::to_value(&bundle.modules))?),
    };
    let problems = request.problems()?;
    if !problems.is_empty() {
        return Err(problems.join("; "));
    }
    Ok(Bundle { rig, ..bundle.clone() })
}

/// the draft's rig over the bundle, if the file is there and still valid
pub fn load_draft(path: &Path, bundle: Bundle) -> Bundle {
    let Ok(text) = std::fs::read_to_string(path) else { return bundle };
    let read = Doc::from_toml("rigs", &text).map_err(|e| format!("{e:#}")).and_then(|doc| match doc {
        Doc::Rigs(rig) => checked(&bundle, rig),
        _ => Err("not a rig".to_string()),
    });
    match read {
        Ok(b) => {
            tracing::info!("draft: {}", path.display());
            b
        }
        Err(e) => {
            tracing::warn!("draft {}: {e}, left out", path.display());
            bundle
        }
    }
}

pub fn save_draft(path: &Path, rig: &Rig) -> Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).with_context(|| format!("making {}", dir.display()))?;
    }
    std::fs::write(path, Doc::Rigs(rig.clone()).to_toml()?).with_context(|| format!("writing {}", path.display()))
}

/// where the `i`th of a node's `n` inputs or outputs sits around its point (logical pixels): inputs left, outputs right
pub fn around(i: usize, n: usize, input: bool) -> (f32, f32) {
    const REACH: f32 = 32.0;
    const STEP: f32 = 0.5;
    let base = if input { std::f32::consts::PI } else { 0.0 };
    let side = if input { -1.0 } else { 1.0 };
    let angle = base + side * (i as f32 - (n as f32 - 1.0) * 0.5) * STEP;
    (angle.cos() * REACH, angle.sin() * REACH)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn catpope() -> (Bundle, Vec<Port>) {
        let b = Bundle::core("character.catpope").expect("the core rig").expect("it reads");
        let p = ports(&b.play(None, None).expect("it plays")).expect("it wires");
        (b, p)
    }

    fn port<'a>(all: &'a [Port], address: &str) -> &'a Port {
        all.iter().find(|p| p.address == address).unwrap_or_else(|| panic!("no port {address} in {:?}", all.iter().map(|p| &p.address).collect::<Vec<_>>()))
    }

    #[test]
    fn the_ports_are_the_tracks_by_their_route_addresses() {
        let (_, p) = catpope();
        assert!(!port(&p, "voice:voice.out").input);
        assert!(port(&p, "voice:voice.cv").linked, "the melody feeds it");
        assert!(p.iter().all(|x| x.address.contains(':')), "the scene's own nodes have no address");
    }

    #[test]
    fn a_dragged_cable_lands_in_the_rig_and_plays() {
        let (b, p) = catpope();
        let before = b.rig.routes.len();
        let next = connect(&b, port(&p, "voice:voice.out"), port(&p, "drone:mix.in3")).expect("it may stand");
        assert_eq!(next.rig.routes.len(), before + 1);
        next.play(None, None).expect("the new rig plays");
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
    fn a_cable_inside_a_track_is_the_instruments_and_refused() {
        let (b, p) = catpope();
        let e = connect(&b, port(&p, "drone:drone.out"), port(&p, "drone:drone.cv")).expect_err("the instrument's link is its home");
        assert!(e.contains("both ends are in track"), "{e}");
    }

    #[test]
    fn a_reload_shows_the_same_cable() {
        let (b, p) = catpope();
        let next = connect(&b, port(&p, "voice:voice.out"), port(&p, "drone:mix.in3")).expect("it may stand");
        let dir = std::env::temp_dir().join(format!("deck-editor-draft-{}", std::process::id()));
        let file = dir.join("rig.toml");
        save_draft(&file, &next.rig).expect("written");
        assert_eq!(load_draft(&file, b.clone()).rig, next.rig);
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn a_broken_draft_is_left_out() {
        let (b, _) = catpope();
        let dir = std::env::temp_dir().join(format!("deck-editor-broken-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("made");
        let file = dir.join("rig.toml");
        std::fs::write(&file, "not toml [").expect("written");
        assert_eq!(load_draft(&file, b.clone()).rig, b.rig);
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn inputs_sit_left_of_the_point_and_outputs_right() {
        for i in 0..5 {
            assert!(around(i, 5, true).0 < 0.0);
            assert!(around(i, 5, false).0 > 0.0);
        }
    }
}
