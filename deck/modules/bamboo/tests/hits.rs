//! The knocks, measured: a pair knocks once a meeting and again only after `debounce`; the key comes from the pair on the
//! scale from the root; the velocity from how fast they closed, louder in a stronger wind; the hit rate counts them.
mod common;
use bamboo_module::{FULL_HIT, SCALES};
use common::*;

fn storm(seed: u64, debounce: f32, seconds: f64) -> Vec<Hit> {
    let mut b = bench(seed);
    b.set("thin", 3.0).set("thick", 2.0).set("wind", 0.9).set("gust", 0.8).set("debounce", debounce);
    play(&mut b, (RATE * seconds) as usize, &[128]).0
}

#[test]
fn a_pair_knocks_again_only_after_the_debounce() {
    for debounce in [0.02, 0.12, 0.5] {
        let hits = storm(1, debounce, 20.0);
        assert!(hits.len() > 50, "{debounce}: a storm knocks ({})", hits.len());
        let mut pairs: Vec<u32> = hits.iter().map(|h| h.pair).collect();
        pairs.sort_unstable();
        pairs.dedup();
        assert!(pairs.len() > 5, "many pairs: {}", pairs.len());
        let mut again = 0;
        for p in pairs {
            let at: Vec<u64> = hits.iter().filter(|h| h.pair == p).map(|h| h.at).collect();
            for w in at.windows(2) {
                assert!((w[1] - w[0]) as f64 >= f64::from(debounce) * RATE - 64.0, "pair {p}: {} frames apart, debounce {debounce}", w[1] - w[0]);
                again += 1;
            }
        }
        assert!(again > 0, "pairs knock more than once");
    }
    assert!(storm(1, 1.0, 20.0).len() < storm(1, 0.02, 20.0).len(), "a longer debounce, fewer knocks");
}

#[test]
fn the_key_is_the_pair_s_on_the_scale() {
    // two octaves of the scale, the shortest pair highest; two thick stalks an octave down
    assert_eq!(Bamboo::key(60.0, 0, (10.0, false), (10.0, false)), 60.0);
    assert_eq!(Bamboo::key(60.0, 0, (2.0, false), (2.0, false)), 72.0 + 9.0);
    assert_eq!(Bamboo::key(60.0, 0, (2.0, true), (2.0, true)), 60.0 + 9.0);
    assert_eq!(Bamboo::key(48.0, 1, (6.0, false), (6.0, true)), 60.0, "the middle: degree 5 of 10, the second octave's root");
    assert_eq!(Bamboo::key(60.0, 3, (2.0, false), (2.0, false)), 72.0 + 10.0, "whole tone: six degrees an octave");
    let mut last = f32::MAX;
    for h in [2.0, 3.0, 4.5, 6.0, 8.0, 10.0] {
        let k = Bamboo::key(60.0, 0, (h, false), (h, false));
        assert!(k <= last, "taller, lower: {h} m → {k}");
        last = k;
    }
    // every knock a storm plays is on the scale
    for (scale, steps) in SCALES.iter().enumerate() {
        let mut b = bench(1);
        b.set("thin", 3.0).set("thick", 2.0).set("wind", 0.9).set("gust", 0.8).set("root", 50.0).set("scale", scale as f32);
        let hits = play(&mut b, (RATE * 10.0) as usize, &[128]).0;
        assert!(!hits.is_empty());
        for h in &hits {
            let above = h.key - 50.0 + 12.0;
            assert!((0.0..37.0).contains(&above), "{h:?}");
            assert!(steps.contains(&(above % 12.0)), "scale {scale}: {} not on it", h.key);
        }
        let keys: std::collections::BTreeSet<i32> = hits.iter().map(|h| h.key as i32).collect();
        assert!(keys.len() >= 3, "a few keys: {keys:?}");
    }
}

#[test]
fn the_velocity_is_how_fast_they_closed() {
    assert_eq!(Bamboo::velocity(FULL_HIT), 1.0);
    assert_eq!(Bamboo::velocity(FULL_HIT * 0.5), 0.5);
    assert_eq!(Bamboo::velocity(0.0), 0.05, "the softest knock still sounds");
    assert_eq!(Bamboo::velocity(10.0), 1.0);
    let mean = |wind: f32| {
        let mut b = bench(1);
        b.set("thin", 3.0).set("thick", 2.0).set("wind", wind).set("gust", 0.6);
        let hits = play(&mut b, (RATE * 20.0) as usize, &[128]).0;
        assert!(!hits.is_empty() && hits.iter().all(|h| (0.05..=1.0).contains(&h.velocity)), "{wind}");
        hits.iter().map(|h| h.velocity).sum::<f32>() / hits.len() as f32
    };
    let (soft, hard) = (mean(0.35), mean(1.0));
    assert!(hard > soft, "a stronger wind knocks harder: {soft} → {hard}");
}

#[test]
fn the_hit_rate_counts_the_knocks() {
    let mut b = bench(1);
    b.set("thin", 3.0).set("thick", 2.0).set("wind", 0.9).set("gust", 0.8);
    play(&mut b, (RATE * 5.0) as usize, &[128]);
    let mut rates = vec![];
    let mut hits = 0;
    for _ in 0..(375 * 20) {
        let (h, _) = play(&mut b, 128, &[128]);
        hits += h.len();
        rates.push(b.rig.control_out(Bamboo::HIT_RATE).0);
    }
    let mean = rates.iter().sum::<f32>() / rates.len() as f32;
    let heard = hits as f32 / 20.0;
    assert!((mean - heard).abs() < 0.25 * heard, "{mean} a second against {heard} heard");
}
