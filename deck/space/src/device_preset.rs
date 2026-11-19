//! Device presets: a machine's audio settings – the host, the device, the buffer, the outlet groups on the device's
//! channels – and the hardware they are for, as the catalog document `device-preset:<id>`. Factory presets ship in every deck
//! (`presets/device-presets.toml`, compiled in, so a fresh deck has them offline) and are read-only; a member's own are catalog
//! documents, and a changed factory preset is a derived one that names it in `origin`. deck.toml names the preset it plays
//! with and keeps only what differs from it (`Settings::under`, `Settings::over`); a deck that finds hardware a preset is for
//! offers it (`Library::recommend`).
use serde::{Deserialize, Serialize};

/// the most device channels (the engine's channel limit, plan 2.6)
pub const MAX_DEVICE_CHANNELS: usize = 32;

/// The device channels an outlet group plays on, counted from 1, in the group's channel order.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct OutletMap {
    pub group: String,
    pub channels: Vec<usize>,
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

/// a device named `name` is the one `want` picks (deck.toml's `audio_device`, a preset's device or hardware name): the name
/// holds it, any case
pub fn device_matches(name: &str, want: &str) -> bool {
    name.to_lowercase().contains(&want.to_lowercase())
}

/// the hosts deck.toml's `audio_host` names (none = the platform's default: PipeWire on a deck)
pub const HOSTS: [&str; 3] = ["default", "asio", "wasapi"];

/// A USB device's ids, where the OS reports them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct Usb {
    pub vendor: u16,
    pub product: u16,
}

/// The hardware a preset is for: everything it names must hold for a device, and it names at least one thing.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct Hardware {
    /// parts of the device's name, any case: one of them is in its name or description ('Fireface')
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub names: Vec<String>,
    /// the driver layer that lists it, lowercase: asio · wasapi · pipewire · alsa · coreaudio
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub driver: Option<String>,
    /// the output channels the driver reports
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub channels: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub usb: Option<Usb>,
}

/// A device as a deck finds it: what a preset's hardware is matched against.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Found {
    pub name: String,
    pub description: String,
    /// the driver layer that lists it, lowercase
    pub driver: String,
    pub channels: usize,
    pub usb: Option<Usb>,
}

impl Hardware {
    /// how much of this the device `f` is (the number of things named), None when one of them does not hold
    pub fn fits(&self, f: &Found) -> Option<u32> {
        let mut score = 0;
        if !self.names.is_empty() {
            let named = self.names.iter().any(|n| device_matches(&f.name, n) || (!f.description.is_empty() && device_matches(&f.description, n)));
            if !named {
                return None;
            }
            score += 1;
        }
        if let Some(d) = &self.driver {
            if !d.eq_ignore_ascii_case(&f.driver) {
                return None;
            }
            score += 1;
        }
        if let Some(c) = self.channels {
            if c != f.channels {
                return None;
            }
            score += 1;
        }
        if let Some(u) = self.usb {
            if f.usb != Some(u) {
                return None;
            }
            score += 2;
        }
        Some(score)
    }
}

/// A device preset: the catalog document `device-preset:<id>` and an entry of the factory file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct DevicePreset {
    /// `[a-z0-9][a-z0-9_.-]{0,63}`; a factory preset's is `<hardware>.<driver>` ('rme-ucx2.asio')
    pub id: String,
    /// what a person reads ('RME Fireface UCX II · ASIO')
    pub name: String,
    /// one of the presets every deck ships with: read-only, only the factory file sets it
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub factory: bool,
    /// the preset this one was derived from
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub origin: Option<String>,
    pub hardware: Hardware,
    /// deck.toml's `audio_host` (default · asio · wasapi); none = the platform's default
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub host: Option<String>,
    /// deck.toml's `audio_device`: a part of the device's name; none = the device the preset is recommended for
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub device: Option<String>,
    /// frames per device period; none = the device's default
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub buffer: Option<u32>,
    /// the outlet groups on the device's channels (deck.toml's `[[audio_outlet]]`)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub outlets: Vec<OutletMap>,
}

