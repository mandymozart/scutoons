//! The SDK's contract (#280): what the derive declares, what `Ctx` reads, how the ABI's pieces behave.
use deck_core::tempo::TempoMap;
use deck_module::abi::*;
use deck_module::host::{read_descriptor, Descriptor, Dir, Offsets, PortKind};
use deck_module::testing::{take_log, Rig};
use deck_module::{log, Block, Ctx, Describe, Event, Module, Outputs};

/// a stereo gain with a control output (the last block's peak) and a note on every beat
#[derive(Module)]
#[module(name = "gain")]
#[port(input, audio, name = "in")]
#[port(output, audio, name = "out")]
#[port(input, control, name = "mod")]
#[port(output, control, name = "peak")]
#[port(output, event, name = "beats")]
struct Gain {
    #[param(range = 0.0..2.0, default = 1.0)]
    amount: f32,
    #[param(range = -1.0..1.0, default = -0.25)]
    offset: f32,
    last_beat: i64,
}

impl Module for Gain {
    fn process(&mut self, block: &mut Block, ctx: &Ctx) {
        let m = block.control_in(0);
        let n = block.frames();
        let mut peak = 0.0f32;
        for i in 0..n {
            let (l, r) = (block.audio_in(0).0[i], block.audio_in(0).1[i]);
            let (ol, or) = block.audio_out(0);
            ol[i] = l * self.amount + m;
            or[i] = r * self.amount + m;
            peak = peak.max(ol[i].abs());
        }
        block.set_control(0, peak);
        let (_, to) = ctx.beats();
        let beat = to.floor() as i64;
        if beat > self.last_beat {
            self.last_beat = beat;
            block.emit(Event::at_beat("music.beat", beat as f64).with("n", beat as f32));
        }
        if ctx.character().mood == "loud" {
            log!("loud at beat {:.2}", to);
        }
    }

    fn control(&mut self, ctx: &Ctx, out: &mut Outputs) {
        out.set_control(0, ctx.var("story.act").unwrap_or(-1.0));
    }
}

#[test]
fn the_derive_writes_the_descriptor() {
    let d = Descriptor::parse(Gain::DESCRIPTOR).unwrap();
    assert_eq!((d.abi, d.name.as_str(), d.version.as_str()), (ABI_VERSION, "gain", env!("CARGO_PKG_VERSION")));
    assert_eq!(d.params.len(), 2);
    assert_eq!((d.params[1].name.as_str(), d.params[1].min, d.params[1].max, d.params[1].default), ("offset", -1.0, 1.0, -0.25));
    assert_eq!(d.ports_of(Dir::In, PortKind::Audio).count(), 1);
    assert_eq!(d.ports_of(Dir::Out, PortKind::Event).map(|p| p.name.as_str()).collect::<Vec<_>>(), ["beats"]);
    assert_eq!(Gain::PARAMS, 2);
    assert_eq!(Gain::NAME, "gain");
}

#[test]
fn parameters_are_set_by_index_clamped_and_never_nan() {
    let mut g = Gain::build();
    assert_eq!((g.param(0), g.param(1)), (1.0, -0.25));
    g.set_param(0, 9.0);
    assert_eq!(g.param(0), 2.0);
    g.set_param(1, -9.0);
    assert_eq!(g.param(1), -1.0);
    g.set_param(0, f32::NAN);
    g.set_param(7, 1.0);
    assert_eq!(g.param(0), 2.0, "NaN and an unknown index are ignored");
}

#[test]
fn audio_control_events_and_logs_go_through_the_io() {
    let mut rig = Rig::<Gain>::new(48_000.0, 5);
    rig.set_param(0, 0.5);
    rig.io.inp.control[0] = 0.25;
    rig.io.inp.audio[0][0][..4].copy_from_slice(&[1.0, -1.0, 0.0, 0.5]);
    rig.io.inp.audio[0][1][..4].copy_from_slice(&[0.0, 0.0, 1.0, 1.0]);
    rig.set_character("catpope", "dream", "loud");
    // 120 bpm at 48 kHz: 24 000 frames a beat; the call crosses beat 1
    rig.process(23_998.0, 4);
    let (l, r) = rig.audio_out(0);
    assert_eq!(l, [0.75, -0.25, 0.25, 0.5]);
    assert_eq!(r, [0.25, 0.25, 0.75, 0.75]);
    assert_eq!(rig.io.out.control[0], 0.75);
    let events = rig.events();
    assert_eq!(events.len(), 1);
    assert_eq!((events[0].name.as_str(), events[0].start, events[0].prop("n")), ("music.beat", 1.0, Some(1.0)));
    let lines = take_log();
    assert_eq!(lines.len(), 1);
    assert!(lines[0].1.starts_with("loud at beat 1.0"), "{lines:?}");
    // the same beat again: nothing more, the audio outputs and events cleared for the new call
    rig.set_character("catpope", "dream", "calm");
    rig.process(24_002.0, 4);
    assert!(rig.events().is_empty() && take_log().is_empty());
    // control tick: a scene variable becomes a control output
    rig.set_vars(&[("a.b", 1.0), ("story.act", 3.0)]);
    rig.control(24_006.0, 800);
    assert_eq!(rig.io.out.control[0], 3.0);
}

