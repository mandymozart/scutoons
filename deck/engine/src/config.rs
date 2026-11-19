//! The deck configuration, audio part (plan 2.6b, #318): one device's hardware binding as plain data. Scenes write to
//! outlets (audio groups: "main", "cue"), never to hardware; this says which device channels exist, what they are called and
//! which of them an outlet group plays on, and what happens to an outlet that has more channels than the device offers. The
//! native deck fills it from its backend (the count of channels the device reports), a saved document may supply the rest
//! (#330); nothing set = a stereo device with `main` on channels 1 and 2.
//!
//! The engine decides everything from it – the adapter only reports the count.
use super::layout::{self, Layout};
use serde::{Deserialize, Serialize};

/// What an outlet with more channels than the device channels it is given does.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Fold {
    /// mix down with the spec's table between the outlet's layout and the layout of the channels it has (5.1 on two
    /// channels = BS.775); where the spec has no table, as `Drop`
    #[default]
    Speakers,
    /// each channel plays on its device channel where the device has it; the others are dropped
    Drop,
}

/// The device channels an outlet group plays on, counted from 1, in the group's channel order.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OutletMap {
    pub group: String,
    pub channels: Vec<usize>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DeckConfig {
    pub name: String,
    /// how many channels the device has (the backend reports it); 1..=32
    pub channels: usize,
    /// the device channels' names ("Out 1" … a Fireface's "ADAT 3"); a missing name is the channel's number
    #[serde(default)]
    pub names: Vec<String>,
    /// group → device channels; `main` without an entry plays on channels 1.., any other group without one is silent
    #[serde(default)]
    pub outlets: Vec<OutletMap>,
    /// speaker layouts of this venue, over the built-ins of the same name (layout.rs)
    #[serde(default)]
    pub layouts: Vec<Layout>,
    #[serde(default)]
    pub fold: Fold,
    /// every outlet plays by its group: a scene's pinned device channels (`first`, `channels`) are set aside (a preview, `folded`)
    #[serde(default)]
    pub unpinned: bool,
}

/// the most device channels (the engine's channel limit, plan 2.6)
pub const MAX_DEVICE_CHANNELS: usize = 32;

impl Default for DeckConfig {
    fn default() -> DeckConfig {
        DeckConfig::stereo()
    }
}

impl DeckConfig {
    /// two channels, `main` on both – what a phone is, and what a build gets when nothing says otherwise
    pub fn stereo() -> DeckConfig {
        DeckConfig::device(2)
    }

    /// a device with `channels` channels (what its backend reports), nothing configured: `main` plays on its first channels
    pub fn device(channels: usize) -> DeckConfig {
        DeckConfig { name: format!("{channels}-channel device"), channels: channels.clamp(1, MAX_DEVICE_CHANNELS), names: Vec::new(), outlets: Vec::new(), layouts: Vec::new(), fold: Fold::Speakers, unpinned: false }
    }

    /// the group plays on these device channels (counted from 1)
    pub fn with_outlet(mut self, group: &str, channels: &[usize]) -> DeckConfig {
        self.outlets.retain(|o| o.group != group);
        self.outlets.push(OutletMap { group: group.to_string(), channels: channels.to_vec() });
        self
    }

    /// A preview's device (cue.rs): two channels that every one of `groups` plays on, folded by the speaker tables, pins set
    /// aside – a scene of any width as one stereo pair.
    pub fn folded(groups: &[String]) -> DeckConfig {
        let deck = groups.iter().fold(DeckConfig::device(2), |d, g| d.with_outlet(g, &[1, 2]));
        DeckConfig { name: "the cue's stereo fold".into(), unpinned: true, ..deck }
    }

    pub fn with_layout(mut self, layout: Layout) -> DeckConfig {
        self.layouts.retain(|l| l.name != layout.name);
        self.layouts.push(layout);
        self
    }

    /// the name of device channel `c` (counted from 0)
    pub fn channel_name(&self, c: usize) -> String {
        self.names.get(c).cloned().unwrap_or_else(|| (c + 1).to_string())
    }

