//! The panel's Audio tab: this machine's audio as deck.toml holds it – the host and output device, the buffer, the rig's
//! outlet groups on the device's channels, a test tone per channel and per group (on the settings shown, saved or not).
//! Apply writes deck.toml (deck-native's `DeckAudio`) and restarts the stage's audio on it. The map's rules are the core's
//! (`deck_engine::check_outlets`, `DeckConfig::channels_of`); this file holds the draft and lays it out.
use crate::stage::Stage;
use deck_engine::{check_outlets, DeckConfig, OutletMap, Tone};
use deck_kit::base::draw::DrawList;
use deck_kit::base::tokens::Tokens;
use deck_kit::editor::controls::{self, Rect};
use deck_native::audio::backend::{self, DeckAudio, Output, Outputs};
use std::sync::mpsc::{Receiver, TryRecvError};

/// the buffers offered, in frames; 0 = the device's default
pub const BUFFERS: [u32; 6] = [0, 64, 128, 256, 512, 1024];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Field {
    Host(usize),
    Device(usize),
    Buffer(usize),
    /// a group (by its row) on a device channel (counted from 1)
    Cell(usize, usize),
    ToneChannel(usize),
    ToneGroup(usize),
    Apply,
    Rescan,
    /// the ASIO device the core offers for a split interface
    Offer,
}

/// what a press asks of the stage
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Ask {
    Nothing,
    Tone(Tone),
    /// the tone stops: the rig plays again
    Quiet,
    Apply,
}

#[derive(Default)]
pub struct Page {
    pub hosts: Vec<Outputs>,
    pub draft: DeckAudio,
    pub saved: DeckAudio,
    /// the rig's groups and those the map names, `main` first
    pub groups: Vec<String>,
    pub tone: Option<Tone>,
    /// the last answer, and whether it is bad
    pub said: Option<(String, bool)>,
    listed: bool,
    /// the listing on its thread (it loads the hosts' drivers: 1.4 s on Windows)
    pending: Option<Receiver<Vec<Outputs>>>,
    /// None = `backend::outputs`
    lister: Option<fn() -> Vec<Outputs>>,
}

impl Page {
    pub fn new(saved: DeckAudio) -> Page {
        Page { draft: saved.clone(), saved, ..Page::default() }
    }

    /// the stage's groups, and the devices: listed once on a thread of their own, taken when they arrive
    pub fn sync(&mut self, rig: &[String]) {
        if !self.listed {
            self.listed = true;
            let (tx, rx) = std::sync::mpsc::channel();
            let list = self.lister.unwrap_or(backend::outputs);
            match std::thread::Builder::new().name("audio-list".into()).spawn(move || {
                let _ = tx.send(list());
            }) {
                Ok(_) => self.pending = Some(rx),
                Err(e) => self.said = Some((format!("no thread to list the devices on: {e}"), true)),
            }
        }
        if let Some(rx) = &self.pending {
            match rx.try_recv() {
                Ok(hosts) => {
                    self.hosts = hosts;
                    self.pending = None;
                }
                Err(TryRecvError::Disconnected) => {
                    self.pending = None;
                    self.said = Some(("listing the devices failed".into(), true));
                }
                Err(TryRecvError::Empty) => {}
            }
        }
        let mut groups: Vec<String> = rig.to_vec();
        for o in &self.draft.audio_outlet {
            if !groups.contains(&o.group) {
                groups.push(o.group.clone());
            }
        }
        self.groups = groups;
    }

    pub fn listing(&self) -> bool {
        self.pending.is_some()
    }

    pub fn host(&self) -> Option<usize> {
        let want = self.draft.audio_host.as_deref().map(str::to_lowercase).unwrap_or_else(|| "default".into());
        self.hosts.iter().position(|h| h.key == Some(want.as_str())).or_else(|| self.hosts.iter().position(|h| h.key == Some("default")))
    }

    pub fn devices(&self) -> &[Output] {
        self.host().map_or(&[], |h| &self.hosts[h].devices)
    }

