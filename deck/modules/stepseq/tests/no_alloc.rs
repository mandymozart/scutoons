//! The audio path never allocates: `process` over thousands of calls with the sliders and the tempo turned makes no call to
//! the allocator. The SDK's counting allocator is this binary's global allocator and the `Rig` is strict (as the build
//! service's tests and the creator's Play): a call that allocated fails the test (research/ABI-V2.md §6).
use deck_module::alloc::Counting;
use deck_module::host;
use deck_module::testing::Rig;
use deck_module::Describe;
use stepseq_module::StepSeq;

#[global_allocator]
static A: Counting = Counting;

#[test]
fn process_never_allocates() {
    let d = host::parse(StepSeq::DESCRIPTOR).unwrap();
    let mut rig = Rig::<StepSeq>::new(48_000.0, 1);
    rig.set_strict(true);
    let (mut notes, mut start) = (0, 0usize);
    for step in 0..3000usize {
        let i = step % d.params.len();
        let (lo, hi) = (d.params[i].min, d.params[i].max);
        rig.set_param(i, lo + (hi - lo) * (step % 13) as f32 / 12.0);
        rig.set_tempo(0.0, 0.0, 60.0 + (step % 90) as f64, 4, step % 11 != 0);
        let n = 64 + (step % 7) * 150;
        rig.run(start as f64, n as u32);
        notes += rig.events_out(StepSeq::NOTES).len();
        start += n;
    }
    assert!(notes > 20, "the run was not silent: {notes} notes");
}
