//! The runtime the derive's exports (and the native `testing::Rig`) call: one code path for wasm and native.
use crate::abi::{Io, MAX_AUDIO_PORTS, MAX_BLOCK};
use crate::block::{Block, Outputs};
use crate::ctx::Ctx;
use crate::Module;
use core::cell::UnsafeCell;
use core::fmt::{self, Write};

/// a new module with its first context (`Io::inp` set by the host: rate, seed, tempo …)
pub fn create<M: Module>(io: &mut Io) -> M {
    let mut m = M::build();
    m.init(&Ctx::new(&io.inp));
    m
}

fn start_call(io: &mut Io, frames: u32) {
    io.inp.frames = frames.min(MAX_BLOCK as u32);
    io.out.events_len = 0;
    io.out.events_dropped = 0;
}

/// `dm_process`: audio outputs cleared for the call's frames, then the module's block
pub fn process<M: Module>(m: &mut M, io: &mut Io, frames: u32) {
    start_call(io, frames);
    let n = io.inp.frames as usize;
    for port in 0..MAX_AUDIO_PORTS {
        io.out.audio[port][0][..n].fill(0.0);
        io.out.audio[port][1][..n].fill(0.0);
    }
    let Io { inp, out } = io;
    let mut block = Block::new(inp, out);
    m.process(&mut block, &Ctx::new(inp));
}

/// `dm_control`
pub fn control<M: Module>(m: &mut M, io: &mut Io, frames: u32) {
    start_call(io, frames);
    let Io { inp, out } = io;
    m.control(&Ctx::new(inp), &mut Outputs::new(out));
}

/// the module and its `Io`, as a static of the wasm (single-threaded: wasm32-unknown-unknown has no threads here)
pub struct Slot<M> {
    io: UnsafeCell<Io>,
    module: UnsafeCell<Option<M>>,
}

// SAFETY: the host calls a module's exports one at a time (one instance, one thread)
unsafe impl<M> Sync for Slot<M> {}

impl<M: Module> Slot<M> {
    pub const fn new() -> Slot<M> {
        Slot { io: UnsafeCell::new(Io::ZERO), module: UnsafeCell::new(None) }
    }

    pub fn io(&self) -> *mut Io {
        self.io.get()
    }

    /// # Safety
    /// the host calls the exports one at a time
    pub unsafe fn new_instance(&self) {
        let io = &mut *self.io.get();
        *self.module.get() = Some(create::<M>(io));
    }

    /// # Safety
    /// the host calls the exports one at a time
    pub unsafe fn set_param(&self, index: u32, value: f32) {
        if let Some(m) = (*self.module.get()).as_mut() {
            m.set_param(index as usize, value);
        }
    }

    /// # Safety
    /// the host calls the exports one at a time
    pub unsafe fn process(&self, frames: u32) {
        if let Some(m) = (*self.module.get()).as_mut() {
            process(m, &mut *self.io.get(), frames);
        }
    }

    /// # Safety
    /// the host calls the exports one at a time
    pub unsafe fn control(&self, frames: u32) {
        if let Some(m) = (*self.module.get()).as_mut() {
            control(m, &mut *self.io.get(), frames);
        }
    }
}

impl<M: Module> Default for Slot<M> {
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

/// the panic message to the host's console, then the trap
#[cfg(target_arch = "wasm32")]
pub fn install_panic_hook() {
    std::panic::set_hook(Box::new(|info| log_at(2, format_args!("panic: {info}"))));
}
