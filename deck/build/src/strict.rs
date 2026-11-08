//! The strict test run (research/ABI-V2.md §6, #344): the compiled module is loaded and played for a quarter of a second
//! by the deck's own host (deck-module-host on wasmtime) in **strict mode** – an allocation inside `process` / `control`
//! traps, a call past 0.75 × the quantum is stopped, a panic or a bad access silences the instance – with noise on every
//! audio input, a note on every event input and (a second time) the module's factory preset. What the deck would count
//! in a show, a creator sees here as an error before the module is published ("the strict allocator": the v1 rule
//! "no allocation in process" without a native `no_alloc` test per module).
//!
//! This is the one place the service *runs* a stranger's code: inside wasmtime, the sandbox every deck runs modules in
//! (64 MiB of memory, no imports but the log, an epoch deadline per call). The same load check also refuses what the
//! compiler flags (`determinism::RUSTFLAGS`) keep out – relaxed SIMD, threads – should a member force them in.
use std::sync::{Arc, Mutex};

use anyhow::Result;
use deck_catalog::Channels;
use deck_module::{Ev, QUANTUM};
use deck_module_host::wasmtime_backend::{Wasmtime, WasmtimeEngine};
use deck_module_host::{GraphPort, InBind, Instance, Loaded, Mode, Setup};
use tracing::field::{Field, Visit};
use tracing_subscriber::layer::{Context, Layer, SubscriberExt};

/// blocks a pass plays: 96 × 128 frames at 48 kHz = 256 ms
const BLOCKS: u32 = 96;
const RATE: f64 = 48_000.0;
/// a note comes on in this block and off in that one
const NOTE_ON: u32 = 2;
const NOTE_OFF: u32 = 60;
/// how often a pass that ran past its time budget is played again before it counts (a loaded machine is not the module's fault)
const BUDGET_RETRIES: u32 = 2;

/// Plays `wasm` strictly. `Err` is the message a creator reads: what happened, what the module itself said, what to do.
pub fn run(engine: &WasmtimeEngine, wasm: &[u8]) -> Result<(), String> {
    let loaded = Arc::new(Loaded::<Wasmtime>::load(engine, wasm).map_err(|e| format!("the module does not load: {e:#}"))?);
    let name = loaded.desc.name.clone();
    let mut passes = vec![Pass::Defaults];
    if loaded.factory.is_some() {
        passes.push(Pass::Preset);
    }
    for pass in passes {
        let mut tries = 0;
        loop {
            match play(&loaded, pass) {
                Ok((None, _)) => break,
                Ok((Some(why), _)) if why.contains("time budget") && tries < BUDGET_RETRIES => tries += 1,
                Ok((Some(why), said)) => return Err(explain(&name, pass, &why, &said)),
                Err(e) => return Err(format!("the strict test run could not start module '{name}': {e:#}")),
            }
        }
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Pass {
    Defaults,
    Preset,
}

impl Pass {
    fn text(self) -> &'static str {
        match self {
            Pass::Defaults => "with its default parameters",
            Pass::Preset => "with its factory preset",
        }
    }
}

/// The message for a silenced instance: why the host stopped it and what the module itself logged before – the SDK's
/// allocator says "allocated inside process / control" and the panic hook where, and then both trap, which alone is only
/// an `unreachable`.
fn explain(name: &str, pass: Pass, why: &str, said: &[String]) -> String {
    let at = format!("the strict test run of module '{name}' {} stopped it", pass.text());
    if why.contains("time budget") {
        let ms = 0.75 * QUANTUM as f64 / RATE * 1000.0;
        return format!("{at}: {why} ({ms:.1} ms for {QUANTUM} frames at {} kHz) – too much work for one block", RATE / 1000.0);
    }
    // the host keeps the first line of wasmtime's error, which is its "error while executing at wasm backtrace:" heading
    let why = why.replace("error while executing at wasm backtrace:", "a trap");
    match said {
        [] => format!("{at}: {why} – a panic, an out-of-bounds access or an `unreachable`"),
        said => format!("{at}: {} ({why})", said.join(" · ")),
    }
}

/// One strict pass of `BLOCKS` blocks: `Some(message)` when the instance was silenced, with the warnings and errors the
/// module logged on the way (`log!`, a panic's place, the allocator's complaint)
fn play(loaded: &Arc<Loaded<Wasmtime>>, pass: Pass) -> Result<(Option<String>, Vec<String>)> {
    let said = Said::default();
    let subscriber = tracing_subscriber::registry().with(said.clone());
    let silenced = tracing::subscriber::with_default(subscriber, || -> Result<Option<String>> {
        let mut inst = Instance::new(loaded, Setup::new(RATE, 1).mode(Mode::Strict))?;
        drive(&mut inst, pass);
        Ok(inst.silenced().map(str::to_string))
    })?;
    // a module's lines are "module '<name>': <text>"; the host's own line about the silencing is not the module's
    let prefix = format!("module '{}': ", loaded.desc.name);
    let mut lines: Vec<String> = said.lines().iter().filter_map(|l| l.strip_prefix(&prefix).map(str::to_string)).collect();
    lines.dedup();
    lines.truncate(3);
    Ok((silenced, lines))
}

/// the warning and error events of one pass (tracing's, where the host puts a module's log lines), on this thread only
#[derive(Clone, Default)]
struct Said(Arc<Mutex<Vec<String>>>);

impl Said {
    fn lines(&self) -> Vec<String> {
        self.0.lock().map(|l| l.clone()).unwrap_or_default()
    }
}

impl<S: tracing::Subscriber> Layer<S> for Said {
    fn on_event(&self, event: &tracing::Event<'_>, _ctx: Context<'_, S>) {
        struct Message(String);
        impl Visit for Message {
            fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
                if field.name() == "message" {
                    self.0 = format!("{value:?}");
                }
            }
        }
        // tracing orders levels by verbosity: WARN and ERROR are the "smaller" ones
        if *event.metadata().level() > tracing::Level::WARN {
            return;
        }
        let mut message = Message(String::new());
        event.record(&mut message);
        if let Ok(mut lines) = self.0.lock() {
            lines.push(message.0);
        }
    }
}

