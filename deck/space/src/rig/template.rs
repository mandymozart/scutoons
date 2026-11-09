//! The `rig_policy` template (#412, T6 of #314; research/RIG-MODULE-GROUPS.md §2 Q4, decision 5): what "New policy" on a
//! rig's Rust tab starts from – a control-domain module for one rig, written by the core from its `RigFace`, the sources of
//! a `module:<id>` document of its own. It compiles and plays as it is and changes nothing: every unmuted track's `@gain`
//! at 1 (`mul`, the track's own gain). Everything else the policy can drive is there as a line to uncomment: each track's
//! `@preset` (its instrument's presets, the track's own first) and every addressable input with its range. The rig's
//! moods are the module's `moods`, so `mood_index` counts them. deck/build's real_compile compiles it for wasm32 and
//! validates the rig with what comes out.
use super::assign::catalog_bundle;
use super::face::{FaceTrack, RigFace};
use super::{Library, Preset, Rig};
use anyhow::{Context, Result};
use serde::Deserialize;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write;

/// the one source file of a new policy
pub const LIB: &str = "lib.rs";

/// `lib.rs` of a policy named `module` for the rig `face` describes (`module` = the `module:<id>` document's id: the checks
/// look the policy up by it)
pub fn rig_policy_template(face: &RigFace, module: &str) -> BTreeMap<String, String> {
    let mut names = Names::default();
    let mut ports = String::new();
    let mut gains = String::new();
    for t in &face.tracks {
        track_ports(t, &mut names, &mut ports, &mut gains);
    }
    let moods: Vec<&str> = face.moods.iter().map(String::as_str).filter(|m| !m.is_empty() && !m.contains('|')).collect();
    let (moods_attr, moods_line) = match moods.is_empty() {
        true => (String::new(), "0: the rig's tables name no mood – list some with `moods = \"…\"` in `#[module]`".to_string()),
        false => {
            let places: Vec<String> = moods.iter().enumerate().map(|(i, m)| format!("{} = {}", i + 1, one_line(m))).collect();
            (format!(", moods = {:?}", moods.join("|")), format!("0 = a mood the list does not name, {}", places.join(", ")))
        }
    };
    if gains.is_empty() {
        gains.push_str("        let _ = out;\n");
    }
    let subject = one_line(&face.subject);
    let doc = format!("the policy of rig {}", face.subject);
    let source = format!(
        r#"//! The policy of rig {subject}: a control module that drives the rig's tracks (once per block, a deck's mood and senses in
//! `ctx.character()`). Each output with a `path` writes into a track:
//! - `<track>:<node>.<port>` – a control input of its instrument;
//! - `<track>:@gain` – its mixer strip;
//! - `<track>:@preset` – its preset, by the index into `choices`.
//!
//! `mode`: `set` (the default) and `mul` make the policy the only writer – the rig's tables may not write that input too;
//! `add` layers over them. Uncomment a port to use it: its constant is `Self::<NAME IN CAPITALS>`.
// one `#[port(…)]` a port: clippy takes them for duplicates
#![allow(clippy::duplicated_attributes)]
use deck_module::{{Ctx, Module, Outputs}};

#[derive(Module)]
#[module(name = {module:?}, domain = "control", doc = {doc:?}{moods_attr})]
{ports}pub struct Policy {{
    /// the mood last written (None before the first block)
    mood: Option<u32>,
}}

impl Module for Policy {{
    fn control(&mut self, ctx: &Ctx, out: &mut Outputs) {{
        // {moods_line}
        let mood = ctx.character().mood_index;
        // an output holds its last value: write on a change only
        if self.mood == Some(mood) {{
            return;
        }}
        self.mood = Some(mood);
{gains}    }}
}}
"#
    );
    BTreeMap::from([(LIB.to_string(), source)])
}

