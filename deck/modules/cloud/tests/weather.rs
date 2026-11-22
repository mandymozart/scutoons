//! What the cloud's weather does, measured: the rain's drops fall from the underside to the place and start again,
//! as many as the rain asks for; a split pulls the puffs apart into parts and back to exactly the cloud it was; a new
//! size and the swell glide, never jump.
mod common;
use cloud_module::DROPS;
use common::*;
use deck_module::instances::shape;
use deck_module::Instance;

fn drops(rows: &[Instance]) -> Vec<Instance> {
    rows.iter().filter(|i| i.shape == shape::SEGMENT).copied().collect()
}

fn puffs(rows: &[Instance]) -> Vec<Instance> {
    rows.iter().filter(|i| i.shape == shape::PUFF).copied().collect()
}

/// seconds of 128-frame calls
fn seconds(b: &mut Bench<Cloud>, s: f64) {
    for _ in 0..(s * RATE / 128.0) as usize {
        b.run(128);
    }
}

#[test]
fn no_rain_no_drops_and_the_rain_is_a_share_of_the_pool() {
    let mut b = bench(1);
    seconds(&mut b, 1.0);
    assert!(drops(&b.rig.instances()).is_empty());
    b.set("rain", 1.0);
    seconds(&mut b, 3.0);
    assert_eq!(drops(&b.rig.instances()).len(), DROPS, "every drop falls");
    b.set("rain", 0.5);
    let mut counts = vec![];
    for _ in 0..40 {
        seconds(&mut b, 0.1);
        counts.push(drops(&b.rig.instances()).len());
    }
    let mean = counts.iter().sum::<usize>() as f32 / counts.len() as f32;
    assert!((DROPS as f32 * 0.3..DROPS as f32 * 0.7).contains(&mean), "about half: {mean}");
}

#[test]
fn drops_fall_from_the_underside_to_the_place_and_start_again() {
    let mut b = bench(1);
    b.set("rain", 1.0).set("height", 10.0).set("drift", 0.0);
    b.run(128);
    let rows = b.rig.instances();
    let bottom = puffs(&rows).iter().map(|p| p.position[1]).fold(f32::MAX, f32::min);
    let first = drops(&rows);
    assert_eq!(first.len(), DROPS);
    assert!(first.iter().all(|d| d.position[1] < bottom && d.position[1] > -1.0 && d.scale[1] < 1.0), "between the underside and the place");
    // one drop followed: down call by call, then back up under the cloud, again and again
    let (mut ups, mut downs, mut last) = (0, 0, first[0].position[1]);
    for _ in 0..(4.0 * RATE / 128.0) as usize {
        b.run(128);
        let y = drops(&b.rig.instances())[0].position[1];
        if y < last {
            downs += 1;
        } else if y > last + 3.0 {
            ups += 1;
        }
        last = y;
    }
    assert!(ups >= 2, "recycled: {ups} times");
    assert!(downs > 1000, "falling: {downs}");
}

/// how far apart the puffs stand, across the stage
fn spread(rows: &[Instance]) -> f32 {
    let p = puffs(rows);
    let xs = p.iter().map(|i| i.position[0]);
    let zs = p.iter().map(|i| i.position[2]);
    xs.clone().fold(f32::MIN, f32::max) - xs.fold(f32::MAX, f32::min) + zs.clone().fold(f32::MIN, f32::max) - zs.fold(f32::MAX, f32::min)
}

#[test]
fn a_split_parts_the_cloud_and_it_comes_back_together() {
    let (mut whole, mut parted) = (bench(1), bench(1));
    for b in [&mut whole, &mut parted] {
        b.set("puffs", 18.0).set("size", 6.0);
    }
    seconds(&mut whole, 0.5);
    seconds(&mut parted, 0.5);
    let before = spread(&whole.rig.instances());
    parted.set("split", 1.0);
    let mut last = spread(&parted.rig.instances());
    for _ in 0..(6.0 * RATE / 128.0) as usize {
        parted.run(128);
        whole.run(128);
        let now = spread(&parted.rig.instances());
        assert!((now - last).abs() < 0.05, "smoothly apart: {last} → {now}");
        last = now;
    }
    let apart = parted.rig.instances();
    assert!(spread(&apart) > before * 1.5, "apart: {} against {before}", spread(&apart));
    assert!(puffs(&apart).iter().zip(puffs(&whole.rig.instances())).all(|(a, w)| a.scale[2] < w.scale[2]), "smaller parts");
    parted.set("split", 0.0);
    seconds(&mut parted, 20.0);
    seconds(&mut whole, 20.0);
    assert_eq!(puffs(&parted.rig.instances()), puffs(&whole.rig.instances()), "together again, the cloud it was");
}

#[test]
fn a_new_size_glides_and_so_does_the_swell() {
    let widest = |b: &Bench<Cloud>| puffs(&b.rig.instances()).iter().map(|p| p.scale[2]).fold(0.0, f32::max);
    let mut b = bench(1);
    b.set("size", 3.0).set("drift", 0.0);
    seconds(&mut b, 0.5);
    let small = widest(&b);
    b.set("size", 12.0).set("swell", 1.0);
    let mut last = small;
    for _ in 0..(5.0 * RATE / 128.0) as usize {
        b.run(128);
        let now = widest(&b);
        assert!((now / last - 1.0).abs() < 0.02, "no jump: {last} → {now}");
        last = now;
    }
    assert!(last / small > 2.0, "it grew: {small} → {last}");
}
