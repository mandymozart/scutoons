//! The module host's conformance suite (#305), ABI v1 – until #343 moves the modules (v2: conformance_v2.rs): every
//! case is generic over [`Backend`], so a second backend (the browser's) runs the same fixtures by adding one
//! `conformance!` line at the bottom. Fixtures: the real modules `walk` and `fixture` (built with deck-module for wasm32 –
//! `rustup target add wasm32-unknown-unknown` – into their own target dir), and hostile modules written in WAT: an import
//! the sandbox does not allow, a wrong ABI, an `Io` outside the memory, a runaway loop in `dm_new` and in `dm_process`,
//! allocation-heavy memory.
use deck_core::tempo::TempoMap;
use deck_module_host::{Backend, Instance, MEMORY_LIMIT};
use std::sync::OnceLock;
use std::time::{Duration, Instant};

mod common;
use common::{err_text, load, wasm, RATE};

/// A module in WAT with the SDK's exports and a descriptor, for what the real SDK would never build. `imports` are WAT
/// import lines, `new` / `process` the bodies of `dm_new` / `dm_process (param frames i32)`, `abi` / `io` what `dm_abi` /
/// `dm_io` return, `pages` the initial memory (64 KiB each).
fn hostile(imports: &str, new: &str, process: &str, abi: u32, io: u32, pages: u32) -> Vec<u8> {
    let desc = r#"{\"abi\":1,\"name\":\"hostile\",\"version\":\"1\",\"params\":[],\"ports\":[]}"#;
    wat::parse_str(format!(
        r#"(module
            {imports}
            (memory (export "memory") {pages})
            (func (export "dm_abi") (result i32) i32.const {abi})
            (func (export "dm_io") (result i32) i32.const {io})
            (func (export "dm_new") {new})
            (func (export "dm_set_param") (param i32 f32))
            (func (export "dm_process") (param i32) {process})
            (func (export "dm_control") (param i32))
            (@custom "deck.module" "{desc}"))"#
    ))
    .expect("wat")
}

fn ok_module() -> Vec<u8> {
    hostile("", "", "", 1, 0, 4)
}

fn fixture_instance<B: Backend>(e: &B::Engine, seed: u64) -> Instance<B> {
    Instance::new(&load::<B>(e, &wasm("fixture_module.wasm")).unwrap(), RATE, seed).unwrap()
}


fn a_module_is_read_and_checked_without_running_it<B: Backend>(e: &B::Engine) {
    let l = load::<B>(e, &wasm("walk_module.wasm")).unwrap();
    assert_eq!(l.desc.name, "walk");
    assert_eq!(l.desc.params.iter().map(|p| p.name.as_ref()).collect::<Vec<_>>(), ["spread", "rate", "glide", "base", "octaves", "length"]);
    assert_eq!(l.layout.outputs, [0]);
    assert_eq!(l.hash.len(), 64);
    // it does not read as a module when it isn't one
    assert!(err_text(load::<B>(e, b"\0asm\x01\0\0\0")).contains("deck.module"));
    assert!(load::<B>(e, b"hello").is_err());
    // and it runs: a block of the walk is not silent
    let mut i = Instance::new(&l, RATE, 42).unwrap();
    assert!(i.process(0.0, 64));
    assert!(i.silenced().is_none());
}

