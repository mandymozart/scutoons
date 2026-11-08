//! The assignment (#283, research/RIG-EDITING.md §2 tab 4, §6): which character plays which rig – what the creator's
//! Assignment tab and its list of characters ask the core (space-web `rig_coverage`, `rig_catalog_bundle`):
//! - `coverage`: every character with the state of its default rig (`character.<id>`) – it plays (it is in the catalog,
//!   what scene-content serves), it is only drafted, or it is missing – and the personas' own rigs
//!   (`character.<id>.<persona>`, which scene-content prefers);
//! - `catalog_bundle`: a rig document with what it names picked from the catalog's instruments and presets (the presets'
//!   ancestors included) – the bundle `rig_play` plays, so a draft is heard before it is published.
//!
//! No standard rig (owner 2026-11-08 [rigs], #425: "eine figur ohne rig, sollte nicht klingen"): a character without a
//! default rig plays no sound – the deck's silent scene (`Played::silent`), the being without a sound. A code rig (#314) is a policy module
//! over the same document (DECISIONS 2026-11-03 [rigs]); the form edits the document, the code drives its parameters.
use super::{policy, Bundle, Expose, Instrument, Preset, PresetBook, Rig};
use anyhow::{bail, Context, Result};
use deck_catalog::Descriptor;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

/// A character as the registry has it: its id and its personas' ids.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Character {
    pub id: String,
    #[serde(default)]
    pub personas: Vec<String>,
}

/// A stored rig as the catalog lists it (GET /api/catalog/docs/rigs): its subject and its state.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct RigState {
    /// the subject (`character.<id>[.<persona>]`, `item.<id>`)
    pub id: String,
    /// in the catalog table – what scene-content serves, so a deck plays it
    #[serde(default)]
    pub live: bool,
    /// a newer version than the published one is waiting
    #[serde(default)]
    pub draft: bool,
    /// the newest version is a removal
    #[serde(default)]
    pub removed: bool,
}

/// How a subject's rig stands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    /// published: decks play it
    Plays,
    /// only a draft: the creator hears it, decks do not
    Drafted,
    Missing,
}

impl Status {
    fn of(rig: Option<&RigState>) -> Status {
        match rig {
            Some(r) if r.live => Status::Plays,
            Some(r) if r.draft && !r.removed => Status::Drafted,
            _ => Status::Missing,
        }
    }
}

/// One character's assignment.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Coverage {
    pub character: String,
    /// the default rig's subject, `character.<id>`
    pub subject: String,
    pub default: Status,
    /// the personas with a rig of their own (drafted or playing) and its state
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub personas: BTreeMap<String, Status>,
    /// no published default rig: the deck plays no sound for it (#425: the silent scene)
    pub missing: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct CoverageRequest {
    characters: Vec<Character>,
    #[serde(default)]
    rigs: Vec<RigState>,
}

/// Every character's assignment, in the order given.
pub fn coverage(characters: &[Character], rigs: &[RigState]) -> Vec<Coverage> {
    let by_id: BTreeMap<&str, &RigState> = rigs.iter().map(|r| (r.id.as_str(), r)).collect();
    characters
        .iter()
        .map(|c| {
            let subject = format!("character.{}", c.id);
            let default = Status::of(by_id.get(subject.as_str()).copied());
            let personas = c
                .personas
                .iter()
                .map(|p| (p.clone(), Status::of(by_id.get(format!("{subject}.{p}").as_str()).copied())))
                .filter(|(_, s)| *s != Status::Missing)
                .collect();
            Coverage { character: c.id.clone(), missing: default != Status::Plays, subject, default, personas }
        })
        .collect()
}