/// a track's lines: the header, its `@gain` (live unless the track is muted: a muted track has no strip), its `@preset`
/// and its inputs (commented out)
fn track_ports(t: &FaceTrack, names: &mut Names, ports: &mut String, gains: &mut String) {
    let preset = t.preset.as_deref().map(|p| format!(" · preset {}", one_line(p))).unwrap_or_default();
    let muted = if t.mute { " · muted" } else { "" };
    let _ = writeln!(ports, "// track {} · instrument {}{preset}{muted}", one_line(&t.id), one_line(&t.instrument));
    let name = names.fresh(&format!("{}_gain", t.id));
    let gain = format!("#[port(output, control, name = {name:?}, path = {:?}, mode = \"mul\")]", format!("{}:@gain", t.id));
    if t.mute {
        let _ = writeln!(ports, "// {gain}");
    } else {
        let _ = writeln!(ports, "{gain}");
        let _ = writeln!(gains, "        out.set(Self::{}, 1.0); // 1 = the track's own gain, 0 = silent", name.to_ascii_uppercase());
    }
    if !t.presets.is_empty() {
        let name = names.fresh(&format!("{}_preset", t.id));
        let _ = writeln!(ports, "// #[port(output, control, name = {name:?}, path = {:?}, choices = {:?})]", format!("{}:@preset", t.id), t.presets.join("|"));
    }
    for a in &t.addressable {
        let name = names.fresh(&a.path.replace([':', '.'], "_"));
        let log = if a.log { ", log" } else { "" };
        let _ = writeln!(ports, "// #[port(output, control, name = {name:?}, path = {:?}, mode = \"set\")] // {} … {}{log}", a.path, a.min, a.max);
    }
}

/// output names: `[a-z0-9_]`, starting with a letter (they become constants), none twice
#[derive(Default)]
struct Names(BTreeSet<String>);

impl Names {
    fn fresh(&mut self, wanted: &str) -> String {
        let mut stem: String = wanted.chars().map(|c| if c.is_ascii_alphanumeric() { c.to_ascii_lowercase() } else { '_' }).collect();
        if !stem.starts_with(|c: char| c.is_ascii_alphabetic()) {
            stem.insert_str(0, "t_");
        }
        let mut name = stem.clone();
        let mut n = 2;
        while !self.0.insert(name.clone()) {
            name = format!("{stem}_{n}");
            n += 1;
        }
        name
    }
}

