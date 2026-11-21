//! The headless deck host as wasm (#363): deck-headless built for wasm32 and wasm32 + simd128 (the cdylib's `hl_*`
//! exports), every scenario run in wasmtime – the report (audio hashes, beats, lines, quality levels, the messages that
//! left) is byte for byte the native one. What the browser runs is that core; its bridge adds nothing a scenario can see.
//! The wasm files are built into target dirs of their own (`<target>/headless-wasm*`), once per change. The same
//! modules answer a model's bones (`hl_bones`): the reference matrices hold natively and in wasm.
mod common;
#[path = "../../space/tests/fixtures/bones.rs"]
mod bones;
use std::path::PathBuf;
use std::process::Command;
use std::sync::OnceLock;
use std::time::{Duration, Instant};
use wasmtime::{Engine, Instance, Module, Store};

fn build(simd: bool) -> PathBuf {
    // the deck's target dir, wherever cargo put it (#526: a worktree's is in <git common dir>/deck-target/<id>). Not the
    // conformance suite's dirs (#536): there `-p live-core` and this `-p deck-headless` built live-core over each other
    // ("info of dependency `deck-space` changed") and each test binary's run re-linked the other's fat-LTO wasm
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).parent().unwrap().join(if simd { "headless-wasm-simd" } else { "headless-wasm" });
    let mut cmd = Command::new("cargo");
    cmd.current_dir(common::deck_dir()).args(["build", "--release", "--target", "wasm32-unknown-unknown", "-p", "deck-headless", "--lib", "--target-dir"]).arg(&dir);
    // a fixed build label (#536), as in conformance/tests/wasm.rs: without it deck-space's build.rs watches the worktree's
    // git index, which every `git status` (its own included) rewrites – each run rebuilt the fat-LTO wasm. The label is
    // only drawn (deck_space::BUILD), no report holds it
    cmd.env("DECK_BUILD_VERSION", "wasm-test");
    if simd {
        cmd.env("RUSTFLAGS", "-C target-feature=+simd128");
    } else {
        cmd.env_remove("RUSTFLAGS");
    }
    let out = cmd.output().expect("cargo runs");
    assert!(out.status.success(), "the wasm build failed:\n{}", String::from_utf8_lossy(&out.stderr));
    dir.join("wasm32-unknown-unknown").join("release").join("deck_headless.wasm")
}

fn engine() -> &'static Engine {
    static ENGINE: OnceLock<Engine> = OnceLock::new();
    ENGINE.get_or_init(Engine::default)
}

/// (simd128, the module, its build time, its compile time): built (each in its own target dir) and compiled side by
/// side on two threads, once for the tests of this binary (#536)
fn modules() -> &'static [(bool, Module, Duration, Duration); 2] {
    static MODULES: OnceLock<[(bool, Module, Duration, Duration); 2]> = OnceLock::new();
    MODULES.get_or_init(|| {
        let prepare = |simd: bool| -> (bool, Module, Duration, Duration) {
            let t = Instant::now();
            let path = build(simd);
            let built = t.elapsed();
            let t = Instant::now();
            (simd, Module::from_file(engine(), path).expect("the module compiles"), built, t.elapsed())
        };
        std::thread::scope(|s| {
            let simd = s.spawn(|| prepare(true));
            [prepare(false), simd.join().expect("the simd128 module")]
        })
    })
}

/// the inbox through one of the module's exports (`hl_run`: a bundle → the report's JSON) → the outbox
fn run_wasm(module: &Module, engine: &Engine, export: &str, bundle: &[u8]) -> String {
    let mut store = Store::new(engine, ());
    let inst = Instance::new(&mut store, module, &[]).expect("no imports");
    let inbox = inst.get_typed_func::<u32, u32>(&mut store, "hl_inbox").unwrap();
    let run = inst.get_typed_func::<u32, i32>(&mut store, export).unwrap();
    let outbox = inst.get_typed_func::<(), u32>(&mut store, "hl_outbox").unwrap();
    let at = inbox.call(&mut store, bundle.len() as u32).unwrap() as usize;
    let mem = inst.get_memory(&mut store, "memory").unwrap();
    mem.data_mut(&mut store)[at..at + bundle.len()].copy_from_slice(bundle);
    let n = run.call(&mut store, bundle.len() as u32).unwrap();
    let ptr = outbox.call(&mut store, ()).unwrap() as usize;
    let text = String::from_utf8(mem.data(&store)[ptr..ptr + n.unsigned_abs() as usize].to_vec()).unwrap();
    assert!(n > 0, "{export}: {text}");
    text
}

