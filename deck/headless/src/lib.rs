//! deck-headless (#363) – the headless deck host: the third adapter of the universal core, with no audio device, no
//! window, no DOM and no socket of its own. It drives the core through the adapter API the browser's worker uses
//! (live-core's `Host`, the Rust side of the `lc_*` exports) and deck-space's governor and rig player, on a virtual clock
//! with a virtual audio sink (deck.rs), from a scripted scenario (scenarios/*.toml), and asserts what comes out.
//!
//! The point (owner 2026-11-03): anything an adapter needs beyond bytes in / out shows up here as a **missing
//! capability** (caps.rs) – the headless host has nothing to hide a rule behind. Such a scenario is written anyway and
//! marked `expected_fail = "#N …"`; when the move ticket lands, it turns green and the runner fails until the line goes.
//!
//! A scenario (TOML): `name`, `about`, optional `rate` / `channels` / `expected_fail`, and `[[step]]`s – each a `do` with
//! its arguments and an optional `expect` table checked against the step's answer (see `expect`). The steps:
//!
//! | do | arguments | answer |
//! |---|---|---|
//! | `load` | `scene` (a deck/ path; `lower`: the browser's split, default on), `toml` (a small scene as it is) or `rig` (`character.catpope`) + `mood` | nodes, page (ids the engine does not carry), channels |
//! | `warm` | as `load`, `quanta` | – |
//! | `run` | `ms`; `mark` (keep the audio hash) | audio (hash), peak, frames, the clock state, line, clock_out, anchors, pings, signals |
//! | `send` | `path`, `value`, `mode` (force / blend / base), `amount` | applied |
//! | `heard` | `path` | value (the analysis frame's) |
//! | `choose` | `source`, `lead` | the clock state |
//! | `key` | `key` (tap, faster, slower, startstop, click) | – |
//! | `clock_out` · `joined` · `midi_in` · `midi` · `leader` · `anchor` | the page's and the devices' input | – |
//! | `quality` / `frames` | `level`, `auto` / `ms`, `secs`, `glitches` | level, ceiling, scale, auto, changes |
//! | `reset` | – | a fresh host (the clock too) |
//! | a capability (caps.rs) | `with` | its answer, or `Missing` |
//!
//! Built for wasm32 too (`hl_inbox`, `hl_run`, `hl_outbox`): the same bundle in, the same report out – tests/wasm.rs
//! proves native ≡ wasm32 ≡ wasm32 + simd128, so the browser's bridge adds nothing a scenario can see.
pub mod caps;
pub mod deck;

use deck::{round, Deck};
use deck_space::quality::{Governor, Level};
use deck_space::rig::Bundle as Rig;
use deck_space::scene::Scene;
use live_core::wire::Mode;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::cell::RefCell;
use std::collections::BTreeMap;

/// what a run gets: the scenario's TOML and the deck/ files it names (the host reads no file itself)
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Bundle {
    pub name: String,
    pub scenario: String,
    #[serde(default)]
    pub files: BTreeMap<String, String>,
}

/// how a scenario ended
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    /// every step and expectation held
    Pass,
    /// an expected failure: a missing capability its `expected_fail` names
    Xfail,
    /// a failure – or an expected failure that passed, or failed for another reason
    Fail,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Report {
    pub name: String,
    pub status: Status,
    pub reason: String,
    pub steps: Vec<Value>,
}

/// the paths of deck/ files a scenario's steps name (`scene = …`) – the CLI and the tests bundle them
pub fn files_of(scenario: &str) -> Vec<String> {
    let Ok(t) = scenario.parse::<toml::Table>() else { return Vec::new() };
    let steps = t.get("step").and_then(|s| s.as_array()).cloned().unwrap_or_default();
    let mut out: Vec<String> = steps.iter().filter_map(|s| s.get("scene").and_then(|v| v.as_str()).map(String::from)).collect();
    out.sort();
    out.dedup();
    out
}

