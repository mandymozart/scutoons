//! A frame's bone queries never allocate: once a model is loaded, `bone_world` (by name, in a clip, at a time), the
//! index form and the whole pose into the caller's buffer run under a counting allocator with zero calls.
use deck_space::space::glb;
use glam::Mat4;
use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
use std::sync::atomic::{AtomicUsize, Ordering};

#[path = "fixtures/bones.rs"]
mod bones;

struct Counting;
static CALLS: AtomicUsize = AtomicUsize::new(0);

thread_local! {
    /// this thread only: the harness' main thread prints while the test measures
    static MEASURED: Cell<bool> = const { Cell::new(false) };
}

fn count(l: Layout) {
    if l.size() > 0 && MEASURED.try_with(Cell::get).unwrap_or(false) {
        CALLS.fetch_add(1, Ordering::Relaxed);
    }
}

unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, l: Layout) -> *mut u8 {
        count(l);
        System.alloc(l)
    }
    unsafe fn dealloc(&self, p: *mut u8, l: Layout) {
        count(l);
        System.dealloc(p, l)
    }
    unsafe fn realloc(&self, p: *mut u8, l: Layout, new: usize) -> *mut u8 {
        count(l);
        System.realloc(p, l, new)
    }
}

#[global_allocator]
static ALLOC: Counting = Counting;

#[test]
fn the_bone_queries_of_a_frame_do_not_allocate() {
    let anatomy = glb::anatomy(&bones::glb()).unwrap();
    let wave = anatomy.clip("wave");
    let lower = anatomy.bone("lower").unwrap();
    let mut pose = vec![Mat4::ZERO; anatomy.nodes.len()];
    let mut sum = Mat4::ZERO;

    MEASURED.with(|m| m.set(true));
    for frame in 0..600 {
        let t = frame as f32 / 60.0 - 1.0;
        sum += anatomy.bone_world("lower", Some("wave"), t).unwrap();
        sum += anatomy.bone_world("upper", None, t).unwrap();
        sum += anatomy.node_world(lower, wave, t);
        anatomy.pose(wave, t, &mut pose);
        sum += pose[lower];
    }
    MEASURED.with(|m| m.set(false));

    assert!(sum.is_finite());
    assert_eq!(CALLS.load(Ordering::Relaxed), 0, "allocator calls in 600 frames of bone queries");
}