fn audio_control_tempo_character_and_variables_cross_the_boundary<B: Backend>(e: &B::Engine) {
    let mut i = fixture_instance::<B>(e, 7);
    // mode 0: 0.25 + bias on both channels, the first frame plus the input's sum
    let ones = vec![1.0f32; 64];
    i.set_audio_in(0, &ones, &ones);
    assert!(i.process(0.0, 64));
    let (mut l, mut r) = (vec![0.0; 64], vec![0.0; 64]);
    i.read_audio_out(0, &mut l, &mut r);
    assert_eq!((l[0], l[1], r[0], r[63]), (0.25 + 64.0, 0.25, 0.25 + 64.0, 0.25));
    let ev = i.events();
    assert_eq!(ev.len(), 1);
    assert_eq!(deck_module::abi::get_str(&ev[0].name), "tick");
    // a parameter: bias (index 1), clamped by the module to its range
    i.set_param(1, 5.0);
    i.process(64.0, 64);
    i.read_audio_out(0, &mut l, &mut r);
    assert_eq!(l[1], 1.25, "bias clamped to 1.0");
    // mode 4 plays the tempo and a variable as numbers
    i.set_param(0, 4.0);
    i.set_tempo(&TempoMap::new(90.0, 4, RATE).with_beat(0.0, 2.0));
    i.set_vars(&[("fx.level".into(), 0.5)]);
    i.set_character("catpope", "dream", "calm");
    assert_eq!(i.character_id(), "catpope");
    i.process(24_000.0, 64);
    i.read_audio_out(0, &mut l, &mut r);
    let beat = TempoMap::new(90.0, 4, RATE).with_beat(0.0, 2.0).beat_at(24_000.0) as f32;
    assert!((l[0] - (0.9 + 0.5)).abs() < 1e-6, "bpm / 100 + the variable: {}", l[0]);
    assert!((r[0] - beat).abs() < 1e-5, "the beat: {} vs {beat}", r[0]);
}

fn a_panic_silences_that_instance_and_only_it<B: Backend>(e: &B::Engine) {
    let mut a = fixture_instance::<B>(e, 7);
    let mut b = fixture_instance::<B>(e, 7);
    a.set_param(0, 1.0); // mode 1 panics
    assert!(!a.process(0.0, 64));
    let why = a.silenced().expect("silenced").to_owned();
    assert!(why.contains("dm_process"), "{why}");
    assert!(!a.process(64.0, 64), "and it stays silent");
    assert!(b.process(0.0, 64), "another instance of the same module runs on");
    assert!(b.silenced().is_none());
}

fn a_runaway_loop_is_stopped_by_the_time_budget<B: Backend>(e: &B::Engine) {
    let mut i = fixture_instance::<B>(e, 1);
    i.set_param(0, 2.0); // mode 2 never returns
    let t = Instant::now();
    assert!(!i.process(0.0, 512));
    assert!(t.elapsed() < Duration::from_millis(250), "stopped after {:?}", t.elapsed());
    assert!(i.silenced().unwrap().contains("time budget"), "{:?}", i.silenced());
}

fn a_chatty_module_is_rate_limited_not_stopped<B: Backend>(e: &B::Engine) {
    let mut i = fixture_instance::<B>(e, 1);
    i.set_param(0, 3.0); // mode 3: 50 log lines a block
    for k in 0..4 {
        assert!(i.process(k as f64 * 64.0, 64));
    }
    assert!(i.silenced().is_none());
}

fn a_set_param_after_a_pause_is_not_past_a_stale_deadline<B: Backend>(e: &B::Engine) {
    let mut i = fixture_instance::<B>(e, 1);
    assert!(i.process(0.0, 64));
    std::thread::sleep(Duration::from_millis(120));
    i.set_param(1, 0.5);
    assert!(i.silenced().is_none(), "{:?}", i.silenced());
    assert!(i.process(64.0, 64));
}

