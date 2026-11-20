//! Rig presets (decisions/deck.md [deck-editor], provisional): a named set of every knob of one rig ("soft", "dirty"), the
//! catalog document `rig-preset:<id>`. The rig keeps one working state (its documents as the hand left them); a preset is
//! saved on purpose from it, loaded into it, and shows dirty once a knob moves away from it. A factory preset is never
//! changed: saving it makes a derived one that names it in `origin` (the device presets' model, device_preset.rs). A scene's
//! holder plays its rig on a preset with single knob overrides (scene_document.rs `Holder::preset`).
use super::address::{Address, Leaf};
use super::Bundle;
use crate::device_preset::valid_id;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// knob address `<track>:<node>.<port>` → its value
pub type Values = BTreeMap<String, f32>;

/// how far two values may lie apart and still be the same knob position
const SAME: f32 = 1e-5;

#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RigPreset {
    /// `[a-z0-9][a-z0-9_.-]{0,63}`; a saved one's is `<rig subject>.<slug of its name>`
    pub id: String,
    /// the rig it is for: its subject (`character.catpope`)
    pub rig: String,
    /// what a person reads ("soft")
    pub name: String,
    /// read-only: saving it makes a derived preset
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub factory: bool,
    /// the preset this one was derived from
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub origin: Option<String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub values: Values,
}

/// What Save does with the loaded preset.
#[derive(Debug, Clone, PartialEq)]
pub enum Saved {
    /// your own, overwritten with the working state
    Over(RigPreset),
    /// a factory preset's child: a new preset of your own that names it in `origin`
    Derived(RigPreset),
}

impl Saved {
    pub fn preset(&self) -> &RigPreset {
        match self {
            Saved::Over(p) | Saved::Derived(p) => p,
        }
    }
}

impl RigPreset {
    /// Everything that keeps it from being a preset (empty = valid). Whether its knobs are the rig's is checked where it
    /// plays: a knob the rig lost is left out there.
    pub fn problems(&self) -> Vec<String> {
        let mut p = Vec::new();
        if !valid_id(&self.id) {
            p.push(format!("'{}' is not a preset id (a-z, 0-9, _ . -, at most 64)", self.id));
        }
        if self.name.trim().is_empty() {
            p.push("a preset has a name".into());
        }
        if super::address::Place::of_subject(&self.rig).is_none() {
            p.push(format!("'{}' is not a rig's subject (character.<id>, item.<id>)", self.rig));
        }
        match self.origin.as_deref() {
            Some(o) if o == self.id => p.push("a preset is not derived from itself".into()),
            Some(o) if !valid_id(o) => p.push("origin is a preset id".into()),
            _ => {}
        }
        for (key, v) in &self.values {
            if !matches!(Address::leaf(key), Ok(Leaf::Port { track: Some(_), .. })) {
                p.push(format!("value '{key}' is no knob of a track (<track>:<node>.<port>)"));
            } else if !v.is_finite() {
                p.push(format!("value '{key}' is no number"));
            }
        }
        p
    }

    /// the working state as a preset of `bundle`'s rig named `name`, its id one not in `taken`
    pub fn capture(bundle: &Bundle, name: &str, taken: &[&str]) -> Result<RigPreset, String> {
        Ok(RigPreset::of_values(&bundle.rig.subject, name, values_of(bundle)?, taken))
    }

    /// `values` as a new preset of your own for the rig `subject`, its id one not in `taken`
    pub fn of_values(subject: &str, name: &str, values: Values, taken: &[&str]) -> RigPreset {
        RigPreset { id: new_id(subject, name, taken), rig: subject.to_string(), name: name.trim().to_string(), factory: false, origin: None, values }
    }

    /// The knobs `now` differ from it: a knob it has at another value, or one it does not have.
    pub fn dirty(&self, now: &Values) -> bool {
        now.iter().any(|(k, v)| self.values.get(k).is_none_or(|p| (p - v).abs() > SAME * v.abs().max(1.0)))
    }

