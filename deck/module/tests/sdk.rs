//! The SDK's contract (module ABI v2: #340, #343, research/ABI-V2.md): what the derive declares, that the SDK and the
//! host compute the same layout, the layout hash, the parameter table, `Ctx`, planar audio, events, sensors, assets, control
//! outputs with paths, and – with the counting allocator installed as this binary's global allocator – that nothing on
//! the SDK's own call path allocates.
// clippy sees the repeated `#[port(…)]` of one struct as duplicates (a module crate allows it, README.md)
#![allow(clippy::duplicated_attributes)]
use deck_catalog::{AssetKind as CatalogAsset, Dir, Domain as CatalogDomain, Mode, PortType};
use deck_core::tempo::TempoMap;
use deck_module::cv::GateCv;
use deck_module::alloc::Counting;
use deck_module::hdr::{get_str, put_str};
use deck_module::host::{self, parse, read_descriptor};
use deck_module::layout::QUANTUM;
use deck_module::testing::{take_log, Rig};
use deck_module::{log, math};
use deck_module::{AssetKind, Block, Ctx, Describe, Dirty, Ev, Kind, Layout, Module, Outputs, Sensor, Shape, Smooth, Status};

#[global_allocator]
static ALLOC: Counting = Counting;

/// a gain over any number of channels, a control output (the block's peak), a beat event on every beat, a variable
#[derive(Module)]
#[module(name = "gain", doc = "a gain")]
#[reads("story.act")]
#[port(input, audio, name = "in", channels = "any")]
#[port(output, audio, name = "out", channels = "any")]
#[port(input, control, name = "mod")]
#[port(output, control, name = "peak")]
#[port(output, event, name = "beats", names = "music.beat", props = "n", capacity = 4)]
struct Gain {
    #[param(range = 0.0..2.0, default = 1.0, unit = "x", doc = "the gain")]
    amount: f32,
    #[param(range = -1.0..1.0, default = -0.25)]
    offset: f32,
    last_beat: i64,
    changed: u32,
}

impl Module for Gain {
    fn params_changed(&mut self, dirty: Dirty) {
        self.changed += dirty.iter().count() as u32;
    }

    fn process(&mut self, block: &mut Block, ctx: &Ctx) {
        let m = block.control_in(Self::MOD);
        let inp = block.audio_in(Self::IN);
        let mut out = block.audio_out(Self::OUT);
        let mut peak = 0.0f32;
        for c in 0..out.ch() {
            for (o, i) in out.chan(c).iter_mut().zip(inp.chan(c)) {
                *o = i * self.amount + m;
                peak = peak.max(o.abs());
            }
        }
        block.set(Self::PEAK, peak);
        let (_, to) = ctx.beats();
        let beat = to.floor() as i64;
        if beat > self.last_beat {
            self.last_beat = beat;
            block.emit(Self::BEATS, Ev::event(0, 0).with(0, beat as f32));
        }
        if ctx.character().mood == "loud" {
            log!("loud at beat {:.2}, act {:?}", to, ctx.var(Self::STORY_ACT));
        }
    }
}

/// a synth's skeleton: note events in, a choice parameter, a log curve, the factory preset
#[derive(Module)]
#[module(name = "synth", latency = 32)]
#[port(input, event, name = "notes", names = "music.note", props = "pitch|vel|glide|key", capacity = 8)]
#[port(input, control, name = "sustain")]
#[port(output, audio, name = "out", channels = 2)]
#[preset(name = "bright", doc = "open and sawtooth", values(cutoff = 6000.0, wave = 0))]
struct Synth {
    #[param(choices = "saw|pulse|triangle|sine", default = 3, doc = "waveform")]
    wave: f32,
    #[param(range = 20.0..20000.0, default = 800.0, curve = log, unit = "Hz")]
    cutoff: f32,
    held: [u32; 8],
    voices: usize,
    seen: [(u32, u8); 8],
    seen_n: usize,
}

impl Module for Synth {
    fn process(&mut self, block: &mut Block, _: &Ctx) {
        self.seen_n = 0;
        for ev in block.events_in(Self::NOTES) {
            if self.seen_n < 8 {
                self.seen[self.seen_n] = (ev.frame, ev.kind);
                self.seen_n += 1;
            }
            match ev.kind() {
                Some(Kind::NoteOn) if self.voices < 8 => {
                    self.held[self.voices] = ev.id;
                    self.voices += 1;
                }
                Some(Kind::NoteOff) => {
                    if let Some(i) = self.held[..self.voices].iter().position(|&id| id == ev.id) {
                        self.held.copy_within(i + 1..self.voices, i);
                        self.voices -= 1;
                    }
                }
                Some(Kind::Reset) => self.voices = 0,
                _ => {}
            }
        }
        let mut out = block.audio_out(Self::OUT);
        let (l, r) = out.stereo();
        l.fill(self.voices as f32);
        r.fill(self.cutoff);
    }
}