fn an_import_beyond_the_log_is_refused_at_load<B: Backend>(e: &B::Engine) {
    let evil = hostile(r#"(import "env" "evil" (func))"#, "", "", 1, 0, 4);
    assert!(err_text(load::<B>(e, &evil)).contains("may import only env.deck_log"));
    let wasi = hostile(r#"(import "wasi_snapshot_preview1" "fd_write" (func (param i32 i32 i32 i32) (result i32)))"#, "", "", 1, 0, 4);
    assert!(err_text(load::<B>(e, &wasi)).contains("wasi_snapshot_preview1.fd_write"));
    // the log itself is allowed
    let log = hostile(r#"(import "env" "deck_log" (func (param i32 i32 i32)))"#, "", "", 1, 0, 4);
    assert!(load::<B>(e, &log).is_ok());
}

fn a_module_missing_an_export_is_refused_at_load<B: Backend>(e: &B::Engine) {
    let mut b = ok_module();
    // rename the `dm_control` export in the binary to `dm_xontrol`
    let at = b.windows(10).position(|w| w == b"dm_control").expect("export name");
    b[at + 3] = b'x';
    assert!(err_text(load::<B>(e, &b)).contains("does not export `dm_control`"));
}

fn a_wrong_abi_or_a_stray_io_is_refused_at_instantiate<B: Backend>(e: &B::Engine) {
    let abi = load::<B>(e, &hostile("", "", "", 2, 0, 4)).unwrap();
    assert!(err_text(Instance::new(&abi, RATE, 1)).contains("speaks ABI 2"));
    let io = load::<B>(e, &hostile("", "", "", 1, 0x00ff_ff00, 4)).unwrap();
    assert!(err_text(Instance::new(&io, RATE, 1)).contains("does not lie in its memory"));
    let small = load::<B>(e, &hostile("", "", "", 1, 0, 1)).unwrap();
    assert!(err_text(Instance::new(&small, RATE, 1)).contains("does not lie in its memory"));
}

fn a_runaway_build_is_stopped_and_refused<B: Backend>(e: &B::Engine) {
    let l = load::<B>(e, &hostile("", "(loop $l br $l)", "", 1, 0, 4)).unwrap();
    let t = Instant::now();
    assert!(err_text(Instance::new(&l, RATE, 1)).contains("dm_new"));
    assert!(t.elapsed() < Duration::from_millis(500), "stopped after {:?}", t.elapsed());
}

fn a_runaway_process_in_a_hostile_module_is_silenced<B: Backend>(e: &B::Engine) {
    let l = load::<B>(e, &hostile("", "", "(loop $l br $l)", 1, 0, 4)).unwrap();
    let mut i = Instance::new(&l, RATE, 1).unwrap();
    assert!(!i.process(0.0, 64));
    assert!(i.silenced().unwrap().contains("time budget"));
    assert!(!i.control(0.0, 64), "and it stays silent");
}

fn a_trap_in_a_hostile_module_is_silenced<B: Backend>(e: &B::Engine) {
    let l = load::<B>(e, &hostile("", "", "unreachable", 1, 0, 4)).unwrap();
    let mut i = Instance::new(&l, RATE, 1).unwrap();
    assert!(!i.process(0.0, 64));
    assert!(i.silenced().unwrap().contains("dm_process"), "{:?}", i.silenced());
}

fn memory_beyond_the_cap_is_refused_and_growth_stops_at_it<B: Backend>(e: &B::Engine) {
    // a start memory above the cap cannot be instantiated
    let big = load::<B>(e, &hostile("", "", "", 1, 0, (MEMORY_LIMIT / 65536) as u32 + 16)).unwrap();
    assert!(Instance::new(&big, RATE, 1).is_err());
    // growth: 100 pages are granted, 2000 more are refused (memory.grow gives -1) – the module is not trapped, the memory not larger
    let grow = "i32.const 100 memory.grow drop  i32.const 2000 memory.grow drop";
    let l = load::<B>(e, &hostile("", "", grow, 1, 0, 4)).unwrap();
    let mut i = Instance::new(&l, RATE, 1).unwrap();
    assert!(i.process(0.0, 64), "{:?}", i.silenced());
    assert_eq!(i.backend().unwrap().memory_size(), 104 * 65536);
    assert!(i.backend().unwrap().memory_size() <= MEMORY_LIMIT);
    // many instances at once, each with its own memory
    let many: Vec<Instance<B>> = (0..16).map(|k| Instance::new(&l, RATE, k).unwrap()).collect();
    assert!(many.iter().all(|m| m.backend().unwrap().memory_size() == 4 * 65536));
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
            case!(a_module_is_read_and_checked_without_running_it);
            case!(audio_control_tempo_character_and_variables_cross_the_boundary);
            case!(a_panic_silences_that_instance_and_only_it);
            case!(a_runaway_loop_is_stopped_by_the_time_budget);
            case!(a_chatty_module_is_rate_limited_not_stopped);
            case!(a_set_param_after_a_pause_is_not_past_a_stale_deadline);
            case!(an_import_beyond_the_log_is_refused_at_load);
            case!(a_module_missing_an_export_is_refused_at_load);
            case!(a_wrong_abi_or_a_stray_io_is_refused_at_instantiate);
            case!(a_runaway_build_is_stopped_and_refused);
            case!(a_runaway_process_in_a_hostile_module_is_silenced);
            case!(a_trap_in_a_hostile_module_is_silenced);
            case!(memory_beyond_the_cap_is_refused_and_growth_stops_at_it);
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
