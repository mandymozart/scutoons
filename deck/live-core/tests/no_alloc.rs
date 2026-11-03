//! live-core's hot path never allocates (plan 3.3, P6 #320): what the worker calls per block – `apply` (wire records:
//! sends, blends, bases, releases), `render` (the engine's quantum: the control half, the walk and events hosts in the
//! block, the voices, the noise, the mixer, the fft, the limiter, the device channels), `analyse` (the bands, the
//! spectrum, the heard block's values from the snapshot ring, the meters) and `take_signals` (the scene's `[outputs]`) –
//! zero allocations and frees over 20 seconds after the warm-up, on a stereo and an 8-channel device. A GC pause or an
//! allocator lock in the worker is a dropout; the native deck's engine has the same proof (deck-engine tests/no_alloc.rs).
use live_core::wire::{encode_send, Mode};
use live_core::Core;
use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
use std::sync::atomic::{AtomicUsize, Ordering};

struct Counting;
static CALLS: AtomicUsize = AtomicUsize::new(0);

thread_local! {
    /// counting on this thread only (the harness prints on its own while a test measures)
    static MEASURED: Cell<bool> = const { Cell::new(false) };
}

fn counting() -> bool {
    MEASURED.try_with(Cell::get).unwrap_or(false)
}

fn measure(on: bool) {
    MEASURED.with(|m| m.set(on));
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

/// every kind the browser carries in one scene: a walk and an arp into two voices, an LFO and the helpers on a noise's
/// cutoff, a saw, the tempo's pulse, a click, a seq, a bus with a send, a mixer, the fft, the being, the main outlet and
/// a cue outlet, a scene `[outputs]` map
const SCENE: &str = r#"
[[node]]
id = "t"
kind = "tempo"
bpm = 96.0
click = true
[[node]]
id = "melody"
kind = "walk"
seed = 7
spread = 0.8
rate = 4.0
[[node]]
id = "voice"
kind = "fm"
ratio = 1.5
index = 2.0
gain = 0.15
[[node]]
id = "arp"
kind = "events"
out = "arp.notes"
knobs = ["arp.rate"]
generator = [{ id = "walk", kind = "random_walk", seed = 5 }, { id = "arp", kind = "arp", ratios = [1.0, 1.5], rate = 4.0 }]
edge = [{ from = "walk.notes", to = "arp.notes" }]
[[node]]
id = "lead"
kind = "osc"
wave = "saw"
gain = 0.08
[[node]]
id = "s"
kind = "seq"
steps = [57, 0, 60, 64]
[[node]]
id = "bass"
kind = "osc"
wave = "square"
gain = 0.05
[[node]]
id = "sway"
kind = "lfo"
shape = "saw"
rate = 0.5
depth = 1.0
[[node]]
id = "m"
kind = "mul"
b = 2000.0
[[node]]
id = "a"
kind = "add"
b = 300.0
[[node]]
id = "sm"
kind = "smooth"
time = 0.05
[[node]]
id = "hiss"
kind = "noise"
color = "pink"
gain = 0.05
[[node]]
id = "k"
kind = "click"
gain = 0.2
[[node]]
id = "room"
kind = "bus"
[[node]]
id = "mix"
kind = "mixer"
[[node]]
id = "fft"
kind = "fft"
[[node]]
id = "space"
kind = "space"
[[node]]
id = "out"
kind = "audio_out"
[[node]]
id = "cue"
kind = "out"
group = "cue"
[[link]]
from = "melody.cv"
to = "voice.cv"
[[link]]
from = "arp.cv"
to = "lead.cv"
[[link]]
from = "s.cv"
to = "bass.cv"
[[link]]
from = "sway"
to = "m.a"
[[link]]
from = "m"
to = "a.a"
[[link]]
from = "a"
to = "sm.in"
[[link]]
from = "sm"
to = "hiss.cutoff"
[[link]]
from = "t.pulse"
to = "voice.index"
scale = 2.0
offset = 1.0
[[link]]
from = "voice"
to = "room.in"
[[link]]
from = "lead"
to = "room.in"
[[link]]
from = "room"
to = "mix.in1"
[[link]]
from = "room.pre"
to = "cue.in"
scale = 0.5
[[link]]
from = "bass"
to = "mix.in2"
[[link]]
from = "hiss"
to = "mix.in3"
[[link]]
from = "k"
to = "mix.in4"
[[link]]
from = "mix"
to = "fft.in"
[[link]]
from = "mix"
to = "out.in"
[[link]]
from = "fft.level"
to = "space.pulse"
[outputs]
midi = "x"
[[outputs.map]]
from = "fft.level"
to = [{ cc = 74, channel = 1 }]
"#;

#[test]
fn the_worker_path_never_allocates() {
    for channels in [2, 8] {
        let mut c = Core::load(SCENE, 48_000.0, channels).unwrap_or_else(|e| panic!("{e}"));
        let mut records = Vec::new();
        encode_send(&mut records, Mode::Base, 1, 0, 0.0, &[0.6]);
        encode_send(&mut records, Mode::Blend, 2, 2, 0.5, &[3.0, 2.5]);
        encode_send(&mut records, Mode::Force, 3, 1, 1.0, &[f32::NAN]);
        encode_send(&mut records, Mode::Release, 2, 4, 0.0, &[0.0]);
        let block = |c: &mut Core, i: usize| {
            if i % 20 == 0 {
                c.apply(&records).unwrap();
            }
            c.render(128);
            if i % 6 == 0 {
                c.analyse(0.016, 2048, &deck_core::tempo::Now::default());
                c.take_signals();
            }
        };
        for i in 0..400 {
            block(&mut c, i);
        }
        CALLS.store(0, Ordering::Relaxed);
        measure(true);
        for i in 0..7500 {
            block(&mut c, i);
        }
        measure(false);
        let calls = CALLS.load(Ordering::Relaxed);
        assert_eq!(calls, 0, "{channels} channels: {calls} allocations or frees over 20 s");
        let peak = c.render(128).iter().fold(0.0f32, |m, x| m.max(x.abs()));
        assert!(peak > 0.01, "the scene sounds");
    }
}
