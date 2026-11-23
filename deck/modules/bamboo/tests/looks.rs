//! The stalks' look from the library: every segment of a stalk names its kind's model slot (`thin`, `thick`) and the
//! stalk as its group – so the space draws the stalk's six segments with one internode model of the set – and every
//! node ring stands in for the model (drawn only while the set is empty). The pick and the empty set's fallback are the
//! space's (deck-space space/drawn.rs); the segments, the sway and the hits are the same as without (golden.rs).
mod common;
use bamboo_module::{Bamboo, PER_STALK, SEGMENTS};
use common::*;
use deck_module::instances::{look, shape};

#[test]
fn a_stalks_segments_name_its_kinds_slot_and_its_rings_stand_in() {
    let mut b = bench(1);
    b.set("thin", 1.0).set("thick", 1.0);
    b.run(128);
    let rows = b.rig.instances();
    assert_eq!(rows.len(), 20 * PER_STALK);
    let mut groups = std::collections::BTreeSet::new();
    for (k, stalk) in rows.chunks(PER_STALK).enumerate() {
        let slot = if k < 10 { Bamboo::THIN_SLOT } else { Bamboo::THICK_SLOT };
        let segments: Vec<_> = stalk.iter().filter(|i| i.look & look::STANDS_IN == 0 && i.shape == shape::SEGMENT).collect();
        assert_eq!(segments.len(), SEGMENTS);
        let group = look::group(segments[0].look);
        assert!(group > 0 && groups.insert(group), "a group of its own");
        assert!(segments.iter().all(|i| i.look == look::of(slot.0, group)), "stalk {k}: its kind's slot, one group");
        let rings: Vec<_> = stalk.iter().filter(|i| i.look & look::STANDS_IN != 0).collect();
        assert_eq!(rings.len(), SEGMENTS - 1);
        assert!(rings.iter().all(|i| i.shape == shape::SEGMENT && look::slot(i.look) == Some(slot.0) && look::group(i.look) == 0));
        assert_eq!(stalk.iter().filter(|i| i.shape == shape::PUFF && i.look == 0).count(), 1, "the crown is its own");
    }
}

/// a stalk keeps its group while the grove changes around it: the same stalk, the same model
#[test]
fn a_stalk_keeps_its_group() {
    let mut b = bench(1);
    b.set("thin", 1.0).set("thick", 0.0);
    b.run(128);
    let first: Vec<u32> = b.rig.instances().chunks(PER_STALK).map(|s| s[0].look).collect();
    b.set("wind", 1.0).set("thick", 2.0);
    b.run(4096);
    let later: Vec<u32> = b.rig.instances().chunks(PER_STALK).take(first.len()).map(|s| s[0].look).collect();
    assert_eq!(first, later);
}