/// A scenario's bundle: its TOML and every deck/ file it names, read by `read` (the CLI's and the tests' file access – the
/// host itself reads nothing)
pub fn bundle(name: &str, scenario: String, read: impl Fn(&str) -> Result<String, String>) -> Result<Bundle, String> {
    let mut files = BTreeMap::new();
    for p in files_of(&scenario) {
        files.insert(p.clone(), read(&p).map_err(|e| format!("{p}: {e}"))?);
    }
    Ok(Bundle { name: name.to_string(), scenario, files })
}

/// A failure of a step: an error, a missing capability, an expectation that did not hold
enum Fault {
    Error(String),
    Missing(caps::Missing),
    Expect(String),
}

struct Run<'a> {
    bundle: &'a Bundle,
    deck: Deck,
    governor: Option<Governor>,
    gov_t: f64,
    marks: BTreeMap<String, String>,
}

/// Run one bundled scenario → its report (the same on every target)
pub fn run(bundle: &Bundle) -> Report {
    run_paced(bundle, None)
}

/// `run` with a pace: called with the ms of each virtual clock frame (a real-time CLI run sleeps in it)
pub fn run_paced(bundle: &Bundle, pace: Option<Box<dyn FnMut(f64)>>) -> Report {
    let mut report = Report { name: bundle.name.clone(), status: Status::Pass, reason: String::new(), steps: Vec::new() };
    let t: toml::Table = match bundle.scenario.parse() {
        Ok(t) => t,
        Err(e) => {
            report.status = Status::Fail;
            report.reason = format!("the scenario is no TOML: {e}");
            return report;
        }
    };
    if let Some(n) = t.get("name").and_then(|v| v.as_str()) {
        report.name = n.to_string();
    }
    let rate = t.get("rate").and_then(|v| v.as_integer()).unwrap_or(48_000) as f32;
    let channels = t.get("channels").and_then(|v| v.as_integer()).unwrap_or(2) as usize;
    let expected = t.get("expected_fail").and_then(|v| v.as_str()).map(String::from);
    let mut r = Run { bundle, deck: Deck::new(rate, channels), governor: None, gov_t: 0.0, marks: BTreeMap::new() };
    r.deck.pace = pace;
    let steps = t.get("step").and_then(|s| s.as_array()).cloned().unwrap_or_default();
    let mut fault = None;
    for (i, s) in steps.iter().enumerate() {
        let Some(step) = s.as_table() else { continue };
        let what = step.get("do").and_then(|v| v.as_str()).unwrap_or("?").to_string();
        let answer = r.step(&what, step);
        let expect = step.get("expect").map(to_json).unwrap_or(Value::Null);
        let checked = match answer {
            Ok(out) => {
                let problems = check(&out, &expect, &r.marks);
                if let Some(m) = step.get("mark").and_then(|v| v.as_str()) {
                    if let Some(h) = out.get("audio").and_then(|v| v.as_str()) {
                        r.marks.insert(m.to_string(), h.to_string());
                    }
                }
                report.steps.push(json!({ "do": what, "out": out }));
                if problems.is_empty() {
                    Ok(())
                } else {
                    Err(Fault::Expect(problems.join("; ")))
                }
            }
            Err(Fault::Error(e)) if expect.get("error").is_some() => {
                let problems = check(&json!({ "error": e }), &json!({ "error": expect["error"] }), &r.marks);
                report.steps.push(json!({ "do": what, "error": e }));
                if problems.is_empty() {
                    Ok(())
                } else {
                    Err(Fault::Expect(problems.join("; ")))
                }
            }
            Err(f) => Err(f),
        };
        if let Err(f) = checked {
            fault = Some((i + 1, what, f));
            break;
        }
    }
    let (status, reason) = match (fault, expected) {
        (None, None) => (Status::Pass, String::new()),
        (None, Some(x)) => (Status::Fail, format!("expected_fail \"{x}\" but every step passes – the capability is in the core now: remove the line")),
        (Some((i, w, f)), x) => {
            let said = match &f {
                Fault::Error(e) => format!("step {i} ({w}): {e}"),
                Fault::Missing(m) => format!("step {i} ({w}): {m}"),
                Fault::Expect(e) => format!("step {i} ({w}): {e}"),
            };
            match (f, x) {
                (Fault::Missing(m), Some(x)) if x.split_whitespace().next() == Some(m.ticket) => (Status::Xfail, said),
                (_, Some(x)) => (Status::Fail, format!("{said} – expected_fail \"{x}\" covers only its missing capability; this failure is another")),
                (_, None) => (Status::Fail, said),
            }
        }
    };
    report.status = status;
    report.reason = reason;
    report
}