/// a code rig's policy (§5): control domain, path outputs, a variable, the character's senses
#[derive(Module)]
#[module(name = "policy", domain = "control", moods = "calm|bright|angry")]
#[reads("story.act", "being.glow")]
#[port(input, control, name = "meter")]
#[port(output, control, name = "lead_preset", path = "lead:@preset", choices = "calm|bright|angry")]
#[port(output, control, name = "cutoff", path = "lead:analog.f1_cutoff", mode = "set")]
#[port(output, control, name = "pad_gain", path = "pad:@gain", mode = "mul")]
struct Policy {
    mood: Option<u32>,
}

impl Module for Policy {
    fn control(&mut self, ctx: &Ctx, out: &mut Outputs) {
        let c = ctx.character();
        if self.mood != Some(c.mood_index) {
            self.mood = Some(c.mood_index);
            out.set(Self::LEAD_PRESET, c.mood_index as f32);
        }
        out.set(Self::CUTOFF, 400.0 * math::exp2(c.energy * 3.91) + ctx.control_in(Self::METER));
        if let Some(act) = ctx.var(Self::STORY_ACT) {
            out.set(Self::PAD_GAIN, if act >= 3.0 { 0.4 } else { 0.8 });
        }
    }
}

/// sensors (§4) and assets (§1.5)
#[derive(Module)]
#[module(name = "tilt", needs = "camera.motion", wants = "motion|position")]
#[asset(name = "kick", kind = "pcm", max_bytes = 4096, doc = "a kick")]
#[asset(name = "seq", kind = "bytes", max_bytes = 64)]
#[port(output, audio, name = "out", channels = 1)]
#[port(output, control, name = "tilt")]
struct Tilt {
    lp: Smooth,
}

impl Module for Tilt {
    fn process(&mut self, block: &mut Block, ctx: &Ctx) {
        let m = ctx.motion();
        let tilt = if m.status().live() { (m.accel[0] / 9.81).clamp(-1.0, 1.0) * 0.5 + 0.5 } else { 0.5 };
        self.lp.to(tilt);
        block.set(Self::TILT, self.lp.step(0.5));
        let kick = ctx.asset(Self::KICK).map(|a| a.pcm(1)).unwrap_or(&[]);
        let mut out = block.audio_out(Self::OUT);
        let (o, _) = out.stereo();
        for (o, k) in o.iter_mut().zip(kick) {
            *o = *k;
        }
    }
}

/// allocates in process – the counting allocator's subject
#[derive(Module)]
#[module(name = "leaky")]
#[port(output, audio, name = "out", channels = 1)]
struct Leaky {
    kept: Vec<Vec<f32>>,
}

impl Module for Leaky {
    fn process(&mut self, _: &mut Block, _: &Ctx) {
        self.kept.push(vec![0.0; 16]);
    }
}

fn layouts<M: Describe>() -> (Shape, Layout) {
    let d = parse(M::DESCRIPTOR).unwrap_or_else(|e| panic!("{}: {e}\n{}", M::NAME, M::DESCRIPTOR));
    let shape = Shape::of(&d).unwrap();
    assert_eq!(host::validate(&d).unwrap(), shape);
    (shape, Layout::of(&shape))
}

/// §7: "a layout-hash test and a host/SDK layout equality test over all fixture descriptors"
#[test]
fn the_host_computes_the_layout_the_derive_compiled() {
    fn same<M: Describe>() {
        let (shape, layout) = layouts::<M>();
        assert_eq!(shape, M::SHAPE, "{}", M::NAME);
        assert_eq!(layout, M::LAYOUT, "{}", M::NAME);
    }
    same::<Gain>();
    same::<Synth>();
    same::<Policy>();
    same::<Tilt>();
    same::<Leaky>();
}

/// the hash a module exports as `dm_layout()`, frozen for these descriptors: a change here means every module built
/// before is refused by the host – only together with a new ABI version (§7). `Policy` lists `moods` (A1, #409): a list
/// that places no byte, so its hash is the one it had before
#[test]
fn the_layout_hash_of_the_fixture_modules_is_frozen() {
    let got = [Gain::LAYOUT, Synth::LAYOUT, Policy::LAYOUT, Tilt::LAYOUT].map(|l| (l.size, l.hash));
    assert_eq!(got, [(0x8240, 0x313c_3681), (0x6b0, 0x7593_d819), (0x180, 0xf6bb_6dfc), (0x400, 0x7294_dbae)], "{got:x?}");
    let hashes: Vec<u32> = got.iter().map(|g| g.1).collect();
    assert!(hashes.iter().enumerate().all(|(i, h)| !hashes[..i].contains(h)), "four shapes, four hashes");
}