    /// the device the stage opens: the first whose name holds deck.toml's, else the host's default
    pub fn device(&self) -> Option<usize> {
        let devices = self.devices();
        match &self.draft.audio_device {
            Some(want) => devices.iter().position(|d| d.matches(want)),
            None => devices.iter().position(|d| d.default),
        }
    }

    /// the chosen device's channels (2 when it is not known)
    pub fn channels(&self) -> usize {
        self.device().map(|i| self.devices()[i].channels).filter(|&c| c > 0).unwrap_or(2)
    }

    /// where a group plays: its map entry, else the core's default (`main` on the first channels); true = from the map
    pub fn shown(&self, group: &str) -> (Vec<usize>, bool) {
        match self.draft.audio_outlet.iter().find(|o| o.group == group) {
            Some(o) => (o.channels.clone(), true),
            None => (DeckConfig::device(self.channels()).channels_of(group, 2).into_iter().flatten().map(|c| c + 1).collect(), false),
        }
    }

    pub fn offer(&self) -> Option<backend::Offer> {
        backend::asio_offer(&self.hosts, self.host()?, self.device()?, &self.draft.audio_outlet)
    }

    pub fn problems(&self) -> Vec<String> {
        check_outlets(&self.draft.audio_outlet, self.channels()).iter().map(ToString::to_string).collect()
    }

    pub fn dirty(&self) -> bool {
        self.draft != self.saved
    }

    pub fn press(&mut self, f: Field) -> Ask {
        match f {
            Field::Host(i) => {
                let Some(h) = self.hosts.get(i) else { return Ask::Nothing };
                self.draft.audio_host = h.key.filter(|k| *k != "default").map(str::to_string);
                self.draft.audio_device = None;
            }
            Field::Device(i) => {
                let Some(d) = self.devices().get(i) else { return Ask::Nothing };
                self.draft.audio_device = Some(d.name.clone());
            }
            Field::Buffer(i) => self.draft.audio_buffer = BUFFERS.get(i).copied().filter(|&b| b > 0),
            Field::Cell(g, c) => {
                let Some(group) = self.groups.get(g).cloned() else { return Ask::Nothing };
                let (mut list, _) = self.shown(&group);
                match list.iter().position(|&x| x == c) {
                    Some(at) => {
                        list.remove(at);
                    }
                    None => {
                        list.push(c);
                        list.sort_unstable();
                    }
                }
                self.draft.audio_outlet.retain(|o| o.group != group);
                if !list.is_empty() || group == "main" {
                    self.draft.audio_outlet.push(OutletMap { group, channels: list });
                }
            }
            Field::ToneChannel(c) => return self.toggle_tone(Tone::Channel(c)),
            Field::ToneGroup(g) => match self.groups.get(g) {
                Some(group) => return self.toggle_tone(Tone::Group(group.clone())),
                None => return Ask::Nothing,
            },
            Field::Apply => {
                let problems = self.problems();
                if let Some(p) = problems.first() {
                    self.said = Some((p.clone(), true));
                    return Ask::Nothing;
                }
                return Ask::Apply;
            }
            Field::Rescan => self.listed = false,
            Field::Offer => {
                let Some(o) = self.offer() else { return Ask::Nothing };
                self.draft.audio_host = self.hosts[o.host].key.filter(|k| *k != "default").map(str::to_string);
                self.draft.audio_device = Some(self.hosts[o.host].devices[o.device].name.clone());
            }
        }
        Ask::Nothing
    }

    fn toggle_tone(&mut self, tone: Tone) -> Ask {
        if self.tone.as_ref() == Some(&tone) {
            self.tone = None;
            return Ask::Quiet;
        }
        self.tone = Some(tone.clone());
        Ask::Tone(tone)
    }

