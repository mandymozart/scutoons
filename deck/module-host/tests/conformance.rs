//! The module host's conformance suite (#305; module ABI v2 #341, #343, research/ABI-V2.md §1.1, §1.5, §4.3, §6): every
//! case is generic over [`Backend`] – a second backend (the browser's) runs the same fixtures by adding one
//! `conformance!` line at the bottom. Fixtures: the real `fixture` and `walk` modules (deck/modules, built with the SDK
//! for wasm32), a recorded sensor log (tests/fixtures/tilt.sensorlog), and hostile modules written in WAT for what the
//! SDK never builds (a wrong layout, a stray io, an unknown need, an import beyond the log, a runaway build, memory past
//! the cap, shared memory, NaN bits, a failing rebuild).
use deck_core::tempo::TempoMap;
use deck_module::host::parse;
use deck_module::{Ev, Kind, Layout as IoLayout, Sensor, Shape, Status, QUANTUM};
use deck_module_host::{AssetData, Backend, Export, Fault, Instance, Mode, Preset, Setup, MEMORY_LIMIT};
use std::cell::Cell;
use std::sync::OnceLock;
use std::time::{Duration, Instant};

mod common;
use common::{err_text, load, wasm, RATE};

const Q: u32 = QUANTUM as u32;

fn fixture<B: Backend>(e: &B::Engine, setup: Setup) -> Instance<B> {
    Instance::new(&load::<B>(e, &wasm("fixture_module.wasm")).unwrap(), setup).unwrap()
}

fn show() -> Setup {
    Setup::new(RATE, 1).mode(Mode::Show)
}

/// block `k` of `QUANTUM` frames
fn block<B: Backend>(i: &mut Instance<B>, k: u64) -> bool {
    i.run(k as f64 * QUANTUM as f64, Q)
}

/// the fixture's ports and params by index (declaration order per direction)
const OUT: usize = 0;
const WIDE: usize = 1;
const LEVEL: usize = 0;
const TILT: usize = 1;
const INFO: usize = 2;
const MODE: usize = 0;
const BIAS: usize = 1;
const WORK: usize = 2;

/// a restart's rebuild, run on a thread of its own (the platform's loader thread); false when none is due
fn rebuild<B: Backend + Send + 'static>(i: &mut Instance<B>) -> bool {
    match i.take_rebuild() {
        Some(job) => {
            let fresh = std::thread::spawn(move || job.run()).join().expect("the rebuild thread");
            i.offer(fresh);
            true
        }
        None => false,
    }
}

// --- WAT modules for what the SDK never builds

const PLAIN: &str = r#"{"abi":2,"name":"hostile","version":"1","params":[],"ports":[]}"#;

fn io_layout(desc: &str) -> IoLayout {
    IoLayout::of(&Shape::of(&parse(desc).unwrap()).unwrap())
}

/// A module in WAT: `desc` (JSON), import lines, the bodies of `dm_new` / `dm_process`, what `dm_abi` returns, the
/// `dm_layout` hash (`None`: the descriptor's; and `Some(0)` drops the export), where `dm_io` points, the memory line's
/// limits, extra module fields.
struct Wat<'a> {
    desc: &'a str,
    imports: &'a str,
    new: &'a str,
    process: &'a str,
    abi: u32,
    hash: Option<u32>,
    io: u32,
    memory: &'a str,
    extra: &'a str,
}

impl Default for Wat<'_> {
    fn default() -> Self {
        Wat { desc: PLAIN, imports: "", new: "", process: "", abi: 2, hash: None, io: 1024, memory: "1", extra: "" }
    }
}

impl Wat<'_> {
    fn build(&self) -> Vec<u8> {
        let hash = self.hash.unwrap_or_else(|| io_layout(self.desc).hash);
        let layout = if hash == 0 { String::new() } else { format!(r#"(func (export "dm_layout") (result i32) i32.const {})"#, hash as i32) };
        let desc = self.desc.replace('"', "\\\"");
        let (imports, abi, io, memory, new, process, extra) = (self.imports, self.abi, self.io, self.memory, self.new, self.process, self.extra);
        wat::parse_str(format!(
            r#"(module
                {imports}
                (memory (export "memory") {memory})
                (func (export "dm_abi") (result i32) i32.const {abi})
                (func (export "dm_io") (result i32) i32.const {io})
                {layout}
                (func (export "dm_new") {new})
                (func (export "dm_process") {process})
                {extra}
                (@custom "deck.module" "{desc}"))"#
        ))
        .expect("wat")
    }
}

/// counts the copies across the boundary (the backend under test inside)
pub struct Counted<B> {
    inner: B,
    writes: u32,
    reads: Cell<u32>,
}