    /// Save with the working state `now`: your own is overwritten; a factory one stays and a derived one of yours names it
    /// (its id one not in `taken`).
    pub fn save(&self, now: &Values, taken: &[&str]) -> Saved {
        if !self.factory {
            return Saved::Over(RigPreset { values: now.clone(), ..self.clone() });
        }
        let name = format!("{} (yours)", self.name);
        Saved::Derived(RigPreset { id: new_id(&self.rig, &name, taken), rig: self.rig.clone(), name, factory: false, origin: Some(self.id.clone()), values: now.clone() })
    }

    /// `bundle` with its values written into the documents that hold its knobs (a track's preset, else its instrument),
    /// and the knobs it names that the rig has not got (left out)
    pub fn load(&self, bundle: &Bundle) -> (Bundle, Vec<String>) {
        let mut out = bundle.clone();
        let mut missing = Vec::new();
        for (key, v) in &self.values {
            let set = match Address::leaf(key) {
                Ok(Leaf::Port { track: Some(t), node, port }) => out.value_set(&t, &format!("{node}.{port}"), *v),
                _ => Err(format!("'{key}' is no knob")),
            };
            match set {
                Ok(b) => out = b,
                Err(_) => missing.push(key.clone()),
            }
        }
        (out, missing)
    }
}

/// Every knob of the rig's tracks as the working state sets it, by its address (a code rig's own parameters are not a
/// track's knobs: left out).
pub fn values_of(bundle: &Bundle) -> Result<Values, String> {
    let played = bundle.play(None, None).map_err(|e| format!("{e:#}"))?;
    Ok(played.knobs.iter().filter_map(|k| Some((knob_address(bundle, &k.path)?, k.value))).collect())
}

/// knob positions by their slots (an editor's hand on the played knobs) by their addresses; a slot no track's node names
/// is left out
pub fn values_at(rig: &super::Rig, slots: impl IntoIterator<Item = (String, f32)>) -> Values {
    slots.into_iter().filter_map(|(s, v)| Some((crate::scene_edit::value_key(rig, &s)?, v))).collect()
}

/// a played knob's slot (`drone_osc.freq`) as its address (`drone:osc.freq`): the track whose id prefixes the node
pub fn knob_address(bundle: &Bundle, slot: &str) -> Option<String> {
    crate::scene_edit::value_key(&bundle.rig, slot)
}

/// `<subject>.<slug of name>`, at most 64, with `-2`, `-3` … when it is taken
pub fn new_id(subject: &str, name: &str, taken: &[&str]) -> String {
    let mut slug = String::new();
    for c in name.trim().chars().flat_map(char::to_lowercase) {
        if c.is_ascii_alphanumeric() {
            slug.push(c);
        } else if !slug.ends_with('-') {
            slug.push('-');
        }
    }
    let slug = slug.trim_matches('-');
    let slug = if slug.is_empty() { "preset" } else { slug };
    let base: String = format!("{subject}.{slug}").chars().take(60).collect();
    let base = base.trim_end_matches(['-', '.']).to_string();
    let mut id = base.clone();
    let mut n = 2;
    while taken.contains(&id.as_str()) {
        id = format!("{base}-{n}");
        n += 1;
    }
    id
}