/// `{ characters: [{ id, personas }], rigs: [{ id, live, draft, removed }] }` → `[Coverage]` as JSON (the wasm boundary)
pub fn coverage_json(request: &str) -> Result<String> {
    let r: CoverageRequest = serde_json::from_str(request).context("not a coverage request { characters, rigs }")?;
    Ok(serde_json::to_string(&coverage(&r.characters, &r.rigs))?)
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct BundleRequest {
    rig: Value,
    #[serde(default)]
    instruments: Vec<Value>,
    #[serde(default)]
    presets: Vec<Value>,
    /// the catalog's `module:<id>` documents (`{ id, descriptor, … }`, #414): a code rig's policy is looked up in them
    #[serde(default)]
    modules: Vec<Value>,
}

fn id_of(v: &Value) -> Option<&str> {
    v.get("id").and_then(Value::as_str)
}

/// The rig with what it names from the catalog: its tracks' instruments, the presets its tracks, personas and moods name
/// with their ancestors, and a code rig's policy module with its descriptor (#414: the `module:<id>` document the compile
/// service wrote it into). Only what it names is read – a broken entry it does not name is not its problem.
pub fn catalog_bundle(rig: Rig, instruments: &[Value], presets: &[Value], modules: &[Value]) -> Result<Bundle> {
    let mut picked: Vec<Instrument> = Vec::new();
    for t in &rig.tracks {
        if picked.iter().any(|i| i.id == t.instrument) {
            continue;
        }
        let Some(v) = instruments.iter().find(|v| id_of(v) == Some(t.instrument.as_str())) else {
            bail!("rig {}: the instrument \"{}\" is not in the catalog", rig.subject, t.instrument)
        };
        picked.push(serde_json::from_value(v.clone()).with_context(|| format!("instrument {}", t.instrument))?);
    }
    // the book of every preset that reads; a named one that does not read says why
    let book = PresetBook::new(presets.iter().filter_map(|v| serde_json::from_value::<Preset>(v.clone()).ok()));
    // a code rig's policy: its module, and the presets its `@preset` outputs choose from – the deck replays the rig with them
    let modules = match &rig.policy {
        Some(policy) => vec![policy_descriptor(&rig.subject, &policy.module, modules)?],
        None => Vec::new(),
    };
    let named = rig
        .tracks
        .iter()
        .filter_map(|t| t.preset.as_deref())
        .chain(rig.personas.values().chain(rig.moods.values()).flat_map(|v| v.presets.values().map(String::as_str)))
        .chain(modules.iter().flat_map(policy::preset_choices));
    let mut chosen: BTreeMap<String, Preset> = BTreeMap::new();
    for id in named {
        if !book.presets.contains_key(id) {
            match presets.iter().find(|v| id_of(v) == Some(id)) {
                Some(v) => serde_json::from_value::<Preset>(v.clone()).map(|_| ()).with_context(|| format!("preset {id}"))?,
                None => bail!("rig {}: the preset \"{id}\" is not in the catalog", rig.subject),
            }
        }
        for p in book.lineage(id)? {
            chosen.entry(p.id.clone()).or_insert_with(|| p.clone());
        }
    }
    Ok(Bundle { rig, instruments: picked, presets: chosen.into_values().collect(), modules })
}

/// A code rig's policy module from the catalog's module documents: the descriptor the compile service stored in it. The
/// document's id is the policy's name (`Policy.module`), so the descriptor must carry it – the checks look it up by name.
fn policy_descriptor(subject: &str, module: &str, docs: &[Value]) -> Result<Descriptor> {
    let Some(doc) = docs.iter().find(|v| id_of(v) == Some(module)) else {
        bail!("rig {subject}: the policy module \"{module}\" is not in the catalog")
    };
    let Some(descriptor) = doc.get("descriptor").filter(|d| !d.is_null()) else {
        bail!("rig {subject}: the policy module \"{module}\" has no descriptor yet – compile it first")
    };
    let d: Descriptor = serde_json::from_value(descriptor.clone()).with_context(|| format!("the descriptor of module {module}"))?;
    if d.name != module {
        bail!("rig {subject}: the module document \"{module}\" carries the descriptor of \"{}\"", d.name)
    }
    Ok(d)
}

/// How many of a module's parameters its instrument shows (the signal strip's knobs): the first step of "start from an
/// existing module" (#425) shows the first few in the descriptor's order – choosing which, and wiring modules together,
/// is the patch editor's (#203).
const MODULE_KNOBS: usize = 8;

/// The instrument that wraps ONE module (#425, the way deck/native/rigs/catpope_analog.toml does by hand): a single
/// `module:<name>` node `module`, its first audio output as the track's `out`, the first few numeric parameters exposed.
/// A module without an audio output (a note source such as stepseq) cannot be a track on its own: it says so.
pub fn module_instrument(module: &str, descriptor: &Descriptor) -> Result<Instrument> {
    use deck_catalog::{Dir, PortType};
    if descriptor.name != module {
        bail!("the module document \"{module}\" carries the descriptor of \"{}\"", descriptor.name)
    }
    let Some(out) = descriptor.ports_of(Dir::Out, PortType::Audio).next() else {
        bail!("module:{module} has no audio output – it makes notes or control, not sound: wire it into a synth (the patch editor, #203)")
    };
    let expose = descriptor
        .params
        .iter()
        .filter(|p| p.choices.is_empty() && p.max > p.min)
        .take(MODULE_KNOBS)
        .map(|p| Expose { path: format!("module.{}", p.name), label: super::Text::from([("en".to_string(), p.name.to_string())]), range: Some([p.min, p.max]) })
        .collect();
    Ok(Instrument {
        id: module.to_string(),
        kind: super::InstrumentKind::Pad,
        name: super::Text::from([("en".to_string(), module.to_string())]),
        nodes: vec![crate::scene::NodeDef { id: "module".into(), kind: format!("module:{module}"), params: toml::Table::new() }],
        links: Vec::new(),
        out: format!("module.{}", out.name),
        expose,
        rights: Default::default(),
    })
}

/// `{ module: { id, descriptor, … } }` (a catalog `module:<id>` document, the descriptor the compile service wrote into
/// it) → the instrument that wraps it, as JSON for the catalog's `instruments` (the rig page's "start from existing modules")
pub fn module_instrument_json(request: &str) -> Result<String> {
    #[derive(Deserialize)]
    struct Request {
        module: Value,
    }
    let r: Request = serde_json::from_str(request).context("not a request { module }")?;
    let id = id_of(&r.module).context("the module document has no id")?;
    let Some(descriptor) = r.module.get("descriptor").filter(|d| !d.is_null()) else {
        bail!("module:{id} has no descriptor yet – compile it first")
    };
    let d: Descriptor = serde_json::from_value(descriptor.clone()).with_context(|| format!("the descriptor of module {id}"))?;
    Ok(serde_json::to_string(&module_instrument(id, &d)?)?)
}

/// `{ rig, instruments: [..], presets: [..], modules?: [..] }` (the catalog's documents) → the bundle as JSON for `rig_play`
pub fn catalog_bundle_json(request: &str) -> Result<String> {
    let r: BundleRequest = serde_json::from_str(request).context("not a bundle request { rig, instruments, presets, modules? }")?;
    let rig: Rig = serde_json::from_value(r.rig).context("the rig")?;
    Ok(serde_json::to_string(&catalog_bundle(rig, &r.instruments, &r.presets, &r.modules)?)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn rig(id: &str, live: bool, draft: bool, removed: bool) -> RigState {
        RigState { id: id.into(), live, draft, removed }
    }

    #[test]
    fn a_character_without_a_published_default_rig_is_missing() {
        let characters = [
            Character { id: "catpope".into(), personas: vec!["regular".into(), "dj".into()] },
            Character { id: "boy".into(), personas: vec!["regular".into()] },
            Character { id: "plf".into(), personas: vec![] },
            Character { id: "gone".into(), personas: vec![] },
        ];
        let rigs = [
            rig("character.catpope", true, true, false),
            rig("character.catpope.dj", false, true, false),
            rig("character.boy", false, true, false),
            rig("character.gone", false, true, true),
            rig("item.incense_burner", true, false, false),
        ];
        let c = coverage(&characters, &rigs);
        assert_eq!(c.iter().map(|c| (c.character.as_str(), c.default, c.missing)).collect::<Vec<_>>(), [
            ("catpope", Status::Plays, false),
            ("boy", Status::Drafted, true),
            ("plf", Status::Missing, true),
            ("gone", Status::Missing, true),
        ]);
        assert_eq!(c[0].subject, "character.catpope");
        assert_eq!(c[0].personas, BTreeMap::from([("dj".to_string(), Status::Drafted)]), "only personas with a rig of their own");
        // a published rig whose removal waits plays until the removal is published
        assert_eq!(coverage(&characters[3..], &[rig("character.gone", true, true, true)])[0].default, Status::Plays);
    }

    #[test]
    fn coverage_json_reads_the_catalog_list_form() {
        let out = coverage_json(r#"{"characters":[{"id":"catpope","personas":["regular"]},{"id":"boy"}],"rigs":[{"id":"character.catpope","live":true,"published":12}]}"#)
            .unwrap();
        let v: Value = serde_json::from_str(&out).unwrap();
        assert_eq!(v, json!([
            { "character": "catpope", "subject": "character.catpope", "default": "plays", "missing": false },
            { "character": "boy", "subject": "character.boy", "default": "missing", "missing": true },
        ]));
        assert!(coverage_json(r#"{"rigs":[]}"#).is_err(), "characters are required");
    }

    fn v<T: Serialize>(x: &T) -> Value {
        serde_json::to_value(x).unwrap()
    }

    /// the core rig of catpope as catalog documents
    fn catpope() -> (Value, Vec<Value>, Vec<Value>) {
        let b = Bundle::core("character.catpope").unwrap().unwrap();
        (v(&b.rig), b.instruments.iter().map(v).collect(), b.presets.iter().map(v).collect())
    }

    #[test]
    fn a_catalog_bundle_holds_what_the_rig_names_and_plays() {
        let (rig_doc, mut instruments, mut presets) = catpope();
        // catalog entries the rig does not name, one of them broken: not its problem
        instruments.push(json!({ "id": "kazoo", "kind": "lead", "color": "red" }));
        presets.push(json!({ "id": "kazoo.base", "instrument": "kazoo", "values": "loud" }));
        let request = json!({ "rig": rig_doc, "instruments": instruments, "presets": presets }).to_string();
        let b = Bundle::parse_json(&catalog_bundle_json(&request).unwrap()).unwrap();
        assert_eq!(b.instruments.iter().map(|i| i.id.as_str()).collect::<Vec<_>>(), ["cathedral_voice", "drone"]);
        assert!(b.presets.iter().all(|p| p.id != "kazoo.base"));
        let core = Bundle::core("character.catpope").unwrap().unwrap();
        let mut want: Vec<&str> = core.presets.iter().map(|p| p.id.as_str()).collect();
        want.sort();
        assert_eq!(b.presets.iter().map(|p| p.id.as_str()).collect::<Vec<_>>(), want, "every named preset with its ancestors");
        for mood in [None, Some("happy"), Some("sad")] {
            assert_eq!(b.play(None, mood).unwrap(), core.play(None, mood).unwrap(), "{mood:?}");
        }
    }

    #[test]
    fn a_catalog_bundle_names_what_is_missing_or_broken() {
        let (rig_doc, instruments, presets) = catpope();
        let ask = |rig: &Value, instruments: &[Value], presets: &[Value]| {
            catalog_bundle_json(&json!({ "rig": rig, "instruments": instruments, "presets": presets }).to_string()).map_err(|e| format!("{e:#}"))
        };
        let e = ask(&rig_doc, &instruments[1..], &presets).unwrap_err();
        assert_eq!(e, "rig character.catpope: the instrument \"cathedral_voice\" is not in the catalog");
        let mut broken = presets.clone();
        let at = broken.iter().position(|p| id_of(p) == Some("cathedral_voice.bright")).unwrap();
        broken[at]["values"] = json!("loud");
        assert!(ask(&rig_doc, &instruments, &broken).unwrap_err().starts_with("preset cathedral_voice.bright: "));
        let orphan: Vec<Value> = presets.iter().filter(|p| id_of(p) != Some("cathedral_voice.bright")).cloned().collect();
        assert_eq!(ask(&rig_doc, &instruments, &orphan).unwrap_err(), "rig character.catpope: the preset \"cathedral_voice.bright\" is not in the catalog");
        assert!(ask(&json!({ "subject": "character.x" }), &[], &[]).unwrap_err().starts_with("the rig"));
    }

    /// #414: a code rig's bundle carries the policy module's descriptor from the catalog's `module:<id>` document
    #[test]
    fn a_catalog_bundle_of_a_code_rig_holds_the_policy_s_descriptor() {
        use crate::rig::policy::tests::{code_catpope, CATPOPE_RIG};
        let code = code_catpope();
        let (rig_doc, instruments, presets) = (v(&code.rig), code.instruments.iter().map(v).collect::<Vec<_>>(), code.presets.iter().map(v).collect::<Vec<_>>());
        let descriptor: Value = serde_json::from_str(CATPOPE_RIG).unwrap();
        let ask = |modules: &[Value]| {
            catalog_bundle_json(&json!({ "rig": rig_doc, "instruments": instruments, "presets": presets, "modules": modules }).to_string()).map_err(|e| format!("{e:#}"))
        };
        // other modules in the catalog are not its problem; the document carries more than the descriptor (sources, wasm)
        let docs = [json!({ "id": "kazoo", "descriptor": "broken" }), json!({ "id": "catpope_rig", "files": { "lib.rs": "//" }, "descriptor": descriptor })];
        let b = Bundle::parse_json(&ask(&docs).unwrap()).unwrap();
        assert_eq!(b.modules, code.modules);
        // the presets the policy chooses from come along (no track, persona or mood names them), ancestors included
        let ids = |b: &Bundle| b.presets.iter().map(|p| p.id.clone()).collect::<std::collections::BTreeSet<_>>();
        assert!(code.rig.moods.is_empty() && ids(&b).contains("cathedral_voice.bright") && ids(&b).contains("cathedral_voice.low"));
        assert_eq!(ids(&b), ids(&code));
        assert_eq!(b.play(None, None).unwrap(), code.play(None, None).unwrap(), "it plays with the policy node");
        // a declarative rig takes no module, whatever the catalog holds
        let (declarative, di, dp) = catpope();
        let plain = catalog_bundle_json(&json!({ "rig": declarative, "instruments": di, "presets": dp, "modules": docs }).to_string()).unwrap();
        assert!(Bundle::parse_json(&plain).unwrap().modules.is_empty());
        // what is wrong with the module, in words
        assert_eq!(ask(&[]).unwrap_err(), "rig character.catpope: the policy module \"catpope_rig\" is not in the catalog");
        assert_eq!(
            ask(&[json!({ "id": "catpope_rig", "files": {} })]).unwrap_err(),
            "rig character.catpope: the policy module \"catpope_rig\" has no descriptor yet – compile it first"
        );
        assert_eq!(ask(&[json!({ "id": "catpope_rig", "descriptor": null })]).unwrap_err(), ask(&[json!({ "id": "catpope_rig" })]).unwrap_err());
        let other = CATPOPE_RIG.replace("\"catpope_rig\"", "\"other_rig\"");
        assert_eq!(
            ask(&[json!({ "id": "catpope_rig", "descriptor": serde_json::from_str::<Value>(&other).unwrap() })]).unwrap_err(),
            "rig character.catpope: the module document \"catpope_rig\" carries the descriptor of \"other_rig\""
        );
        assert!(ask(&[json!({ "id": "catpope_rig", "descriptor": { "domain": "control" } })]).unwrap_err().starts_with("the descriptor of module catpope_rig: "));
    }

    /// #425: "start from existing modules" – an instrument that wraps one module, as catpope_analog.toml does by hand
    #[test]
    fn a_module_becomes_an_instrument_with_one_node() {
        let synth = json!({
            "id": "synthx",
            "descriptor": { "name": "synthx", "domain": "audio", "ports": [{ "name": "notes", "dir": "in", "type": "control" }, { "name": "out", "dir": "out", "type": "audio" }],
                "params": [{ "name": "volume", "min": 0.0, "max": 1.0, "default": 0.6 }, { "name": "wave", "choices": ["saw", "sine"] }, { "name": "flat", "min": 1.0, "max": 1.0 }] }
        });
        let i: Instrument = serde_json::from_str(&module_instrument_json(&json!({ "module": synth }).to_string()).unwrap()).unwrap();
        assert_eq!((i.id.as_str(), i.out.as_str()), ("synthx", "module.out"));
        assert_eq!(i.nodes.iter().map(|n| (n.id.as_str(), n.kind.as_str())).collect::<Vec<_>>(), [("module", "module:synthx")]);
        assert_eq!(i.expose.iter().map(|e| (e.path.as_str(), e.range)).collect::<Vec<_>>(), [("module.volume", Some([0.0, 1.0]))], "a choice and a flat range are no knobs");
        // a note source has no sound of its own; a module nobody compiled has no descriptor; a wrong descriptor is refused
        let notes = json!({ "id": "steps", "descriptor": { "name": "steps", "ports": [{ "name": "notes", "dir": "out", "type": "event" }] } });
        assert!(module_instrument_json(&json!({ "module": notes }).to_string()).unwrap_err().to_string().contains("module:steps has no audio output"));
        assert_eq!(module_instrument_json(&json!({ "module": { "id": "raw" } }).to_string()).unwrap_err().to_string(), "module:raw has no descriptor yet – compile it first");
        let other = json!({ "id": "synthx", "descriptor": { "name": "other", "ports": [] } });
        assert!(module_instrument_json(&json!({ "module": other }).to_string()).unwrap_err().to_string().contains("carries the descriptor of \"other\""));
    }
}