    pub fn help(&self, f: Field) -> (String, &'static str) {
        match f {
            Field::Host(_) => ("Host".into(), "The driver layer the stage opens: PipeWire on the deck, ASIO for the RME on Windows, WASAPI otherwise. Apply restarts the sound on it."),
            Field::Device(_) => ("Device".into(), "The output device; its channels are the columns below."),
            Field::Buffer(_) => ("Buffer".into(), "Frames per device period. ASIO takes the size set in the driver's own dialog."),
            Field::Cell(g, c) => (format!("{} on {c}", self.groups.get(g).map_or("", String::as_str)), "Click to put the group on this channel or take it off."),
            Field::ToneChannel(c) => (format!("Channel {c}"), "A test tone on this channel; click again to stop it."),
            Field::ToneGroup(_) => ("Group".into(), "A test tone where this group plays; click again to stop it."),
            Field::Apply => ("Apply".into(), "Writes deck.toml and restarts the stage's sound on it."),
            Field::Rescan => ("Rescan".into(), "Lists the hosts and devices again."),
            Field::Offer => ("ASIO".into(), "WASAPI shows this interface as stereo pairs; its ASIO device has every channel. Click to take it, Apply restarts the sound on it."),
        }
    }
}

/// a press on the tab: the draft changes, a tone starts or stops, Apply saves and restarts the stage's audio
pub fn act(page: &mut Page, stage: &mut Stage, f: Field) {
    let done = match page.press(f) {
        Ask::Nothing => return,
        Ask::Tone(tone) => stage.tone(&backend::Options::from_deck(&page.draft), &page.draft.audio_outlet, &tone).map(|_| format!("test tone on {}", stage.audio)),
        Ask::Quiet => stage.restart_audio().map(|_| "the rig plays again".to_string()),
        Ask::Apply => page.draft.save().and_then(|path| {
            page.saved = page.draft.clone();
            page.tone = None;
            stage.restart_audio()?;
            Ok(format!("written to {}, {}", path.display(), stage.audio))
        }),
    };
    page.said = Some(match done {
        Ok(s) => (s, false),
        Err(e) => {
            page.tone = None;
            let _ = stage.restart_audio();
            (format!("{e:#}"), true)
        }
    });
}

