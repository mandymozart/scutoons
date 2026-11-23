//! STAND-INS, synthesized here – not recordings: the sounds the cloud-storm and bamboo-forest rigs' samplers name until
//! real ones are uploaded under the same CDN paths (a thunder, a kick, a wooden knock at middle C; 48 kHz mono 16-bit
//! WAV, seeded, the same bytes every run) – and the cloud's puff model, the bubble (deck-space `space::bubble`). The rig tests serve them from a library cache of their own with the CDN
//! offline, through the deck's loader. `SAMPLER_OUT_DIR=<dir>` writes them as `<dir>/<file>` (the test
//! `write_the_stand_ins_when_asked`).
#![allow(dead_code)]
use deck_catalog::rendition::Quality;
use deck_engine::AssetLoader;
use deck_native::assets::Assets;
use deck_native::library::Library;
use std::f64::consts::TAU;
use std::sync::Arc;

const RATE: f64 = 48_000.0;

/// the CDN path each rig names and its stand-in (the cloud's puffs: the owner's two blob models, deck-space's test
/// fixtures; the low-poly bubble the library still holds)
pub fn all() -> [(&'static str, Vec<u8>); 6] {
    [
        ("scutoons/sounds/cloud-thunder.wav", thunder()),
        ("scutoons/sounds/cloud-kick.wav", kick()),
        ("scutoons/sounds/bamboo-knock.wav", knock()),
        ("scutoons/3d/cloud-puff.glb", deck_space::space::bubble::glb()),
        ("scutoons/3d/cloud-three-blobs.glb", include_bytes!("../../../space/tests/fixtures/cloud-three-blobs.glb").to_vec()),
        ("scutoons/3d/cloud-four-blobs.glb", include_bytes!("../../../space/tests/fixtures/cloud-four-blobs.glb").to_vec()),
    ]
}

/// the deck's loader over a cache of its own holding the stand-ins (the CDN offline)
pub fn loader(name: &str) -> Arc<dyn AssetLoader> {
    // one folder a call: tests of a file run in parallel and each clears its folder first (#778 – they deleted each other's)
    static CALLS: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let call = CALLS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("w804-stand-ins-{name}-{}-{call}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    for (path, bytes) in all() {
        let p = dir.join("cdn").join(path);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, bytes).unwrap();
    }
    let lib = Library { api: "http://127.0.0.1:9".into(), cdn: "http://127.0.0.1:9".into(), cache: dir };
    Arc::new(Assets::at(lib, Quality::Medium).logging_to(|l| panic!("a stand-in could not be had: {l}")))
}

/// a seeded LCG's next value in −1 … 1
fn noise(state: &mut u64) -> f64 {
    *state = state.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1_442_695_040_888_963_407);
    (*state >> 11) as f64 / (1u64 << 53) as f64 * 2.0 - 1.0
}

/// a one-pole low-pass's coefficient for `hz`
fn pole(hz: f64) -> f64 {
    1.0 - (-TAU * hz / RATE).exp()
}

/// 3 s: a bright crack, then a low rumble that swells twice and rolls away
fn thunder() -> Vec<u8> {
    let n = (3.0 * RATE) as usize;
    let (mut s, mut lo, mut lo2, mut hi) = (0x7468_756e_u64, 0.0, 0.0, 0.0);
    let x: Vec<f64> = (0..n)
        .map(|i| {
            let t = i as f64 / RATE;
            let w = noise(&mut s);
            hi += pole(3000.0) * (w - hi);
            lo += pole(220.0) * (w - lo);
            lo2 += pole(90.0) * (lo - lo2);
            let crack = (w - hi) * (-t / 0.03).exp();
            let swell = 1.0 + 0.6 * (-((t - 0.6) / 0.15).powi(2)).exp() + 0.4 * (-((t - 1.3) / 0.2).powi(2)).exp();
            let rumble = lo2 * (1.0 - (-t / 0.05).exp()) * (-t / 0.9).exp() * swell;
            0.5 * crack + 6.0 * rumble
        })
        .collect();
    wav(&x)
}

/// 0.5 s: a sine falling from 150 to 48 Hz in its first 60 ms, a click on top, gone in a third of a second
fn kick() -> Vec<u8> {
    let n = (0.5 * RATE) as usize;
    let mut phase = 0.0;
    let x: Vec<f64> = (0..n)
        .map(|i| {
            let t = i as f64 / RATE;
            phase += (48.0 + 102.0 * (-t / 0.02).exp()) / RATE;
            let body = (TAU * phase).sin() * (-t / 0.12).exp();
            let click = (-t / 0.002).exp() * (TAU * 1800.0 * t).sin();
            body + 0.3 * click
        })
        .collect();
    wav(&x)
}

/// 0.35 s: a hollow wooden tok at middle C – the fundamental, an inharmonic partial at 2.76× and a breath of filtered noise
fn knock() -> Vec<u8> {
    let n = (0.35 * RATE) as usize;
    let (f, mut s, mut lo, mut band) = (261.625_565_300_598_6, 0x6b6e_6f63_u64, 0.0, 0.0);
    let x: Vec<f64> = (0..n)
        .map(|i| {
            let t = i as f64 / RATE;
            let w = noise(&mut s);
            lo += pole(2400.0) * (w - lo);
            band += pole(900.0) * (lo - band);
            let tone = (TAU * f * t).sin() * (-t / 0.07).exp() + 0.35 * (TAU * 2.76 * f * t).sin() * (-t / 0.025).exp();
            let breath = (lo - band) * (-t / 0.012).exp();
            tone + 0.8 * breath
        })
        .collect();
    wav(&x)
}

/// mono 48 kHz 16-bit, the peak at 0.9
fn wav(x: &[f64]) -> Vec<u8> {
    let peak = x.iter().fold(0.0f64, |m, v| m.max(v.abs())).max(1e-9);
    let data = x.len() as u32 * 2;
    let mut v = Vec::with_capacity(44 + data as usize);
    for part in [&b"RIFF"[..], &(36 + data).to_le_bytes(), b"WAVEfmt ", &16u32.to_le_bytes(), &1u16.to_le_bytes(), &1u16.to_le_bytes()] {
        v.extend_from_slice(part);
    }
    for part in [&48_000u32.to_le_bytes()[..], &96_000u32.to_le_bytes(), &2u16.to_le_bytes(), &16u16.to_le_bytes(), b"data", &data.to_le_bytes()] {
        v.extend_from_slice(part);
    }
    for s in x {
        v.extend_from_slice(&((s / peak * 0.9 * 32767.0).round() as i16).to_le_bytes());
    }
    v
}

/// `SAMPLER_OUT_DIR=<dir>`: the stand-ins as files, for upload under their CDN paths
pub fn write_when_asked() {
    let Ok(dir) = std::env::var("SAMPLER_OUT_DIR") else { return };
    for (path, bytes) in all() {
        let file = path.rsplit('/').next().unwrap();
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(std::path::Path::new(&dir).join(file), bytes).unwrap();
    }
}
