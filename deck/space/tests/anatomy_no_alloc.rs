//! A frame's bone queries never allocate: once a model is loaded, `bone_world` (by name, in a clip, at a time), the
//! index form, the whole pose into the caller's buffer and a figure's palette run under a counting allocator with zero
//! calls.
use deck_space::space::{figure, glb};
use glam::{Mat4, Vec3};
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

/// a member's figure posed every frame – its clip looped, its palette into the renderer's buffer – allocates nothing
#[test]
fn a_figure_s_palette_per_frame_does_not_allocate() {
    let f = figure::read(&bones::glb(), &mut |_, _| anyhow::bail!("no images")).unwrap();
    let mut nodes = vec![Mat4::IDENTITY; f.anatomy.nodes.len()];
    let mut palette = [[0.0f32; 16]; figure::MAX_JOINTS];
    let mut written = 0;

    MEASURED.with(|m| m.set(true));
    for frame in 0..600 {
        let t = frame as f32 / 60.0;
        let world = figure::placed(&f, Vec3::new(frame as f32 * 0.01, 0.0, 0.0), t, 1.7, Vec3::ONE);
        written += f.pose(t, world, &mut nodes, &mut palette);
    }
    MEASURED.with(|m| m.set(false));

    assert_eq!(written, 600 * f.joints(), "the palette stays within the cap: one entry per joint");
    assert!(f.joints() <= figure::MAX_JOINTS && palette.iter().flatten().all(|x| x.is_finite()));
    assert_eq!(CALLS.load(Ordering::Relaxed), 0, "allocator calls in 600 frames of a figure's palette");
}
