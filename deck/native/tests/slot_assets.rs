//! A module node's slot assets through the whole native deck: the scene names a sound in the convolver's `ir`
//! slot, the deck's loader picks the file for its quality (deck-catalog's rule), takes it from the library cache (the
//! CDN is offline here), decodes it (deck-media) and the engine binds it before `dm_new` – the output is the input
//! through that response. A missing or undecodable sound plays the module's built-in room, with a line in the log.
#![cfg(all(feature = "modules", feature = "chapter"))]
use deck_catalog::rendition::Quality;
use deck_engine::{AssetLoader, DeckConfig, Engine, ModuleSource, Opened, Seams, Sequencing};
use deck_module_host::{AssetData, Mode, Setup};
use deck_native::assets::Assets;
use deck_native::audio::Build;
use deck_native::graph::Graph;
use deck_native::library::Library;
use deck_native::module_host::{Instance, Library as Modules};
use deck_native::modules;
use deck_native::scene::Scene;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, OnceLock};

const RATE: f64 = 48_000.0;
/// the convolver's latency (its descriptor's)
const LATENCY: usize = 128;
const FOLDER: &str = "w782test";

fn load_convolver() {
    static LOADED: OnceLock<()> = OnceLock::new();
    LOADED.get_or_init(|| {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).parent().unwrap().to_owned();
        let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).parent().unwrap().join("wasm-modules");
        let cargo = std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
        let out = std::process::Command::new(cargo)
            .current_dir(&root)
            .args(["build", "--release", "--target", "wasm32-unknown-unknown", "-p", "convolver-module", "--target-dir"])
            .arg(&dir)
            .output()
            .expect("running cargo");
        assert!(out.status.success(), "building the convolver for wasm32 failed:\n{}", String::from_utf8_lossy(&out.stderr));
        Modules::global().load(&std::fs::read(dir.join("wasm32-unknown-unknown/release/convolver_module.wasm")).unwrap()).unwrap();
    });
}

struct Unbudgeted;

impl ModuleSource for Unbudgeted {
    fn open(&self, kind: deck_space::modreg::ModuleKind, rate: f64, seed: u64, assets: Vec<AssetData>) -> anyhow::Result<Opened> {
        let loaded = Modules::global().of_kind(kind).ok_or_else(|| anyhow::anyhow!("not loaded"))?;
        let setup = assets.into_iter().fold(Setup::new(rate, seed).mode(Mode::Show).unbudgeted(), Setup::asset);
        let instance = Instance::with_setup(&loaded, setup)?.into_host();
        Ok(Opened { instance: Box::new(instance), layout: loaded.layout.clone(), params: loaded.desc.params.len() })
    }
}

/// a library cache of its own with these files under `w782test/irs/` (the CDN offline)
fn cache(name: &str, files: &[(&str, Vec<u8>)]) -> Library {
    let dir = std::env::temp_dir().join(format!("w782-slot-assets-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    for (file, bytes) in files {
        let p = dir.join("cdn").join(FOLDER).join("irs").join(file);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, bytes).unwrap();
    }
    Library { api: "http://127.0.0.1:9".into(), cdn: "http://127.0.0.1:9".into(), cache: dir }
}

/// a mono 48 kHz response: one click at `at` frames
fn click(at: usize) -> Vec<u8> {
    let mut s = vec![0i16; at + 64];
    s[at] = i16::MAX;
    wav_mono_16(48_000, &s)
}

fn scene(assets: Option<&str>) -> Scene {
    let assets = assets.map_or(String::new(), |a| format!("assets = {a}\n"));
    Scene::parse(&format!(
        "[[node]]\nid=\"o\"\nkind=\"osc\"\ngain=0.5\n[[node]]\nid=\"c\"\nkind=\"module:convolver\"\nmix=100.0\n{assets}\
         [[node]]\nid=\"out\"\nkind=\"audio_out\"\nvolume=1\n[[link]]\nfrom=\"o\"\nto=\"c.in\"\n[[link]]\nfrom=\"c.out\"\nto=\"out.in\"\n"
    ))
    .unwrap()
}

fn dry() -> Scene {
    Scene::parse("[[node]]\nid=\"o\"\nkind=\"osc\"\ngain=0.5\n[[node]]\nid=\"out\"\nkind=\"audio_out\"\nvolume=1\n[[link]]\nfrom=\"o\"\nto=\"out.in\"\n").unwrap()
}

/// one second of the scene's left channel
fn render(scene: &Scene, loader: Option<Arc<dyn AssetLoader>>) -> Vec<f32> {
    load_convolver();
    modules::check(scene).unwrap();
    let g = Graph::build(scene).unwrap();
    let mut seams = Seams::with_deck(DeckConfig::device(2));
    seams.modules = Some(Arc::new(Unbudgeted));
    seams.assets = loader;
    let (mut e, _h) = Engine::build_seams(&g, RATE as f32, 1.0, Build::detect(), Sequencing::InBlock, &seams).unwrap();
    let mut out = Vec::new();
    for _ in 0..48_000 / 512 {
        let mut buf = vec![0.0f32; 512 * 2];
        e.render(&mut buf, 2, |x| x);
        out.extend(buf.chunks(2).map(|f| f[0]));
    }
    out
}

/// the largest difference between `wet` and `dry` `lag` frames later (past the start)
fn off_by(wet: &[f32], dry: &[f32], lag: usize) -> f32 {
    (4096..wet.len()).map(|i| (wet[i] - dry[i - lag]).abs()).fold(0.0, f32::max)
}