/// feeds `BLOCKS` blocks to `inst`: a repeatable noise on every audio input, 0.5 on every control input, a note on and
/// off on every event input
fn drive(inst: &mut Instance<Wasmtime>, pass: Pass) {
    let loaded = inst.loaded().clone();
    if pass == Pass::Preset {
        inst.apply_factory_preset();
    }
    // the cables: each audio port carries its declared channels, at most what the layout reserves (an `any` port two)
    let shape = loaded.io.shape;
    let channels = |port: &GraphPort, reserved: u8| match port.channels {
        Some(Channels::Count(n)) => n.min(reserved),
        _ => reserved.min(2),
    };
    let mut audio_in = Vec::new();
    for (bind, port) in loaded.layout.inputs.iter().zip(&loaded.layout.in_ports) {
        match *bind {
            InBind::Audio(p) => {
                let ch = channels(port, shape.audio_in[p]);
                inst.set_channels(true, p, ch);
                audio_in.push((p, usize::from(ch)));
            }
            InBind::Control(i) => inst.set_control_in(i, 0.5),
            InBind::Param(_) => {}
        }
    }
    for (p, out) in loaded.layout.out_ports.iter().filter(|p| p.audio()).enumerate() {
        inst.set_channels(false, p, channels(out, shape.audio_out[p]));
    }
    let mut noise = 0x2545_f491u32;
    for block in 0..BLOCKS {
        for &(p, ch) in &audio_in {
            for c in 0..ch {
                for sample in inst.audio_in_mut(p, c).iter_mut() {
                    noise = noise.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
                    *sample = (noise >> 8) as f32 / (1u32 << 24) as f32 - 0.5;
                }
            }
        }
        for p in 0..loaded.layout.events_in.len() {
            let events: Vec<Ev> = match block {
                NOTE_ON => vec![Ev::note_on(10, 1, 440.0, 69.0, 0.8)],
                NOTE_OFF => vec![Ev::note_off(0, 1)],
                _ => vec![],
            };
            inst.set_events_in(p, &events);
        }
        inst.run(f64::from(block) * QUANTUM as f64, QUANTUM as u32);
        if inst.silenced().is_some() {
            break;
        }
    }
}
