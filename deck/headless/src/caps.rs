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
use deck_core::devices::{DeviceMonitor, Kind};
use deck_core::lead::{AdaptiveLead, DeviceClass};
use deck_core::meters::{self, Meters};
use deck_core::monitor::{AudioWindow, Monitor};
use deck_space::controls::Controls;
use deck_space::deck::{Deck, Slider};
use deck_space::gamepad::Pad;
use deck_space::gesture::{Act, Fingers, Touch, ZOOM_STEP};
use deck_space::mood::Ladder;
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

    ("relay_join", "#366", "the relay client as a sans-IO state machine: hello, welcome, backoff, ping, status text", "native space/link.rs, live/client.rs; live-deck client.ts; viewer live-vote-link.ts"),
    ("sync", "#371", "the conductor's scene sync: items, progress, the PUSH line", "native sync.rs push_line, live-deck sync.ts"),
    ("quality_options", "#375", "the quality page's option tables (fps caps, ratios, sizes) and labels", "native quality.rs Page, deck-quality-sheet.svelte"),
];

/// the steps the core answers now (their move tickets landed): name, ticket
pub const MOVED: &[(&str, &str)] = &[("stats", "#364"), ("lead", "#365"), ("mood", "#367"), ("rig_facts", "#368"), ("device_text", "#373"), ("meters", "#373"), ("gesture", "#376"), ("knob", "#369")];

