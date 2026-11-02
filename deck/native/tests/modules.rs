//! The module host (#280, feature `modules`): real wasm built with deck-module, run in wasmtime. The `walk` module is
//! proven against the built-in `walk` node through the whole engine; the fixture module misbehaves on request to show that
//! a panic silences that node and nothing else. The host's own cases (the Io staging, a panic, a runaway loop, hostile
//! imports, memory) are the conformance suite of deck-module-host, generic over its backend. The wasm is built here with the wasm32 target
//! (`rustup target add wasm32-unknown-unknown`), into its own target dir.
#![cfg(feature = "modules")]
use deck_native::audio::{Build, Engine};
use deck_native::graph::{Graph, Kind};
use deck_native::module_host::{Instance, Library};
use deck_native::modules;
use deck_native::scene::Scene;
use std::path::PathBuf;
use std::sync::OnceLock;
use std::time::{Duration, Instant};

fn deck_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).parent().unwrap().to_owned()
}

/// the release wasm of a module crate (built once for all the tests)
fn wasm(file: &str) -> Vec<u8> {
    static BUILD: OnceLock<()> = OnceLock::new();
    let root = deck_root();
    let dir = root.join("target").join("wasm-modules");
    BUILD.get_or_init(|| {
        let cargo = std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
        let out = std::process::Command::new(cargo)
            .current_dir(&root)
            .args(["build", "--release", "--target", "wasm32-unknown-unknown", "-p", "walk-module", "-p", "fixture-module", "--target-dir"])
            .arg(&dir)
            .output()
            .expect("running cargo");
        assert!(
            out.status.success(),
            "building the example modules for wasm32 failed (rustup target add wasm32-unknown-unknown?):\n{}",
            String::from_utf8_lossy(&out.stderr)
        );
    });
    std::fs::read(dir.join("wasm32-unknown-unknown/release").join(file)).unwrap()
}

fn walk() -> Vec<u8> {
    wasm("walk_module.wasm")
}

fn fixture() -> Vec<u8> {
    wasm("fixture_module.wasm")
}

const RATE: f64 = 48_000.0;

#[test]
fn a_module_is_read_checked_and_registered_without_running_it() {
    let l = Library::global().load(&walk()).unwrap();
    assert_eq!(l.desc.name, "walk");
    assert_eq!(l.desc.params.iter().map(|p| p.name.as_str()).collect::<Vec<_>>(), ["spread", "rate", "glide", "base", "octaves", "length"]);
    // its kind is a graph kind: ports from the descriptor, checked like any
    let kind = Kind::parse("module:walk").expect("registered");
    assert_eq!(kind, Kind::Module(l.kind));
    assert_eq!(kind.inputs().iter().map(|p| p.name).collect::<Vec<_>>(), ["spread", "rate", "glide", "base", "octaves", "length"]);
    assert_eq!(kind.outputs().iter().map(|p| p.name).collect::<Vec<_>>(), ["cv"]);
    assert_eq!(deck_native::params::default_for(kind, "rate"), 2.0);
    let same = Library::global().load(&walk()).unwrap();
    assert!(std::sync::Arc::ptr_eq(&l, &same), "the compile is cached");
    // the same bytes as a file in DECK_MODULES' folder
    let scene = Scene::parse("[[node]]\nid=\"w\"\nkind=\"module:walk\"\n").unwrap();
    assert!(modules::check(&scene).is_ok());
    assert!(modules::check_with(&scene, |f| f == "core").unwrap_err().to_string().contains("'modules' (node w)"));
    // it does not read as a module when it isn't one
    assert!(Library::global().load(b"\0asm\x01\0\0\0").unwrap_err().to_string().contains("deck.module"));
    assert!(Library::global().load(b"hello").is_err());
}