#[test]
fn the_derive_writes_the_descriptor() {
    let d = parse(Gain::DESCRIPTOR).unwrap();
    assert_eq!((d.abi, d.name.as_ref(), d.version.as_ref(), d.doc.as_ref(), d.domain), (Some(2), "gain", env!("CARGO_PKG_VERSION"), "a gain", CatalogDomain::Audio));
    assert_eq!((d.params[0].unit.as_ref(), d.params[0].doc.as_ref(), d.params[0].port), ("x", "the gain", true));
    assert_eq!(d.reads.iter().map(|s| s.as_ref()).collect::<Vec<_>>(), ["story.act"]);
    let beats = d.ports_of(Dir::Out, PortType::Event).next().unwrap();
    assert_eq!((beats.names[0].as_ref(), beats.props[0].as_ref(), beats.capacity), ("music.beat", "n", 4));
    assert_eq!(d.ports_of(Dir::In, PortType::Audio).next().unwrap().channels, Some(deck_catalog::Channels::Any(deck_catalog::AnyChannels::Any)));
    assert_eq!((Gain::IN, Gain::OUT, Gain::MOD, Gain::PEAK, Gain::BEATS, Gain::AMOUNT, Gain::OFFSET), (0, 0, 0, 0, 0, 0, 1));

    // A1 (#409): the moods a module knows, in its order; none listed = none in the JSON
    let p = parse(Policy::DESCRIPTOR).unwrap();
    assert_eq!(p.moods.iter().map(|m| m.as_ref()).collect::<Vec<_>>(), ["calm", "bright", "angry"]);
    assert!(!Gain::DESCRIPTOR.contains("moods"));

    let s = parse(Synth::DESCRIPTOR).unwrap();
    assert_eq!((s.latency, s.params[0].choices.len(), s.params[0].max, s.params[0].default), (32, 4, 3.0, Some(3.0)));
    assert_eq!(s.params[1].curve, deck_catalog::Curve::Log);
    let preset = s.preset.as_ref().unwrap();
    assert_eq!((preset.name.as_str(), preset.values["cutoff"], preset.values["wave"]), ("bright", 6000.0, 0.0));
    assert_eq!(Synth::PRESET, &[(1, 6000.0), (0, 0.0)]);

    let p = parse(Policy::DESCRIPTOR).unwrap();
    assert_eq!(p.domain, CatalogDomain::Control);
    let lead = p.ports.iter().find(|x| x.name == "lead_preset").unwrap();
    assert_eq!((lead.path.as_ref(), lead.choices.len(), p.ports.iter().find(|x| x.name == "pad_gain").unwrap().mode), ("lead:@preset", 3, Mode::Mul));
    assert_eq!((Policy::STORY_ACT.0, Policy::BEING_GLOW.0, Policy::PAD_GAIN), (0, 1, 2));

    let t = parse(Tilt::DESCRIPTOR).unwrap();
    assert_eq!((t.needs.len(), t.wants.len(), t.assets[0].kind, t.assets[1].max_bytes), (1, 2, CatalogAsset::Pcm, 64));
    assert_eq!(Tilt::SHAPE.sensors[..3], [Some(Sensor::CameraMotion), Some(Sensor::Motion), Some(Sensor::Position)]);
    // a module built against another ABI is refused with its reason (no compatibility: recompile)
    assert!(host::parse(&Gain::DESCRIPTOR.replace("\"abi\":2", "\"abi\":1")).unwrap_err().contains("ABI 1"));
}

