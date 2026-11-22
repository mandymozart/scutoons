//! The three-crows demo's presets in `analog.preset.json` (research/audio-nodes-foundation.md §4.3, §4.4): the hoover and
//! the four drum lanes resolve cleanly against the descriptor, each renders its own golden hash, and each does what it is
//! for – the kick drops in pitch, the hats are noise, the hoover's oscillator 2 scoops up into the note. With
//! `ANALOG_WAV_DIR` set, each is also written there as a 2 s WAV.
mod common;
use analog_module::Analog;
use common::*;
use deck_module::Describe;
use deck_module_host::Preset;

const DEMO: [&str; 5] = ["hoover-dissonant", "kick", "snare", "hat-closed", "hat-open"];
const A3: f32 = 220.0;
const C4: f32 = 261.63;
/// 160 BPM: a 16th is 4500 frames at 48 kHz
const SIXTEENTH: usize = 4500;

fn fnv(x: &[f32], mut h: u64) -> u64 {
    for v in x {
        for b in v.to_bits().to_le_bytes() {
            h = (h ^ u64::from(b)).wrapping_mul(0x0000_0100_0000_01b3);
        }
    }
    h
}

/// the §4.4 groove's lane for a drum (16 steps, 0 = rest), the hoover an arp-like 8th retrigger of one held note
fn pattern(name: &str) -> [f32; 16] {
    let (x, a) = (0.7, 1.0);
    match name {
        "kick" => [a, 0., 0., 0., 0., 0., 0., 0., a, 0., x, 0., 0., 0., 0., 0.],
        "snare" => [0., 0., 0., 0., a, 0., 0., 0., 0., 0., 0., 0., a, 0., 0., 0.],
        "hat-closed" => [a, 0., x, 0., a, 0., x, 0., a, 0., x, 0., a, 0., 0., 0.],
        "hat-open" => [0., 0., 0., 0., 0., 0., 0., 0., 0., 0., 0., 0., 0., 0., x, 0.],
        _ => [a, 0., x, 0., x, 0., x, 0., a, 0., x, 0., x, 0., x, 0.],
    }
}

/// `frames` of the preset at 160 BPM: on each step of its pattern a note (A3 for the hoover, C4 for a drum – the drums
/// ignore it) held for half its length – an 8th's gate for the hoover, a 16th's for a drum
fn play(name: &str, frames: usize) -> (Vec<f32>, Vec<f32>) {
    let mut b = Bench::seeded(744);
    b.apply(&preset(name));
    b.rig.set_tempo(0.0, 0.0, 160.0, 4, true);
    let hoover = name == "hoover-dissonant";
    let (hz, gate) = if hoover { (A3, SIXTEENTH) } else { (C4, SIXTEENTH / 2) };
    let steps = pattern(name);
    let (mut l, mut r) = (Vec::with_capacity(frames), Vec::with_capacity(frames));
    let mut step = 0;
    while l.len() < frames {
        let v = steps[step % 16];
        if v > 0.0 {
            b.note_on(0, hz, v);
        }
        let n = gate.min(frames - l.len());
        let (a, c) = b.run(n);
        l.extend(a);
        r.extend(c);
        b.note_off(0);
        let n = (SIXTEENTH - gate).min(frames - l.len());
        if n > 0 {
            let (a, c) = b.run(n);
            l.extend(a);
            r.extend(c);
        }
        step += 1;
    }
    (l, r)
}

#[test]
fn the_demo_presets_resolve_cleanly() {
    let d = deck_module::host::parse(Analog::DESCRIPTOR).unwrap();
    for name in DEMO {
        let values = preset(name);
        let (resolved, report) = Preset::resolve(&d, name, values.iter().map(|(k, v)| (k.as_str(), *v)));
        assert!(report.is_clean(), "{name}: {report:?}");
        assert_eq!(resolved.name, name);
    }
}

#[test]
fn each_demo_preset_renders_its_golden_one() {
    const GOLDEN: [(&str, u64); 5] = [
        ("hoover-dissonant", 0x6ad2_4c33_cc3d_a3dc),
        ("kick", 0x1a2a_4a7e_1db9_57dd),
        ("snare", 0xa1f6_6635_bef2_f8dd),
        ("hat-closed", 0x4ae6_5057_7411_8bdd),
        ("hat-open", 0x8db9_466c_afae_6175),
    ];
    let mut got = vec![];
    for (name, _) in GOLDEN {
        let (l, r) = play(name, 96_000);
        assert!(l.iter().chain(&r).all(|v| v.is_finite()), "{name}");
        let top = peak(&l).max(peak(&r));
        assert!(top > 0.05 && top < 1.0, "{name}: peak {top}");
        got.push((name, fnv(&r, fnv(&l, 0xcbf2_9ce4_8422_2325))));
    }
    assert_eq!(got, GOLDEN, "a demo preset's render changed: {got:#018x?}");
}

