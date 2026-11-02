//! The audio path never allocates: after the module is built, `process` over thousands of blocks – notes on and off on all
//! four lanes, steals, the pedal, parameters turned, every waveform and filter mode, a tempo-synced LFO – makes no call
//! to the allocator (a counting allocator, as the stage's and the event layer's `no_alloc` tests).
mod common;
use common::*;
use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
use std::sync::atomic::{AtomicUsize, Ordering};

struct Counting;
static CALLS: AtomicUsize = AtomicUsize::new(0);
thread_local! { static ON: Cell<bool> = const { Cell::new(false) }; }

fn counting() -> bool {
    ON.try_with(Cell::get).unwrap_or(false)
}

unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, l: Layout) -> *mut u8 {
        if counting() {
            CALLS.fetch_add(1, Ordering::Relaxed);
        }
        System.alloc(l)
    }
    unsafe fn dealloc(&self, p: *mut u8, l: Layout) {
        if counting() {
            CALLS.fetch_add(1, Ordering::Relaxed);
        }
        System.dealloc(p, l)
    }
    unsafe fn realloc(&self, p: *mut u8, l: Layout, n: usize) -> *mut u8 {
        if counting() {
            CALLS.fetch_add(1, Ordering::Relaxed);
        }
        System.realloc(p, l, n)
    }
}

#[global_allocator]
static A: Counting = Counting;

#[test]
fn process_never_allocates() {
    let mut b = Bench::new();
    b.apply(&preset("pad"));
    b.set("noise_level", 0.2).set("noise_route", 2.0).set("o2_sync", 1.0).set("f_serial", 1.0).set("l1_sync", 4.0).set("l1_shape", 4.0);
    b.set("o2_level", 0.5).set("o1_sub", 0.5).set("o2_sub", 0.5).set("sub1_route", 2.0).set("sub2_route", 1.0);
    // warm up: the Rig's own buffers and the first blocks (page faults are not allocations, but be safe)
    b.note_on(0, 220.0, 0.8);
    b.run_peak(2048, 512);

    // parameters to turn: everything but the ones that would shut the sound (master, levels, the amp envelopes)
    let desc = deck_module::host::Descriptor::parse(<analog_module::Analog as deck_module::Describe>::DESCRIPTOR).unwrap();
    let movable: Vec<usize> = (0..desc.params.len())
        .filter(|&i| {
            let n = desc.params[i].name.as_str();
            !(n == "volume" || n.ends_with("_level") || n.starts_with("ae") || n == "a1_vel" || n == "a2_vel")
        })
        .collect();
    let mut seen = 0.0f32;
    ON.with(|c| c.set(true));
    let before = CALLS.load(Ordering::Relaxed);
    for step in 0..3000usize {
        let lane = step % 4;
        if step % 5 == 0 {
            b.note_on(lane, 110.0 * (1 + step % 7) as f32, 0.3 + 0.1 * (step % 7) as f32);
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
        seen = seen.max(b.run_peak(64 + (step % 7) * 150, 1024));
    }
    let calls = CALLS.load(Ordering::Relaxed) - before;
    ON.with(|c| c.set(false));
    assert_eq!(calls, 0, "the audio path called the allocator {calls} times");
    assert!(seen > 0.0, "the run was not silent: {seen}");
    assert!(b.rig.module.active_voices() <= analog_module::VOICES);
}