/// the fields two reports differ in: step, key, native → wasm
fn diff(native: &str, wasm: &str) -> String {
    let (a, b): (serde_json::Value, serde_json::Value) = (serde_json::from_str(native).unwrap(), serde_json::from_str(wasm).unwrap());
    let mut out = Vec::new();
    if a["status"] != b["status"] {
        out.push(format!("status {} → {} ({})", a["status"], b["status"], b["reason"]));
    }
    for (i, (x, y)) in a["steps"].as_array().unwrap().iter().zip(b["steps"].as_array().unwrap()).enumerate() {
        for (k, v) in x["out"].as_object().into_iter().flatten() {
            if y["out"].get(k) != Some(v) {
                out.push(format!("step {} {}: {k} {v} → {}", i + 1, x["do"], y["out"].get(k).unwrap_or(&serde_json::Value::Null)));
            }
        }
    }
    out.join("\n")
}

/// Scenarios whose audio differs between the targets today, with the ticket that makes them bit-exact: only their
/// `audio` hashes may differ (every other field – beats, lines, levels, messages – must still be the same). One that
/// matches again fails the test until its line goes (an expected difference that is gone).
const KNOWN_DIVERGENT: &[(&str, &str)] = &[
    // the cathedral's fft.bass → drone.gain feeds the FFT's values into the sound; those values differ native vs wasm
    ("01-cathedral-boot", "#381"),
];

#[test]
fn every_scenario_reports_the_same_native_and_in_wasm32_with_and_without_simd128() {
    let bundles = common::bundles();
    let native: Vec<String> = bundles.iter().map(|b| serde_json::to_string(&deck_headless::run(b)).unwrap()).collect();
    for (simd, module, built, compiled) in modules() {
        let target = if *simd { "wasm32 + simd128" } else { "wasm32" };
        let t = Instant::now();
        let mut wrong = Vec::new();
        let mut same = 0;
        for (b, want) in bundles.iter().zip(&native) {
            let got = run_wasm(module, engine(), "hl_run", &serde_json::to_vec(b).unwrap());
            let known = KNOWN_DIVERGENT.iter().find(|(n, _)| *n == b.name);
            let d = diff(want, &got);
            match known {
                None if &got != want => wrong.push(format!("{} differs in {target}:\n{d}", b.name)),
                None => same += 1,
                Some((_, t)) if &got == want => wrong.push(format!("{} is the same in {target} now – {t} is done: remove its KNOWN_DIVERGENT line", b.name)),
                Some((_, t)) if d.lines().any(|l| !l.contains(": audio ")) => wrong.push(format!("{} ({t}) may differ only in its audio hash in {target}:\n{d}", b.name)),
                Some((_, t)) => println!("{target}: {} differs only in its audio ({t})", b.name),
            }
        }
        assert!(wrong.is_empty(), "{}", wrong.join("\n"));
        println!("{target}: {same} of {} scenarios report exactly what native does", bundles.len());
        // where the test's time goes (#536): cargo's build of the wasm, Cranelift's compile, the runs
        println!("{target}: built in {built:.1?}, compiled in {compiled:.1?}, ran in {:.1?}", t.elapsed());
    }
}

/// `hl_bones`' answer: a matrix per line
fn matrices(answer: &str) -> Vec<[f32; 16]> {
    answer
        .lines()
        .map(|l| {
            let bits: Vec<u32> = l.split(' ').map(|x| u32::from_str_radix(x, 16).unwrap_or_else(|_| panic!("not a matrix: {l}"))).collect();
            std::array::from_fn(|i| f32::from_bits(bits[i]))
        })
        .collect()
}

#[test]
fn a_bone_s_world_matrix_matches_the_reference_native_and_in_wasm32_with_and_without_simd128() {
    let mut models = vec![("the two-bone model", bones::glb(), bones::REFERENCE.to_vec())];
    if let Some(file) = bones::media("scutoons/3d/mannequin-more-legs.glb") {
        models.push(("the mannequin", file, bones::export(bones::MANNEQUIN)));
    }
    for (model, glb, reference) in &models {
        let questions: Vec<String> = reference.iter().map(|(bone, clip, t, _)| format!("{bone}\t{}\t{t}", clip.unwrap_or("-"))).collect();
        let input = deck_headless::bones::input(&questions.join("\n"), glb);
        let check = |target: &str, answer: &str| {
            let got = matrices(answer);
            assert_eq!(got.len(), reference.len(), "{model} in {target}: {answer}");
            for (m, (bone, clip, t, want)) in got.iter().zip(reference) {
                assert!(bones::close(m, want), "{model} in {target}: {bone} in {clip:?} at {t} is {m:?}, the reference says {want:?}");
            }
        };
        let native = deck_headless::bones::answer(&input).expect("the native answer");
        check("native", &native);
        for (simd, module, ..) in modules() {
            let target = if *simd { "wasm32 + simd128" } else { "wasm32" };
            let answer = run_wasm(module, engine(), "hl_bones", &input);
            check(target, &answer);
            let same = answer.lines().zip(native.lines()).filter(|(a, b)| a == b).count();
            println!("{model}, {target}: {} matrices match the reference, {same} of them bit for bit what native answers", reference.len());
        }
    }
}