    /// a layout by name: this configuration's, else a built-in
    pub fn layout(&self, name: &str) -> Option<Layout> {
        self.layouts.iter().find(|l| l.name == name).cloned().or_else(|| layout::builtin(name))
    }

    /// The device channel (counted from 0) each channel of a group of `width` channels plays on, in the group's order: the
    /// configuration's entry, else for `main` channels 1..=width, else none. A channel the device lacks is None – it keeps its
    /// place, the next one does not move up.
    pub fn channels_of(&self, group: &str, width: usize) -> Vec<Option<usize>> {
        let listed: Vec<usize> = match self.outlets.iter().find(|o| o.group == group) {
            Some(o) => o.channels.clone(),
            None if group == "main" => (1..=width).collect(),
            None => Vec::new(),
        };
        listed.into_iter().map(|c| (c >= 1 && c <= self.channels).then(|| c - 1)).collect()
    }
}

/// What is wrong with a machine's channel map (deck.toml's `[[audio_outlet]]`) on a device: the map is refused while there is one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MapProblem {
    Unnamed,
    Twice(String),
    Outside { group: String, channel: usize, device: usize },
    Repeated { group: String, channel: usize },
}

impl std::fmt::Display for MapProblem {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            MapProblem::Unnamed => write!(f, "an outlet group without a name"),
            MapProblem::Twice(g) => write!(f, "the group '{g}' is mapped twice"),
            MapProblem::Outside { group, channel, device } => write!(f, "'{group}': channel {channel} is not on the device (it has 1–{device})"),
            MapProblem::Repeated { group, channel } => write!(f, "'{group}': channel {channel} is listed twice"),
        }
    }
}

/// The problems of `outlets` on a device of `device` channels: every group once and named, its channels counted from 1 within
/// the device, none twice in a group. Two groups may share a channel (they sum there).
pub fn check_outlets(outlets: &[OutletMap], device: usize) -> Vec<MapProblem> {
    let mut out = Vec::new();
    for (i, o) in outlets.iter().enumerate() {
        if o.group.trim().is_empty() {
            out.push(MapProblem::Unnamed);
        } else if outlets[..i].iter().any(|p| p.group == o.group) {
            out.push(MapProblem::Twice(o.group.clone()));
        }
        for (k, &c) in o.channels.iter().enumerate() {
            if c < 1 || c > device {
                out.push(MapProblem::Outside { group: o.group.clone(), channel: c, device });
            } else if o.channels[..k].contains(&c) {
                out.push(MapProblem::Repeated { group: o.group.clone(), channel: c });
            }
        }
    }
    out
}

/// A test tone on a machine's device: one device channel (counted from 1), or where an outlet group plays.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Tone {
    Channel(usize),
    Group(String),
}

impl Tone {
    /// the tone's scene: a quiet 440 Hz sine (−20 dBFS) into an `out` pinned to the channel, or into the group's outlet
    pub fn scene(&self) -> String {
        let out = match self {
            Tone::Channel(c) => format!("channels = [{c}]"),
            Tone::Group(g) => format!("group = {}", toml::Value::String(g.clone())),
        };
        format!("[stage]\nname = \"test tone\"\n\n[[node]]\nid = \"tone\"\nkind = \"osc\"\nfreq = 440\ngain = 0.1\n\n[[node]]\nid = \"out\"\nkind = \"out\"\n{out}\n\n[[link]]\nfrom = \"tone\"\nto = \"out.in\"\n")
    }
}

/// What the room does to the sum of the outlets on the device channels, before the engine's limiter (plan 2.6b): speaker
/// EQ, delays, shaping. It stays loaded while scenes come and go. The engine has the hook and a pass-through; the
/// environment's documents and graph are #330's.
pub trait StageEnvironment: Send {
    /// the first `n` frames of every device channel of `dev`, in place
    fn process(&mut self, dev: &mut super::Buf, n: usize);
}

/// the stage environment of a deck nobody assigned one to: the outlets go straight on
pub struct PassThrough;