#[test]
fn ctx_tempo_is_deck_cores_tempo_map() {
    let mut rig = Rig::<Gain>::new(48_000.0, 1);
    let map = TempoMap::new(97.5, 3, 48_000.0).with_bpm(10_000.0, 140.0).with_beat(30_000.0, 5.25);
    rig.set_tempo(map.frame, map.beat, map.bpm, map.per_bar, map.running);
    rig.io.inp.frame = 31_234.0;
    rig.io.inp.frames = 512;
    let ctx = Ctx::new(&rig.io.inp);
    let t = ctx.tempo();
    for f in [0.0, 12_345.5, 31_234.0, 99_999.0] {
        assert_eq!(t.beat_at(f), map.beat_at(f), "beat_at({f})");
        assert_eq!(t.frame_of(t.beat_at(f)), map.frame_of(map.beat_at(f)));
    }
    assert_eq!(ctx.beats(), (map.beat_at(31_234.0), map.beat_at(31_746.0)));
    let stopped = map.with_running(31_234.0, false);
    rig.set_tempo(stopped.frame, stopped.beat, stopped.bpm, stopped.per_bar, false);
    let ctx = Ctx::new(&rig.io.inp);
    assert_eq!(ctx.tempo().frame_of(1.0), None);
    assert_eq!(ctx.beats().0, ctx.beats().1);
}

#[test]
fn the_character_the_variables_and_the_seed_are_read_only_data() {
    let mut rig = Rig::<Gain>::new(44_100.0, 0xDEAD_BEEF_0123);
    rig.set_character("catpope", "dream", "calm");
    let long = "x".repeat(40);
    rig.set_vars(&[("story.act", 2.0), (&long, 1.0)]);
    let ctx = Ctx::new(&rig.io.inp);
    assert_eq!((ctx.character().id, ctx.character().persona, ctx.character().mood), ("catpope", "dream", "calm"));
    assert_eq!((ctx.var("story.act"), ctx.var("nope"), ctx.seed(), ctx.rate()), (Some(2.0), None, 0xDEAD_BEEF_0123, 44_100.0));
    assert_eq!(ctx.var(&long[..VAR_KEY_LEN]), Some(1.0), "a long key is cut at the ABI's width");
}

