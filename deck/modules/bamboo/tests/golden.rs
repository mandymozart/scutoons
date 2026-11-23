//! The golden table: the script through the grove, every step's instance table, notes and control outputs hashed. A
//! change of the hash is a change of the picture or the knocks. And what the grove is, measured: its counts and their
//! cap, the stalk's build, the seed's layout, the cut of the calls.
mod common;
use bamboo_module::{CAPACITY, PER_STALK, SEGMENTS};
use common::*;
use deck_module::instances::shape;

const GOLDEN: u64 = 0x337f_306b_2050_530b;

#[test]
fn the_table_is_the_golden_one() {
    let mut b = bench(1);
    let mut h = Hash::new();
    let mut drew = 0;
    for k in 0..900 {
        drew += usize::from(!step(&mut b, k, &mut h).is_empty());
    }
    assert!(drew > 800);
    assert_eq!(h.0, GOLDEN, "the table changed: {:#018x}", h.0);
}

#[test]
fn the_multipliers_count_the_stalks_up_to_forty_a_kind() {
    for (thin, thick, stalks) in [(1.0, 1.0, 20), (0.0, 1.0, 10), (2.5, 0.0, 25), (4.0, 4.0, 80), (0.0, 0.0, 0)] {
        let mut b = bench(1);
        b.set("thin", thin).set("thick", thick);
        b.run(128);
        assert_eq!(b.rig.instances().len(), stalks * PER_STALK, "thin {thin} thick {thick}");
    }
    assert_eq!(80 * PER_STALK, CAPACITY, "the table holds the most");
}

#[test]
fn a_stalk_is_six_segments_with_rings_and_a_crown_from_the_floor_up() {
    let mut b = bench(1);
    b.set("thin", 1.0).set("thick", 0.0).set("thin_height", 7.0).set("thin_width", 0.06);
    b.run(128);
    let rows = b.rig.instances();
    for stalk in rows.chunks(PER_STALK) {
        let segments: Vec<_> = stalk.iter().filter(|i| i.shape == shape::SEGMENT && i.scale[1] > 0.3).collect();
        assert_eq!(segments.len(), SEGMENTS);
        assert_eq!(stalk.iter().filter(|i| i.shape == shape::PUFF).count(), 1, "one crown");
        assert_eq!(segments[0].position[1], 0.0, "on the floor");
        let tall: f32 = segments.iter().map(|s| s.scale[1]).sum();
        assert!((5.0..9.0).contains(&tall), "about 7 m: {tall}");
        assert!(segments.iter().all(|s| (0.035..0.08).contains(&s.scale[0])), "about 6 cm thick");
        assert!(segments.windows(2).all(|w| w[1].position[1] > w[0].position[1]), "stacked");
    }
}

#[test]
fn the_seed_is_the_layout_and_the_cut_of_the_calls_changes_nothing() {
    let run = |seed: u64, cut: &[usize]| {
        let mut b = bench(seed);
        b.set("wind", 0.8).set("gust", 0.8);
        play(&mut b, 48_000 * 4, cut)
    };
    let (hits, table) = run(1, &[128]);
    assert!(hits.len() > 5, "it knocks: {}", hits.len());
    assert_ne!(run(2, &[128]).1, table, "another seed, another grove");
    for cut in [&[64usize][..], &[37, 128, 5], &[1, 127]] {
        assert!(run(1, cut) == (hits.clone(), table.clone()), "cut {cut:?}: the same grove and the same knocks to the frame");
    }
}
