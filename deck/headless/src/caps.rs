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
use serde_json::Value;

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
    ("stats", "#364", "one monitor: FPS, frame time, CPU, audio load, glitches per window and their judgement from samples", "main.rs Stats / bar_items, hud.rs PerMinute; live-core index.ts, timing.ts, controls.ts FrameStats"),
    ("hud", "#374", "the deck UI model: the HUD lines and the stats bar items", "main.rs hud_lines / bar_items"),
    ("tempo_line", "#370", "the tempo line's wording (BPM, BAR, BEAT, STOP, CLK) and the clock choices as core state", "main.rs tempo_line, deck-clock.svelte"),
    ("lead", "#365", "the adaptive output lead (glitches grow it a step, a calm minute shrinks it)", "native audio/lead.rs, live-core lead.ts (different numbers)"),
    ("mood", "#367", "the energy ladder: level, tempo and touch over time to the being's mood", "live-deck scene.ts Moods / LADDER (deck-space being.rs behind `being`)"),
    ("relay_join", "#366", "the relay client as a sans-IO state machine: hello, welcome, backoff, ping, status text", "native space/link.rs, live/client.rs; live-deck client.ts; viewer live-vote-link.ts"),
    ("rig_facts", "#368", "rig facts: streamed paths, knob targets, whether a mood change needs a reload", "live-deck rig.ts, engine-host controller.ts"),
    ("knob", "#369", "the controls' rules: knob turn, value and position, gamepad press and release", "live-deck controls.ts, gamepad.ts, scene.ts toValue / toPos; native pad.rs"),
    ("sync", "#371", "the conductor's scene sync: items, progress, the PUSH line", "native sync.rs push_line, live-deck sync.ts"),
    ("device_text", "#373", "a device message's text and the meter ballistics", "engine-host monitor.ts describeSignal, meters.ts; creator signal-strip.ts"),
    ("quality_options", "#375", "the quality page's option tables (fps caps, ratios, sizes) and labels", "native quality.rs Page, deck-quality-sheet.svelte"),
    ("gesture", "#376", "stage gestures: the pinch and the two-finger state machine", "live-deck space.ts bind, native main.rs space_event"),
    ("transport", "#359", "transport as engine state: start / stop / continue / position, sequencers following it", "nowhere yet (the MIDI clock's Start / Stop only move the tempo map)"),
];

/// Call the capability `name` with the step's input: its answer, or what is missing.
pub fn call(name: &str, _input: &Value) -> Result<Value, Missing> {
    // when a move lands: `"stats" => Ok(serde_json::to_value(deck_core::stats::report(...))?)` – an adapter's call, no rule
    match CAPS.iter().find(|c| c.0 == name) {
        Some(&(_, ticket, what, lives)) => Err(Missing { ticket, what, lives }),
        None => Err(Missing { ticket: "#363", what: "an unknown step", lives: "nowhere" }),
    }
}

pub fn known(name: &str) -> bool {
    CAPS.iter().any(|c| c.0 == name)
}