impl Run<'_> {
    fn file(&self, path: &str) -> Result<&str, Fault> {
        self.bundle.files.get(path).map(String::as_str).ok_or_else(|| Fault::Error(format!("the bundle lacks {path}")))
    }

    /// the scene a `load` / `warm` step names, as live-core gets it (lowered: the browser's split) → (TOML, page ids)
    fn scene(&self, step: &toml::Table) -> Result<(String, Vec<String>, Value), Fault> {
        let lower = step.get("lower").and_then(|v| v.as_bool()).unwrap_or(true);
        let err = |e: String| Fault::Error(e);
        if let Some(subject) = step.get("rig").and_then(|v| v.as_str()) {
            let rig = Rig::core(subject).ok_or_else(|| err(format!("no core rig {subject}")))?.map_err(|e| err(format!("{e:#}")))?;
            let mood = step.get("mood").and_then(|v| v.as_str());
            let played = rig.play(None, mood).map_err(|e| err(format!("{e:#}")))?;
            let page = played.engine.page.iter().map(|n| n.id.clone()).collect();
            let facts = json!({ "knobs": played.knobs.len(), "tracks": played.tracks.len(), "character": played.character });
            return Ok((played.engine.scene, page, facts));
        }
        // a scene file of deck/, or a small one written into the scenario (`toml`, loaded as it is)
        let text = match (step.get("scene").and_then(|v| v.as_str()), step.get("toml").and_then(|v| v.as_str())) {
            (Some(path), _) => self.file(path)?.to_string(),
            (None, Some(t)) => return Ok((t.to_string(), Vec::new(), Value::Null)),
            _ => return Err(err("a load without scene, toml or rig".into())),
        };
        if !lower {
            return Ok((text, Vec::new(), Value::Null));
        }
        let scene = Scene::parse(&text).map_err(|e| err(format!("{e:#}")))?;
        let e = deck_space::rig::engine::lower(&scene).map_err(|e| err(format!("{e:#}")))?;
        Ok((e.scene, e.page.iter().map(|n| n.id.clone()).collect(), Value::Null))
    }

    fn step(&mut self, what: &str, step: &toml::Table) -> Result<Value, Fault> {
        let num = |k: &str, d: f64| step.get(k).and_then(|v| v.as_float().or_else(|| v.as_integer().map(|i| i as f64))).unwrap_or(d);
        let text = |k: &str| step.get(k).and_then(|v| v.as_str()).map(String::from);
        let flag = |k: &str, d: bool| step.get(k).and_then(|v| v.as_bool()).unwrap_or(d);
        let d = &mut self.deck;
        Ok(match what {
            "reset" => {
                let (rate, channels) = (d.rate, d.channels);
                let pace = d.pace.take();
                self.deck = Deck::new(rate, channels);
                self.deck.pace = pace;
                json!({})
            }
            "load" => {
                let (toml, page, facts) = self.scene(step)?;
                let table = self.deck.load(&toml).map_err(Fault::Error)?;
                let ids: Vec<Value> = table["nodes"].as_array().map(|ns| ns.iter().map(|n| n["id"].clone()).collect()).unwrap_or_default();
                json!({ "nodes": ids.len(), "ids": ids, "page": page, "channels": table["channels"], "rig": facts })
            }
            "warm" => {
                let (toml, _, _) = self.scene(step)?;
                self.deck.warm(&toml, num("quanta", 256.0) as usize).map_err(Fault::Error)?;
                json!({ "warmed": true })
            }
            "run" => {
                let seen = d.run(num("ms", 1000.0));
                let mut out = d.state();
                out.insert("line".into(), json!(d.line()));
                out.insert("audio".into(), json!(seen.audio.hex()));
                out.insert("peak".into(), json!(round(f64::from(seen.peak), 4)));
                out.insert("sound".into(), json!(seen.peak > 1e-4));
                out.insert("frames".into(), json!(seen.frames));
                let [level, bass, mid, high] = d.bands();
                out.insert("bands".into(), json!([round(f64::from(level), 3), round(f64::from(bass), 3), round(f64::from(mid), 3), round(f64::from(high), 3)]));
                let statuses: Vec<Value> = seen.clock_out.iter().map(|m| json!(m[1] as u32)).collect();
                let ticks = seen.clock_out.iter().filter(|m| m[1] as u32 == 0xF8).count();
                out.insert("clock_out".into(), json!(seen.clock_out.len()));
                out.insert("clock_ticks".into(), json!(ticks));
                out.insert("clock_first".into(), statuses.first().cloned().unwrap_or(Value::Null));
                out.insert("clock_stops".into(), json!(seen.clock_out.iter().filter(|m| m[1] as u32 == 0xFC).count()));
                out.insert("anchors".into(), json!(seen.anchors.len()));
                out.insert("anchor_last".into(), seen.anchors.last().cloned().unwrap_or(Value::Null));
                out.insert("pings".into(), json!(seen.pings));
                out.insert("signals".into(), json!(seen.signals));
                out.insert("signal_bytes".into(), json!(seen.signal_bytes.hex()));
                Value::Object(out)
            }
            "send" => {
                let mode = match text("mode").as_deref().unwrap_or("force") {
                    "force" => Mode::Force,
                    "blend" => Mode::Blend,
                    "release" => Mode::Release,
                    "base" => Mode::Base,
                    m => return Err(Fault::Error(format!("no send mode '{m}'"))),
                };
                let path = text("path").unwrap_or_default();
                let n = d.send(&path, num("value", 0.0) as f32, mode, num("amount", 1.0) as f32).map_err(Fault::Error)?;
                json!({ "applied": n })
            }
            "heard" => {
                let v = d.heard(&text("path").unwrap_or_default()).map_err(Fault::Error)?;
                json!({ "value": round(f64::from(v), 3) })
            }
            "choose" => {
                let s = text("source").unwrap_or_else(|| "internal".into());
                let i = deck_core::tempo::Source::ALL.iter().position(|x| x.name() == s).ok_or_else(|| Fault::Error(format!("no clock source '{s}'")))?;
                d.choose(i, flag("lead", false));
                json!({})
            }
            "key" => {
                const KEYS: [&str; 5] = ["tap", "faster", "slower", "startstop", "click"];
                let k = text("key").unwrap_or_default();
                d.key(KEYS.iter().position(|x| *x == k).ok_or_else(|| Fault::Error(format!("no key '{k}'")))?);
                json!({})
            }
            "clock_out" => {
                d.clock_out(flag("on", true));
                json!({})
            }
            "joined" => {
                d.joined();
                json!({})
            }
            "midi_in" => {
                d.midi_in(step.get("bpm").map(|_| num("bpm", 120.0)));
                json!({})
            }
            "midi" => {
                let b: Vec<u8> = step.get("bytes").and_then(|v| v.as_array()).map(|a| a.iter().filter_map(|x| x.as_integer()).map(|x| x as u8).collect()).unwrap_or_default();
                d.midi([b.first().copied().unwrap_or(0), b.get(1).copied().unwrap_or(0), b.get(2).copied().unwrap_or(0)]);
                json!({})
            }
            "leader" => {
                d.leader(num("offset_ms", 0.0), num("rtt_ms", 2.0));
                json!({})
            }
            "anchor" => {
                d.anchor(num("bpm", 120.0), num("per_bar", 4.0) as u32, num("beat", 0.0), flag("running", true));
                json!({})
            }
            "quality" => {
                let l = text("level").unwrap_or_else(|| "high".into());
                let level = Level::parse(&l).ok_or_else(|| Fault::Error(format!("no quality level '{l}'")))?;
                let g = Governor::new(level, flag("auto", true));
                let out = quality(&g, 0);
                self.governor = Some(g);
                out
            }
            "frames" => {
                let g = self.governor.as_mut().ok_or_else(|| Fault::Error("frames before quality".into()))?;
                let (ms, secs, glitches) = (num("ms", 16.667), num("secs", 1.0), num("glitches", 0.0) as u64);
                let mut changes = 0;
                let end = self.gov_t + secs;
                while self.gov_t < end {
                    self.gov_t += ms / 1e3;
                    changes += usize::from(g.frame(self.gov_t, ms as f32, glitches));
                }
                quality(g, changes)
            }
            c if caps::known(c) => {
                let input = step.get("with").map(to_json).unwrap_or(Value::Null);
                caps::call(c, &input).map_err(Fault::Missing)?
            }
            other => return Err(Fault::Error(format!("no step '{other}'"))),
        })
    }
}