/// the presets of `subject` among `all`: the factory ones first, then by name
pub fn of_rig<'a>(all: &'a [RigPreset], subject: &str) -> Vec<&'a RigPreset> {
    let mut out: Vec<&RigPreset> = all.iter().filter(|p| p.rig == subject).collect();
    out.sort_by(|a, b| b.factory.cmp(&a.factory).then_with(|| a.name.cmp(&b.name)).then_with(|| a.id.cmp(&b.id)));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn catpope() -> Bundle {
        Bundle::core("character.catpope").expect("a core rig").expect("it reads")
    }

    #[test]
    fn capture_takes_every_knob_of_the_working_state_by_its_address() {
        let b = catpope();
        let p = RigPreset::capture(&b, " Soft  voice ", &[]).unwrap();
        assert_eq!((p.id.as_str(), p.name.as_str(), p.rig.as_str()), ("character.catpope.soft-voice", "Soft  voice", "character.catpope"));
        assert_eq!(p.values.len(), b.play(None, None).unwrap().knobs.iter().filter(|k| knob_address(&b, &k.path).is_some()).count());
        assert!(!p.values.is_empty() && p.values.keys().all(|k| k.contains(':')), "{:?}", p.values);
        assert_eq!(p.problems(), Vec::<String>::new());
        assert!(!p.dirty(&values_of(&b).unwrap()), "just captured: clean");
        assert_eq!(RigPreset::capture(&b, "Soft voice", &["character.catpope.soft-voice"]).unwrap().id, "character.catpope.soft-voice-2");
    }

    #[test]
    fn a_moved_knob_makes_it_dirty_and_loading_it_brings_the_knobs_back() {
        let b = catpope();
        let p = RigPreset::capture(&b, "soft", &[]).unwrap();
        let (key, v) = p.values.iter().next().map(|(k, v)| (k.clone(), *v)).unwrap();
        let mut moved = p.clone();
        moved.values.insert(key.clone(), v * 0.5 + 0.01);
        let (turned, missing) = moved.load(&b);
        assert!(missing.is_empty(), "{missing:?}");
        let now = values_of(&turned).unwrap();
        assert!((now[&key] - (v * 0.5 + 0.01)).abs() < 1e-4, "the knob is where the preset puts it");
        assert!(p.dirty(&now), "the knob moved away from the loaded preset");
        let (back, _) = p.load(&turned);
        assert!(!p.dirty(&values_of(&back).unwrap()), "loaded again: clean");
        let mut lost = p.clone();
        lost.values.insert("nothing:osc.freq".into(), 1.0);
        assert_eq!(lost.load(&b).1, ["nothing:osc.freq"], "a knob the rig lost is left out");
    }

    #[test]
    fn saving_your_own_overwrites_it_and_a_factory_one_derives() {
        let b = catpope();
        let now = values_of(&b).unwrap();
        let mut factory = RigPreset::capture(&b, "dirty", &[]).unwrap();
        factory.factory = true;
        factory.values.clear();
        let Saved::Derived(child) = factory.save(&now, &[]) else { panic!("a factory preset is never changed") };
        assert_eq!((child.origin.as_deref(), child.factory, child.name.as_str()), (Some("character.catpope.dirty"), false, "dirty (yours)"));
        assert_eq!((child.id.as_str(), &child.values), ("character.catpope.dirty-yours", &now));
        let Saved::Over(again) = child.save(&BTreeMap::new(), &[]) else { panic!("your own is overwritten") };
        assert_eq!((again.id, again.origin, again.values.len()), (child.id.clone(), child.origin.clone(), 0));
    }

    #[test]
    fn its_problems_name_what_is_wrong() {
        let p = RigPreset { id: "Bad id".into(), rig: "catpope".into(), name: " ".into(), factory: false, origin: Some("Bad id".into()), values: [("voice.gain".to_string(), 1.0), ("voice:osc.freq".into(), f32::NAN)].into() };
        assert_eq!(p.problems(), [
            "'Bad id' is not a preset id (a-z, 0-9, _ . -, at most 64)",
            "a preset has a name",
            "'catpope' is not a rig's subject (character.<id>, item.<id>)",
            "a preset is not derived from itself",
            "value 'voice.gain' is no knob of a track (<track>:<node>.<port>)",
            "value 'voice:osc.freq' is no number",
        ]);
        let json = r#"{"id":"character.catpope.soft","rig":"character.catpope","name":"Soft","values":{"voice:voice.index":4.0},"colour":1}"#;
        assert!(serde_json::from_str::<RigPreset>(json).unwrap_err().to_string().contains("unknown field `colour`"));
    }

    #[test]
    fn a_rigs_presets_list_factory_first_then_by_name() {
        let p = |id: &str, rig: &str, name: &str, factory: bool| RigPreset { id: id.into(), rig: rig.into(), name: name.into(), factory, origin: None, values: Values::new() };
        let all = [p("a.b", "character.catpope", "zeta", false), p("c", "character.nun", "x", true), p("d", "character.catpope", "alpha", false), p("e", "character.catpope", "warm", true)];
        let ids: Vec<&str> = of_rig(&all, "character.catpope").iter().map(|p| p.id.as_str()).collect();
        assert_eq!(ids, ["e", "d", "a.b"]);
    }
}
