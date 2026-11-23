//! The audio path never allocates: `process` over thousands of calls of the golden script, with every parameter turned
//! through its range on top, makes no call to the allocator. The SDK's counting allocator is this binary's global
//! allocator and the `Rig` is strict (as the build service's tests and the creator's Play): a call that allocated fails
//! the test (research/ABI-V2.md §6).
mod common;
use common::*;
use deck_module::alloc::Counting;
use deck_module::golden::Hash;

#[global_allocator]
static A: Counting = Counting;

#[test]
fn process_never_allocates() {
    let mut b = bench_with(3, &sound(2, 20_000), 44_100.0);
    let mut h = Hash::new();
    let mut heard = 0.0f32;
    for k in 0..3000 {
        b.turn(k);
        heard = heard.max(step(&mut b, k, &mut h));
    }
    assert!(heard > 0.0, "the run was not silent");
}
