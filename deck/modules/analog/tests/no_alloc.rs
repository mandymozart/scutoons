//! The audio path never allocates: after the module is built, `process` over thousands of calls – notes on and off on
//! four lanes, steals, bends, the pedal, parameters turned, every waveform and filter mode, a tempo-synced LFO – makes
//! no call to the allocator. The SDK's counting allocator is this binary's global allocator and the `Rig` is strict (as
//! the build service's tests and the creator's Play): a call that allocated fails the test (research/ABI-V2.md §6).
mod common;
use common::*;
use deck_module::alloc::Counting;

#[global_allocator]
static A: Counting = Counting;

#[test]
fn process_never_allocates() {
    let mut b = Bench::new();
    b.apply(&preset("pad"));
    b.set("noise_level", 0.2).set("noise_route", 2.0).set("o2_sync", 1.0).set("f_serial", 1.0).set("l1_sync", 4.0).set("l1_shape", 4.0);
    b.set("o2_level", 0.5).set("o1_sub", 0.5).set("o2_sub", 0.5).set("sub1_route", 2.0).set("sub2_route", 1.0);
    b.note_on(0, 220.0, 0.8);
    b.run_peak(2048, 512);

    // parameters to turn: everything but the ones that would shut the sound (master, levels, the amp envelopes)
    let desc = deck_module::host::parse(<analog_module::Analog as deck_module::Describe>::DESCRIPTOR).unwrap();
    let movable: Vec<usize> = (0..desc.params.len())
        .filter(|&i| {
            let n = desc.params[i].name.as_ref();
            !(n == "volume" || n.ends_with("_level") || n.starts_with("ae") || n == "a1_vel" || n == "a2_vel")
        })
        .collect();
    let mut seen = 0.0f32;
    for step in 0..3000usize {
        let lane = step % 4;
        if step % 5 == 0 {
            b.note_on(lane, 110.0 * (1 + step % 7) as f32, 0.3 + 0.1 * (step % 7) as f32);
        }
        if step % 5 == 2 {
            b.glide_to(lane, 100.0 * (1 + step % 5) as f32);
        }
        if step % 5 == 3 {
            b.note_off(lane);
        }
        b.sustain = if step % 97 < 12 { 1.0 } else { 0.0 };
        // every parameter moves a little and every mode is visited
        let i = movable[step % movable.len()];
        let (lo, hi) = (desc.params[i].min, desc.params[i].max);
        b.rig.set_param(i, lo + (hi - lo) * (step % 13) as f32 / 12.0);
        b.rig.set_tempo(0.0, 0.0, 60.0 + (step % 90) as f64, 4, step % 11 != 0);
        // strict: the Rig panics after a call that allocated
        seen = seen.max(b.run_peak(64 + (step % 7) * 150, 1024));
    }
    assert!(seen > 0.0, "the run was not silent: {seen}");
    assert!(b.rig.module().active_voices() <= analog_module::VOICES);
}
