//! The golden table: the script through the cloud, every step's instance table, notes and control outputs hashed. A change
//! of the hash is a change of the picture or the storm. And what the cloud does, measured: its count, its height over the place, the drift and the wrap,
//! the seed's layout, the cut of the calls.
mod common;
use common::*;
use deck_module::instances::shape;

const GOLDEN: u64 = 0xad19_0e39_06aa_1762;

#[test]
fn the_table_is_the_golden_one() {
    let mut b = bench(1);
    let mut h = Hash::new();
    for k in 0..900 {
        assert!(!step(&mut b, k, &mut h).is_empty());
    }
    assert_eq!(h.0, GOLDEN, "the table changed: {:#018x}", h.0);
}

/// the cluster's middle across x
fn middle(rows: &[deck_module::Instance]) -> f32 {
    rows.iter().map(|i| i.position[0]).sum::<f32>() / rows.len() as f32
}

#[test]
fn puffs_float_at_their_height_and_drift_with_the_wind() {
    let mut b = bench(1);
    b.set("puffs", 12.0).set("size", 4.0).set("height", 10.0).set("drift", 1.0).set("range", 100.0);
    b.run(128);
    let first = b.rig.instances();
    assert_eq!(first.len(), 12);
    assert!(first.iter().all(|i| i.shape == shape::PUFF && i.colour[3] == 1.0));
    assert!(first.iter().all(|i| (9.0..12.0).contains(&i.position[1])), "above the place: {first:?}");
    assert!(first.iter().all(|i| i.position[0].abs() < 60.0 && i.scale.iter().all(|s| *s > 0.1)));
    // one second at 1 m/s: the cluster moved a metre along x
    for _ in 0..375 {
        b.run(128);
    }
    let later = b.rig.instances();
    let moved = middle(&later) - middle(&first);
    assert!((moved - 1.0).abs() < 0.05, "{moved}");
}

#[test]
fn it_wraps_at_the_end_of_its_range() {
    let mut b = bench(1);
    b.set("drift", 5.0).set("range", 4.0).set("size", 0.5);
    let mut xs = vec![];
    for _ in 0..400 {
        b.run(128);
        xs.push(middle(&b.rig.instances()));
    }
    assert!(xs.iter().all(|x| x.abs() < 2.5), "inside the range");
    assert!(xs.windows(2).any(|w| w[1] < w[0] - 3.0), "it came back to the start");
}

#[test]
fn the_seed_is_the_layout_and_the_cut_of_the_calls_changes_nothing() {
    let run = |seed: u64, block: usize| {
        let mut b = bench(seed);
        for _ in 0..(4096 / block) {
            b.run(block);
        }
        b.rig.instances()
    };
    assert_eq!(run(1, 128), run(1, 128));
    assert_ne!(run(1, 128), run(2, 128), "another seed, another cloud");
    let (a, c) = (run(1, 128), run(1, 64));
    let close = a.iter().zip(&c).all(|(x, y)| x.position.iter().zip(&y.position).all(|(p, q)| (p - q).abs() < 1e-4));
    assert!(close, "64-frame calls draw the cloud where 128-frame calls do");
}
