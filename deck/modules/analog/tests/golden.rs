//! The golden render: one script over every waveform, filter mode, the noise, hard sync, serial filters, a synced
//! sample & hold LFO, steals, bends and the pedal, hashed (FNV-1a 64 over the f32 bits of both channels). A change of
//! the hash is a change of the sound: stored presets and shows would play differently.
mod common;
use common::*;

const GOLDEN: u64 = 0x6a60_47b9_cea7_76e6;

fn fnv(h: &mut u64, x: &[f32]) {
    for v in x {
        for b in v.to_bits().to_le_bytes() {
            *h = (*h ^ u64::from(b)).wrapping_mul(0x0000_0100_0000_01b3);
        }
    }
}

#[test]
fn the_render_is_the_golden_one() {
    let mut h = 0xcbf2_9ce4_8422_2325u64;
    let mut peak = 0.0f32;
    for (k, name) in ["pad", "bass", "lead", "sync lead"].into_iter().enumerate() {
        let mut b = Bench::seeded(7 + k as u64);
        b.apply(&preset(name));
        b.set("noise_level", 0.2).set("noise_route", 2.0).set("l1_sync", 4.0).set("l1_shape", 4.0).set("o1_sub", 0.4).set("o2_sub", 0.3);
        for step in 0..160usize {
            let lane = step % 4;
            match step % 5 {
                0 => b.note_on(lane, 55.0 * (1 + step % 9) as f32, 0.3 + 0.1 * (step % 7) as f32),
                2 => b.glide_to(lane, 100.0 * (1 + step % 5) as f32),
                3 => b.note_off(lane),
                _ => {}
            }
            b.sustain = if step % 37 < 6 { 1.0 } else { 0.0 };
            b.set("o1_wave", (step / 8 % 4) as f32).set("o2_wave", (step / 12 % 4) as f32);
            b.set("f1_mode", (step / 16 % 4) as f32).set("f2_mode", (step / 20 % 4) as f32);
            b.set("f_serial", (step / 40 % 2) as f32).set("o2_sync", (step / 30 % 2) as f32);
            b.set("f1_drive", (step % 11) as f32 / 10.0);
            b.rig.set_tempo(0.0, 0.0, 90.0 + (step % 60) as f64, 4, true);
            let (l, r) = b.run_in(700 + (step % 5) * 131, 512);
            peak = l.iter().chain(&r).fold(peak, |m, v| m.max(v.abs()));
            fnv(&mut h, &l);
            fnv(&mut h, &r);
        }
    }
    assert!(peak > 0.05, "the script is not silent: {peak}");
    assert_eq!(h, GOLDEN, "the render changed: {h:#018x}");
}
