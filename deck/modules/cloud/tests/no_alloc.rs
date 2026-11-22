//! The control step never allocates: thousands of steps of the golden script, every parameter turned through its range
//! on top, make no call to the allocator. The SDK's counting allocator is this binary's global allocator and the `Rig`
//! is strict: a step that allocated fails the test.
mod common;
use common::*;
use deck_module::alloc::Counting;
use deck_module::golden::Hash;

#[global_allocator]
static A: Counting = Counting;

#[test]
fn control_never_allocates() {
    let mut b = bench(3);
    let mut h = Hash::new();
    let mut most = 0;
    for k in 0..3000 {
        b.turn(k);
        most = most.max(step(&mut b, k, &mut h).len());
    }
    assert!(most >= 3, "the cloud drew");
}