fn quality(g: &Governor, changes: usize) -> Value {
    json!({ "level": g.level().name(), "ceiling": g.ceiling().name(), "scale": round(f64::from(g.scale()), 3), "auto": g.auto(), "changes": changes, "frame_ms": round(f64::from(g.frame_ms()), 2) })
}

fn to_json(v: &toml::Value) -> Value {
    serde_json::to_value(v).unwrap_or(Value::Null)
}

/// An answer against its `expect` table → what did not hold. A plain value must be equal (numbers within 1e-9); a table
/// is a test: `near` + `tol`, `min` / `max`, `starts`, `contains`, `has` (an array holds it), `same_as` (a `mark`ed hash).
pub fn check(out: &Value, expect: &Value, marks: &BTreeMap<String, String>) -> Vec<String> {
    let Some(want) = expect.as_object() else { return Vec::new() };
    let mut problems = Vec::new();
    for (k, w) in want {
        let got = out.get(k).unwrap_or(&Value::Null);
        if let Some(e) = test(got, w, marks) {
            problems.push(format!("{k}: {e}"));
        }
    }
    problems
}

fn test(got: &Value, want: &Value, marks: &BTreeMap<String, String>) -> Option<String> {
    let n = |v: &Value| v.as_f64();
    match want {
        Value::Object(t) if t.keys().all(|k| ["near", "tol", "min", "max", "starts", "contains", "has", "same_as"].contains(&k.as_str())) => {
            let mut bad = Vec::new();
            if let Some(x) = t.get("near").and_then(n) {
                let tol = t.get("tol").and_then(n).unwrap_or(1e-6);
                if !n(got).is_some_and(|g| (g - x).abs() <= tol) {
                    bad.push(format!("near {x} ± {tol}"));
                }
            }
            if let Some(x) = t.get("min").and_then(n) {
                if !n(got).is_some_and(|g| g >= x) {
                    bad.push(format!("≥ {x}"));
                }
            }
            if let Some(x) = t.get("max").and_then(n) {
                if !n(got).is_some_and(|g| g <= x) {
                    bad.push(format!("≤ {x}"));
                }
            }
            if let Some(s) = t.get("starts").and_then(|v| v.as_str()) {
                if !got.as_str().is_some_and(|g| g.starts_with(s)) {
                    bad.push(format!("starts with \"{s}\""));
                }
            }
            if let Some(s) = t.get("contains").and_then(|v| v.as_str()) {
                if !got.as_str().is_some_and(|g| g.contains(s)) {
                    bad.push(format!("contains \"{s}\""));
                }
            }
            if let Some(x) = t.get("has") {
                if !got.as_array().is_some_and(|a| a.contains(x)) {
                    bad.push(format!("has {x}"));
                }
            }
            if let Some(m) = t.get("same_as").and_then(|v| v.as_str()) {
                if marks.get(m).map(|h| Value::String(h.clone())).as_ref() != Some(got) {
                    bad.push(format!("the same as '{m}' ({})", marks.get(m).map_or("not marked", String::as_str)));
                }
            }
            (!bad.is_empty()).then(|| format!("{got} – want {}", bad.join(", ")))
        }
        Value::Object(t) => {
            let problems = check(got, &Value::Object(t.clone()), marks);
            (!problems.is_empty()).then(|| problems.join(", "))
        }
        Value::Number(x) => (!n(got).is_some_and(|g| (g - x.as_f64().unwrap_or(f64::NAN)).abs() <= 1e-9)).then(|| format!("{got} – want {x}")),
        w => (got != w).then(|| format!("{got} – want {w}")),
    }
}