#[test]
fn events_are_capped_and_counted() {
    let mut io = Io::boxed();
    let Io { inp: _, out } = &mut *io;
    let mut o = Outputs::new(out);
    for i in 0..MAX_EVENTS + 3 {
        o.emit(Event::at_beat("x", i as f64));
    }
    assert_eq!(io.out.events_len as usize, MAX_EVENTS);
    assert_eq!(io.out.events_dropped, 3);
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
fn the_io_layout_is_what_the_host_computes() {
    let o = Offsets::get();
    assert_eq!(o.size, std::mem::size_of::<Io>());
    assert_eq!(o.event_size, 88, "an EventRec is 88 bytes on wasm32 and on the host");
    assert_eq!(o.tempo_frame % 8, 0);
    assert_eq!(o.audio_out % 4, 0);
    assert!(o.events > o.events_len && o.events_len > o.control_out && o.control_out > o.audio_out && o.audio_out > o.audio_in);
    // an Io written through the offsets reads back through the type
    let mut io = Io::boxed();
    let base = &mut *io as *mut Io as *mut u8;
    unsafe {
        (base.add(o.bpm) as *mut f64).write(133.0);
        (base.add(o.frames) as *mut u32).write(77);
    }
    assert_eq!((io.inp.bpm, io.inp.frames), (133.0, 77));
}

#[test]
fn descriptors_are_validated() {
    let ok = |extra: &str| format!("{{\"abi\":1,\"name\":\"m\",\"version\":\"1\",\"params\":[{extra}],\"ports\":[]}}");
    assert!(Descriptor::parse(&ok("")).is_ok());
    assert!(Descriptor::parse(&ok("{\"name\":\"a\",\"min\":0,\"max\":1,\"default\":2}")).unwrap_err().contains("default"));
    assert!(Descriptor::parse(&ok("{\"name\":\"A\",\"min\":0,\"max\":1,\"default\":0}")).unwrap_err().contains("identifier"));
    assert!(Descriptor::parse(&ok("{\"name\":\"a\",\"min\":0,\"max\":1,\"default\":0},{\"name\":\"a\",\"min\":0,\"max\":1,\"default\":0}")).unwrap_err().contains("twice"));
    assert!(Descriptor::parse("{\"abi\":2,\"name\":\"m\",\"version\":\"1\",\"params\":[],\"ports\":[]}").unwrap_err().contains("ABI 2"));
    let many: Vec<String> = (0..5).map(|i| format!("{{\"name\":\"p{i}\",\"dir\":\"in\",\"kind\":\"audio\"}}")).collect();
    let d = format!("{{\"abi\":1,\"name\":\"m\",\"version\":\"1\",\"params\":[],\"ports\":[{}]}}", many.join(","));
    assert!(Descriptor::parse(&d).unwrap_err().contains("at most 4"));
    assert!(Descriptor::parse("not json").is_err());
}

#[test]
fn the_descriptor_is_found_in_a_wasm_without_running_it() {
    // a minimal module: magic, version, one custom section "deck.module", one empty custom section after it
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

/// shared/live-core/src/module-abi.ts – the browser host's copy of the ABI's numbers, written from this very crate:
/// `UPDATE_FIXTURES=1 cargo test -p deck-module module_abi_ts` writes it, a plain run fails when it is stale
#[test]
fn module_abi_ts_is_current() {
    use std::mem::offset_of;
    let o = Offsets::get();
    let ts = format!(
        "// GENERATED from deck/module/src/abi.rs – do not edit: `UPDATE_FIXTURES=1 cargo test -p deck-module module_abi_ts` writes it,\n\
         // a plain run fails when it is stale. The module ABI's numbers (deck/module/README.md) for the browser host (modules.ts).\n\
         export const ABI_VERSION = {ABI_VERSION};\n\
         /** the wasm custom section holding the descriptor's JSON */\n\
         export const SECTION = 'deck.module';\n\
         export const MAX_BLOCK = {MAX_BLOCK};\n\
         export const MAX_AUDIO_PORTS = {MAX_AUDIO_PORTS};\n\
         export const MAX_CONTROL_IN = {MAX_CONTROL_IN};\n\
         export const MAX_CONTROL_OUT = {MAX_CONTROL_OUT};\n\
         export const MAX_PARAMS = {MAX_PARAMS};\n\
         export const MAX_VARS = {MAX_VARS};\n\
         export const MAX_EVENTS = {MAX_EVENTS};\n\
         export const MAX_PROPS = {MAX_PROPS};\n\
         export const STR_LEN = {STR_LEN};\n\
         export const VAR_KEY_LEN = {VAR_KEY_LEN};\n\
         export const EVENT_NAME_LEN = {EVENT_NAME_LEN};\n\
         export const PROP_KEY_LEN = {PROP_KEY_LEN};\n\
         /** byte offsets of `Io`'s fields from the address `dm_io()` returns */\n\
         export const OFFSETS = {{\n\
         \tsize: {},\n\tevent: {{ size: {}, start: {}, len: {}, port: {}, nprops: {}, name: {}, keys: {}, values: {} }},\n\
         \ttempoFrame: {},\n\ttempoBeat: {},\n\tbpm: {},\n\trate: {},\n\tframe: {},\n\tseed: {},\n\tframes: {},\n\trunning: {},\n\tperBar: {},\n\
         \tvarsLen: {},\n\tcontrolIn: {},\n\tcharacterId: {},\n\tcharacterPersona: {},\n\tcharacterMood: {},\n\tvarKeys: {},\n\tvarValues: {},\n\
         \taudioIn: {},\n\taudioOut: {},\n\tcontrolOut: {},\n\teventsLen: {},\n\teventsDropped: {},\n\tevents: {},\n}} as const;\n",
        o.size,
        o.event_size,
        offset_of!(EventRec, start),
        offset_of!(EventRec, len),
        offset_of!(EventRec, port),
        offset_of!(EventRec, nprops),
        offset_of!(EventRec, name),
        offset_of!(EventRec, keys),
        offset_of!(EventRec, values),
        o.tempo_frame,
        o.tempo_beat,
        o.bpm,
        o.rate,
        o.frame,
        o.seed,
        o.frames,
        o.running,
        o.per_bar,
        o.vars_len,
        o.control_in,
        o.character_id,
        o.character_persona,
        o.character_mood,
        o.var_keys,
        o.var_values,
        o.audio_in,
        o.audio_out,
        o.control_out,
        o.events_len,
        o.events_dropped,
        o.events,
    );
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../shared/live-core/src/module-abi.ts");
    if std::env::var_os("UPDATE_FIXTURES").is_some() {
        std::fs::write(&path, &ts).unwrap();
    }
    let have = std::fs::read_to_string(&path).unwrap_or_default().replace("\r\n", "\n");
    assert_eq!(have, ts, "module-abi.ts is stale – UPDATE_FIXTURES=1 cargo test -p deck-module module_abi_ts");
}