#[test]
fn descriptors_are_validated_against_the_layouts_limits() {
    let base = r#"{"abi":2,"name":"m","version":"1","params":[],"ports":[PORTS]NEEDS}"#;
    let with = |ports: &str, needs: &str| base.replace("PORTS", ports).replace("NEEDS", needs);
    assert!(parse(&with("", "")).is_ok());
    assert!(parse(&with("", r#","needs":["camera"]"#)).unwrap_err().contains("does not know"));
    assert!(parse(&with(r#"{"name":"o","dir":"out","type":"audio","channels":33}"#, "")).unwrap_err().contains("channels"));
    let nine: Vec<String> = (0..9).map(|i| format!(r#"{{"name":"e{i}","dir":"in","type":"event"}}"#)).collect();
    assert!(parse(&with(&nine.join(","), "")).unwrap_err().contains("event ports"));
    let control = r#"{"abi":2,"name":"m","version":"1","domain":"control","ports":[{"name":"o","dir":"out","type":"audio"}]}"#;
    assert!(parse(control).unwrap_err().contains("control-domain"));
    assert!(parse(&with("", "").replace("\"abi\":2", "\"abi\":1")).unwrap_err().contains("ABI 1"));
    assert!(parse(&with(r#"{"name":"o","dir":"out","type":"control","path":"a","mode":"add"}"#, "")).is_ok());
    // names and ranges
    let param = |p: &str| base.replace("\"params\":[]", &format!("\"params\":[{p}]")).replace("PORTS", "").replace("NEEDS", "");
    assert!(parse(&param(r#"{"name":"a","min":0,"max":1,"default":2}"#)).unwrap_err().contains("default"));
    assert!(parse(&param(r#"{"name":"A","min":0,"max":1,"default":0}"#)).unwrap_err().contains("identifier"));
    assert!(parse(&param(r#"{"name":"a","min":0,"max":1,"default":0},{"name":"a","min":0,"max":1,"default":0}"#)).unwrap_err().contains("twice"));
    assert!(parse("not json").is_err());
    assert!(parse(r#"{"name":"m","version":"1"}"#).unwrap_err().contains("no ABI"));
}

/// #339: every field of the one descriptor (deck-catalog) filled, through `validate`, JSON and back
#[test]
fn a_descriptor_with_every_field_round_trips_through_validate() {
    let json = r#"{"abi":2,"name":"moody","version":"2.0.0","latency":32,
        "needs":["motion"],"wants":["position"],"reads":["story.act","story.tension"],
        "params":[{"name":"cutoff","min":20,"max":20000,"default":800},{"name":"drive","min":0,"max":1,"default":0}],
        "ports":[{"name":"in","dir":"in","type":"audio"},
                 {"name":"side","dir":"in","type":"audio","channels":2,"sidechain":true},
                 {"name":"notes","dir":"in","type":"event","names":["music.note"],"props":["pitch","vel","glide","key"],"capacity":128},
                 {"name":"out","dir":"out","type":"audio","channels":"any"},
                 {"name":"cues","dir":"out","type":"event","names":["story.cue","music.note"],"props":["strength"]},
                 {"name":"level","dir":"out","type":"control"},
                 {"name":"mood","dir":"out","type":"control","path":"lead.preset","mode":"set","choices":["calm","wild"]},
                 {"name":"push","dir":"out","type":"control","path":"lead.gain","mode":"mul"}],
        "assets":[{"name":"kick","kind":"pcm","doc":"the kick drum","max_bytes":8388608},{"name":"seq","kind":"bytes","max_bytes":65536},
                  {"name":"face","kind":"image","max_bytes":262144}],
        "preset":{"name":"warm","doc":"soft and round","values":{"cutoff":1200,"drive":0.25}}}"#;
    let d = parse(json).unwrap();
    let again = parse(&serde_json::to_string(&d).unwrap()).unwrap();
    assert_eq!(again, d);
    host::validate(&again).unwrap();
    assert_eq!((d.latency, d.needs.len(), d.wants.len(), d.reads[1].as_ref()), (32, 1, 1, "story.tension"));
    let port = |n: &str| d.ports.iter().find(|p| p.name == n).unwrap();
    assert!(port("side").sidechain);
    assert_eq!((port("notes").capacity, port("notes").props.len(), port("cues").capacity), (128, 4, deck_catalog::EVENT_CAPACITY));
    assert_eq!((port("mood").path.as_ref(), port("mood").mode, port("mood").choices.len(), port("push").mode), ("lead.preset", Mode::Set, 2, Mode::Mul));
    assert_eq!(d.assets.iter().map(|a| a.kind).collect::<Vec<_>>(), [CatalogAsset::Pcm, CatalogAsset::Bytes, CatalogAsset::Image]);
    assert_eq!(d.preset.as_ref().map(|p| p.values["drive"]), Some(0.25));
    // the catalog's rules hold for modules too: a misplaced field is refused by the host
    assert!(parse(&json.replace(r#""sidechain":true"#, r#""sidechain":true,"path":"x.y""#)).unwrap_err().contains("control outputs"));
    assert!(parse(&json.replace(r#""drive":0.25"#, r#""drive":2"#)).unwrap_err().contains("outside"));
}

#[test]
fn the_descriptor_is_found_in_a_wasm_without_running_it() {
    // a minimal module: magic, version, one custom section "deck.module"
    let mut w = b"\0asm\x01\0\0\0".to_vec();
    let json = Gain::DESCRIPTOR.as_bytes();
    let name = b"deck.module";
    let mut payload = vec![name.len() as u8];
    payload.extend_from_slice(name);
    payload.extend_from_slice(json);
    w.push(0);
    // LEB128 size
    let mut n = payload.len();
    loop {
        let b = (n & 0x7f) as u8;
        n >>= 7;
        if n == 0 {
            w.push(b);
            break;
        }
        w.push(b | 0x80);
    }
    w.extend_from_slice(&payload);
    assert_eq!(read_descriptor(&w), Some(Gain::DESCRIPTOR));
    assert_eq!(read_descriptor(&w[..20]), None, "cut short");
    assert_eq!(read_descriptor(b"not wasm"), None);
}

#[test]
fn strings_are_nul_padded_and_cut_on_char_boundaries() {
    let mut f = [0u8; 4];
    put_str(&mut f, "héé");
    assert_eq!(get_str(&f), "hé");
    put_str(&mut f, "a");
    assert_eq!(get_str(&f), "a");
}

#[test]
fn parameters_go_through_the_table_clamped_dirty_and_into_the_fields() {
    let mut rig = Rig::<Gain>::new(48_000.0, 1);
    assert_eq!((rig.module().amount, rig.module().offset, rig.params()), (1.0, -0.25, &[1.0, -0.25][..]));
    assert_eq!(rig.module().changed, 0, "dm_new reads the defaults, no params_changed");
    rig.set_param(Gain::AMOUNT, 9.0);
    rig.set_param(Gain::OFFSET, f32::NAN);
    rig.set_param(7, 1.0);
    assert_eq!((rig.params(), rig.dirty()), (&[2.0, -0.25][..], vec![0]));
    assert_eq!(rig.module().amount, 1.0, "the field moves at the next call");
    rig.run(0.0, 16);
    assert_eq!((rig.module().amount, rig.module().changed, rig.dirty()), (2.0, 1, vec![]));
    rig.run(16.0, 16);
    assert_eq!(rig.module().changed, 1, "nothing dirty, no params_changed");
    rig.set_param(Gain::AMOUNT, 2.0);
    assert!(rig.dirty().is_empty(), "the same value is no change");
}

#[test]
fn the_factory_preset_is_defaults_then_its_values() {
    let mut rig = Rig::<Synth>::new(48_000.0, 1);
    assert_eq!(rig.params(), [3.0, 800.0]);
    rig.set_param(Synth::CUTOFF, 100.0);
    rig.apply_preset();
    rig.run(0.0, 4);
    assert_eq!((rig.module().wave, rig.module().cutoff), (0.0, 6000.0));
}

#[test]
fn audio_control_events_and_logs_go_through_the_io() {
    let mut rig = Rig::<Gain>::new(48_000.0, 5);
    rig.set_param(Gain::AMOUNT, 0.5);
    rig.set_control_in(Gain::MOD, 0.25);
    rig.set_channels(true, Gain::IN, 2);
    rig.set_channels(false, Gain::OUT, 2);
    rig.set_audio_in(Gain::IN, 0, &[1.0, -1.0, 0.0, 0.5]);
    rig.set_audio_in(Gain::IN, 1, &[0.0, 0.0, 1.0, 1.0]);
    rig.set_character("catpope", "dream", "loud", 2);
    rig.set_var(Gain::STORY_ACT, Some(3.0));
    // 120 bpm at 48 kHz: 24 000 frames a beat; the call crosses beat 1
    rig.run(23_998.0, 4);
    let out = rig.audio_out(Gain::OUT);
    assert_eq!((out.ch(), out.chan(0), out.chan(1)), (2, &[0.75, -0.25, 0.25, 0.5][..], &[0.25, 0.25, 0.75, 0.75][..]));
    assert_eq!(rig.control_out(Gain::PEAK), (0.75, true));
    let events = rig.events_out(Gain::BEATS);
    assert_eq!(events.len(), 1);
    assert_eq!((events[0].kind(), events[0].name, events[0].port, events[0].prop(0)), (Some(Kind::Event), 0, 0, Some(1.0)));
    let lines = take_log();
    assert_eq!(lines.len(), 1);
    assert!(lines[0].1.starts_with("loud at beat 1.0") && lines[0].1.ends_with("act Some(3.0)"), "{lines:?}");
    // the same beat again: nothing more; the events are cleared, the control output not written but held
    rig.set_character("catpope", "dream", "calm", 0);
    rig.run(24_002.0, 4);
    assert!(rig.events_out(Gain::BEATS).is_empty() && take_log().is_empty());
    assert!(rig.control_out(Gain::PEAK).1, "written again (the module sets it every block)");
}

#[test]
fn planar_ports_follow_the_cables_channels() {
    let mut rig = Rig::<Gain>::new(48_000.0, 1);
    rig.set_param(Gain::OFFSET, 0.0);
    rig.set_channels(true, Gain::IN, 5);
    rig.set_channels(false, Gain::OUT, 5);
    for c in 0..5 {
        rig.set_audio_in(Gain::IN, c, &[c as f32; QUANTUM]);
    }
    rig.run(0.0, QUANTUM as u32);
    let out = rig.audio_out(Gain::OUT);
    assert_eq!((out.ch(), out.frames()), (5, QUANTUM));
    assert!((0..5).all(|c| out.chan(c).iter().all(|&v| v == c as f32)));
    assert!(out.chan(7).iter().all(|&v| v == 0.0), "a channel the cable does not carry reads silence");
    // mono: stereo() gives the one channel twice; a call is at most a quantum
    rig.set_channels(true, Gain::IN, 1);
    rig.set_channels(false, Gain::OUT, 1);
    rig.run(128.0, 1000);
    let (l, r) = rig.audio_out(Gain::OUT).stereo();
    assert_eq!((l.len(), l.as_ptr(), l[0]), (QUANTUM, r.as_ptr(), 0.0));
}

#[test]
fn event_outputs_are_capped_and_the_rest_counted() {
    #[derive(Module)]
    #[module(name = "burst", domain = "control")]
    #[port(output, event, name = "out", capacity = 3)]
    struct Burst {}
    impl Module for Burst {
        fn control(&mut self, _: &Ctx, out: &mut Outputs) {
            for i in 0..5 {
                out.emit(Self::OUT, Ev::note_on(i, i, 440.0, f32::NAN, 1.0));
            }
            assert!(!out.emit(3, Ev::reset(0)), "an undeclared port takes nothing");
        }
    }
    let mut rig = Rig::<Burst>::new(48_000.0, 1);
    rig.run(0.0, 128);
    assert_eq!((rig.events_out(Burst::OUT).len(), rig.out_hdr().dropped[0], rig.out_hdr().ev_out_len[0]), (3, 2, 3));
    rig.run(128.0, 128);
    assert_eq!(rig.out_hdr().dropped[0], 2, "counted per call");
}

#[test]
fn note_events_arrive_sorted_with_ids_and_reset() {
    let mut rig = Rig::<Synth>::new(48_000.0, 1);
    let ok = rig.set_events_in(
        Synth::NOTES,
        &[Ev::note_off(9, 1), Ev::note_on(2, 1, 440.0, 69.0, 0.8), Ev::note_on(2, 2, 220.0, 57.0, 0.5), Ev::note_on(5, 3, 330.0, f32::NAN, 0.5)],
    );
    assert!(ok);
    rig.run(0.0, 16);
    assert_eq!(&rig.module().seen[..4], &[(2, 1), (2, 1), (5, 1), (9, 2)], "by frame, stable");
    assert_eq!((rig.module().voices, &rig.module().held[..2]), (2, &[2, 3][..]), "the off matches its on by id");
    assert_eq!(rig.audio_out(Synth::OUT).chan(0)[0], 2.0);
    rig.run(16.0, 16);
    assert_eq!(rig.module().seen_n, 0, "the host clears the inputs after a call");
    rig.set_events_in(Synth::NOTES, &[Ev::reset(0)]);
    rig.run(32.0, 16);
    assert_eq!(rig.module().voices, 0);
    // past the capacity: the host keeps the first and says so
    assert!(!rig.set_events_in(Synth::NOTES, &[Ev::reset(0); 9]));
}

#[test]
fn ctx_tempo_is_deck_cores_tempo_map_and_the_times_are_there() {
    let mut rig = Rig::<Gain>::new(48_000.0, 1);
    let map = TempoMap::new(97.5, 3, 48_000.0).with_bpm(10_000.0, 140.0).with_beat(30_000.0, 5.25);
    rig.set_tempo(map.frame, map.beat, map.bpm, map.per_bar, map.running);
    (rig.hdr_mut().frame, rig.hdr_mut().frames) = (31_234.0, 100);
    let ctx = Ctx::new(rig.inp(), &[]);
    let t = ctx.tempo();
    for f in [0.0, 12_345.5, 31_234.0, 99_999.0] {
        assert_eq!(t.beat_at(f), map.beat_at(f), "beat_at({f})");
        assert_eq!(t.frame_of(t.beat_at(f)), map.frame_of(map.beat_at(f)));
    }
    assert_eq!(ctx.beats(), (map.beat_at(31_234.0), map.beat_at(31_334.0)));
    assert_eq!(ctx.bar(), map.beat_at(31_234.0) / 3.0);
    assert_eq!(ctx.show_ms(), None);
    rig.set_show_ms(Some(1234.5));
    assert_eq!(Ctx::new(rig.inp(), &[]).show_ms(), Some(1234.5));
}

#[test]
fn the_character_the_senses_the_variables_and_the_seed_are_read_only_data() {
    let mut rig = Rig::<Policy>::new(44_100.0, 0xDEAD_BEEF_0123);
    rig.set_character("catpope", "dream", "calm", 4);
    rig.set_senses(0.5, 0.25, 1.0);
    rig.set_var(Policy::BEING_GLOW, Some(0.7));
    let ctx = Ctx::new(rig.inp(), &[]);
    let c = ctx.character();
    assert_eq!((c.id, c.persona, c.mood, c.mood_index, c.energy, c.touch, c.strain), ("catpope", "dream", "calm", 4, 0.5, 0.25, 1.0));
    assert_eq!((ctx.var(Policy::STORY_ACT), ctx.var(Policy::BEING_GLOW), ctx.seed(), ctx.rate()), (None, Some(0.7), 0xDEAD_BEEF_0123, 44_100.0));
}

#[test]
fn a_policy_writes_path_targets_and_unwritten_outputs_hold() {
    let mut rig = Rig::<Policy>::new(48_000.0, 1);
    rig.set_character("catpope", "", "bright", 1);
    rig.set_senses(1.0, 0.0, 0.0);
    rig.set_control_in(Policy::METER, 10.0);
    rig.run(0.0, 128);
    assert_eq!(rig.control_out(Policy::LEAD_PRESET), (1.0, true));
    let (cutoff, written) = rig.control_out(Policy::CUTOFF);
    assert!(written && (cutoff - (400.0 * 2f32.powf(3.91) + 10.0)).abs() < 0.01, "{cutoff}");
    assert_eq!(rig.control_out(Policy::PAD_GAIN), (0.0, false), "no act set: not written");
    rig.set_var(Policy::STORY_ACT, Some(3.0));
    rig.run(128.0, 128);
    assert_eq!(rig.control_out(Policy::LEAD_PRESET), (1.0, false), "same mood: held, not written again");
    assert_eq!(rig.control_out(Policy::PAD_GAIN), (0.4, true));
}

#[test]
fn sensors_read_their_status_and_channels_and_undeclared_ones_are_unavailable() {
    let mut rig = Rig::<Tilt>::new(48_000.0, 1);
    rig.run(0.0, 4);
    assert_eq!(rig.control_out(Tilt::TILT).0, 0.25, "motion not live: halfway from 0 to the rest value 0.5");
    rig.set_sensor(Sensor::Motion, Status::Live, &[9.81, 0.0, 0.0, 0.1, 0.2, 0.3, 0.0, 0.0, 0.0, 1.0]);
    rig.run(4.0, 4);
    assert_eq!(rig.control_out(Tilt::TILT).0, 0.625, "halfway from 0.25 to 1.0");
    let ctx = Ctx::new(rig.inp(), &[]);
    assert_eq!((ctx.motion().status, ctx.motion().rotation, ctx.motion().orientation[3]), (Status::Live, [0.1, 0.2, 0.3], 1.0));
    assert_eq!((ctx.position().status, ctx.camera_motion().status), (Status::Unavailable, Status::Unavailable));
    assert_eq!(ctx.face().status, Status::Unavailable, "not declared");
    rig.set_sensor(Sensor::Position, Status::Replay, &[52.5, 13.4, 100.0, 90.0, 1.5]);
    let ctx = Ctx::new(rig.inp(), &[]);
    assert!(ctx.position().status.live() && ctx.position().lon == 13.4);
}

#[test]
fn assets_are_filled_once_at_instantiation_and_read_in_place() {
    let mut rig = Rig::<Tilt>::new(48_000.0, 1);
    rig.run(0.0, 4);
    assert!(rig.audio_out(Tilt::OUT).chan(0).iter().all(|&v| v == 0.0), "not ready: silence");
    let pcm: Vec<f32> = [0.0, 0.0, 0.0, 0.0, 1.0, 0.5, 0.25, 0.125].to_vec();
    let bytes: Vec<u8> = pcm.iter().flat_map(|v| v.to_le_bytes()).collect();
    rig.load_asset(Tilt::KICK, AssetKind::Pcm, &bytes, 2, 4, 44_100.0);
    rig.run(4.0, 4);
    assert_eq!(rig.audio_out(Tilt::OUT).chan(0), [1.0, 0.5, 0.25, 0.125], "channel 1 of the planar pcm");
    rig.load_asset(Tilt::SEQ, AssetKind::Bytes, b"midi", 0, 0, 0.0);
    let ctx = Ctx::new(rig.inp(), &[]);
    assert!(ctx.asset(Tilt::SEQ).is_none(), "a Ctx without the module's buffers sees no data");
    let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| rig.load_asset(Tilt::KICK, AssetKind::Pcm, &bytes, 2, 4, 44_100.0)));
    assert!(r.is_err(), "a slot is filled once (immutable per instance)");
}

/// §6, the acceptance of #340: nothing on the SDK's own path allocates – parameters copied, params_changed, events in
/// and out, planar audio, sensors, assets, logs, control outputs – over many calls in strict mode (a strict call that
/// allocated panics in `Rig::run`)
#[test]
fn nothing_on_the_sdks_call_path_allocates() {
    let mut gain = Rig::<Gain>::new(48_000.0, 1);
    let mut synth = Rig::<Synth>::new(48_000.0, 1);
    let mut policy = Rig::<Policy>::new(48_000.0, 1);
    let mut tilt = Rig::<Tilt>::new(48_000.0, 1);
    let bytes = [0u8; 64];
    tilt.load_asset(Tilt::KICK, AssetKind::Pcm, &bytes, 2, 8, 48_000.0);
    gain.set_character("catpope", "", "loud", 0);
    for k in 0..400u32 {
        let frame = (k * 128) as f64;
        gain.set_param(Gain::AMOUNT, (k % 7) as f32 * 0.25);
        gain.run(frame * 200.0, 128);
        synth.set_events_in(Synth::NOTES, &[Ev::note_on(1, k, 440.0, 69.0, 1.0), Ev::note_off(100, k)]);
        synth.run(frame, 128);
        policy.set_character("c", "", "m", k % 3);
        policy.run(frame, 128);
        tilt.set_sensor(Sensor::Motion, Status::Live, &[k as f32; 10]);
        tilt.run(frame, 128);
        for (allocs, who) in [(gain.out_hdr().allocs, "gain"), (synth.out_hdr().allocs, "synth"), (policy.out_hdr().allocs, "policy"), (tilt.out_hdr().allocs, "tilt")] {
            assert_eq!(allocs, 0, "{who} at call {k}");
        }
    }
    take_log();
}

#[test]
fn an_allocation_in_a_call_is_counted_in_a_show_and_fails_a_strict_run() {
    let mut rig = Rig::<Leaky>::new(48_000.0, 1);
    rig.set_strict(false);
    rig.run(0.0, 128);
    assert!(rig.out_hdr().allocs >= 2, "the Vec and its growth: {}", rig.out_hdr().allocs);
    rig.run(128.0, 128);
    assert!(rig.out_hdr().allocs >= 1, "counted per call");
    rig.set_strict(true);
    let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| rig.run(256.0, 128)));
    let msg = r.unwrap_err().downcast::<String>().map(|s| *s).unwrap_or_default();
    assert!(msg.contains("'leaky' allocated"), "{msg}");
}

#[test]
fn gate_cv_renders_notes_held_until_their_off() {
    let tempo = deck_module::Tempo { bpm: 120.0, rate: 48_000.0, running: true, per_bar: 4, anchor_frame: 0.0, anchor_beat: 0.0 };
    let mut cv = GateCv::new(100.0);
    let (mut l, mut r) = ([0.0f32; 8], [0.0f32; 8]);
    // a held note at frame 1 (no length), its off at frame 5; a note of another id does not close it
    cv.render_notes(&tempo, &[Ev::note_on(1, 7, 440.0, 69.0, 1.0), Ev::note_off(3, 8), Ev::note_off(5, 7)], &mut l, &mut r);
    assert_eq!(r, [0.0, 1.0, 1.0, 1.0, 1.0, 0.0, 0.0, 0.0]);
    assert_eq!((l[0], l[1], l[7]), (100.0, 440.0, 440.0), "no glide on a held note");
    // a note with a length: 2 frames at 120 bpm / 48 kHz = 2 / 24 000 beats, gliding over its whole length
    let beats = 2.0 / 24_000.0;
    cv.render_notes(&tempo, &[Ev::note_on(0, 9, 880.0, f32::NAN, 1.0).lasting(beats).gliding(1.0)], &mut l, &mut r);
    assert_eq!(&r[..3], [1.0, 1.0, 0.0]);
    assert!(l[0] == 440.0 && l[1] > 440.0 && l[2] == 880.0, "{l:?}");
}
