//! The audio path never allocates: `process` over thousands of calls of the golden script, with every parameter turned
//! through its range on top, makes no call to the allocator. The SDK's counting allocator is this binary's global
//! allocator and the `Rig` is strict (as the build service's tests and the creator's Play): a call that allocated fails
//! the test (research/ABI-V2.md §6).
mod common;
use common::*;
use deck_module::alloc::Counting;

#[global_allocator]
static A: Counting = Counting;

#[test]
fn process_never_allocates() {
    let mut b = bench(3);
    b.rig.set_strict(true);
    let mut h = Hash::new();
    let mut notes = 0.0;
    for k in 0..3000 {
        b.turn(k);
        notes += step(&mut b, k, &mut h);
    }
    assert!(notes > 25.0, "the run was not silent: {notes} notes");
}