impl<B: Backend> Backend for Counted<B> {
    type Engine = B::Engine;
    type Compiled = B::Compiled;

    fn compile(engine: &Self::Engine, wasm: &[u8], hash: &str) -> anyhow::Result<(Self::Compiled, bool, Duration)> {
        B::compile(engine, wasm, hash)
    }
    fn imports(c: &Self::Compiled) -> Vec<(String, String)> {
        B::imports(c)
    }
    fn has_export(c: &Self::Compiled, name: &str) -> bool {
        B::has_export(c, name)
    }
    fn instantiate(c: &Self::Compiled, name: &str) -> anyhow::Result<Self> {
        Ok(Counted { inner: B::instantiate(c, name)?, writes: 0, reads: Cell::new(0) })
    }
    fn call(&mut self, export: Export, args: &[deck_module_host::Arg]) -> Result<u32, Fault> {
        self.inner.call(export, args)
    }
    fn memory_size(&self) -> usize {
        self.inner.memory_size()
    }
    fn write(&mut self, off: usize, bytes: &[u8]) {
        self.writes += 1;
        self.inner.write(off, bytes)
    }
    fn read(&self, off: usize, out: &mut [u8]) {
        self.reads.set(self.reads.get() + 1);
        self.inner.read(off, out)
    }
    fn set_budget(&mut self, ms: u64) {
        self.inner.set_budget(ms)
    }
    fn freeze_memory(&mut self) {
        self.inner.freeze_memory()
    }
    fn clock_us(&self) -> u64 {
        self.inner.clock_us()
    }
}

// --- the cases

fn a_module_is_read_with_the_layout_the_derive_compiled<B: Backend>(e: &B::Engine) {
    let l = load::<B>(e, &wasm("fixture_module.wasm")).unwrap();
    assert_eq!((l.desc.name.as_ref(), l.desc.abi), ("fixture", Some(2)));
    assert_eq!(l.hash.len(), 64);
    assert_eq!(l.io, IoLayout::of(&<fixture_module::Fixture as deck_module::Describe>::SHAPE), "host and SDK place the same bytes");
    assert_eq!(l.io, <fixture_module::Fixture as deck_module::Describe>::LAYOUT);
    use deck_module_host::InBind::*;
    assert_eq!(l.layout.inputs, [Audio(0), Audio(1), Control(0), Param(0), Param(1), Param(2)]);
    use deck_module_host::OutBind;
    assert_eq!(l.layout.outputs, [OutBind::Audio(0), OutBind::Audio(1), OutBind::Control(0), OutBind::Control(1), OutBind::Control(2)], "#342: control outputs are graph outputs");
    assert_eq!(l.layout.targets, [(2, "fx:@gain".to_string(), deck_catalog::Mode::Set)]);
    assert_eq!((l.layout.events_in.len(), l.layout.events_out.len()), (1, 1), "event ports are the event host's");
    assert_eq!(l.factory.as_ref().map(|p| (p.name.as_str(), p.values().to_vec())), Some(("init", vec![(BIAS, 0.5)])));
    assert_eq!((l.var_slot("fx.level"), l.asset_slot("sample"), l.param_index("work")), (Some(0), Some(0), Some(WORK)));
    // the engine's seam: channel slices in and out
    let mut seam = Instance::new(&l, Setup::new(RATE, 1)).unwrap();
    seam.set_audio_chans(0, &[&[1.0; 64], &[2.0; 64]]);
    assert!(seam.process(0.0, 64));
    let (mut a, mut b) = ([0.0f32; 64], [0.0f32; 64]);
    seam.read_audio_chan(0, 0, &mut a);
    seam.read_audio_chan(0, 1, &mut b);
    assert_eq!((a[0], a[63], b[0]), (1.25, 1.25, 2.25));
    // it does not read as a module when it isn't one
    assert!(err_text(load::<B>(e, b"\0asm\x01\0\0\0")).contains("deck.module"));
    assert!(load::<B>(e, b"hello").is_err());
}

fn the_walk_module_is_read_without_running_it_and_plays<B: Backend>(e: &B::Engine) {
    let l = load::<B>(e, &wasm("walk_module.wasm")).unwrap();
    assert_eq!(l.desc.name, "walk");
    assert_eq!(l.desc.params.iter().map(|p| p.name.as_ref()).collect::<Vec<_>>(), ["spread", "rate", "glide", "base", "octaves", "length"]);
    assert_eq!(l.layout.outputs, [deck_module_host::OutBind::Audio(0)], "cv out; its notes are an event port");
    assert_eq!(l.layout.events_out.len(), 1);
    let mut i = Instance::new(&l, Setup::new(RATE, 42)).unwrap();
    i.set_param(0, 1.0);
    let mut notes = 0;
    for k in 0..400 {
        assert!(block(&mut i, k), "{:?}", i.silenced());
        notes += i.events_out(0).len();
    }
    assert!(notes >= 2, "120 bpm, 2 steps a beat, ~1 s: {notes} notes");
    assert!(i.audio_out(0).chan(0).iter().all(|&hz| hz > 20.0), "the cv's pitch is in Hz");
}