/// a name inside a comment stays on its line
fn one_line(s: &str) -> String {
    s.replace(['\n', '\r'], " ")
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct TemplateRequest {
    module: String,
    rig: Value,
    #[serde(default)]
    instruments: Vec<Value>,
    #[serde(default)]
    presets: Vec<Value>,
}

/// `{ module, rig, instruments, presets }` (the rig document and the catalog's documents) → `{ "lib.rs": … }` as JSON (the
/// wasm boundary): the template of a rig without a policy, every catalog preset of its instruments as `@preset` choices
pub fn template_json(request: &str) -> Result<String> {
    let r: TemplateRequest = serde_json::from_str(request).context("not a template request { module, rig, instruments, presets }")?;
    let mut rig: Rig = serde_json::from_value(r.rig).context("the rig")?;
    rig.policy = None;
    let bundle = catalog_bundle(rig, &r.instruments, &r.presets, &[])?;
    let own: BTreeSet<&str> = bundle.instruments.iter().map(|i| i.id.as_str()).collect();
    let presets = r.presets.iter().filter_map(|v| serde_json::from_value::<Preset>(v.clone()).ok()).filter(|p| own.contains(p.instrument.as_str()));
    let lib = Library::new(bundle.instruments.clone(), presets);
    Ok(serde_json::to_string(&rig_policy_template(&bundle.rig.describe(&lib)?, &r.module))?)
}

#[cfg(test)]
mod tests {
    use super::super::bundle::Bundle;
    use super::*;
    use serde_json::json;

    fn v<T: serde::Serialize>(x: &T) -> Value {
        serde_json::to_value(x).unwrap()
    }

    fn catpope() -> Bundle {
        Bundle::core("character.catpope").unwrap().unwrap()
    }

    fn source(b: &Bundle) -> String {
        rig_policy_template(&b.rig.describe(&b.library()).unwrap(), "catpope_rig").remove(LIB).unwrap()
    }

    #[test]
    fn the_template_names_the_module_its_moods_and_every_target_of_the_face() {
        let b = catpope();
        let text = source(&b);
        assert!(text.contains(r#"#[module(name = "catpope_rig", domain = "control", doc = "the policy of rig character.catpope", moods = "#), "{text}");
        let moods: Vec<String> = b.rig.moods.keys().cloned().collect();
        assert!(text.contains(&format!("moods = {:?}", moods.join("|"))), "{text}");
        assert!(text.contains(&format!("1 = {}", moods[0])), "the places of the moods: {text}");
        // the strip of each track is live, at 1
        for track in ["voice", "drone"] {
            assert!(text.contains(&format!("\n#[port(output, control, name = \"{track}_gain\", path = \"{track}:@gain\", mode = \"mul\")]\n")), "{text}");
            assert!(text.contains(&format!("out.set(Self::{}_GAIN, 1.0);", track.to_uppercase())), "{text}");
        }
        // its preset to choose, its own first; every addressable input with its range, commented out
        let face = b.rig.describe(&b.library()).unwrap();
        let voice = &face.tracks[0];
        assert_eq!(voice.presets[0], voice.preset.clone().unwrap(), "its own preset first");
        assert!(text.contains(&format!("// #[port(output, control, name = \"voice_preset\", path = \"voice:@preset\", choices = {:?})]", voice.presets.join("|"))), "{text}");
        for a in face.tracks.iter().flat_map(|t| &t.addressable) {
            assert!(text.contains(&format!("path = {:?}, mode = \"set\")] // {} … {}", a.path, a.min, a.max)), "{}: {text}", a.path);
        }
        assert!(!text.lines().any(|l| l.starts_with("#[port(") && l.contains("mode = \"set\"")), "no input is taken from the tables: {text}");
    }

    #[test]
    fn a_muted_track_has_no_live_strip_and_names_stay_constants() {
        let mut b = catpope();
        b.rig.moods.clear();
        b.rig.tracks[1].mute = true;
        // a track id that is no Rust identifier (a track id holds no ':', '.', '/', '@', '#' or space – rig/address.rs – but may start with a digit or hold a '-')
        b.rig.tracks[1].id = "2-drone".into();
        let text = source(&b);
        assert!(text.contains("// #[port(output, control, name = \"t_2_drone_gain\", path = \"2-drone:@gain\", mode = \"mul\")]"), "{text}");
        assert!(!text.contains("T_2_DRONE"), "not written: {text}");
        assert!(text.contains("// track 2-drone ·"), "a name stays on its comment line: {text}");
        assert!(text.contains("the rig's tables name no mood"), "{text}");
        assert!(!text.contains(", moods = "), "{text}");
        // nothing live at all: the module still uses `out`
        b.rig.tracks[0].mute = true;
        assert!(source(&b).contains("let _ = out;"));

        let mut names = Names::default();
        assert_eq!(["a_gain", "a-gain", "a.gain", "9"].map(|n| names.fresh(n)), ["a_gain", "a_gain_2", "a_gain_3", "t_9"]);
    }

    #[test]
    fn the_request_form_offers_the_catalog_s_presets_and_the_core_s_words() {
        let b = catpope();
        let (mut rig, instruments, mut presets) = (v(&b.rig), b.instruments.iter().map(v).collect::<Vec<_>>(), b.presets.iter().map(v).collect::<Vec<_>>());
        // a preset no track names is a choice all the same; another instrument's or a broken one is not
        presets.push(json!({ "id": "drone.extra", "instrument": "drone" }));
        presets.push(json!({ "id": "kazoo.base", "instrument": "kazoo" }));
        presets.push(json!({ "id": "drone.broken", "instrument": "drone", "values": "loud" }));
        rig["policy"] = json!({ "module": "old" }); // a template is written for the tracks: a policy there is not asked for
        let ask = |rig: &Value, module: &str| template_json(&json!({ "module": module, "rig": rig, "instruments": instruments, "presets": presets }).to_string()).map_err(|e| format!("{e:#}"));
        let files: BTreeMap<String, String> = serde_json::from_str(&ask(&rig, "mine").unwrap()).unwrap();
        assert_eq!(files.keys().collect::<Vec<_>>(), [LIB]);
        let text = &files[LIB];
        assert!(text.contains("name = \"mine\""), "{text}");
        assert!(text.contains("drone.extra") && !text.contains("kazoo.base") && !text.contains("drone.broken"), "{text}");
        let mut gone = v(&b.rig);
        gone["track"][0]["instrument"] = json!("nobody");
        assert_eq!(ask(&gone, "mine").unwrap_err(), "rig character.catpope: the instrument \"nobody\" is not in the catalog");
    }

    #[test]
    fn the_face_says_who_owns_a_track_s_preset_and_gain() {
        use crate::rig::policy::tests::code_catpope;
        let b = code_catpope();
        let face: Value = serde_json::from_str(&crate::rig::face::face_json(&serde_json::to_string(&b).unwrap()).unwrap()).unwrap();
        let voice = &face["tracks"][0];
        assert_eq!(voice["preset_driven"], json!({ "output": "policy.voice_preset", "mode": "set", "owned": true }));
        assert!(voice.get("gain_driven").is_none());
        let index = voice["addressable"].as_array().unwrap().iter().find(|a| a["path"] == "voice:voice.index").unwrap();
        assert_eq!(index["driven"], json!({ "output": "policy.voice_index", "mode": "set", "owned": true }));
        assert!(crate::rig::face::face_json("{").is_err());
    }
}
