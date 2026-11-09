//! A code rig's policy (#408, T1 of #314; research/RIG-MODULE-GROUPS.md, decided 2026-11-05 [rigs]): a rig document may
//! name a control-domain module (its own `module:<id>` document) that drives the tracks. Flattened, it is the node
//! `policy` (a reserved track id) and drives children only through control outputs with a `path`:
//! - `<track>:<node>.<port>` – a control input of the track's instrument (graph.rs binds it like any module target);
//! - `<track>:@gain` – the track's mixer strip (`mixN.gainK`; the node's `@gain` table names it for graph.rs);
//! - `<track>:@preset` – the track's preset, its `choices` presets of the track's instrument (value = the index); the
//!   deck applies the choice on the frame thread by replaying the rig (#410), so it is no link.
//!
//! Outputs without a path are cables (`route from = "policy:<out>"`), inputs take routes (`to = "policy:<in>"`), its
//! parameters are rig knobs (`policy:<param>` in a persona's or mood's values). Who writes an input (decision 2): a
//! `set` or `mul` target is its only writer – no table value, route or second target may write it, and a preset's value
//! there is dropped with a warning (`Rig::warnings`); an `add` target layers over the tables. Only `@preset` and `@gain`
//! are reserved members (decision 4).
use super::address::{Address, Leaf};
use super::{instrument::canonical, Library, Rig};
use anyhow::{bail, Context, Result};
use deck_catalog::{Descriptor, Dir, Domain, Mode, PortType};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// The policy's node id in a flattened rig – a reserved track id
pub const NODE: &str = "policy";
/// The node's table naming each `@gain` target's strip input (`{ <track> = "mixN.gainK" }`), read by graph.rs
pub const GAIN_TABLE: &str = "@gain";

/// A rig's policy: the module that drives its tracks, the node's seed and its parameters' values.
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Policy {
    /// the `module:<id>` document's id – its descriptor's name
    pub module: String,
    /// the node's seed (the module seed rule: 1 unless set)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub seed: Option<u32>,
    /// its parameters' values by name (the rest keep their defaults); persona / mood values `policy:<param>` go on top
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub values: BTreeMap<String, f32>,
}

/// What a target path names inside its track
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Member {
    /// a control input of the track's instrument, `node.port`
    Input(String),
    /// the track's mixer strip
    Gain,
    /// the track's preset, one of `choices`
    Preset(Vec<String>),
}

/// One control output of the policy with a `path`.
#[derive(Debug, Clone, PartialEq)]
pub struct Target {
    /// the output's name (`policy.<output>` streams its value)
    pub output: String,
    pub path: String,
    pub track: String,
    pub member: Member,
    pub mode: Mode,
}

impl Target {
    /// a `set` or `mul` target is its input's only writer (an `add` layers over the tables)
    pub fn owns(&self) -> bool {
        self.mode != Mode::Add
    }
}

/// a mode as the documents write it
pub fn mode_name(m: Mode) -> &'static str {
    match m {
        Mode::Set => "set",
        Mode::Add => "add",
        Mode::Mul => "mul",
    }
}

/// The presets a policy module may choose from: the `choices` of its `<track>:@preset` outputs. A rig's bundle carries
/// them with their ancestors, though no track, persona or mood names them (the catalog's bundle, scene-content, #414).
pub fn preset_choices(d: &Descriptor) -> impl Iterator<Item = &str> {
    d.ports
        .iter()
        .filter(|p| p.dir == Dir::Out && p.ty == PortType::Control && matches!(Address::leaf(&p.path), Ok(Leaf::Member { member, .. }) if member == "preset"))
        .flat_map(|p| p.choices.iter().map(|c| &**c))
}

