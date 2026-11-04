//! The synth as a module: what the descriptor promises, and what the voices do with notes (note events by id), the
//! routing, the filters, the amps, the modulation and the presets. The DSP units have their analytic tests next to them
//! (`src/dsp`).
mod common;
use analog_module::{Analog, PARAM_NAMES};
use common::*;
use deck_catalog::{Descriptor, Dir, PortType};
use deck_module::{Describe, Ev, Expr};

fn descriptor() -> Descriptor {
    deck_module::host::parse(Analog::DESCRIPTOR).expect("the descriptor is valid for the host")
}

// ── the declaration ─────────────────────────────────────────────────────────────────────────────────────────────

#[test]
fn the_descriptor_declares_the_ports_and_every_parameter() {
    let d = descriptor();
    assert_eq!(d.name, "analog");
    assert_eq!(d.params.iter().map(|p| p.name.as_ref()).collect::<Vec<_>>(), PARAM_NAMES);
    assert!(d.params.len() > 80, "{} parameters", d.params.len());
    assert_eq!(d.ports_of(Dir::In, PortType::Audio).count(), 0, "no cv lanes: notes are events");
    let notes = d.ports_of(Dir::In, PortType::Event).collect::<Vec<_>>();
    assert_eq!(notes.iter().map(|p| (p.name.as_ref(), p.names[0].as_ref(), p.capacity)).collect::<Vec<_>>(), [("notes", "music.note", 64)]);
    assert_eq!(d.ports_of(Dir::In, PortType::Control).map(|p| p.name.as_ref()).collect::<Vec<_>>(), ["sustain"]);
    assert_eq!(d.ports_of(Dir::Out, PortType::Audio).map(|p| (p.name.as_ref(), p.channels)).collect::<Vec<_>>(), [("out", Some(deck_catalog::Channels::Count(2)))]);
}

#[test]
fn every_parameter_has_a_unit_a_doc_and_choices_that_fit_its_range() {
    let d = descriptor();
    for p in d.params.iter() {
        assert!(p.doc.len() > 8, "{}: a doc string", p.name);
        assert!(["", "st", "cent", "Hz", "s", "oct"].contains(&p.unit.as_ref()), "{}: unit {}", p.name, p.unit);
        if !p.choices.is_empty() {
            assert_eq!(p.choices.len() as f32, p.max - p.min + 1.0, "{}: one choice per step", p.name);
            assert_eq!(p.min, 0.0, "{}: a choice is its index", p.name);
        }
        if !p.unit.is_empty() {
            assert!(p.max > p.min);
        }
    }
    assert_eq!(d.params[idx("f1_cutoff")].curve, deck_catalog::Curve::Log, "a cutoff moves in octaves");
    assert_eq!(d.params[idx("l1_shape")].choices.len(), 5);
}

#[test]
fn a_parameter_is_clamped_to_its_range() {
    let mut b = Bench::new();
    b.set("o1_semi", 99.0).set("f1_cutoff", -5.0);
    b.run(1);
    assert_eq!(b.rig.module().param(idx("o1_semi")), 24.0);
    assert_eq!(b.rig.module().param(idx("f1_cutoff")), 20.0);
}

// ── the factory presets ─────────────────────────────────────────────────────────────────────────────────────────

#[test]
fn one_factory_preset_in_the_descriptor_the_others_a_preset_file_inside_their_ranges() {
    let d = descriptor();
    let factory = d.preset.as_ref().expect("the factory preset");
    assert_eq!(factory.name, "pad");
    assert!(factory.doc.len() > 20);
    // the derive resolved it by index; applying it is the host's `Preset::apply` (defaults, then the values)
    assert_eq!(<Analog as Describe>::PRESET.len(), factory.values.len());
    let file: serde_json::Value = serde_json::from_str(PRESET_FILE).unwrap();
    assert_eq!((file["format"].as_str(), file["version"].as_u64(), file["module"].as_str()), (Some("scutoons.preset"), Some(1), Some("analog")));
    let list = file["presets"].as_array().unwrap();
    assert_eq!(list.iter().map(|p| p["name"].as_str().unwrap()).collect::<Vec<_>>(), ["bass", "lead", "sync lead"]);
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
    assert_eq!(b.rig.module().active_voices(), 1);
    b.note_off(0);
    b.run((0.25 * RATE) as usize + 512);
    assert_eq!(b.rig.module().active_voices(), 0, "the voice freed itself when its amp release was done");
    let (l, r) = b.run(2400);
    assert!(peak(&l) == 0.0 && peak(&r) == 0.0);
}