/// Call the capability `name` with the step's input: its answer, or what is missing.
pub fn call(name: &str, input: &Value) -> Result<Value, Missing> {
    match name {
        "stats" => return Ok(stats(input)),
        "lead" => return Ok(lead(input)),
        "mood" => return Ok(mood(input)),
        "rig_facts" => return Ok(rig_facts(input)),
        "device_text" => return Ok(device_text(input)),
        "meters" => return Ok(meters(input)),
        "gesture" => return Ok(gesture(input)),
        "knob" => return Ok(knob(input)),
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

/// #367: deck-space's energy ladder (mood.rs – the walk the native being's head runs) over `moods`, fed like an adapter
/// feeds it: each of `level` (with `touch`, a number or one per level) held `step_s` seconds in frames of `frame_ms`, the
/// clock at `bpm` (`running`, default on) – the mood each frame said, the first and the last, every one seen in order,
/// the mood at the end of each level and the energy then.
fn mood(i: &Value) -> Value {
    let list = |k: &str| i.get(k).and_then(Value::as_array).cloned().unwrap_or_default();
    let moods: Vec<String> = list("moods").iter().filter_map(Value::as_str).map(String::from).collect();
    let mut d = Deck::new();
    d.ladder = Ladder::new(moods);
    let (bpm, running) = (num(i, "bpm", 0.0) as f32, i.get("running").and_then(Value::as_bool).unwrap_or(true));
    let (step_s, frame_s) = (num(i, "step_s", 1.0), num(i, "frame_ms", 1000.0 / 60.0) / 1000.0);
    let touches = list("touch");
    let mut said: Vec<String> = Vec::new();
    let mut rungs = Vec::new();
    for (k, level) in list("level").iter().filter_map(Value::as_f64).enumerate() {
        let touch = touches.get(k).and_then(Value::as_f64).unwrap_or_else(|| num(i, "touch", 0.0));
        for _ in 0..(step_s / frame_s).round() as usize {
            if let Some(m) = d.step(level as f32, bpm, running, touch as f32, frame_s as f32) {
                said.push(m.mood);
            }
        }
        rungs.push(d.mood().to_string());
    }
    let mut seen: Vec<&String> = Vec::new();
    for m in &said {
        if !seen.contains(&m) {
            seen.push(m);
        }
    }
    json!({ "first": said.first(), "last": said.last(), "said": said, "seen": seen, "rungs": rungs, "energy": crate::deck::round(f64::from(d.ladder.energy()), 4) })
}

/// #368: deck-space's deck playing a core rig (`subject`) at the mood `from`, then replayed for `to` – what a page
/// applies: the scene's sticks, streamed paths and knob targets, whether the mood's turn reloads and what it moves.
fn rig_facts(i: &Value) -> Value {
    let text = |k: &str| i.get(k).and_then(Value::as_str).unwrap_or_default().to_string();
    let mut d = Deck::new();
    d.ladder.pin(Some(&text("from")));
    let play = d.play_core(&text("subject"), "");
    let p = d.played();
    let sticks: Vec<&String> = p.sticks.iter().flatten().filter(|s| !s.is_empty()).collect();
    let streamed: Vec<&String> = p.streamed.iter().map(|(path, _)| path).collect();
    let targets: Vec<&str> = p.knobs.iter().filter_map(|k| d.target(&k.path)).collect();
    let mut out = json!({ "rig": play.rig, "failed": play.failed, "note": play.note, "sticks": sticks, "streamed": streamed, "targets": targets });
    d.ladder.pin(Some(&text("to")));
    let step = d.step(0.0, 0.0, false, 0.0, 1.0 / 60.0);
    let turn = step.as_ref().and_then(|s| s.turn.as_ref());
    let mut changed: Vec<&String> = turn.map(|t| t.values.iter().map(|(p, _)| p).collect()).unwrap_or_default();
    changed.sort();
    out["mood"] = json!(step.as_ref().map(|s| s.mood.as_str()));
    out["reload"] = json!(turn.map(|t| t.reload));
    out["changed"] = json!(changed);
    out
}

/// #373: deck-core's device-message monitor fed like an adapter feeds it – `messages` ({ at (ms), kind = midi | dmx | osc, … }
/// as protocol/schema.json's `DeviceMsg`) – then read at `read_at` ms (default: the last message's): per protocol the
/// count, the line, the activity and whether the lamp is lit.
fn device_text(i: &Value) -> Value {
    let mut m = DeviceMonitor::new();
    let mut last = 0.0;
    for msg in i.get("messages").and_then(Value::as_array).into_iter().flatten() {
        last = num(msg, "at", 0.0);
        deck_space::devices::push_json(&mut m, msg, last);
    }
    let now = num(i, "read_at", last);
    let mut out = serde_json::Map::new();
    for k in Kind::ALL {
        let activity = m.activity(k, now);
        out.insert(k.name().into(), json!({ "count": m.count(k), "text": m.text(k), "activity": crate::deck::round(activity, 4), "active": activity >= deck_core::devices::LIT }));
    }
    out.insert("rows".into(), json!(m.rows(now).iter().map(|r| r.kind.name()).collect::<Vec<_>>()));
    Value::Object(out)
}

/// #373: deck-core's meters fed like the engine feeds them – `frames` ({ dt (s), peaks }) – then what a page draws: the held
/// peaks, their places on the dB scale, their zones; the limiter's bar for `limiter_db`.
fn meters(i: &Value) -> Value {
    let list = |v: &Value| v.as_array().into_iter().flatten().filter_map(Value::as_f64).map(|x| x as f32).collect::<Vec<f32>>();
    let frames: Vec<&Value> = i.get("frames").and_then(Value::as_array).into_iter().flatten().collect();
    let n = frames.iter().map(|f| list(&f["peaks"]).len()).max().unwrap_or(0);
    let mut m = Meters::new(n);
    for f in &frames {
        m.push(&list(&f["peaks"]), num(f, "dt", 0.0) as f32);
    }
    let r4 = |x: f32| crate::deck::round(f64::from(x), 4);
    let readings: Vec<_> = (0..n).map(|k| m.reading(k)).collect();
    json!({
        "held": readings.iter().map(|r| r4(r.level)).collect::<Vec<_>>(),
        "db": m.held().iter().map(|&x| r4(meters::level_db(x))).collect::<Vec<_>>(),
        "positions": readings.iter().map(|r| r4(r.position)).collect::<Vec<_>>(),
        "zones": readings.iter().map(|r| r.zone.name()).collect::<Vec<_>>(),
        "limiter_bar": r4(meters::limiter_bar(num(i, "limiter_db", 0.0) as f32)),
    })
}

/// #376: deck-space's fingers fed like an adapter feeds them – `events` ([id, phase, x, y], phase 0 down · 1 moved · 2 up:
/// the page's pointer events and winit's touches as the stage takes them) – what the stage did: the acts in order, the
/// pinch's zoom steps and the camera's distance factor (`ZOOM_STEP` to their power), which finger drives at the end.
fn gesture(i: &Value) -> Value {
    let mut fingers = Fingers::new();
    let (mut acts, mut steps) = (Vec::new(), 0.0f32);
    for e in i.get("events").and_then(Value::as_array).into_iter().flatten().filter_map(Value::as_array) {
        let n = |k: usize| e.get(k).and_then(Value::as_f64).unwrap_or(0.0);
        match fingers.event(n(0) as u64, Touch::from_code(n(1) as u32), n(2) as f32, n(3) as f32) {
            Some(Act::Press(..)) => acts.push("press"),
            Some(Act::Drag(..)) => acts.push("drag"),
            Some(Act::Lift(..)) => acts.push("lift"),
            Some(Act::Pinch(s)) => {
                steps += s;
                acts.push("pinch");
            }
            None => {}
        }
    }
    let r4 = |x: f32| crate::deck::round(f64::from(x), 4);
    json!({ "acts": acts, "zoom_steps": r4(steps), "camera_factor": r4(ZOOM_STEP.powf(steps)), "step": r4(ZOOM_STEP), "driver": fingers.driver(), "touching": fingers.touching() })
}

/// #369: deck-space's controls over one knob (`min`, `max`, `log`) as a page hands it in – its value at `position`, that
/// value dragged by `drag_px` over `range_px` (`fine`); a gamepad's left stick out to each of `pad` (its x) – whether the
/// stick is down after each poll.
fn knob(i: &Value) -> Value {
    let k = Slider { path: "k.v".into(), min: num(i, "min", 0.0) as f32, max: num(i, "max", 1.0) as f32, value: f32::NAN, log: i["log"].as_bool().unwrap_or(false), label: String::new() };
    let mut c = Controls::new::<&str>(&[]);
    c.set_knobs(&[k], None);
    let value = c.value("k.v", num(i, "position", 0.0) as f32);
    let fine = i["fine"].as_bool().unwrap_or(false);
    let dragged = value.and_then(|v| c.turn("k.v", v, num(i, "drag_px", 0.0) as f32, num(i, "range_px", 1.0) as f32, fine));
    let mut pad = Pad::default();
    let pressed: Vec<bool> = i["pad"].as_array().into_iter().flatten().filter_map(Value::as_f64).map(|x| (pad.poll(&[x as f32], &[]), pad.is_down(0)).1).collect();
    let r4 = |x: Option<f32>| x.map(|x| crate::deck::round(f64::from(x), 4));
    json!({ "value": r4(value), "dragged": r4(dragged), "pressed": pressed })
}