pub fn draw(d: &mut DrawList, t: &Tokens, area: Rect, page: &mut Page, stage: &Stage, pointer: (f32, f32)) -> Vec<(Rect, Field)> {
    page.sync(&stage.groups);
    let (font, small, row) = (t.size.font, t.size.small, t.size.tree_row);
    let mut hits = Vec::new();
    let mut y = area.y;
    let text_y = |y: f32, px: f32| y + (row - px * 1.25) * 0.5;
    let fit = |d: &DrawList, s: &str, px: f32, w: f32| controls::fit(s, (w / d.advance(px)).max(0.0) as usize);

    y += controls::section(d, t, area.x, y, area.w, "HOST", true);
    let names: Vec<&str> = page.hosts.iter().map(|h| h.name.as_str()).collect();
    for (i, r) in controls::pills(d, t, area.x, y, row - 4.0, &names, page.host().unwrap_or(usize::MAX)).into_iter().enumerate() {
        hits.push((r, Field::Host(i)));
    }
    y += row + 4.0;
    if page.listing() {
        d.text(area.x, text_y(y, small), small, "listing…", t.color.muted);
        y += row;
    }
    let chosen = page.device();
    for (i, dev) in page.devices().iter().enumerate() {
        let r = Rect { x: area.x, y, w: area.w, h: row };
        if chosen == Some(i) {
            d.rect(r.x, r.y, r.w, r.h, t.color.selection);
        } else if r.has(pointer.0, pointer.1) {
            d.rect(r.x, r.y, r.w, r.h, t.color.control);
        }
        let what = match &dev.error {
            Some(e) => format!("{} – {e}", dev.label()),
            None => format!("{} · {} ch · {} Hz", dev.label(), dev.channels, dev.rate),
        };
        d.text(r.x + 4.0, text_y(y, font), font, &fit(d, &what, font, r.w - 8.0), if dev.error.is_some() { t.color.muted } else { t.color.text });
        hits.push((r, Field::Device(i)));
        y += row;
    }

    y += controls::section(d, t, area.x, y + 4.0, area.w, "BUFFER", false) + 4.0;
    let labels: Vec<String> = BUFFERS.iter().map(|&b| if b == 0 { "default".into() } else { b.to_string() }).collect();
    let labels: Vec<&str> = labels.iter().map(String::as_str).collect();
    let on = BUFFERS.iter().position(|&b| b == page.draft.audio_buffer.unwrap_or(0)).unwrap_or(usize::MAX);
    for (i, r) in controls::pills(d, t, area.x, y, row - 4.0, &labels, on).into_iter().enumerate() {
        hits.push((r, Field::Buffer(i)));
    }
    y += row;
    if let Some(dev) = chosen.map(|i| &page.devices()[i]).filter(|o| o.error.is_none()) {
        d.text(area.x, text_y(y, small), small, &fit(d, &backend::buffer_line(dev.buffer), small, area.w), t.color.muted);
        y += row;
    }

    // the map: a line of channel numbers (each a tone), then one line of cells per group; wrapped at the panel's width
    y += controls::section(d, t, area.x, y + 4.0, area.w, "GROUPS ON CHANNELS", false) + 4.0;
    let channels = page.channels();
    let label_w = d.advance(font) * 8.0;
    let cell = (row - 4.0).max(d.advance(small) * 3.0);
    let per_line = (((area.w - label_w) / (cell + 2.0)).floor() as usize).max(1);
    let shown: Vec<(Vec<usize>, bool)> = page.groups.iter().map(|g| page.shown(g)).collect();
    for first in (1..=channels).step_by(per_line) {
        let last = (first + per_line - 1).min(channels);
        for c in first..=last {
            let r = Rect { x: area.x + label_w + (c - first) as f32 * (cell + 2.0), y, w: cell, h: row - 4.0 };
            let on = page.tone == Some(Tone::Channel(c));
            hits.push((controls::toggle(d, t, r, &c.to_string(), on), Field::ToneChannel(c)));
        }
        y += row;
        for (g, name) in page.groups.iter().enumerate() {
            let on = page.tone == Some(Tone::Group(name.clone()));
            let lr = Rect { x: area.x, y: y + 2.0, w: label_w - 4.0, h: row - 4.0 };
            hits.push((controls::toggle(d, t, lr, &fit(d, name, small, lr.w - 4.0), on), Field::ToneGroup(g)));
            let (list, mapped) = &shown[g];
            for c in first..=last {
                let r = Rect { x: area.x + label_w + (c - first) as f32 * (cell + 2.0), y: y + 2.0, w: cell, h: row - 4.0 };
                let color = match (list.contains(&c), mapped) {
                    (true, true) => t.color.accent,
                    (true, false) => t.color.muted,
                    _ => t.color.control,
                };
                d.round_rect(r.x, r.y, r.w, r.h, t.size.radius, color);
                hits.push((r, Field::Cell(g, c)));
            }
            y += row;
        }
        y += 4.0;
    }
    d.text(area.x, text_y(y, small), small, &fit(d, "grey: no entry – main on 1/2, others silent", small, area.w), t.color.muted);
    y += row;

    for p in page.problems() {
        d.text(area.x, text_y(y, small), small, &fit(d, &p, small, area.w), t.color.danger);
        y += row;
    }
    if let Some(o) = page.offer() {
        let r = Rect { x: area.x, y, w: area.w, h: row };
        if r.has(pointer.0, pointer.1) {
            d.rect(r.x, r.y, r.w, r.h, t.color.control);
        }
        d.text(area.x, text_y(y, small), small, &fit(d, &o.text, small, area.w), t.color.accent);
        hits.push((r, Field::Offer));
        y += row;
    }
    if let Some((s, bad)) = &page.said {
        d.text(area.x, text_y(y, small), small, &fit(d, s, small, area.w), if *bad { t.color.danger } else { t.color.muted });
        y += row;
    }
    y += 4.0;
    let bw = d.advance(small) * 9.0;
    hits.push((controls::toggle(d, t, Rect { x: area.x, y, w: bw, h: row - 4.0 }, "Rescan", false), Field::Rescan));
    hits.push((controls::toggle(d, t, Rect { x: area.x + area.w - bw, y, w: bw, h: row - 4.0 }, "Apply", page.dirty()), Field::Apply));
    y += row;
    d.text(area.x, text_y(y, small), small, &fit(d, &DeckAudio::path().display().to_string(), small, area.w), t.color.muted);
    hits
}