impl Rig {
    /// the policy and its module's descriptor; None for a rig without a policy
    pub fn policy_module<'a>(&'a self, lib: &'a Library) -> Result<Option<(&'a Policy, &'a Descriptor)>> {
        let Some(p) = &self.policy else { return Ok(None) };
        match lib.modules.get(&p.module) {
            Some(d) => Ok(Some((p, d))),
            None => bail!("rig {}: no module '{}' for its policy – the policy's module:{} document comes with the rig", self.subject, p.module, p.module),
        }
    }

    /// the policy's targets as the descriptor declares them, resolved against the tracks (empty without a policy)
    pub fn targets(&self, lib: &Library) -> Result<Vec<Target>> {
        let Some((_, d)) = self.policy_module(lib)? else { return Ok(Vec::new()) };
        let mut out: Vec<Target> = Vec::new();
        for p in d.ports.iter().filter(|p| p.dir == Dir::Out && p.ty == PortType::Control && !p.path.is_empty()) {
            let at = format!("rig {}: the policy's output '{}' drives '{}'", self.subject, p.name, p.path);
            let (track, inner) = match Address::leaf(&p.path) {
                Ok(Leaf::Port { track: Some(t), node, port }) => (t, format!("{node}.{port}")),
                Ok(Leaf::Member { track, member }) => (track, format!("@{member}")),
                _ => bail!("{at} – a target is <track>:<node>.<port>, <track>:@gain or <track>:@preset"),
            };
            let t = self.track(&track).with_context(|| at.clone())?;
            let member = match inner.as_str() {
                "@gain" => {
                    if t.mute {
                        bail!("{at} – the track is muted, so it has no strip");
                    }
                    if lib.instrument(&t.instrument)?.out.is_none() {
                        bail!("{at} – the track is silent (no `out`), so it has no strip");
                    }
                    Member::Gain
                }
                "@preset" => {
                    if p.mode != Mode::Set {
                        bail!("{at} – a preset is chosen (mode set), not {}", mode_name(p.mode));
                    }
                    if p.choices.is_empty() {
                        bail!("{at} – its `choices` name the presets it chooses from");
                    }
                    for c in p.choices.iter() {
                        self.preset_fits(lib, t, c).with_context(|| at.clone())?;
                    }
                    Member::Preset(p.choices.iter().map(|c| c.to_string()).collect())
                }
                m if m.starts_with('@') => bail!("{at} – only @preset and @gain are a track's reserved members"),
                _ => {
                    lib.instrument(&t.instrument)?.control_input(&inner).with_context(|| at.clone())?;
                    Member::Input(canonical(&inner))
                }
            };
            let target = Target { output: p.name.to_string(), path: p.path.to_string(), track, member, mode: p.mode };
            if let Some(o) = out.iter().find(|o| o.track == target.track && same_member(&o.member, &target.member)) {
                bail!("{at} – the output '{}' drives it too", o.output);
            }
            out.push(target);
        }
        Ok(out)
    }

    /// A code rig checked (Q5): the module is a compiled control module of ABI 2, its parameters' values in range, every
    /// target resolves (`targets`), and each input has one writer – a `set` / `mul` target's input is no table value and
    /// no route's target; a chosen preset is named by no persona or mood.
    pub(crate) fn check_policy(&self, lib: &Library) -> Result<()> {
        let Some((policy, d)) = self.policy_module(lib)? else { return Ok(()) };
        let who = format!("rig {}: the policy module '{}'", self.subject, policy.module);
        d.check().map_err(|e| anyhow::anyhow!("{who}: {e}"))?;
        if d.domain != Domain::Control {
            bail!("{who} runs in the {:?} domain – a policy is a control module", d.domain);
        }
        if d.abi != Some(2) {
            bail!("{who} speaks ABI {} – a policy is built for ABI 2", d.abi.map_or("1".to_string(), |a| a.to_string()));
        }
        for name in policy.values.keys() {
            self.policy_param(lib, name)?;
        }
        let variants = || self.personas.iter().map(|(n, v)| (format!("persona {n}"), v)).chain(self.moods.iter().map(|(n, v)| (format!("mood {n}"), v)));
        for t in self.targets(lib)?.iter().filter(|t| t.owns()) {
            match &t.member {
                Member::Input(path) => {
                    let writes = |p: &str| matches!(Address::leaf(p), Ok(Leaf::Port { track: Some(track), node, port }) if track == t.track && format!("{node}.{port}") == *path);
                    let only = "a set / mul target is the only writer of its input";
                    for (name, v) in variants() {
                        if v.values.keys().any(|p| writes(p)) {
                            bail!("rig {}: '{}' is the policy's ({}, {}): {name} sets it too – {only}", self.subject, t.path, t.output, mode_name(t.mode));
                        }
                    }
                    if let Some(r) = self.routes.iter().find(|r| writes(&r.to)) {
                        bail!("rig {}: '{}' is the policy's ({}, {}): the route from {} drives it too – {only}", self.subject, t.path, t.output, mode_name(t.mode), r.from);
                    }
                }
                Member::Preset(_) => {
                    for (name, v) in variants() {
                        if v.presets.contains_key(&t.track) {
                            bail!("rig {}: the policy chooses {}'s preset ({}): {name} names one too", self.subject, t.track, t.output);
                        }
                    }
                }
                Member::Gain => {}
            }
        }
        Ok(())
    }

    /// `name` is a parameter of the policy's module, and the policy's own value for it is in its range
    pub(crate) fn policy_param(&self, lib: &Library, name: &str) -> Result<()> {
        let Some((p, d)) = self.policy_module(lib)? else { bail!("rig {}: 'policy:{name}' – the rig has no policy", self.subject) };
        let Some(param) = d.param(name) else { bail!("rig {}: the policy module '{}' has no parameter '{name}'", self.subject, p.module) };
        if let Some(v) = p.values.get(name) {
            if !(v.is_finite() && (param.min..=param.max).contains(v)) {
                bail!("rig {}: the policy's {name} = {v} is outside {} … {}", self.subject, param.min, param.max);
            }
        }
        Ok(())
    }

    /// The inputs a `set` / `mul` target owns, as (track, `node.port`) – a preset's value there is dropped.
    pub fn owned(&self, lib: &Library) -> Result<BTreeSet<(String, String)>> {
        Ok(self
            .targets(lib)?
            .into_iter()
            .filter(|t| t.owns())
            .filter_map(|t| match t.member {
                Member::Input(path) => Some((t.track, path)),
                _ => None,
            })
            .collect())
    }

    /// What the core says but does not refuse (decision 2): each preset a track may play (its own, a persona's or mood's,
    /// a policy's choice) whose value on an owned input is dropped – once per preset and input; and a far world node
    /// (#432), which plays only while the deck its address names is in the session (#442).
    pub fn warnings(&self, lib: &Library) -> Result<Vec<String>> {
        let targets = self.targets(lib)?;
        let mut out = Vec::new();
        for (track, node, a) in self.far_world_nodes(lib) {
            out.push(format!("world node '{node}' (track {track}): listens over the relay for '{a}' – it plays only while the deck it names is in the session (#442)"));
        }
        for t in targets.iter().filter(|t| t.owns()) {
            let Member::Input(path) = &t.member else { continue };
            let track = self.track(&t.track)?;
            let mut plays: Vec<&str> = track.preset.iter().map(String::as_str).collect();
            for v in self.personas.values().chain(self.moods.values()) {
                plays.extend(v.presets.get(&t.track).map(String::as_str));
            }
            for c in targets.iter().filter(|c| c.track == t.track) {
                if let Member::Preset(choices) = &c.member {
                    plays.extend(choices.iter().map(String::as_str));
                }
            }
            for preset in plays {
                let resolved = lib.presets.resolve(preset)?;
                let line = format!("{preset} sets {path}, which the policy drives ({}, {}): dropped", t.path, mode_name(t.mode));
                if resolved.values.contains_key(path) && !out.contains(&line) {
                    out.push(line);
                }
            }
        }
        Ok(out)
    }

    /// The policy's parameters for `persona` and `mood`: its own values, then the persona's and the mood's
    /// `policy:<param>` values on top.
    pub(crate) fn policy_values(&self, persona: Option<&str>, mood: Option<&str>) -> BTreeMap<String, f32> {
        let mut out = self.policy.as_ref().map(|p| p.values.clone()).unwrap_or_default();
        for v in self.variants(persona, mood) {
            for (path, value) in &v.values {
                if let Ok(Leaf::Policy(name)) = Address::leaf(path) {
                    out.insert(name, *value);
                }
            }
        }
        out
    }
}

/// two targets on one track write the same thing
fn same_member(a: &Member, b: &Member) -> bool {
    match (a, b) {
        (Member::Input(x), Member::Input(y)) => x == y,
        (Member::Gain, Member::Gain) | (Member::Preset(_), Member::Preset(_)) => true,
        _ => false,
    }
}

#[cfg(test)]
#[path = "policy_tests.rs"]
pub(crate) mod tests;

/// The mixer strip of the `i`-th played track → (mixer number, its input): `mix1` takes the first four tracks, each next
/// mixer the one before on `in1` and three more tracks – the chain flatten.rs builds and `@gain` / `describe` name.
pub(crate) fn strip(i: usize) -> (usize, usize) {
    if i < 4 {
        (1, i + 1)
    } else {
        (2 + (i - 4) / 3, 2 + (i - 4) % 3)
    }
}