impl StageEnvironment for PassThrough {
    fn process(&mut self, _: &mut super::Buf, _: usize) {}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_device_plays_main_on_its_first_channels_and_nothing_else() {
        let on = |c: &[usize]| c.iter().map(|&c| Some(c)).collect::<Vec<_>>();
        let stereo = DeckConfig::stereo();
        assert_eq!(stereo.channels_of("main", 2), on(&[0, 1]));
        assert_eq!(stereo.channels_of("main", 6), [Some(0), Some(1), None, None, None, None], "a 5.1 main on two channels: the channels there are");
        assert_eq!(stereo.channels_of("cue", 2), []);
        let six = DeckConfig::device(6);
        assert_eq!(six.channels_of("main", 6), on(&[0, 1, 2, 3, 4, 5]));
        let rack = DeckConfig::device(8).with_outlet("main", &[1, 2]).with_outlet("cue", &[3, 4]);
        assert_eq!((rack.channels_of("main", 2), rack.channels_of("cue", 2)), (on(&[0, 1]), on(&[2, 3])));
        assert_eq!(DeckConfig::device(3).with_outlet("cue", &[3, 4]).channels_of("cue", 2), [Some(2), None], "channel 4 does not exist");
        assert_eq!(DeckConfig::device(3).with_outlet("cue", &[4, 3]).channels_of("cue", 2), [None, Some(2)], "the second keeps its place");
    }

    #[test]
    fn a_channel_map_is_checked_against_the_device() {
        let map = |g: &str, c: &[usize]| OutletMap { group: g.into(), channels: c.to_vec() };
        assert!(check_outlets(&[map("main", &[1, 2]), map("cue", &[3, 4])], 4).is_empty());
        assert!(check_outlets(&[map("main", &[1, 2]), map("cue", &[1, 2])], 2).is_empty(), "two groups may share channels");
        assert_eq!(check_outlets(&[map("cue", &[3, 4])], 2), [
            MapProblem::Outside { group: "cue".into(), channel: 3, device: 2 },
            MapProblem::Outside { group: "cue".into(), channel: 4, device: 2 },
        ]);
        assert_eq!(check_outlets(&[map("cue", &[3]), map("cue", &[4])], 8), [MapProblem::Twice("cue".into())]);
        assert_eq!(check_outlets(&[map("main", &[1, 1])], 8), [MapProblem::Repeated { group: "main".into(), channel: 1 }]);
        assert_eq!(check_outlets(&[map(" ", &[1]), map("x", &[0])], 8)[0], MapProblem::Unnamed);
        assert_eq!(check_outlets(&[map("x", &[0])], 8)[0].to_string(), "'x': channel 0 is not on the device (it has 1–8)");
    }

    #[test]
    fn a_test_tone_is_a_scene_with_one_outlet() {
        let ch = Tone::Channel(3).scene();
        assert!(ch.contains("channels = [3]") && ch.contains("kind = \"osc\""));
        let g = Tone::Group("a \"b\"".into()).scene();
        let parsed: toml::Value = toml::from_str(&g).unwrap();
        assert_eq!(parsed["node"][1]["group"].as_str(), Some("a \"b\""));
        let graph = |s: &str| deck_space::graph::Graph::build(&deck_space::scene::Scene::parse(s).unwrap()).unwrap();
        assert_eq!(crate::outlet_groups(&graph(&Tone::Group("cue".into()).scene())), ["main", "cue"], "main always, then the scene's groups");
        assert_eq!(crate::outlet_groups(&graph(&Tone::Channel(2).scene())), ["main"]);
    }

    #[test]
    fn it_is_a_document() {
        let c = DeckConfig::device(8).with_outlet("cue", &[3, 4]).with_layout(Layout::new("pair", &[layout::builtin("stereo").unwrap().speakers[0]]));
        let json = serde_json::to_string(&c).unwrap();
        assert_eq!(serde_json::from_str::<DeckConfig>(&json).unwrap(), c);
        let small: DeckConfig = serde_json::from_str(r#"{"name":"phone","channels":2}"#).unwrap();
        assert_eq!((small.fold, small.outlets.len(), small.channel_name(1)), (Fold::Speakers, 0, "2".to_string()));
        assert_eq!(c.layout("stereo").unwrap().channels(), 2, "the built-ins stay");
    }
}