#[cfg(test)]
mod tests {
    use super::*;

    fn out(name: &str, channels: usize, default: bool) -> Output {
        Output { name: name.into(), channels, rate: 48_000, format: "I32".into(), buffer: Some((128, 128)), default, ..Output::default() }
    }

    #[test]
    fn the_listing_runs_off_the_frame_thread() {
        fn slow() -> Vec<Outputs> {
            std::thread::sleep(std::time::Duration::from_millis(300));
            vec![Outputs { name: "WASAPI".into(), key: Some("default"), devices: vec![out("Speakers", 2, true)] }]
        }
        let mut p = Page { lister: Some(slow), ..Page::new(DeckAudio::default()) };
        let t = std::time::Instant::now();
        p.sync(&["main".into()]);
        assert!(t.elapsed() < std::time::Duration::from_millis(100), "sync waited {:?}", t.elapsed());
        assert!(p.listing() && p.hosts.is_empty());
        let end = std::time::Instant::now() + std::time::Duration::from_secs(5);
        while p.listing() && std::time::Instant::now() < end {
            std::thread::sleep(std::time::Duration::from_millis(10));
            p.sync(&["main".into()]);
        }
        assert!(!p.listing());
        assert_eq!((p.host(), p.device(), p.channels()), (Some(0), Some(0), 2));
    }

    #[test]
    fn a_pipewire_sink_is_picked_by_its_node_name_and_sizes_the_map() {
        let sinks = [
            backend::SinkProps { name: "alsa_output.pci.analog-stereo".into(), description: "Built-in Audio".into(), position: Some("FL,FR".into()), channels: Some(2) },
            backend::SinkProps {
                name: "alsa_output.usb-RME_Fireface_UCX_II-00.pro-output-0".into(),
                description: "Fireface UCX II Pro".into(),
                position: Some((0..20).map(|i| format!("AUX{i}")).collect::<Vec<_>>().join(",")),
                channels: Some(20),
            },
        ];
        let saved = DeckAudio { audio_device: Some("Fireface".into()), ..DeckAudio::default() };
        let mut p = Page::new(saved);
        p.listed = true;
        p.hosts = vec![
            Outputs { name: "PipeWire".into(), key: Some("default"), devices: backend::sink_outputs(&sinks, Some("alsa_output.pci.analog-stereo")) },
            Outputs { name: "ALSA".into(), key: None, devices: vec![out("default", 2, true)] },
        ];
        p.sync(&["main".into()]);
        assert_eq!((p.host(), p.device(), p.channels()), (Some(0), Some(1), 20));
        p.press(Field::Device(0));
        assert_eq!((p.draft.audio_device.as_deref(), p.channels()), (Some("alsa_output.pci.analog-stereo"), 2));
        p.press(Field::Device(1));
        assert_eq!(p.draft.audio_device.as_deref(), Some("alsa_output.usb-RME_Fireface_UCX_II-00.pro-output-0"));
        assert_eq!(p.draft.audio_host, None, "the stage plays PipeWire without a host key");
    }

    fn page() -> Page {
        let saved = DeckAudio { audio_host: Some("asio".into()), audio_device: Some("Fireface".into()), audio_buffer: Some(128), ..DeckAudio::default() };
        let mut p = Page::new(saved);
        p.listed = true;
        p.hosts = vec![
            Outputs { name: "WASAPI".into(), key: Some("default"), devices: vec![out("Speakers (Realtek)", 2, true), out("Analog (1+2) (RME Fireface UCX II)", 2, false)] },
            Outputs { name: "ASIO".into(), key: Some("asio"), devices: vec![out("ASIO Fireface USB", 20, true)] },
        ];
        p.sync(&["main".into(), "cue".into()]);
        p
    }

