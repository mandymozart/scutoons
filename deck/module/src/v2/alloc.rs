//! The counting allocator (ABI-V2 §6, DECISIONS 2026-11-03 "Count in shows"): every allocation made while a call is
//! armed (`dm_process` / `dm_control`, from the parameter copy to the module's return) is counted into
//! `OutHdr.allocs`. With the header's `strict` flag set (the build service's tests, the creator's Play) the first one
//! traps on wasm32 – after a line to the console – so a creator sees the bug instead of a README rule; in a show it
//! is only counted. The derive installs it as a v2 module's `#[global_allocator]` on wasm32; a native test binary
//! installs it itself (`#[global_allocator] static A: Counting = Counting;`) and `testing::Rig` panics after a strict
//! call that allocated. The bookkeeping is a thread-local `Cell` (no destructor, so reading it never allocates).
use core::cell::Cell;
use std::alloc::{GlobalAlloc, Layout, System};

thread_local! {
    /// 0 = not in a call · 1 = counting · 2 = counting, strict
    static ARMED: Cell<u8> = const { Cell::new(0) };
    static COUNT: Cell<u32> = const { Cell::new(0) };
}

/// The system allocator, counting while a call is armed.
pub struct Counting;

#[inline]
fn note() {
    let armed = ARMED.try_with(Cell::get).unwrap_or(0);
    if armed == 0 {
        return;
    }
    let _ = COUNT.try_with(|c| c.set(c.get().saturating_add(1)));
    if armed == 2 {
        strict();
    }
}

#[cfg(target_arch = "wasm32")]
fn strict() {
    let _ = ARMED.try_with(|a| a.set(0));
    crate::rt::log_at(2, format_args!("allocated inside process / control (strict): allocate in init, or keep a buffer in the struct"));
    core::arch::wasm32::unreachable()
}

#[cfg(not(target_arch = "wasm32"))]
fn strict() {
    // natively an allocator must not unwind: `testing::Rig` panics after the call instead
}

// SAFETY: every call forwards to `System` unchanged; the bookkeeping touches only thread-local `Cell`s
unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        note();
        System.alloc(layout)
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        note();
        System.alloc_zeroed(layout)
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        note();
        System.realloc(ptr, layout, new_size)
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        System.dealloc(ptr, layout)
    }
}

/// a call starts: the count from 0
pub fn arm(strict: bool) {
    let _ = COUNT.try_with(|c| c.set(0));
    let _ = ARMED.try_with(|a| a.set(if strict { 2 } else { 1 }));
}

/// stop counting for a moment (the native log sink's own bookkeeping); give the result to `resume`
pub fn suspend() -> u8 {
    ARMED.try_with(|a| a.replace(0)).unwrap_or(0)
}

pub fn resume(armed: u8) {
    let _ = ARMED.try_with(|a| a.set(armed));
}

/// the call ends: what it allocated (0 when `Counting` is not the global allocator)
pub fn disarm() -> u32 {
    let _ = ARMED.try_with(|a| a.set(0));
    COUNT.try_with(Cell::get).unwrap_or(0)
}