/// many reports → the line a CI log shows per scenario, and whether the run is green
pub fn summary(reports: &[Report]) -> (Vec<String>, bool) {
    let lines = reports
        .iter()
        .map(|r| match r.status {
            Status::Pass => format!("pass   {}", r.name),
            Status::Xfail => format!("xfail  {} – {}", r.name, r.reason),
            Status::Fail => format!("FAIL   {} – {}", r.name, r.reason),
        })
        .collect();
    (lines, reports.iter().all(|r| r.status != Status::Fail))
}

thread_local! {
    static IO: RefCell<(Vec<u8>, Vec<u8>)> = const { RefCell::new((Vec::new(), Vec::new())) };
}

/// wasm: the inbox, at least `len` bytes (a bundle's JSON goes here)
#[no_mangle]
pub extern "C" fn hl_inbox(len: usize) -> *mut u8 {
    IO.with(|io| {
        let mut io = io.borrow_mut();
        if io.0.len() < len {
            io.0.resize(len, 0);
        }
        io.0.as_mut_ptr()
    })
}

/// wasm: the bundle in the inbox's first `len` bytes run → the report's JSON length in the outbox (< 0: an error text)
#[no_mangle]
pub extern "C" fn hl_run(len: usize) -> i32 {
    IO.with(|io| {
        let mut io = io.borrow_mut();
        let (text, ok) = match serde_json::from_slice::<Bundle>(&io.0[..len.min(io.0.len())]) {
            Ok(b) => (serde_json::to_string(&run(&b)).unwrap_or_default(), true),
            Err(e) => (format!("the bundle: {e}"), false),
        };
        io.1 = text.into_bytes();
        let n = io.1.len() as i32;
        if ok {
            n
        } else {
            -n
        }
    })
}