    #[test]
    fn the_page_shows_the_machines_device_and_where_the_groups_play() {
        let p = page();
        assert_eq!((p.host(), p.device(), p.channels()), (Some(1), Some(0), 20));
        assert_eq!(p.groups, ["main", "cue"]);
        assert_eq!(p.shown("main"), (vec![1, 2], false), "main without an entry: the core's default");
        assert_eq!(p.shown("cue"), (vec![], false), "any other group without one is silent");
        assert!(!p.dirty() && p.problems().is_empty());
    }

    #[test]
    fn cells_map_a_group_and_apply_waits_for_a_valid_map() {
        let mut p = page();
        assert_eq!(p.press(Field::Cell(1, 3)), Ask::Nothing);
        p.press(Field::Cell(1, 4));
        assert_eq!(p.shown("cue"), (vec![3, 4], true));
        p.press(Field::Cell(0, 2));
        assert_eq!(p.draft.audio_outlet.iter().find(|o| o.group == "main").unwrap().channels, [1], "main starts from its default");
        assert!(p.dirty());
        assert_eq!(p.press(Field::Apply), Ask::Apply);
        // a smaller device: the map no longer fits, Apply says why
        p.press(Field::Host(0));
        assert_eq!((p.draft.audio_host.as_deref(), p.draft.audio_device.as_deref(), p.channels()), (None, None, 2));
        assert_eq!(p.press(Field::Apply), Ask::Nothing);
        assert!(p.said.as_ref().is_some_and(|(s, bad)| *bad && s.contains("channel 3")), "{:?}", p.said);
        // emptied, a group's entry goes: silent again
        p.press(Field::Cell(1, 3));
        p.press(Field::Cell(1, 4));
        assert!(p.draft.audio_outlet.iter().all(|o| o.group != "cue"));
        assert_eq!(p.press(Field::Apply), Ask::Apply);
    }

    #[test]
    fn devices_and_buffers_are_picked_and_tones_toggle() {
        let mut p = page();
        p.press(Field::Host(0));
        p.press(Field::Device(1));
        assert_eq!(p.draft.audio_device.as_deref(), Some("Analog (1+2) (RME Fireface UCX II)"));
        assert_eq!(p.device(), Some(1));
        p.press(Field::Buffer(0));
        assert_eq!(p.draft.audio_buffer, None, "default");
        p.press(Field::Buffer(3));
        assert_eq!(p.draft.audio_buffer, Some(256));
        assert_eq!(p.press(Field::ToneChannel(2)), Ask::Tone(Tone::Channel(2)));
        assert_eq!(p.press(Field::ToneGroup(1)), Ask::Tone(Tone::Group("cue".into())), "another tone takes over");
        assert_eq!(p.press(Field::ToneGroup(1)), Ask::Quiet, "the same again stops it");
        assert_eq!(p.tone, None);
    }

    #[test]
    fn a_split_wasapi_endpoint_offers_the_asio_device() {
        let mut p = page();
        p.press(Field::Cell(1, 3));
        p.press(Field::Cell(1, 4));
        assert_eq!(p.offer(), None, "on ASIO the map fits");
        p.press(Field::Host(0));
        p.press(Field::Device(1));
        assert!(!p.problems().is_empty());
        assert!(p.offer().is_some_and(|o| o.text.contains("ASIO Fireface USB")), "{:?}", p.offer());
        p.press(Field::Offer);
        assert_eq!((p.draft.audio_host.as_deref(), p.draft.audio_device.as_deref(), p.channels()), (Some("asio"), Some("ASIO Fireface USB"), 20));
        assert!(p.problems().is_empty() && p.offer().is_none());
        p.press(Field::Host(0));
        p.press(Field::Device(0));
        assert_eq!(p.offer(), None, "the Realtek is another interface");
    }

    #[test]
    fn a_group_only_the_map_names_is_shown_too() {
        let mut p = page();
        p.draft.audio_outlet.push(OutletMap { group: "stem".into(), channels: vec![5, 6] });
        p.sync(&["main".into()]);
        assert_eq!(p.groups, ["main", "stem"]);
    }
}