#[test]
fn the_pitch_is_the_notes_pitch_and_the_offsets_move_it() {
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
fn the_velocity_is_the_notes() {
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
fn a_bend_under_a_held_note_moves_one_voice() {
    let mut b = Bench::new();
    b.set("o1_wave", 3.0).set("f1_cutoff", 20000.0);
    b.note_on(0, 220.0, 1.0);
    let (l, _) = b.run(9600);
    assert!((pitch_of(&l[2400..]) - 220.0).abs() < 0.5);
    b.glide_to(0, 330.0);
    let (l, _) = b.run(9600);
    assert!((pitch_of(&l[2400..]) - 330.0).abs() < 0.8);
    assert_eq!(b.rig.module().active_voices(), 1, "legato: no second voice");
}

#[test]
fn four_notes_at_once() {
    let mut b = Bench::new();
    b.set("o1_wave", 3.0).set("f1_cutoff", 20000.0).set("volume", 1.0);
    for (k, hz) in [220.0, 330.0, 440.0, 550.0].iter().enumerate() {
        b.note_on(k, *hz, 1.0);
    }
    let (l, _) = b.run(16_384 * 2);
    assert_eq!(b.rig.module().active_voices(), 4);
    for hz in [220.0, 330.0, 440.0, 550.0] {
        assert!(mag(&l[16_384..], hz) > 0.03, "{hz} Hz is missing: {}", mag(&l[16_384..], hz));
    }
}

#[test]
fn notes_are_keyed_by_id_and_land_on_their_frame() {
    // two notes of the same pitch: the off of one releases that one only (never matched by pitch)
    let mut b = Bench::new();
    b.set("o1_wave", 3.0).set("f1_cutoff", 20000.0);
    assert!(b.rig.set_events_in(0, &[Ev::note_on(0, 7, 440.0, f32::NAN, 1.0), Ev::note_on(0, 8, 440.0, f32::NAN, 1.0)]));
    b.run(4800);
    assert_eq!(b.rig.module().active_voices(), 2);
    assert!(b.rig.set_events_in(0, &[Ev::note_off(0, 7)]));
    b.run((0.25 * RATE) as usize + 512);
    assert_eq!(b.rig.module().active_voices(), 1, "id 8 still sounds");
    // a note on at frame 50 of a call: silence before it, sound from it on
    let mut c = Bench::new();
    c.set("o1_wave", 3.0).set("f1_cutoff", 20000.0).set("ae1_a", 0.001);
    assert!(c.rig.set_events_in(0, &[Ev::note_on(50, 1, 440.0, f32::NAN, 1.0)]));
    let (l, _) = c.run_in(128, 128);
    assert!(peak(&l[..50]) == 0.0 && peak(&l[50..]) > 0.0, "it starts on its frame");
}

#[test]
fn a_bend_moves_the_note_from_its_own_pitch_and_a_reset_releases_all() {
    let mut b = Bench::new();
    b.set("o1_wave", 3.0).set("f1_cutoff", 20000.0).set("ae1_r", 0.01);
    assert!(b.rig.set_events_in(0, &[Ev::note_on(0, 3, 220.0, f32::NAN, 1.0)]));
    b.run(9600);
    assert!(b.rig.set_events_in(0, &[Ev::note_expr(0, 3, Expr::Bend, 12.0)]));
    let (l, _) = b.run(9600);
    assert!((pitch_of(&l[2400..]) - 440.0).abs() < 0.8, "a 12-semitone bend is an octave: {}", pitch_of(&l[2400..]));
    assert!(b.rig.set_events_in(0, &[Ev::note_on(0, 4, 330.0, f32::NAN, 1.0), Ev::reset(0)]));
    b.run((0.25 * RATE) as usize + 512);
    assert_eq!(b.rig.module().active_voices(), 0, "the reset released both");
}
