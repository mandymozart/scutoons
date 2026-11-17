//! The fixture clips' goldens native ≡ wasm32: deck-media-wasm built for wasm32-wasip1 (AV1, HAP, audio: rav1d builds
//! there) and for wasm32-unknown-unknown (the browser's target: HAP and audio – rav1d does not build there, lib.rs says
//! why), each run in wasmtime. The WASI imports are stubs: the decoders touch no file, clock or socket. Pictures are bit
//! for bit; the sound of the float codecs (AAC, Vorbis) is not yet – see `FLOAT_AUDIO`.
mod common;
use std::path::PathBuf;
use std::process::Command;
use wasmtime::{Engine, Instance, Linker, Module, Store, Val};

/// Codecs whose decoded sound differs in the last bits between the targets, and how far at most: symphonia's AAC and
/// Vorbis decoders compute in f32 with the target's own `sin` / `cos` / `powf` (the C runtime natively, Rust's libm in
/// wasm). TODO(#326): bit-exact float audio (symphonia on libm everywhere, or the sample pool decoded once and shared).
const FLOAT_AUDIO: &[(&str, f32)] = &[("av1.mp4", 1e-7)];

/// the clip's sound in wasm against native: "bit for bit", or within its `FLOAT_AUDIO` bound
fn sound(wasm: &mut Wasm, name: &str, clip: &[u8]) -> String {
    let pcm = deck_media::audio::decode(clip.to_vec()).unwrap().map(|p| p.samples).unwrap_or_default();
    let theirs: Vec<f32> = wasm.call("md_pcm", clip).unwrap().chunks_exact(4).map(|b| f32::from_le_bytes([b[0], b[1], b[2], b[3]])).collect();
    assert_eq!(pcm.len(), theirs.len(), "{name}: as many samples");
    if pcm.iter().zip(&theirs).all(|(a, b)| a.to_bits() == b.to_bits()) {
        return "sound bit for bit".into();
    }
    let tol = FLOAT_AUDIO.iter().find(|f| f.0 == name).map(|f| f.1).unwrap_or_else(|| panic!("{name}: the sound differs"));
    let most = pcm.iter().zip(&theirs).map(|(a, b)| (a - b).abs()).fold(0.0f32, f32::max);
    assert!(most <= tol, "{name}: the sound differs by up to {most:e} (allowed {tol:e})");
    format!("sound within {most:e}")
}

fn build(target: &str) -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).parent().unwrap().join("media-wasm");
    let deck = PathBuf::from(env!("CARGO_MANIFEST_DIR")).parent().unwrap().to_path_buf();
    let out = Command::new("cargo")
        .current_dir(&deck)
        .args(["build", "--release", "--target", target, "-p", "deck-media-wasm", "--lib", "--target-dir"])
        .arg(&dir)
        // a thin release: the goldens compare bits, not speed, and fat LTO over rav1d takes minutes
        .args(["--config", "profile.release.lto=false", "--config", "profile.release.codegen-units=16"])
        .env_remove("RUSTFLAGS")
        .output()
        .expect("cargo runs");
    assert!(out.status.success(), "the {target} build failed:\n{}", String::from_utf8_lossy(&out.stderr));
    dir.join(target).join("release").join("deck_media_wasm.wasm")
}

struct Wasm {
    store: Store<()>,
    inst: Instance,
}

impl Wasm {
    fn new(engine: &Engine, module: &Module) -> Wasm {
        let mut linker: Linker<()> = Linker::new(engine);
        for import in module.imports() {
            if let wasmtime::ExternType::Func(ty) = import.ty() {
                linker
                    .func_new(import.module(), import.name(), ty.clone(), |_, _, results| {
                        results.iter_mut().for_each(|r| *r = Val::I32(0));
                        Ok(())
                    })
                    .unwrap();
            }
        }
        let mut store = Store::new(engine, ());
        let inst = linker.instantiate(&mut store, module).expect("instantiates");
        if let Ok(init) = inst.get_typed_func::<(), ()>(&mut store, "_initialize") {
            init.call(&mut store, ()).unwrap();
        }
        Wasm { store, inst }
    }

    /// a clip through export `f` (`md_golden`, `md_pcm`): Ok(its bytes) or Err(the error's text)
    fn call(&mut self, f: &str, clip: &[u8]) -> Result<Vec<u8>, String> {
        let s = &mut self.store;
        let inbox = self.inst.get_typed_func::<u32, u32>(&mut *s, "md_inbox").unwrap();
        let run = self.inst.get_typed_func::<u32, i32>(&mut *s, f).unwrap();
        let outbox = self.inst.get_typed_func::<(), u32>(&mut *s, "md_outbox").unwrap();
        let at = inbox.call(&mut *s, clip.len() as u32).unwrap() as usize;
        let mem = self.inst.get_memory(&mut *s, "memory").unwrap();
        mem.data_mut(&mut *s)[at..at + clip.len()].copy_from_slice(clip);
        let n = run.call(&mut *s, clip.len() as u32).unwrap();
        let ptr = outbox.call(&mut *s, ()).unwrap() as usize;
        let bytes = mem.data(&*s)[ptr..ptr + n.unsigned_abs() as usize].to_vec();
        if n >= 0 { Ok(bytes) } else { Err(String::from_utf8_lossy(&bytes).into_owned()) }
    }
}

#[test]
fn every_fixture_decodes_alike_native_and_in_wasm32() {
    let engine = Engine::default();
    let mut report = Vec::new();
    for target in ["wasm32-wasip1", "wasm32-unknown-unknown"] {
        let module = Module::from_file(&engine, build(target)).expect("the module compiles");
        let mut wasm = Wasm::new(&engine, &module);
        for (name, _, line) in common::FIXTURES {
            let clip = common::fixture(name);
            let native = deck_media::golden(&clip).unwrap().line();
            assert_eq!(native, line);
            let got = wasm.call("md_golden", &clip).map(|b| String::from_utf8(b).unwrap());
            let pictures = match got {
                Err(e) if name.starts_with("av1") && target == "wasm32-unknown-unknown" => {
                    assert!(e.contains("no software decoder for Av1"), "{target} {name}: {e}");
                    "AV1 not built".to_string()
                }
                Err(e) => panic!("{target} {name}: {e}"),
                Ok(w) => {
                    assert_eq!(w.split(' ').next(), native.split(' ').next(), "{target} {name}: the pictures native ≡ wasm");
                    "pictures bit for bit".to_string()
                }
            };
            report.push(format!("{target} {name}: {pictures}, {}", sound(&mut wasm, name, &clip)));
        }
    }
    println!("{}", report.join("\n"));
}