/// the deck's loader at `q` with its log kept
struct Logged {
    loader: Arc<dyn AssetLoader>,
    lines: Arc<Mutex<Vec<String>>>,
}

impl Logged {
    fn loader(&self) -> Option<Arc<dyn AssetLoader>> {
        Some(self.loader.clone())
    }
    fn lines(&self) -> Vec<String> {
        self.lines.lock().unwrap().clone()
    }
}

fn logged(lib: Library, q: Quality) -> Logged {
    let lines = Arc::new(Mutex::new(Vec::new()));
    let keep = lines.clone();
    Logged { loader: Arc::new(Assets::at(lib, q).logging_to(move |l| keep.lock().unwrap().push(l.to_string()))), lines }
}

#[test]
fn a_wav_in_the_ir_slot_is_the_response_the_convolver_plays() {
    let lib = cache("wav", &[("click.wav", click(0))]);
    let loader = logged(lib, Quality::Medium);
    let dry = render(&dry(), None);
    let wet = render(&scene(Some(r#"{ ir = "w782test/irs/click.wav" }"#)), loader.loader());
    assert!(off_by(&wet, &dry, LATENCY) < 1e-3, "a click at 0: the input 128 frames late (off by {})", off_by(&wet, &dry, LATENCY));
    let room = render(&scene(None), loader.loader());
    assert!(off_by(&room, &dry, LATENCY) > 1e-2, "without an asset the built-in room plays");
    assert_eq!(loader.lines(), Vec::<String>::new());
}

#[test]
fn low_and_high_play_different_renditions() {
    let lib = || cache("levels", &[("r.wav", click(0)), ("r.low.wav", click(0)), ("r.high.wav", click(40))]);
    let assets = r#"{ ir = { src = "w782test/irs/r.wav", variants = { low = "w782test/irs/r.low.wav", high = "w782test/irs/r.high.wav" } } }"#;
    let dry = render(&dry(), None);
    let low = render(&scene(Some(assets)), logged(lib(), Quality::Low).loader());
    let high = render(&scene(Some(assets)), logged(lib(), Quality::High).loader());
    assert!(off_by(&low, &dry, LATENCY) < 1e-3, "low: the click at 0");
    assert!(off_by(&high, &dry, LATENCY + 40) < 2e-2 && off_by(&high, &dry, LATENCY) > 0.1, "high: the click at 40");
}

#[test]
fn offline_the_best_cached_rendition_plays_and_says_so() {
    // high is not in the cache and the CDN is offline: the default file the cache holds
    let lib = cache("offline", &[("o.wav", click(20))]);
    let loader = logged(lib, Quality::High);
    let assets = r#"{ ir = { src = "w782test/irs/o.wav", variants = { high = "w782test/irs/o.high.wav" } } }"#;
    let wet = render(&scene(Some(assets)), loader.loader());
    assert!(off_by(&wet, &render(&dry(), None), LATENCY + 20) < 2e-2, "the cached default plays");
    let lines = loader.lines();
    assert!(lines.len() == 1 && lines[0].contains("w782test/irs/o.high.wav could not be had at high – w782test/irs/o.wav plays"), "{lines:?}");
}

#[test]
fn a_missing_or_undecodable_sound_plays_the_built_in_room_with_a_log_line() {
    let lib = cache("broken", &[("broken.wav", b"RIFF not really a wave".to_vec())]);
    let loader = logged(lib, Quality::Medium);
    let room = render(&scene(None), None);
    for file in ["missing.wav", "broken.wav"] {
        let wet = render(&scene(Some(&format!(r#"{{ ir = "w782test/irs/{file}" }}"#))), loader.loader());
        assert_eq!(wet, room, "{file}: the built-in room");
    }
    let lines = loader.lines();
    assert_eq!(lines.len(), 2, "{lines:?}");
    assert!(lines.iter().all(|l| l.starts_with("node 'c': asset 'ir' (w782test/irs/") && l.ends_with("– it plays without it")), "{lines:?}");
}

#[test]
fn the_check_refuses_a_slot_the_module_lacks_and_a_file_of_the_wrong_kind() {
    load_convolver();
    for (assets, says) in [(r#"{ room = "w782test/irs/a.wav" }"#, "no such slot (it has ir)"), (r#"{ ir = "w782test/irs/a.webp" }"#, "is no sound file")] {
        let e = Graph::build(&scene(Some(assets))).unwrap_err();
        assert!(format!("{e:#}").contains(says), "{e:#}");
    }
    let osc = Scene::parse("[[node]]\nid=\"o\"\nkind=\"osc\"\nassets = { ir = \"w782test/irs/a.wav\" }\n").unwrap();
    assert!(format!("{:#}", Graph::build(&osc).unwrap_err()).contains("only a module node has asset slots"));
}

fn wav_mono_16(rate: u32, samples: &[i16]) -> Vec<u8> {
    let data = samples.len() as u32 * 2;
    let mut v = Vec::new();
    v.extend(b"RIFF");
    v.extend((36 + data).to_le_bytes());
    v.extend(b"WAVEfmt ");
    v.extend(16u32.to_le_bytes());
    v.extend(1u16.to_le_bytes());
    v.extend(1u16.to_le_bytes());
    v.extend(rate.to_le_bytes());
    v.extend((rate * 2).to_le_bytes());
    v.extend(2u16.to_le_bytes());
    v.extend(16u16.to_le_bytes());
    v.extend(b"data");
    v.extend(data.to_le_bytes());
    for s in samples {
        v.extend(s.to_le_bytes());
    }
    v
}
