//! The runtime the derive's exports and the native `testing::Rig` both call: one code path for wasm and native.
//! The io bytes are the derive's static buffer (wasm) or the Rig's (native); this holds the module and its asset
//! buffers, and the `log!` line to the host. Nothing in `process` / `control` allocates (tests/sdk.rs proves it with
//! the counting allocator).
use crate::alloc;
use crate::block::{Block, Outputs};
use crate::ctx::Ctx;
use crate::io::{Inp, Out};
use crate::layout::{Layout, MAX_ASSETS};
use crate::params::Dirty;
use crate::{Domain, Module};
use core::cell::UnsafeCell;
use core::fmt::{self, Write};

/// An asset slot's bytes in the module's own heap: allocated once by `dm_asset_buf` (at instantiation, never in a
/// call), 16-aligned so planar f32 PCM reads in place.
pub struct AssetBuf {
    words: Box<[u128]>,
    len: usize,
}

impl AssetBuf {
    fn new(len: usize) -> AssetBuf {
        AssetBuf { words: vec![0u128; len.div_ceil(16)].into_boxed_slice(), len }
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn bytes(&self) -> &[u8] {
        // SAFETY: `words` holds at least `len` bytes
        unsafe { core::slice::from_raw_parts(self.words.as_ptr() as *const u8, self.len) }
    }

    pub fn as_mut_ptr(&mut self) -> *mut u8 {
        self.words.as_mut_ptr() as *mut u8
    }
}

/// A module instance and its asset buffers (the io bytes live outside).
pub struct Runtime<M> {
    module: Option<M>,
    assets: [Option<AssetBuf>; MAX_ASSETS],
}

impl<M: Module> Default for Runtime<M> {
    fn default() -> Self {
        Self::new()
    }
}

impl<M: Module> Runtime<M> {
    pub const fn new() -> Runtime<M> {
        Runtime { module: None, assets: [const { None }; MAX_ASSETS] }
    }

    pub fn module(&self) -> Option<&M> {
        self.module.as_ref()
    }

    pub fn module_mut(&mut self) -> Option<&mut M> {
        self.module.as_mut()
    }

    /// `dm_new`: the module with every parameter at its default, then the table's dirty entries (the host marks all
    /// before `dm_new`), then `init`.
    ///
    /// # Safety
    /// `base` is the io (16-aligned, `M::LAYOUT.size` bytes), not touched by anyone else during the call
    pub unsafe fn create(&mut self, base: *mut u8) {
        let layout: Layout = M::LAYOUT;
        let inp = Inp::new(base, &layout);
        let mut m = M::build();
        m.load_params(inp.params(), Dirty(inp.dirty()));
        m.init(&Ctx::new(inp, &self.assets));
        self.module = Some(m);
    }

    /// `dm_asset_buf`: a buffer of `len` bytes for asset `slot`, once, at instantiation; null for a slot the module did
    /// not declare or a second request
    pub fn asset_buf(&mut self, slot: usize, len: usize) -> *mut u8 {
        if slot >= M::SHAPE.assets as usize || slot >= MAX_ASSETS || self.assets[slot].is_some() {
            return core::ptr::null_mut();
        }
        let buf = self.assets[slot].insert(AssetBuf::new(len));
        buf.as_mut_ptr()
    }

    /// `dm_process` / `dm_control`: the outputs reset, the dirty parameters copied in, then the module's call for its
    /// domain – counted by the allocator from the first step to the last.
    ///
    /// # Safety
    /// as `create`
    pub unsafe fn call(&mut self, base: *mut u8) {
        let Some(m) = self.module.as_mut() else { return };
        let layout: Layout = M::LAYOUT;
        let inp = Inp::new(base, &layout);
        let mut out = Out::new(base, &layout);
        alloc::arm(inp.hdr().strict != 0);
        out.reset(inp.frames());
        let dirty = Dirty(inp.dirty());
        if dirty.any() {
            m.load_params(inp.params(), dirty);
            m.params_changed(dirty);
        }
        let ctx = Ctx::new(inp, &self.assets);
        match M::DOMAIN {
            Domain::Audio => m.process(&mut Block::new(inp, Out::new(base, &layout)), &ctx),
            Domain::Control => m.control(&ctx, &mut Outputs::new(Out::new(base, &layout))),
        }
        out.hdr_mut().allocs = alloc::disarm();
    }
}

/// The runtime as a static of the wasm (single-threaded: the host calls one export at a time).
pub struct Slot<M>(UnsafeCell<Runtime<M>>);

// SAFETY: the host calls a module's exports one at a time (one instance, one thread)
unsafe impl<M> Sync for Slot<M> {}

impl<M: Module> Default for Slot<M> {
    fn default() -> Self {
        Self::new()
    }
}

impl<M: Module> Slot<M> {
    pub const fn new() -> Slot<M> {
        Slot(UnsafeCell::new(Runtime::new()))
    }

    /// # Safety
    /// the host calls the exports one at a time
    #[allow(clippy::mut_from_ref)]
    pub unsafe fn get(&self) -> &mut Runtime<M> {
        &mut *self.0.get()
    }
}

/// The io bytes as a static of the wasm: `N` is `LAYOUT.size`, 16-aligned.
#[repr(C, align(16))]
pub struct IoBuf<const N: usize>(UnsafeCell<[u8; N]>);

// SAFETY: as `Slot`
unsafe impl<const N: usize> Sync for IoBuf<N> {}

impl<const N: usize> IoBuf<N> {
    pub const fn new() -> IoBuf<N> {
        IoBuf(UnsafeCell::new([0; N]))
    }

    pub fn ptr(&self) -> *mut u8 {
        self.0.get() as *mut u8
    }
}

impl<const N: usize> Default for IoBuf<N> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(target_arch = "wasm32")]
mod import {
    #[link(wasm_import_module = "env")]
    extern "C" {
        /// level 0 = info, 1 = warn, 2 = error; the host rate-limits and prefixes the module's name
        pub fn deck_log(level: u32, ptr: *const u8, len: u32);
    }
}

/// a stack buffer for one log line (no allocation)
struct Line {
    buf: [u8; 240],
    len: usize,
}

impl Write for Line {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        let n = s.len().min(self.buf.len() - self.len);
        self.buf[self.len..self.len + n].copy_from_slice(&s.as_bytes()[..n]);
        self.len += n;
        Ok(())
    }
}

/// what `log!` calls; level info
pub fn log(args: fmt::Arguments) {
    log_at(0, args)
}

pub fn log_at(level: u32, args: fmt::Arguments) {
    let mut line = Line { buf: [0; 240], len: 0 };
    let _ = line.write_fmt(args);
    // cut at a char boundary
    let mut n = line.len;
    while n > 0 && core::str::from_utf8(&line.buf[..n]).is_err() {
        n -= 1;
    }
    #[cfg(target_arch = "wasm32")]
    unsafe {
        import::deck_log(level, line.buf.as_ptr(), n as u32);
    }
    #[cfg(not(target_arch = "wasm32"))]
    crate::testing::sink(level, core::str::from_utf8(&line.buf[..n]).unwrap_or(""));
}

/// the panic message to the host's console, then the trap (the allocator stops counting first, so a strict call
/// reports the panic, not the panic's own bookkeeping)
#[cfg(target_arch = "wasm32")]
pub fn install_panic_hook() {
    std::panic::set_hook(Box::new(|info| {
        crate::alloc::disarm();
        log_at(2, format_args!("panic: {info}"))
    }));
}
