//! The synth as a module: what the descriptor promises, and what the voices do with notes (cv lanes), the routing, the
//! filters, the amps, the modulation and the presets. The DSP units have their analytic tests next to them (`src/dsp`).
mod common;
use analog_module::{Analog, PARAMS_JSON, PARAM_NAMES, PRESETS_JSON};
use common::*;
use deck_catalog::{Descriptor, Dir, PortType};
use deck_module::Describe;

fn descriptor() -> Descriptor {
    deck_module::host::parse(Analog::DESCRIPTOR).expect("the descriptor is valid for the host")
}

// ── the declaration ─────────────────────────────────────────────────────────────────────────────────────────────

#[test]
fn the_descriptor_declares_the_ports_and_every_parameter() {
    let d = descriptor();
    assert_eq!(d.name, "analog");
    assert_eq!(d.params.iter().map(|p| p.name.as_ref()).collect::<Vec<_>>(), PARAM_NAMES);
    assert!(d.params.len() > 80 && d.params.len() <= deck_module::abi::MAX_PARAMS, "{} parameters", d.params.len());
    let audio_in = d.ports_of(Dir::In, PortType::Audio).map(|p| p.name.as_ref()).collect::<Vec<_>>();
    assert_eq!(audio_in, ["cv1", "cv2", "cv3", "cv4"]);
    assert_eq!(d.ports_of(Dir::In, PortType::Control).map(|p| p.name.as_ref()).collect::<Vec<_>>(), ["sustain"]);
    assert_eq!(d.ports_of(Dir::Out, PortType::Audio).map(|p| p.name.as_ref()).collect::<Vec<_>>(), ["out"]);
}

#[test]
fn every_parameter_has_a_unit_a_doc_and_choices_that_fit_its_range() {
    let d = descriptor();
    let meta: Vec<serde_json::Value> = serde_json::from_str(PARAMS_JSON).unwrap();
    assert_eq!(meta.len(), d.params.len());
    for (m, p) in meta.iter().zip(d.params.iter()) {
        assert_eq!(m["name"], p.name.as_ref());
        let doc = m["doc"].as_str().unwrap();
        assert!(doc.len() > 8, "{}: a doc string", p.name);
        assert!(["", "st", "cent", "Hz", "s", "oct"].contains(&m["unit"].as_str().unwrap()), "{}: unit {}", p.name, m["unit"]);
        let choices = m["choices"].as_str().unwrap();
        if !choices.is_empty() {
            assert_eq!(choices.split('|').count() as f32, p.max - p.min + 1.0, "{}: one choice per step", p.name);
        }
        let unit = m["unit"].as_str().unwrap();
        if unit == "s" || unit == "Hz" || unit == "st" || unit == "cent" || unit == "oct" {
            assert!(p.max > p.min);
        }
    }
}

#[test]
fn a_parameter_is_clamped_to_its_range() {
    let mut b = Bench::new();
    b.set("o1_semi", 99.0).set("f1_cutoff", -5.0);
    assert_eq!(b.rig.module.param(idx("o1_semi")), 24.0);
    assert_eq!(b.rig.module.param(idx("f1_cutoff")), 20.0);
}

// ── the factory presets ─────────────────────────────────────────────────────────────────────────────────────────

#[test]
fn four_factory_presets_use_real_parameters_inside_their_ranges() {
    let doc: serde_json::Value = serde_json::from_str(PRESETS_JSON).unwrap();
    let list = doc["presets"].as_array().unwrap();
    let names: Vec<&str> = list.iter().map(|p| p["name"].as_str().unwrap()).collect();
    assert_eq!(names, ["pad", "bass", "lead", "sync lead"]);
    let d = descriptor();
    for p in list {
        assert!(p["doc"].as_str().unwrap().len() > 20);
        for (k, v) in p["values"].as_object().unwrap() {
            let i = d.param_index(k).unwrap_or_else(|| panic!("preset '{}': no parameter '{k}'", p["name"]));
            let v = v.as_f64().unwrap() as f32;
            assert!((d.params[i].min..=d.params[i].max).contains(&v), "preset '{}': {k} = {v} is outside its range", p["name"]);
        }
    }
}

#[test]
fn each_preset_plays_a_clean_distinct_sound() {
    let mut renders = vec![];
    for name in ["pad", "bass", "lead", "sync lead"] {
        let mut b = Bench::new();
        b.apply(&preset(name));
        b.note_on(0, 110.0, 0.9);
        let (l, r) = b.run(48_000);
        assert!(l.iter().chain(&r).all(|v| v.is_finite()), "{name}");
        assert!(peak(&l).max(peak(&r)) < 1.0, "{name}: peak {}", peak(&l).max(peak(&r)));
        assert!(rms(&l[24_000..]) > 0.02, "{name} is silent: {}", rms(&l[24_000..]));
        renders.push((name, l, r));
    }
    for i in 0..renders.len() {
        for j in i + 1..renders.len() {
            let diff: Vec<f32> = renders[i].1.iter().zip(&renders[j].1).map(|(a, b)| a - b).collect();
            assert!(rms(&diff) > 0.01, "'{}' and '{}' sound alike", renders[i].0, renders[j].0);
        }
    }
    // the pad is a stereo pad: its two filters sit left and right
    let (_, l, r) = &renders[0];
    let diff: Vec<f32> = l.iter().zip(r).map(|(a, b)| a - b).collect();
    assert!(rms(&diff) > 0.2 * rms(l), "the pad is wide: side {} against {}", rms(&diff), rms(l));
}

