//! The capabilities an adapter needs beyond bytes in / out that the core does not offer yet (BRIDGE-AUDIT.md §1, §2).
//! Each lives today only in the native platform crate (main.rs and its modules) or in the browser's TypeScript – a place
//! the headless host cannot reach, because it has nothing to hide a rule behind. A scenario step calls one by name
//! (`do = "stats"`, …) and states what it must answer; until the move ticket lands, the call fails with `Missing` and the
//! scenario carries `expected_fail = "#N …"`.
//!
//! **How a move ticket closes one:** the core grows the capability (a function in deck-core / deck-space / a UI-model
//! crate, built for native and wasm); its arm here calls it and turns the answer into the JSON the scenario expects; the
//! scenario passes, the runner then fails on its `expected_fail` (an expected failure that passes) – remove the line.
//! No logic is written here: an arm that computes the answer itself is the same defect in a third place.
use deck_core::lead::{AdaptiveLead, DeviceClass};
use deck_core::monitor::{AudioWindow, Monitor};
use serde_json::{json, Value};

/// a capability the core lacks: its move ticket, what it is, where it lives today
#[derive(Debug, Clone, PartialEq)]
pub struct Missing {
    pub ticket: &'static str,
    pub what: &'static str,
    pub lives: &'static str,
}

impl std::fmt::Display for Missing {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "missing capability {}: {} – today: {}", self.ticket, self.what, self.lives)
    }
}

/// every capability the runner knows by name: (name, ticket, what, where it lives today)
pub const CAPS: &[(&str, &str, &str, &str)] = &[
    ("hud", "#374", "the deck UI model: the HUD lines and the stats bar items", "main.rs hud_lines / bar_items"),
    ("tempo_line", "#370", "the tempo line's wording (BPM, BAR, BEAT, STOP, CLK) and the clock choices as core state", "main.rs tempo_line, deck-clock.svelte"),
    ("mood", "#367", "the energy ladder: level, tempo and touch over time to the being's mood", "live-deck scene.ts Moods / LADDER (deck-space being.rs behind `being`)"),
    ("relay_join", "#366", "the relay client as a sans-IO state machine: hello, welcome, backoff, ping, status text", "native space/link.rs, live/client.rs; live-deck client.ts; viewer live-vote-link.ts"),
    ("rig_facts", "#368", "rig facts: streamed paths, knob targets, whether a mood change needs a reload", "live-deck rig.ts, engine-host controller.ts"),
    ("knob", "#369", "the controls' rules: knob turn, value and position, gamepad press and release", "live-deck controls.ts, gamepad.ts, scene.ts toValue / toPos; native pad.rs"),
    ("sync", "#371", "the conductor's scene sync: items, progress, the PUSH line", "native sync.rs push_line, live-deck sync.ts"),
    ("device_text", "#373", "a device message's text and the meter ballistics", "engine-host monitor.ts describeSignal, meters.ts; creator signal-strip.ts"),
    ("quality_options", "#375", "the quality page's option tables (fps caps, ratios, sizes) and labels", "native quality.rs Page, deck-quality-sheet.svelte"),
    ("gesture", "#376", "stage gestures: the pinch and the two-finger state machine", "live-deck space.ts bind, native main.rs space_event"),
];

/// the steps the core answers now (their move tickets landed): name, ticket
pub const MOVED: &[(&str, &str)] = &[("stats", "#364"), ("lead", "#365")];

/// Call the capability `name` with the step's input: its answer, or what is missing.
pub fn call(name: &str, input: &Value) -> Result<Value, Missing> {
    match name {
        "stats" => return Ok(stats(input)),
        "lead" => return Ok(lead(input)),
        _ => {}
    }
    match CAPS.iter().find(|c| c.0 == name) {
        Some(&(_, ticket, what, lives)) => Err(Missing { ticket, what, lives }),
        None => Err(Missing { ticket: "#363", what: "an unknown step", lives: "nowhere" }),
    }
}

pub fn known(name: &str) -> bool {
    CAPS.iter().any(|c| c.0 == name) || MOVED.iter().any(|c| c.0 == name)
}

fn num(v: &Value, k: &str, or: f64) -> f64 {
    v.get(k).and_then(Value::as_f64).unwrap_or(or)
}

/// #364: deck-core's monitor fed like an adapter feeds it – `frames` animation frames `frame_ms` apart with `work_ms`
/// each, every `drawn_every`-th drawn (the fps cap's skips), under a `target_fps` cap; underruns at `glitches_at_s`;
/// an audio window of `blocks` ([load, count] pairs), the `limiter_db`, the `lead` at `rate` – then its report.
fn stats(i: &Value) -> Value {
    let mut m = Monitor::new();
    m.target(num(i, "target_fps", 0.0) as f32);
    let (ms, frames, work, every) = (num(i, "frame_ms", 16.667), num(i, "frames", 0.0) as u64, num(i, "work_ms", 0.0) as f32, num(i, "drawn_every", 1.0).max(1.0) as u64);
    let mut t = 0.0;
    for k in 0..=frames {
        m.frame(t, work, k % every == 0);
        t += ms;
    }
    let mut totals = [0u64; 3];
    for at in i.get("glitches_at_s").and_then(Value::as_array).into_iter().flatten().filter_map(Value::as_f64) {
        totals[2] += 1;
        m.glitches(at * 1000.0, totals);
    }
    if let Some(blocks) = i.get("blocks").and_then(Value::as_array) {
        let mut w = AudioWindow::with_lead(num(i, "lead", 0.0) as u32, num(i, "rate", 48_000.0) as f32);
        for b in blocks.iter().filter_map(Value::as_array) {
            let (load, n) = (b.first().and_then(Value::as_f64).unwrap_or(0.0) as f32, b.get(1).and_then(Value::as_u64).unwrap_or(1));
            (0..n).for_each(|_| w.block(load, 1.0));
        }
        w.limiter(num(i, "limiter_db", 0.0) as f32);
        m.audio(t, w);
    }
    serde_json::to_value(m.report(t)).unwrap_or(Value::Null)
}

/// #365: deck-core's adaptive lead for a device `class`, fed a `glitches` script of [s, running total] – the lead after
/// each look
fn lead(i: &Value) -> Value {
    let class = i.get("class").and_then(Value::as_str).and_then(DeviceClass::from_name).unwrap_or(DeviceClass::Native);
    let mut l = AdaptiveLead::of(class);
    let leads: Vec<u32> = i
        .get("glitches")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_array)
        .map(|g| l.update(g.first().and_then(Value::as_f64).unwrap_or(0.0), g.get(1).and_then(Value::as_u64).unwrap_or(0)))
        .collect();
    json!({ "class": format!("{class:?}").to_lowercase(), "floor": l.policy.floor, "start": l.policy.start, "cap": l.policy.cap, "leads": leads })
}