/// a preset's id: lowercase, digits, `_ . -`, at most 64
pub fn valid_id(id: &str) -> bool {
    let mut c = id.chars();
    matches!(c.next(), Some(f) if f.is_ascii_lowercase() || f.is_ascii_digit())
        && id.len() <= 64
        && c.all(|x| x.is_ascii_lowercase() || x.is_ascii_digit() || matches!(x, '_' | '.' | '-'))
}

impl DevicePreset {
    /// Everything that keeps it from being a preset (empty = valid). `factory` is the catalog's to refuse: a catalog write
    /// never makes one.
    pub fn problems(&self) -> Vec<String> {
        let mut p = Vec::new();
        if !valid_id(&self.id) {
            p.push(format!("'{}' is not a preset id (a-z, 0-9, _ . -, at most 64)", self.id));
        }
        if self.name.trim().is_empty() {
            p.push("a preset has a name".into());
        }
        if self.origin.as_deref() == Some(self.id.as_str()) {
            p.push("a preset is not derived from itself".into());
        } else if self.origin.as_deref().is_some_and(|o| !valid_id(o)) {
            p.push("origin is a preset id".into());
        }
        let h = &self.hardware;
        if h.names.is_empty() && h.usb.is_none() && h.channels.is_none() {
            p.push("the hardware it is for: a name, a channel count or a USB id".into());
        }
        if h.names.iter().any(|n| n.trim().is_empty()) {
            p.push("a hardware name is empty".into());
        }
        if h.driver.as_deref().is_some_and(|d| d.is_empty() || d.chars().any(|c| !c.is_ascii_lowercase())) {
            p.push("the driver is a lowercase word (asio, wasapi, pipewire, alsa, coreaudio)".into());
        }
        if h.channels.is_some_and(|c| !(1..=MAX_DEVICE_CHANNELS).contains(&c)) {
            p.push(format!("the hardware has 1–{MAX_DEVICE_CHANNELS} channels"));
        }
        if self.host.as_deref().is_some_and(|x| !HOSTS.contains(&x)) {
            p.push(format!("host is one of {}", HOSTS.join(", ")));
        }
        if self.device.as_deref().is_some_and(|d| d.trim().is_empty()) {
            p.push("the device is a part of its name, or left out".into());
        }
        if self.buffer.is_some_and(|b| !(16..=8192).contains(&b)) {
            p.push("the buffer is 16–8192 frames, or left out".into());
        }
        p.extend(check_outlets(&self.outlets, h.channels.unwrap_or(MAX_DEVICE_CHANNELS)).iter().map(ToString::to_string));
        p
    }

    /// the settings it gives a machine that found `device` (the device's name, when the preset names none)
    pub fn settings(&self, device: Option<&str>) -> Settings {
        Settings {
            host: self.host.clone().filter(|h| h != "default"),
            device: self.device.clone().or_else(|| device.map(str::to_string)),
            buffer: self.buffer,
            outlets: self.outlets.clone(),
        }
    }

    /// a preset of the member's own from `settings` on this preset's hardware: a factory one's child names it as its origin
    pub fn derive(&self, id: &str, name: &str, settings: &Settings) -> DevicePreset {
        DevicePreset {
            id: id.into(),
            name: name.into(),
            factory: false,
            origin: Some(self.id.clone()),
            hardware: self.hardware.clone(),
            host: settings.host.clone(),
            device: settings.device.clone(),
            buffer: settings.buffer,
            outlets: settings.outlets.clone(),
        }
    }

    /// a new preset from a machine's `settings` on the device it found
    pub fn from_machine(id: &str, name: &str, found: &Found, settings: &Settings) -> DevicePreset {
        DevicePreset {
            id: id.into(),
            name: name.into(),
            factory: false,
            origin: None,
            hardware: Hardware { names: vec![if found.description.is_empty() { found.name.clone() } else { found.description.clone() }], driver: Some(found.driver.clone()).filter(|d| !d.is_empty()), channels: Some(found.channels).filter(|&c| c > 0), usb: found.usb },
            host: settings.host.clone(),
            device: settings.device.clone(),
            buffer: settings.buffer,
            outlets: settings.outlets.clone(),
        }
    }
}

/// A machine's audio settings: deck.toml's keys, or what a preset gives.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Settings {
    pub host: Option<String>,
    pub device: Option<String>,
    pub buffer: Option<u32>,
    pub outlets: Vec<OutletMap>,
}