// ── notes ───────────────────────────────────────────────────────────────────────────────────────────────────────

#[test]
fn silent_until_a_note_and_after_its_release() {
    let mut b = Bench::new();
    let (l, r) = b.run(4800);
    assert!(peak(&l) == 0.0 && peak(&r) == 0.0);
    b.set("o1_wave", 3.0);
    b.note_on(0, 440.0, 1.0);
    let (l, _) = b.run(9600);
    assert!(rms(&l[4800..]) > 0.1);
    assert_eq!(b.rig.module.active_voices(), 1);
    b.note_off(0);
    b.run((0.25 * RATE) as usize + 512);
    assert_eq!(b.rig.module.active_voices(), 0, "the voice freed itself when its amp release was done");
    let (l, r) = b.run(2400);
    assert!(peak(&l) == 0.0 && peak(&r) == 0.0);
}

#[test]
fn the_pitch_is_the_lanes_pitch_and_the_offsets_move_it() {
    let freq = |setup: &dyn Fn(&mut Bench), hz: f32| {
        let mut b = Bench::new();
        b.set("o1_wave", 3.0).set("f1_cutoff", 20000.0);
        setup(&mut b);
        b.note_on(0, hz, 1.0);
        let (l, _) = b.run(24_000);
        pitch_of(&l[4800..])
    };
    assert!((freq(&|_| {}, 440.0) - 440.0).abs() < 0.5);
    assert!((freq(&|_| {}, 110.0) - 110.0).abs() < 0.2);
    assert!((freq(&|b| { b.set("o1_semi", 12.0); }, 220.0) - 440.0).abs() < 0.5, "+12 semitones is an octave");
    assert!((freq(&|b| { b.set("o1_fine", 100.0); }, 440.0) - 440.0 * 2f64.powf(1.0 / 12.0)).abs() < 0.8, "+100 cent is a semitone");
    assert!((freq(&|b| { b.set("o1_key", 0.0); }, 880.0) - 261.63).abs() < 0.5, "no key follow holds middle C");
    assert!((freq(&|b| { b.set("o1_key", 0.5); }, 1046.5) - 523.25).abs() < 1.0, "half key follow: an octave of the keyboard is half an octave of pitch");
}

#[test]
fn the_gate_is_the_velocity() {
    let level = |vel: f32| {
        let mut b = Bench::new();
        b.set("o1_wave", 3.0).set("a1_vel", 1.0).set("f1_cutoff", 20000.0);
        b.note_on(0, 440.0, vel);
        let (l, _) = b.run(12_000);
        rms(&l[6000..])
    };
    assert!((level(0.5) / level(1.0) - 0.5).abs() < 0.02);
    assert!(level(0.25) < level(0.5));
    let mut b = Bench::new();
    b.set("o1_wave", 3.0).set("a1_vel", 0.0).set("f1_cutoff", 20000.0);
    b.note_on(0, 440.0, 0.2);
    let (l, _) = b.run(12_000);
    let full = level(1.0);
    assert!((rms(&l[6000..]) / full - 1.0).abs() < 0.01, "velocity amount 0 ignores it");
}

#[test]
fn a_changing_pitch_under_a_held_gate_glides_one_voice() {
    let mut b = Bench::new();
    b.set("o1_wave", 3.0).set("f1_cutoff", 20000.0);
    b.note_on(0, 220.0, 1.0);
    let (l, _) = b.run(9600);
    assert!((pitch_of(&l[2400..]) - 220.0).abs() < 0.5);
    b.glide_to(0, 330.0);
    let (l, _) = b.run(9600);
    assert!((pitch_of(&l[2400..]) - 330.0).abs() < 0.8);
    assert_eq!(b.rig.module.active_voices(), 1, "legato: no second voice");
}

#[test]
fn four_lanes_play_four_notes_at_once() {
    let mut b = Bench::new();
    b.set("o1_wave", 3.0).set("f1_cutoff", 20000.0).set("volume", 1.0);
    for (k, hz) in [220.0, 330.0, 440.0, 550.0].iter().enumerate() {
        b.note_on(k, *hz, 1.0);
    }
    let (l, _) = b.run(16_384 * 2);
    assert_eq!(b.rig.module.active_voices(), 4);
    for hz in [220.0, 330.0, 440.0, 550.0] {
        assert!(mag(&l[16_384..], hz) > 0.03, "{hz} Hz is missing: {}", mag(&l[16_384..], hz));
    }
}