fn an_import_beyond_the_log_is_refused_at_load<B: Backend>(e: &B::Engine) {
    let with = |imports| Wat { imports, ..Wat::default() }.build();
    assert!(err_text(load::<B>(e, &with(r#"(import "env" "evil" (func))"#))).contains("may import only env.deck_log"));
    let wasi = with(r#"(import "wasi_snapshot_preview1" "fd_write" (func (param i32 i32 i32 i32) (result i32)))"#);
    assert!(err_text(load::<B>(e, &wasi)).contains("wasi_snapshot_preview1.fd_write"));
    // the log itself is allowed
    assert!(load::<B>(e, &with(r#"(import "env" "deck_log" (func (param i32 i32 i32)))"#)).is_ok());
}

fn a_runaway_build_is_stopped_and_refused<B: Backend>(e: &B::Engine) {
    let l = load::<B>(e, &Wat { new: "(loop $l br $l)", ..Wat::default() }.build()).unwrap();
    let t = Instant::now();
    assert!(err_text(Instance::new(&l, Setup::new(RATE, 1))).contains("dm_new"));
    assert!(t.elapsed() < Duration::from_millis(500), "stopped after {:?}", t.elapsed());
}

fn memory_beyond_the_cap_is_refused_and_each_instance_has_its_own<B: Backend>(e: &B::Engine) {
    // a start memory above the cap cannot be instantiated
    let pages = format!("{}", MEMORY_LIMIT / 65536 + 16);
    let big = load::<B>(e, &Wat { memory: &pages, ..Wat::default() }.build()).unwrap();
    assert!(Instance::new(&big, Setup::new(RATE, 1)).is_err());
    // many instances at once, each with its own memory
    let l = load::<B>(e, &Wat { memory: "4", ..Wat::default() }.build()).unwrap();
    let many: Vec<Instance<B>> = (0..16).map(|k| Instance::new(&l, Setup::new(RATE, k)).unwrap()).collect();
    assert!(many.iter().all(|m| m.memory_size() == 4 * 65536));
}

fn a_chatty_module_is_rate_limited_not_stopped<B: Backend>(e: &B::Engine) {
    let mut i = fixture::<B>(e, Setup::new(RATE, 1));
    i.set_param(MODE, 5.0); // 50 log lines a call
    for k in 0..4 {
        assert!(block(&mut i, k));
    }
    assert!(i.silenced().is_none());
}

fn one_copy_in_and_one_out_carry_audio_control_context_and_parameters<B: Backend>(e: &B::Engine) {
    let mut i = fixture::<Counted<B>>(e, Setup::new(RATE, 7));
    i.audio_in_mut(0, 0).fill(1.0);
    i.audio_in_mut(0, 1).fill(2.0);
    for c in 0..5 {
        i.audio_in_mut(1, c).fill(1.0);
    }
    assert!(i.audio_in_mut(1, 5).is_empty() && i.audio_in_mut(2, 0).is_empty(), "undeclared channels and ports");
    i.set_control_in(0, 0.3);
    i.set_vars(&[("other".into(), 9.0), ("fx.level".into(), 0.5)]);
    let copies = |i: &Instance<Counted<B>>| {
        let b = i.backend().unwrap();
        (b.writes, b.reads.get())
    };
    let before = copies(&i);
    assert!(block(&mut i, 0));
    let after = copies(&i);
    assert_eq!((after.0 - before.0, after.1 - before.1), (1, 1), "one write of inp, one read of out");
    let out = i.audio_out(OUT);
    assert_eq!((out.ch(), out.frames(), out.chan(0)[0], out.chan(0)[127], out.chan(1)[64]), (2, 128, 1.25, 1.25, 2.25));
    let wide = i.audio_out(WIDE);
    assert_eq!((0..5).map(|c| wide.chan(c)[9]).collect::<Vec<_>>(), [1.0, 2.0, 3.0, 4.0, 5.0], "five channels each way");
    assert_eq!(i.control_out(LEVEL), (0.6, true));
    assert_eq!(i.control_out(TILT), (0.0, false), "no motion: not written, the consumer holds");
    assert_eq!(i.control_out(INFO), (1.2 + 0.5, true), "bpm / 100 + the declared variable + beat 0");
    // parameters: the host clamps, NaN keeps the last good value; the factory preset and a resolved one
    i.set_param(BIAS, 5.0);
    assert_eq!(i.param(BIAS), Some(1.0));
    i.set_param(BIAS, f32::NAN);
    assert!(block(&mut i, 1));
    assert_eq!(i.audio_out(OUT).chan(0)[3], 2.25);
    assert!(i.apply_factory_preset());
    assert!(block(&mut i, 2));
    assert_eq!(i.audio_out(OUT).chan(0)[3], 1.75);
    let (p, report) = Preset::resolve(&i.loaded().desc, "dark", [("bias", -3.0), ("gone", 1.0)]);
    assert_eq!((report.unknown, report.out_of_range), (vec!["gone".to_string()], vec!["bias".to_string()]));
    i.apply_preset(&p);
    assert!(block(&mut i, 3));
    assert_eq!(i.audio_out(OUT).chan(0)[3], 0.25, "bias −1, clamped from −3");
    // the tempo and a shorter call
    let map = TempoMap::new(90.0, 4, RATE).with_beat(0.0, 2.0);
    i.set_tempo(&map);
    assert!(i.run(24_000.0, 37));
    assert_eq!(i.audio_out(OUT).frames(), 37);
    let want = 0.9 + 0.5 + map.beat_at(24_000.0) as f32 / 1000.0;
    assert!((i.control_out(INFO).0 - want).abs() < 1e-6, "{:?} vs {want}", i.control_out(INFO));
    assert_eq!((i.stats().calls, i.stats().written), (5, 10));
}

fn events_cross_both_ways_sorted_and_counted<B: Backend>(e: &B::Engine) {
    let mut i = fixture::<B>(e, Setup::new(RATE, 1));
    let input = [Ev::note_on(50, 1, 440.0, 69.0, 0.5), Ev::note_off(10, 2), Ev::note_on(10, 3, 220.0, 57.0, 1.0)];
    assert_eq!(i.set_events_in(0, &input), 3);
    i.add_late(2);
    assert!(block(&mut i, 0));
    let echo: Vec<(u32, Option<Kind>, u32)> = i.events_out(0).iter().map(|e| (e.frame, e.kind(), e.id)).collect();
    assert_eq!(echo, [(10, Some(Kind::NoteOff), 1002), (10, Some(Kind::NoteOn), 1003), (50, Some(Kind::NoteOn), 1001)], "sorted by frame, stably");
    assert_eq!(i.events_out(0)[2].pitch_hz(), 440.0);
    // past the input's capacity (8) and the output's (4): dropped and counted
    let many: Vec<Ev> = (0..10).map(|k| Ev::note_on(k, k, 100.0, 40.0, 1.0)).collect();
    assert_eq!(i.set_events_in(0, &many), 8);
    assert!(block(&mut i, 1));
    assert_eq!((i.events_out(0).len(), i.out_hdr().dropped[0]), (4, 4));
    // the inputs are cleared after a call
    assert!(block(&mut i, 2));
    assert!(i.events_out(0).is_empty());
    let s = i.stats();
    assert_eq!((s.events_in, s.dropped_in, s.events_out, s.dropped_out, s.late), (11, 2, 7, 4, 2));
}

fn assets_are_bound_before_init_and_checked_against_their_slot<B: Backend>(e: &B::Engine) {
    let samples: Vec<f32> = (0..128).map(|k| k as f32 / 128.0).collect();
    let mut i = fixture::<B>(e, Setup::new(RATE, 1).asset(AssetData::pcm(0, &[&samples], 22_050.0)));
    assert!(block(&mut i, 0));
    assert_eq!(i.audio_out(WIDE).chan(4)[..4], samples[..4], "wide_out 4 plays the asset");
    // a slot without data is not ready: silence, never a wait
    let mut bare = fixture::<B>(e, Setup::new(RATE, 1));
    assert!(block(&mut bare, 0));
    assert!(bare.audio_out(WIDE).chan(4).iter().all(|&s| s == 0.0));
    let l = load::<B>(e, &wasm("fixture_module.wasm")).unwrap();
    let refuse = |a: AssetData| err_text(Instance::new(&l, Setup::new(RATE, 1).asset(a)));
    assert!(refuse(AssetData::bytes(0, b"midi")).contains("takes Pcm"));
    assert!(refuse(AssetData::pcm(0, &[&vec![0.0; 20_000]], 48_000.0)).contains("at most 65536"));
    assert!(refuse(AssetData::bytes(3, b"x")).contains("no asset slot 3"));
    let big = r#"{"abi":2,"name":"big","version":"1","params":[],"ports":[],"assets":[{"name":"huge","kind":"pcm","max_bytes":70000000}]}"#;
    let extra = r#"(func (export "dm_asset_buf") (param i32 i32) (result i32) i32.const 0)"#;
    assert!(err_text(load::<B>(e, &Wat { desc: big, extra, ..Wat::default() }.build())).contains("cannot fit"));
    let one = r#"{"abi":2,"name":"stingy","version":"1","params":[],"ports":[],"assets":[{"name":"a","kind":"bytes","max_bytes":64}]}"#;
    let l = load::<B>(e, &Wat { desc: one, extra, ..Wat::default() }.build()).unwrap();
    assert!(err_text(Instance::new(&l, Setup::new(RATE, 1).asset(AssetData::bytes(0, b"abc")))).contains("gave no buffer"));
}

fn strict_mode_silences_an_allocation_a_show_counts_it<B: Backend>(e: &B::Engine) {
    let mut strict = fixture::<B>(e, Setup::new(RATE, 1));
    strict.set_param(MODE, 3.0);
    assert!(!block(&mut strict, 0));
    assert!(strict.silenced().unwrap().contains("dm_process"), "{:?}", strict.silenced());
    assert!(!block(&mut strict, 1), "and it stays silent");
    let mut live = fixture::<B>(e, show());
    live.set_param(MODE, 3.0);
    for k in 0..4 {
        assert!(block(&mut live, k));
    }
    assert!(live.silenced().is_none() && live.restarting().is_none());
    assert!(live.stats().allocs >= 4, "{:?}", live.stats());
    assert_eq!(live.audio_out(OUT).chan(0)[0], 0.25, "and it plays on");
}

fn the_memory_is_frozen_after_init<B: Backend>(e: &B::Engine) {
    // dm_new may grow; dm_process's `memory.grow` fails (-1) instead of mapping memory on the audio thread
    let wat = Wat {
        memory: "1",
        new: "i32.const 2 memory.grow drop",
        process: "(if (i32.ne (memory.grow (i32.const 1)) (i32.const -1)) (then unreachable))",
        ..Wat::default()
    };
    let l = load::<B>(e, &wat.build()).unwrap();
    let mut i = Instance::new(&l, Setup::new(RATE, 1)).unwrap();
    assert!(block(&mut i, 0), "{:?}", i.silenced());
    assert_eq!(i.memory_size(), 3 * 65536);
    // the SDK's module: an 8 MiB allocation in a show is counted, then fails on the frozen memory – a trap, a restart
    let mut f = fixture::<B>(e, show());
    let size = f.memory_size();
    f.set_param(MODE, 4.0);
    assert!(!block(&mut f, 0));
    assert!(f.restarting().is_some(), "{:?}", f.stats());
    assert!(f.stats().memory_peak <= size as u64);
}

fn a_trap_in_a_show_restarts_the_instance_faded_off_the_audio_thread<B: Backend + Send + 'static>(e: &B::Engine) {
    let mut i = fixture::<B>(e, show());
    i.set_param(BIAS, 0.5);
    i.set_vars(&[("fx.level".into(), 0.5)]);
    assert!(block(&mut i, 0));
    assert_eq!(i.audio_out(OUT).chan(0)[0], 0.75);
    i.set_param(MODE, 1.0);
    assert!(!block(&mut i, 1));
    assert!(i.restarting().unwrap().contains("dm_process"), "{:?}", i.restarting());
    assert!(i.silenced().is_none(), "a show never cuts the instrument");
    let out = i.audio_out(OUT).chan(0).to_vec();
    assert_eq!((out[0], out[63], out[127]), (0.75 * 127.0 / 128.0, 0.75 * 64.0 / 128.0, 0.0), "the last block fades out");
    assert!(!i.control_out(LEVEL).1, "control outputs hold");
    assert!(i.take_rebuild().is_none(), "not before the pause");
    i.set_param(MODE, 0.0);
    assert!(!block(&mut i, 2));
    assert!(i.audio_out(OUT).chan(0).iter().all(|&s| s == 0.0));
    assert!(rebuild(&mut i), "after one block the rebuild runs on the loader thread");
    assert!(i.take_rebuild().is_none(), "one at a time");
    assert!(block(&mut i, 3), "swapped in");
    let out = i.audio_out(OUT).chan(0).to_vec();
    assert_eq!((out[0], out[127]), (0.75 / 128.0, 0.75), "faded in, with the current parameters");
    // 120 bpm at 48 kHz: frame 384 is beat 0.016
    assert_eq!(i.control_out(INFO).0, 1.2 + 0.5 + (384.0f64 / 24_000.0) as f32 / 1000.0, "and the current context");
    assert!(block(&mut i, 4));
    assert_eq!(i.audio_out(OUT).chan(0)[0], 0.75);
    let s = i.stats();
    assert_eq!((s.traps, s.restarts), (1, 1));
    assert!(i.restarting().is_none());
}

fn a_module_that_keeps_trapping_waits_longer_each_time<B: Backend + Send + 'static>(e: &B::Engine) {
    let mut i = fixture::<B>(e, show());
    i.set_param(MODE, 1.0);
    assert!(!block(&mut i, 0));
    let mut k = 1;
    let mut waits = Vec::new();
    for _ in 0..3 {
        let mut n = 0;
        loop {
            assert!(!block(&mut i, k));
            k += 1;
            n += 1;
            if rebuild(&mut i) {
                break;
            }
            assert!(n < 1000, "no rebuild after {n} blocks");
        }
        waits.push(n);
        // the swap, and the fresh instance traps again at once
        assert!(!block(&mut i, k));
        k += 1;
    }
    // one block, then 0.1 s, then 1 s (at 128 frames, 48 kHz)
    assert_eq!(waits, [1, 38, 375]);
    assert_eq!((i.stats().traps, i.stats().restarts), (4, 3));
    // a rebuild that fails (its `dm_new` traps on the parameter it is handed) counts as one more fault
    let desc = r#"{"abi":2,"name":"fragile","version":"1","params":[{"name":"p","min":0,"max":1,"default":0}],"ports":[]}"#;
    let p = 1024 + io_layout(desc).params;
    let new = format!("(if (f32.gt (f32.load (i32.const {p})) (f32.const 0.5)) (then unreachable))");
    let l = load::<B>(e, &Wat { desc, new: &new, process: "unreachable", ..Wat::default() }.build()).unwrap();
    let mut f = Instance::new(&l, show()).unwrap();
    assert!(!block(&mut f, 0));
    f.set_param(0, 1.0);
    assert!(!block(&mut f, 1));
    assert!(rebuild(&mut f));
    assert!(f.restarting().unwrap().contains("dm_new"), "{:?}", f.restarting());
    assert_eq!(f.stats().traps, 2);
    assert!(!block(&mut f, 2));
    assert!(f.take_rebuild().is_none(), "the pause grew to 0.1 s");
}

fn a_runaway_is_stopped_at_the_budget_when_strict_and_at_four_quanta_in_a_show<B: Backend>(e: &B::Engine) {
    for (setup, why) in [(Setup::new(RATE, 1), "time budget"), (show(), "runaway")] {
        let mut i = fixture::<B>(e, setup);
        i.set_param(MODE, 2.0);
        let t = Instant::now();
        assert!(!block(&mut i, 0));
        assert!(t.elapsed() < Duration::from_millis(250), "stopped after {:?}", t.elapsed());
        let msg = i.silenced().or(i.restarting()).unwrap_or("").to_owned();
        assert!(msg.contains(why), "{msg}");
    }
}

fn an_overrun_in_a_show_is_counted_and_accepted<B: Backend>(e: &B::Engine) {
    // find the work that takes ≈ 4.5 ms here: past the budget (2 ms), well inside the ceiling (4 × 2.67 ms)
    let mut i = fixture::<B>(e, show());
    i.set_param(MODE, 6.0);
    let mut work = 4.0f32;
    let target = loop {
        i.set_param(WORK, work);
        assert!(block(&mut i, 0));
        let us = i.stats().us_last.max(1) as f32;
        if us > 400.0 || work >= 500_000.0 {
            break (work * 4500.0 / us).min(1_000_000.0);
        }
        work *= 4.0;
    };
    i.set_param(WORK, target);
    let before = i.stats().overruns;
    for k in 1..=5 {
        block(&mut i, k);
    }
    assert!(i.stats().overruns > before, "{:?}", i.stats());
    assert!(i.stats().soft > 0);
    assert!(i.silenced().is_none());
    // that work (four times over: the calibration may have run under load) traps a strict instance
    let mut s = fixture::<B>(e, Setup::new(RATE, 1));
    s.set_param(MODE, 6.0);
    s.set_param(WORK, (4.0 * target).min(1_000_000.0));
    assert!(!block(&mut s, 0), "work {target}: {:?}", s.stats());
    assert!(s.silenced().unwrap().contains("time budget"), "{:?}", s.silenced());
}

fn a_needed_sensor_that_is_not_live_keeps_the_module_from_running<B: Backend>(e: &B::Engine) {
    let desc = r#"{"abi":2,"name":"needy","version":"1","needs":["motion"],"params":[],"ports":[]}"#;
    let l = load::<B>(e, &Wat { desc, process: "unreachable", ..Wat::default() }.build()).unwrap();
    let mut i = Instance::new(&l, Setup::new(RATE, 1)).unwrap();
    for status in [Status::Unavailable, Status::Asking, Status::Denied] {
        i.set_sensor(Sensor::Motion, status, &[]);
        assert!(!block(&mut i, 0));
        assert_eq!(i.unavailable(), Some("motion"));
        assert!(i.silenced().is_none(), "not run, not a fault");
    }
    assert_eq!(i.stats().unavailable, 3);
    assert!(!i.set_sensor(Sensor::Position, Status::Live, &[]), "not declared");
    // live: it runs (and this one traps, which proves it)
    i.set_sensor(Sensor::Motion, Status::Replay, &[0.0; 10]);
    assert!(!block(&mut i, 1));
    assert!(i.unavailable().is_none() && i.silenced().is_some());
}

/// `{frame, channel, value}` lines; `channel` is `<sensor>.<index>`
fn sensor_log(text: &str) -> Vec<(u64, Sensor, usize, f32)> {
    text.lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| {
            let v: serde_json::Value = serde_json::from_str(l).expect("a log line");
            let (name, index) = v["channel"].as_str().unwrap().rsplit_once('.').unwrap();
            (v["frame"].as_u64().unwrap(), Sensor::from_name(name).unwrap(), index.parse().unwrap(), v["value"].as_f64().unwrap() as f32)
        })
        .collect()
}

/// §4.3: the log is the input – each block gets the latest sample at or before its first frame, status `Replay`
fn replay<B: Backend>(i: &mut Instance<B>, log: Option<&[(u64, Sensor, usize, f32)]>, blocks: u64) -> Vec<(u32, bool)> {
    let mut motion = [0.0f32; 10];
    (0..blocks)
        .map(|k| {
            let start = k * QUANTUM as u64;
            match log {
                Some(log) => {
                    for &(_, _, c, v) in log.iter().filter(|(f, s, _, _)| *f <= start && *s == Sensor::Motion) {
                        motion[c] = v;
                    }
                    i.set_sensor(Sensor::Motion, Status::Replay, &motion);
                }
                None => {
                    i.set_sensor(Sensor::Motion, Status::Unavailable, &[]);
                }
            }
            assert!(block(i, k));
            let (v, w) = i.control_out(TILT);
            (v.to_bits(), w)
        })
        .collect()
}

fn a_sensor_log_replays_the_same_on_every_run<B: Backend>(e: &B::Engine) {
    let text = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/tilt.sensorlog")).unwrap();
    let log = sensor_log(&text);
    let a = replay(&mut fixture::<B>(e, Setup::new(RATE, 1)), Some(&log), 64);
    let b = replay(&mut fixture::<B>(e, Setup::new(RATE, 1)), Some(&log), 64);
    assert_eq!(a, b, "the same log, the same output");
    let tilt: Vec<f32> = a.iter().map(|(v, _)| f32::from_bits(*v)).collect();
    // blocks start at 0, 128, 256 (0.0) · 384 … 896 (1.5) · 1024 … 1920 (−2.25) · 2048 … 4992 (4.0) · 5120 … (0.5)
    assert_eq!((tilt[0], tilt[2], tilt[3], tilt[7], tilt[8], tilt[15], tilt[16], tilt[39], tilt[40], tilt[63]), (0.0, 0.0, 1.5, 1.5, -2.25, -2.25, 4.0, 4.0, 0.5, 0.5));
    assert!(a.iter().all(|(_, w)| *w));
    // without a log the module runs on (`wants`), its tilt never written – the consumer holds
    let none = replay(&mut fixture::<B>(e, Setup::new(RATE, 1)), None, 8);
    assert!(none.iter().all(|(_, w)| !*w));
}

fn a_wrong_layout_abi_io_or_export_is_refused<B: Backend>(e: &B::Engine) {
    let instance = |w: Wat| err_text(Instance::new(&load::<B>(e, &w.build()).unwrap(), Setup::new(RATE, 1)));
    assert!(instance(Wat { hash: Some(0x1234), ..Wat::default() }).contains("its layout (00001234)"));
    assert!(instance(Wat { abi: 1, ..Wat::default() }).contains("speaks ABI 1"));
    assert!(instance(Wat { io: 1028, ..Wat::default() }).contains("does not lie in its memory"));
    assert!(instance(Wat { io: 65536 - 64, ..Wat::default() }).contains("does not lie in its memory"));
    assert!(instance(Wat { new: "unreachable", ..Wat::default() }).contains("dm_new"));
    assert!(err_text(load::<B>(e, &Wat { hash: Some(0), ..Wat::default() }.build())).contains("does not export `dm_layout`"));
    let control = r#"{"abi":2,"name":"policy","version":"1","domain":"control","params":[],"ports":[]}"#;
    assert!(err_text(load::<B>(e, &Wat { desc: control, ..Wat::default() }.build())).contains("does not export `dm_control`"));
    let unknown = r#"{"abi":2,"name":"nosy","version":"1","needs":["camera"],"params":[],"ports":[]}"#;
    assert!(err_text(load::<B>(e, &Wat { desc: unknown, hash: Some(1), ..Wat::default() }.build())).contains("does not know"));
}

fn threads_shared_memory_and_relaxed_simd_are_refused_and_nans_are_canonical<B: Backend>(e: &B::Engine) {
    assert!(load::<B>(e, &Wat { memory: "1 1 shared", ..Wat::default() }.build()).is_err(), "shared memory");
    let relaxed = r#"(func (result v128) v128.const i32x4 0 0 0 0 v128.const i32x4 0 0 0 0 i8x16.relaxed_swizzle)"#;
    assert!(load::<B>(e, &Wat { extra: relaxed, ..Wat::default() }.build()).is_err(), "relaxed SIMD");
    // 0 / 0 from memory (not foldable): the one canonical NaN on every deck, whatever the CPU makes of it
    let desc = r#"{"abi":2,"name":"nan","version":"1","params":[],"ports":[{"name":"k","dir":"out","type":"control"}]}"#;
    let io = io_layout(desc);
    let process = format!(
        "(f32.store (i32.const {}) (f32.div (f32.load (i32.const 0)) (f32.load (i32.const 4)))) (i32.store (i32.const {}) (i32.const 1))",
        1024 + io.control_out,
        1024 + io.written
    );
    let l = load::<B>(e, &Wat { desc, process: &process, ..Wat::default() }.build()).unwrap();
    let mut i = Instance::new(&l, Setup::new(RATE, 1)).unwrap();
    assert!(block(&mut i, 0));
    let (v, written) = i.control_out(0);
    assert!(written && v.is_nan());
    assert_eq!(v.to_bits(), 0x7fc0_0000, "the canonical NaN");
}

macro_rules! conformance {
    ($name:ident, $backend:ty, $engine:expr) => {
        mod $name {
            use super::*;
            macro_rules! case {
                ($case:ident) => {
                    #[test]
                    fn $case() {
                        super::$case::<$backend>($engine);
                    }
                };
            }
            case!(a_module_is_read_with_the_layout_the_derive_compiled);
            case!(the_walk_module_is_read_without_running_it_and_plays);
            case!(an_import_beyond_the_log_is_refused_at_load);
            case!(a_runaway_build_is_stopped_and_refused);
            case!(memory_beyond_the_cap_is_refused_and_each_instance_has_its_own);
            case!(a_chatty_module_is_rate_limited_not_stopped);
            case!(one_copy_in_and_one_out_carry_audio_control_context_and_parameters);
            case!(events_cross_both_ways_sorted_and_counted);
            case!(assets_are_bound_before_init_and_checked_against_their_slot);
            case!(strict_mode_silences_an_allocation_a_show_counts_it);
            case!(the_memory_is_frozen_after_init);
            case!(a_trap_in_a_show_restarts_the_instance_faded_off_the_audio_thread);
            case!(a_module_that_keeps_trapping_waits_longer_each_time);
            case!(a_runaway_is_stopped_at_the_budget_when_strict_and_at_four_quanta_in_a_show);
            case!(an_overrun_in_a_show_is_counted_and_accepted);
            case!(a_needed_sensor_that_is_not_live_keeps_the_module_from_running);
            case!(a_sensor_log_replays_the_same_on_every_run);
            case!(a_wrong_layout_abi_io_or_export_is_refused);
            case!(threads_shared_memory_and_relaxed_simd_are_refused_and_nans_are_canonical);
        }
    };
}

#[cfg(feature = "wasmtime")]
mod wasmtime_backend {
    use super::*;
    use deck_module_host::wasmtime_backend::{Wasmtime, WasmtimeEngine};

    fn engine() -> &'static WasmtimeEngine {
        static E: OnceLock<WasmtimeEngine> = OnceLock::new();
        E.get_or_init(|| WasmtimeEngine::new(None))
    }

    conformance!(wasmtime, Wasmtime, engine());
}
