//! The wind, measured: it bends the stalks along its direction (the crowns move downwind, the sway says how far), a
//! stronger wind bends them further, the gusts make them swing; no wind, no bend and no knock.
mod common;
use common::*;
use deck_module::instances::shape;
use deck_module::Instance;

/// the crowns' mean place across the floor (x, z)
fn crowns(rows: &[Instance]) -> [f32; 2] {
    let c: Vec<_> = rows.iter().filter(|i| i.shape == shape::PUFF).collect();
    let n = c.len() as f32;
    [c.iter().map(|i| i.position[0]).sum::<f32>() / n, c.iter().map(|i| i.position[2]).sum::<f32>() / n]
}

/// the grove still for a call, then `seconds` in a steady `wind` towards `direction`: the crowns' shift, the sway
fn blown(wind: f32, direction: f32, seconds: f64) -> ([f32; 2], f32) {
    let mut b = bench(1);
    b.set("gust", 0.0).set("wind", 0.0).set("direction", direction);
    b.run(128);
    let still = crowns(&b.rig.instances());
    b.set("wind", wind);
    play(&mut b, (RATE * seconds) as usize, &[128]);
    let now = crowns(&b.rig.instances());
    ([now[0] - still[0], now[1] - still[1]], b.rig.control_out(Bamboo::SWAY).0)
}

#[test]
fn the_wind_bends_the_stalks_downwind() {
    let (x, sway) = blown(0.6, 0.0, 8.0);
    assert!(x[0] > 0.3 && x[1].abs() < 0.1 * x[0], "along x: {x:?}");
    assert!(sway > 0.04, "{sway}");
    let (z, _) = blown(0.6, 90.0, 8.0);
    assert!(z[1] > 0.3 && z[0].abs() < 0.1 * z[1], "along z: {z:?}");
    let (back, _) = blown(0.6, 180.0, 8.0);
    assert!(back[0] < -0.3, "back along x: {back:?}");
    let (strong, more) = blown(1.0, 0.0, 8.0);
    assert!(strong[0] > x[0] * 1.3 && more > sway, "a stronger wind bends further: {strong:?} {more}");
}

#[test]
fn gusts_make_the_stalks_swing() {
    let swing = |gust: f32| {
        let mut b = bench(1);
        b.set("wind", 0.6).set("gust", gust);
        play(&mut b, (RATE * 5.0) as usize, &[128]);
        let mut sways = vec![];
        for _ in 0..1500 {
            b.run(128);
            sways.push(b.rig.control_out(Bamboo::SWAY).0);
        }
        let mean = sways.iter().sum::<f32>() / sways.len() as f32;
        (sways.iter().map(|s| (s - mean) * (s - mean)).sum::<f32>() / sways.len() as f32).sqrt()
    };
    let (calm, gusty) = (swing(0.0), swing(1.0));
    assert!(gusty > 3.0 * calm, "gusts swing the grove: {calm} → {gusty}");
}

#[test]
fn no_wind_no_bend_no_knock() {
    let mut b = bench(1);
    b.set("wind", 0.0).set("gust", 1.0).set("thin", 4.0).set("thick", 4.0);
    b.run(128);
    let first = b.rig.instances();
    let (hits, last) = play(&mut b, (RATE * 20.0) as usize, &[128]);
    assert!(hits.is_empty(), "{hits:?}");
    assert_eq!(first, last, "standing still");
    assert_eq!((b.rig.control_out(Bamboo::SWAY).0, b.rig.control_out(Bamboo::BREEZE).0, b.rig.control_out(Bamboo::HIT_RATE).0), (0.0, 0.0, 0.0));
}

#[test]
fn the_breeze_follows_the_wind_and_its_gusts() {
    let mut b = bench(1);
    b.set("wind", 0.5).set("gust", 0.0);
    play(&mut b, (RATE * 2.0) as usize, &[128]);
    assert!((b.rig.control_out(Bamboo::BREEZE).0 - 0.5).abs() < 1e-3, "a steady wind: {}", b.rig.control_out(Bamboo::BREEZE).0);
    b.set("gust", 1.0);
    let mut seen = vec![];
    for _ in 0..3750 {
        b.run(128);
        seen.push(b.rig.control_out(Bamboo::BREEZE).0);
    }
    let (lo, hi) = seen.iter().fold((f32::MAX, 0.0f32), |(l, h), v| (l.min(*v), h.max(*v)));
    assert!(lo < 0.4 && hi > 0.7, "gusts come and go: {lo} … {hi}");
}