impl Settings {
    /// these (deck.toml's own keys) over a preset's: a key set here wins, a group mapped here replaces the preset's entry
    pub fn over(&self, preset: &Settings) -> Settings {
        let mut outlets: Vec<OutletMap> = preset.outlets.iter().filter(|o| !self.outlets.iter().any(|m| m.group == o.group)).cloned().collect();
        outlets.extend(self.outlets.iter().cloned());
        Settings {
            host: self.host.clone().or_else(|| preset.host.clone()),
            device: self.device.clone().or_else(|| preset.device.clone()),
            buffer: self.buffer.or(preset.buffer),
            outlets,
        }
    }

    /// what deck.toml keeps of these settings under a preset: only what differs from it (`over` gives them back)
    pub fn under(&self, preset: &Settings) -> Settings {
        Settings {
            host: self.host.clone().filter(|h| Some(h) != preset.host.as_ref()),
            device: self.device.clone().filter(|d| Some(d) != preset.device.as_ref()),
            buffer: self.buffer.filter(|b| Some(*b) != preset.buffer),
            outlets: self.outlets.iter().filter(|o| !preset.outlets.contains(o)).cloned().collect(),
        }
    }
}

/// Where a preset comes from: the order a recommendation takes them in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Source {
    /// published in the catalog by a member (the org's)
    Catalog,
    /// shipped with the deck
    Factory,
}

/// A preset as a deck knows it: the document, its version, where it is from, how many machines use it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Listed {
    pub version: u32,
    #[serde(default = "factory_source")]
    pub source: Source,
    #[serde(default)]
    pub uses: u32,
    pub document: DevicePreset,
}

fn factory_source() -> Source {
    Source::Factory
}

/// A preset offered for a device the deck found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Offer {
    /// the preset's place in the library
    pub preset: usize,
    /// the device's place in the list it was found in
    pub device: usize,
}

/// The presets a deck knows: the factory ones and those it got from the catalog.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Library {
    pub presets: Vec<Listed>,
}

#[derive(Deserialize)]
struct File {
    #[serde(default)]
    preset: Vec<Listed>,
}

/// the factory presets' file: shipped in every deck build
pub const FACTORY: &str = include_str!("../presets/device-presets.toml");

impl Library {
    /// the factory presets (a broken file is a build defect: the test reads it)
    pub fn factory() -> Library {
        Library::parse(FACTORY).unwrap_or_default()
    }

    /// a file of `[[preset]]` entries (`version`, `source`, `uses`, `[preset.document]`)
    pub fn parse(text: &str) -> Result<Library, String> {
        let f: File = toml::from_str(text).map_err(|e| e.to_string())?;
        Ok(Library { presets: f.preset })
    }

    /// the catalog's presets as the API lists them (JSON `[{ version, uses?, document }]`), after the factory ones
    pub fn with_catalog(mut self, listed: &[Listed]) -> Library {
        for l in listed {
            if l.document.factory || self.presets.iter().any(|p| p.document.id == l.document.id && p.version == l.version) {
                continue;
            }
            self.presets.push(Listed { source: Source::Catalog, ..l.clone() });
        }
        self
    }

    /// the catalog's answer (`GET /api/library/device-presets`: `{ presets: [{ version, document }] }`) after these; an answer
    /// that is none adds nothing, an entry that is not a preset is left out
    pub fn with_answer(self, json: &str) -> Library {
        let v: serde_json::Value = serde_json::from_str(json).unwrap_or_default();
        let listed: Vec<Listed> = v["presets"].as_array().into_iter().flatten().filter_map(|p| serde_json::from_value(p.clone()).ok()).collect();
        self.with_catalog(&listed)
    }

    /// the preset `id` at `version` (none = its newest)
    pub fn find(&self, id: &str, version: Option<u32>) -> Option<&Listed> {
        let all = self.presets.iter().filter(|p| p.document.id == id);
        match version {
            Some(v) => all.into_iter().find(|p| p.version == v),
            None => all.max_by_key(|p| p.version),
        }
    }

    /// a newer version of the preset `id` than `version`: offered as an update
    pub fn newer(&self, id: &str, version: u32) -> Option<&Listed> {
        self.find(id, None).filter(|p| p.version > version)
    }