#[test]
fn the_compile_is_cached_on_disk_by_the_wasm_hash() {
    let dir = std::env::temp_dir().join(format!("deck-module-cache-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let a = Library::with_cache(&dir).load(&walk()).unwrap();
    assert!(!a.from_disk);
    let files: Vec<_> = std::fs::read_dir(&dir).unwrap().filter_map(|e| e.ok()).collect();
    assert_eq!(files.len(), 1);
    assert!(files[0].file_name().to_string_lossy().starts_with(&a.hash));
    let b = Library::with_cache(&dir).load(&walk()).unwrap();
    assert!(b.from_disk, "a second library takes it from the cache");
    // a damaged cache file is compiled again, not trusted (the mapped file is let go first: Windows locks it)
    drop((a, b));
    std::fs::write(files[0].path(), b"not a compiled module").unwrap();
    assert!(!Library::with_cache(&dir).load(&walk()).unwrap().from_disk);
    let _ = std::fs::remove_dir_all(&dir);
}

/// the scene of the proof: a walk (the built-in or the module) playing an oscillator through the output
fn walk_scene(kind: &str) -> Scene {
    Scene::parse(&format!(
        "[[node]]\nid=\"w\"\nkind=\"{kind}\"\nseed=42\nspread=1.0\nrate=3.0\nglide=0.5\n\
         [[node]]\nid=\"o\"\nkind=\"osc\"\ngain=0.5\n[[node]]\nid=\"out\"\nkind=\"audio_out\"\nvolume=1\n\
         [[link]]\nfrom=\"w.cv\"\nto=\"o.cv\"\n[[link]]\nfrom=\"o\"\nto=\"out.in\"\n"
    ))
    .unwrap()
}

fn render(scene: &Scene, seconds: usize) -> Vec<f32> {
    modules::check(scene).unwrap();
    let g = Graph::build(scene).unwrap();
    let (mut e, _h) = Engine::build_with(&g, RATE as f32, 1.0, Build::detect()).unwrap();
    let mut out = Vec::new();
    // device-sized buffers, as the callback gets them
    for _ in 0..seconds * 48_000 / 512 {
        let mut buf = vec![0.0f32; 512 * 2];
        e.render(&mut buf, 2, |x| x);
        out.extend(buf);
    }
    out
}

/// the proof (#280): the module, compiled to wasm and run in wasmtime inside the engine, plays what the built-in `walk`
/// node plays – the same melody through the same voice. (Natively the two are bit-equal – modules/walk/tests; in wasm
/// `powf` is the compiler's own libm, which may differ in the last bit, hence the tolerance.)
#[test]
fn the_walk_module_plays_what_the_built_in_walk_plays() {
    Library::global().load(&walk()).unwrap();
    let builtin = render(&walk_scene("walk"), 4);
    let module = render(&walk_scene("module:walk"), 4);
    assert_eq!(builtin.len(), module.len());
    let peak = builtin.iter().fold(0.0f32, |m, x| m.max(x.abs()));
    assert!(peak > 0.1, "the built-in plays: {peak}");
    let worst = builtin.iter().zip(&module).fold(0.0f32, |m, (a, b)| m.max((a - b).abs()));
    assert!(worst < 2e-3, "the module is the built-in's melody: worst sample difference {worst}");
    // and it is not the same by silence or by accident: another seed plays another melody
    let other = render(&walk_scene("module:walk").with_seed(7), 4);
    let diff = builtin.iter().zip(&other).fold(0.0f32, |m, (a, b)| m.max((a - b).abs()));
    assert!(diff > 0.05, "another seed, another melody ({diff})");
}

trait WithSeed {
    fn with_seed(self, seed: i64) -> Scene;
}

impl WithSeed for Scene {
    fn with_seed(mut self, seed: i64) -> Scene {
        for n in &mut self.nodes {
            if n.id == "w" {
                n.params.insert("seed".into(), toml::Value::Integer(seed));
            }
        }
        self
    }
}

/// through the engine: a module that panics is zeros, the rest of the scene plays on
#[test]
fn a_trapped_node_is_silent_and_the_show_goes_on() {
    Library::global().load(&fixture()).unwrap();
    let scene = Scene::parse(
        "[[node]]\nid=\"f\"\nkind=\"module:fixture\"\nmode=1\n[[node]]\nid=\"o\"\nkind=\"osc\"\nfreq=440\ngain=0.5\n\
         [[node]]\nid=\"m\"\nkind=\"mixer\"\n[[node]]\nid=\"out\"\nkind=\"audio_out\"\nvolume=1\n\
         [[link]]\nfrom=\"f.out\"\nto=\"m.in1\"\n[[link]]\nfrom=\"o\"\nto=\"m.in2\"\n[[link]]\nfrom=\"m\"\nto=\"out.in\"\n",
    )
    .unwrap();
    let out = render(&scene, 1);
    let peak = out.iter().fold(0.0f32, |m, x| m.max(x.abs()));
    assert!(peak > 0.3 && peak < 0.6, "the oscillator plays through the mixer: {peak}");
    assert!(deck_native::module_host::ANY_SILENCED.load(std::sync::atomic::Ordering::Relaxed));
}

/// numbers for the record (`cargo test -p deck-native --features modules -- --nocapture module_costs`)
#[test]
fn module_costs() {
    let l = Library::global().load(&walk()).unwrap();
    let t = Instant::now();
    let mut i = Instance::new(&l, RATE, 42).unwrap();
    let inst = t.elapsed();
    i.set_param(0, 1.0);
    i.set_param(1, 8.0);
    i.process(0.0, 1024);
    let mut total = Duration::ZERO;
    for k in 1..=200 {
        let t = Instant::now();
        i.process(k as f64 * 1024.0, 1024);
        total += t.elapsed();
    }
    let per = total / 200;
    println!("walk: compile {:?}, instantiate {:?}, a 1024-frame block {:?} ({:.2}% of its {:.1} ms)", l.compile, inst, per, per.as_secs_f64() / (1024.0 / RATE) * 100.0, 1024.0 / RATE * 1000.0);
    assert!(i.silenced().is_none());
}