/// wasm: the outbox's address (its length is what `hl_run` returned)
#[no_mangle]
pub extern "C" fn hl_outbox() -> *const u8 {
    IO.with(|io| io.borrow().1.as_ptr())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expectations_hold_or_say_why() {
        let marks = BTreeMap::from([("cold".to_string(), "ab".to_string())]);
        let out = json!({ "bpm": 120.0, "line": "SHOW +0.1MS RTT 2.0MS", "audio": "ab", "page": ["music"], "n": 3 });
        let ok = json!({ "bpm": { "near": 120.0, "tol": 0.1 }, "line": { "starts": "SHOW " }, "audio": { "same_as": "cold" }, "page": { "has": "music" }, "n": 3 });
        assert!(check(&out, &ok, &marks).is_empty(), "{:?}", check(&out, &ok, &marks));
        let bad = json!({ "bpm": { "min": 121 }, "n": 4, "missing": true });
        assert_eq!(check(&out, &bad, &marks).len(), 3);
    }

    #[test]
    fn an_expected_failure_is_only_its_missing_capability() {
        let b = |s: &str| Bundle { name: "x".into(), scenario: s.into(), files: BTreeMap::new() };
        let r = run(&b("expected_fail = \"#364 stats\"\n[[step]]\ndo = \"stats\"\n"));
        assert_eq!(r.status, Status::Xfail, "{}", r.reason);
        assert!(r.reason.contains("missing capability #364"));
        // the wrong ticket, another failure, or a pass: red
        assert_eq!(run(&b("expected_fail = \"#365\"\n[[step]]\ndo = \"stats\"\n")).status, Status::Fail);
        assert_eq!(run(&b("expected_fail = \"#364\"\n[[step]]\ndo = \"nonsense\"\n")).status, Status::Fail);
        let passed = run(&b("expected_fail = \"#364\"\n[[step]]\ndo = \"reset\"\n"));
        assert_eq!(passed.status, Status::Fail);
        assert!(passed.reason.contains("remove the line"));
    }
}