    /// The presets for the devices found, best first: a device a preset fits, the catalog's (the org's) before the factory's,
    /// then the most used, then the one that names more of the device; one offer per preset (its best device).
    pub fn recommend(&self, found: &[Found]) -> Vec<Offer> {
        let mut offers: Vec<(Offer, (Source, std::cmp::Reverse<u32>, std::cmp::Reverse<u32>))> = Vec::new();
        for (i, p) in self.presets.iter().enumerate() {
            if self.find(&p.document.id, None).map(|n| n.version) != Some(p.version) {
                continue;
            }
            let best = found.iter().enumerate().filter_map(|(d, f)| p.document.hardware.fits(f).map(|s| (s, d))).max_by_key(|&(s, d)| (s, std::cmp::Reverse(d)));
            if let Some((score, d)) = best {
                offers.push((Offer { preset: i, device: d }, (p.source, std::cmp::Reverse(p.uses), std::cmp::Reverse(score))));
            }
        }
        offers.sort_by_key(|(_, k)| *k);
        offers.into_iter().map(|(o, _)| o).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn map(g: &str, c: &[usize]) -> OutletMap {
        OutletMap { group: g.into(), channels: c.to_vec() }
    }

    #[test]
    fn a_channel_map_is_checked_against_the_device() {
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
    fn the_factory_presets_are_valid_and_read_only() {
        let lib = Library::parse(FACTORY).expect("the factory file");
        let ids: Vec<&str> = lib.presets.iter().map(|p| p.document.id.as_str()).collect();
        assert_eq!(ids, ["rme-ucx2.asio", "rme-ucx2.pipewire", "x1.onboard", "stereo", "analog-four"]);
        for p in &lib.presets {
            assert!(p.document.problems().is_empty(), "{}: {:?}", p.document.id, p.document.problems());
            assert!(p.document.factory && p.source == Source::Factory && p.version >= 1, "{}", p.document.id);
        }
        assert_eq!(Library::factory(), lib);
    }

    #[test]
    fn a_preset_names_its_hardware_and_a_valid_map() {
        let mut p = Library::factory().find("rme-ucx2.asio", None).unwrap().document.clone();
        p.factory = false;
        assert!(p.problems().is_empty());
        p.hardware = Hardware::default();
        p.host = Some("jack".into());
        p.buffer = Some(3);
        p.outlets.push(map("cue", &[40]));
        p.origin = Some(p.id.clone());
        let said = p.problems().join(" | ");
        for want in ["the hardware it is for", "host is one of", "the buffer is", "channel 40", "not derived from itself"] {
            assert!(said.contains(want), "{want}: {said}");
        }
        assert!(!valid_id("Big") && !valid_id("") && valid_id("rme-ucx2.asio_2"));
    }

    fn rme_asio() -> Found {
        Found { name: "ASIO Fireface USB".into(), description: String::new(), driver: "asio".into(), channels: 20, usb: None }
    }

    #[test]
    fn hardware_fits_a_device_only_when_everything_it_names_holds() {
        let lib = Library::factory();
        let asio = &lib.find("rme-ucx2.asio", None).unwrap().document.hardware;
        assert_eq!(asio.fits(&rme_asio()), Some(3));
        assert_eq!(asio.fits(&Found { driver: "wasapi".into(), ..rme_asio() }), None, "another driver");
        assert_eq!(asio.fits(&Found { channels: 2, ..rme_asio() }), None, "a stereo endpoint of the same interface");
        let usb = Hardware { usb: Some(Usb { vendor: 1, product: 2 }), ..Hardware::default() };
        assert_eq!(usb.fits(&rme_asio()), None, "a USB id it cannot see");
        assert_eq!(usb.fits(&Found { usb: Some(Usb { vendor: 1, product: 2 }), ..rme_asio() }), Some(2));
    }

    #[test]
    fn the_catalogs_preset_comes_before_the_factorys_then_the_most_used() {
        let found = [Found { name: "Speakers (Realtek)".into(), driver: "wasapi".into(), channels: 2, ..Found::default() }, rme_asio()];
        let lib = Library::factory();
        let first = |lib: &Library| lib.recommend(&found).first().map(|o| (lib.presets[o.preset].document.id.clone(), o.device));
        assert_eq!(first(&lib), Some(("rme-ucx2.asio".into(), 1)), "the factory's most specific");
        let factory_stereo = lib.recommend(&found).iter().any(|o| lib.presets[o.preset].document.id == "stereo" && o.device == 0);
        assert!(factory_stereo, "every fitting preset is offered");
        let mut ours = lib.find("rme-ucx2.asio", None).unwrap().document.derive("studio.rme", "Studio RME", &Settings { buffer: Some(256), ..Settings::default() });
        ours.hardware.channels = None;
        let lib = lib.with_catalog(&[Listed { version: 4, source: Source::Factory, uses: 0, document: ours }]);
        assert_eq!(lib.presets.last().unwrap().source, Source::Catalog, "what the catalog lists is the catalog's");
        assert_eq!(first(&lib), Some(("studio.rme".into(), 1)), "the org's own first, even when it names less");
        let mut busy = lib.find("stereo", None).unwrap().clone();
        busy.document.id = "busy".into();
        busy.document.factory = false;
        busy.uses = 9;
        let lib = Library::default().with_catalog(&[Listed { uses: 1, ..lib.find("studio.rme", None).unwrap().clone() }, busy]);
        assert_eq!(first(&lib), Some(("busy".into(), 0)), "among the catalog's: the most used");
    }

    #[test]
    fn deck_toml_keeps_only_what_differs_from_its_preset() {
        let p = Library::factory().find("rme-ucx2.asio", None).unwrap().document.settings(None);
        let local = Settings { buffer: Some(256), outlets: vec![map("cue", &[9, 10])], ..Settings::default() };
        let playing = local.over(&p);
        assert_eq!((playing.host.as_deref(), playing.device.as_deref(), playing.buffer), (Some("asio"), Some("Fireface"), Some(256)));
        assert_eq!(playing.outlets, [map("main", &[1, 2]), map("cue", &[9, 10])], "the local entry replaces the preset's group");
        assert_eq!(playing.under(&p), local, "under gives back what over took");
        assert_eq!(p.under(&p), Settings::default(), "the preset as it is: nothing local");
    }

    #[test]
    fn the_catalogs_answer_is_read_as_it_comes() {
        let answer = r#"{"presets":[{"version":7,"document":{"id":"mine","name":"Mine","hardware":{"channels":2}}},{"version":1,"document":{"id":"bad"}}]}"#;
        let lib = Library::default().with_answer(answer);
        assert_eq!(lib.presets.iter().map(|p| (p.document.id.as_str(), p.version, p.source)).collect::<Vec<_>>(), [("mine", 7, Source::Catalog)]);
        assert_eq!(Library::default().with_answer("<html>"), Library::default());
    }

    #[test]
    fn a_version_is_found_and_a_newer_one_offered() {
        let doc = Library::factory().find("stereo", None).unwrap().document.clone();
        let lib = Library::default().with_catalog(&[Listed { version: 1, source: Source::Catalog, uses: 0, document: DevicePreset { factory: false, ..doc.clone() } }, Listed { version: 2, source: Source::Catalog, uses: 0, document: DevicePreset { factory: false, ..doc } }]);
        assert_eq!(lib.find("stereo", Some(1)).map(|p| p.version), Some(1));
        assert_eq!(lib.newer("stereo", 1).map(|p| p.version), Some(2));
        assert_eq!(lib.newer("stereo", 2), None);
        let found = [Found { name: "x".into(), channels: 2, ..Found::default() }];
        assert_eq!(lib.recommend(&found).len(), 1, "only the newest version is offered");
    }

    #[test]
    fn a_new_preset_names_the_device_it_was_saved_on() {
        let s = Settings { host: Some("asio".into()), device: Some("Fireface".into()), buffer: Some(128), outlets: vec![map("main", &[1, 2])] };
        let p = DevicePreset::from_machine("mine", "Mine", &rme_asio(), &s);
        assert!(p.problems().is_empty(), "{:?}", p.problems());
        assert_eq!((p.hardware.names.as_slice(), p.hardware.driver.as_deref(), p.hardware.channels), (["ASIO Fireface USB".to_string()].as_slice(), Some("asio"), Some(20)));
        assert_eq!(p.hardware.fits(&rme_asio()), Some(3));
        assert_eq!(p.settings(None), s);
    }
}