#[test]
fn the_kick_drops_from_two_octaves_up_to_65_hz() {
    let mut b = Bench::new();
    b.apply(&preset("kick"));
    b.note_on(0, 440.0, 1.0);
    let (l, _) = b.run(9600);
    let body = pitch_of(&l[4800..9600]);
    assert!((body / 65.41 - 1.0).abs() < 0.03, "the body sits at C2 whatever the note: {body}");
    // the sine starts rising: its first half cycle, to the first falling zero crossing, is far shorter than the body's
    let half = (1..l.len()).find(|&i| l[i - 1] > 0.0 && l[i] <= 0.0).unwrap();
    let strike = RATE / (2 * half) as f64;
    assert!(strike > 2.0 * body, "the strike is far above the body: {strike} against {body}");
}

#[test]
fn the_hats_are_noise_led() {
    let hit = |name: &str, noise: bool, frames: usize| {
        let mut b = Bench::seeded(744);
        b.apply(&preset(name));
        if !noise {
            b.set("noise_level", 0.0);
        }
        b.note_on(0, C4, 1.0);
        b.run(frames).0
    };
    for name in ["hat-closed", "hat-open"] {
        let (full, tone) = (hit(name, true, 2400), hit(name, false, 2400));
        assert!(rms(&tone) < 0.4 * rms(&full), "{name}: the pulse alone {} against the hit {}", rms(&tone), rms(&full));
        assert!(brightness(&full) > 1.0, "{name} is a high hiss: {}", brightness(&full));
    }
    // 100–200 ms after the strike the open hat still rings, the closed one is gone
    let (open, closed) = (hit("hat-open", true, 9600), hit("hat-closed", true, 9600));
    assert!(rms(&open[4800..]) > 10.0 * rms(&closed[4800..]), "the open hat rings on: {} against {}", rms(&open[4800..]), rms(&closed[4800..]));
}

#[test]
fn the_hoover_scoops_up_a_fifth_on_oscillator_2_only() {
    // one oscillator at a time, as a sine, so its pitch can be read
    let pitch = |osc: &str| {
        let mut b = Bench::new();
        b.apply(&preset("hoover-dissonant"));
        b.set("noise_level", 0.0).set("o2_sub", 0.0).set("l2_f1", 0.0).set("l2_f2", 0.0);
        b.set("o1_wave", 3.0).set("o2_wave", 3.0);
        b.set(if osc == "o1" { "o2_level" } else { "o1_level" }, 0.0);
        b.note_on(0, A3, 1.0);
        let (l, r) = b.run(14_400);
        let x = if osc == "o1" { l } else { r };
        (pitch_of(&x[..1200]), pitch_of(&x[7200..14_400]))
    };
    let (start, held) = pitch("o2");
    let cent15 = 2f64.powf(15.0 / 1200.0);
    assert!((held / (220.0 * cent15) - 1.0).abs() < 0.01, "osc 2 settles 15 cent sharp of A3: {held}");
    assert!(start < 0.85 * held, "osc 2 starts well below the note: {start} against {held}");
    let (start, held) = pitch("o1");
    assert!((held / (220.0 / cent15) - 1.0).abs() < 0.01, "osc 1 sits 15 cent flat of A3: {held}");
    assert!((start / held - 1.0).abs() < 0.02, "osc 1 has no scoop: {start} against {held}");
}

/// `ANALOG_WAV_DIR=<dir>`: each demo preset as `<dir>/<name>.wav`, 2 s, 48 kHz, stereo 16-bit
#[test]
fn render_the_demo_presets_when_asked() {
    let Ok(dir) = std::env::var("ANALOG_WAV_DIR") else { return };
    std::fs::create_dir_all(&dir).unwrap();
    for name in DEMO {
        let (l, r) = play(name, 96_000);
        let mut data = Vec::with_capacity(l.len() * 4);
        for (a, b) in l.iter().zip(&r) {
            for v in [a, b] {
                data.extend_from_slice(&((v.clamp(-1.0, 1.0) * 32767.0).round() as i16).to_le_bytes());
            }
        }
        let mut wav = Vec::with_capacity(44 + data.len());
        wav.extend_from_slice(b"RIFF");
        wav.extend_from_slice(&(36 + data.len() as u32).to_le_bytes());
        wav.extend_from_slice(b"WAVEfmt ");
        wav.extend_from_slice(&16u32.to_le_bytes());
        wav.extend_from_slice(&1u16.to_le_bytes());
        wav.extend_from_slice(&2u16.to_le_bytes());
        wav.extend_from_slice(&48_000u32.to_le_bytes());
        wav.extend_from_slice(&(48_000u32 * 4).to_le_bytes());
        wav.extend_from_slice(&4u16.to_le_bytes());
        wav.extend_from_slice(&16u16.to_le_bytes());
        wav.extend_from_slice(b"data");
        wav.extend_from_slice(&(data.len() as u32).to_le_bytes());
        wav.extend_from_slice(&data);
        std::fs::write(std::path::Path::new(&dir).join(format!("{name}.wav")), wav).unwrap();
    }
}
