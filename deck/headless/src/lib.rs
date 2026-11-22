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
//! | `load` | `scene` (a deck/ path; `lower`: the browser's split, default on), `toml` (a small scene as it is), `rig` (`character.catpope`) + `mood`, `silent = true` (no rig plays, #425), or `document` (a scene document, #492) with `with` (its fillers) or `use` (`{ scene, with }`, checked first by the core – in `book`, the toon's references, when given; #504), `rigs`, `previews` | nodes, page (ids the engine does not carry), channels; a document: its stage |
//! | `warm` | as `load`, `quanta` | – |
//! | `run` | `ms`; `mark` (keep the audio hash); `trace` (#467: a line per clock frame) | audio (hash), peak, frames, the clock state (+ the transport's `epoch`), line, clock_out, anchors, pings, signals, piece, crossfade, gliding |
//! | `send` | `path`, `value`, `mode` (force / blend / base), `amount` | applied |
//! | `heard` | `path` | value (the analysis frame's) |
//! | `choose` | `source`, `lead` | the clock state |
//! | `mode` | `mode` (show, own, lead, midi – the page's picker, #370; the next `run` shows it) | – |
//! | `tempo_line` | – | line (the tempo line), status (the source's), hud (the panel's: both), the clock state (mode, out, flash …) |
//! | `clock_report` | – | line (the sound report's clock line, #387), n, p50, p95, max, mean, seconds (the last minute's error window) |
//! | `key` | `key` (tap, faster, slower, startstop, click) | – |
//! | `clock_out` · `joined` · `midi_in` (`bpm`, `start`) · `midi` · `leader` · `anchor` | the page's and the devices' input | – |
//! | `quality` / `frames` | `level`, `auto` / `ms`, `secs`, `glitches` | level, ceiling, scale, auto, changes |
//! | `reset` | – | a fresh host (the clock too) |
//! | `carried_part` | `with` = { `scene`, `modules` } | kept (nodes), dropped (ids) – deck-core's filter (#360) |
//! | `hud` | `with` = { `scene`, `fps`, `audio` } | title, lines, bar, levels – deck-ui's HUD from this deck's clock and monitor (#374) |
//! | `context` | `pin` (a mood, "" = the ladder's), `touch` | deck (its mood, energy), engine (the live context the engine applied last: mood, persona, energy, touch, strain – #409) |
//! | `show` | `show` (a deck/ show document), `outlet` (the one this deck plays), `pieces` (what the deck has: piece → `scene` / `bundle` a deck/ file, `toml` a scene as it is, `rig` a core rig's subject) | rows |
//! | `pieces` | `pieces` (more of them) | – |
//! | `fire` / `go` | `row` / – : the show's conductor fires, its cell for the outlet goes to the deck (deck-space's piece.rs, #467) | do (load, glide, black, same, kept), piece, ms, values, glides, problems, playing; `vote` (#548: the `vote.open` a row's question sends) |
//! | `launch` | `row`, `column` (#551: Perform's ▶ on one cell, the conductor's hand launch; an `ask` cell opens its whole question) | as `fire` |
//! | `vote` | `option` (#548: the relay's result of the last question, the option's place from 0) – the show's event `vote:<question>/<n>`, the row set to it fires | event, row, said (the log's last line), and as `fire` |
//! | `address` | `address`, `action` (#516: as the relay's `address` brings it; a show row's `addresses` reach the same path through `fire` / `go`: a line each under `addresses`, the last change's `stage`) – to the scene document playing (deck-space scene_play.rs: `join:<who>` / `leave:<who>`), a changed cast recast in the engine (live-core `recast`); a camera word (`shot:<id>`, `move:<id>`) to the stage's camera (Space `camera_word`; through `fire` its answer is `camera`) | did (changed, nothing, not_mine; camera), holder, ms, said (the log's last line), ids, stage; viewing, ortho, fov |
//! | `story` | `event` (#517: a reader's story event as the relay brings it – `type`, `params`, `id`) – to the playing scene document's listeners (deck-space scene_play.rs `Playing::story`; a `load` of a `document` may add `listener`s), a changed cast recast once | did (a line per cast word: changed at <holder>, nothing), said, ids, stage |
//! | `pick` | on the last scene document loaded (#496, stage.rs): `click` (a holder, or `[x, y]` of a 1280 × 720 window), `key` (+ `shift`), `drag` (a knob of the hand, `by`) | picked, selection, knobs (the hand's), mark, key, reach, turned (sent to the engine) |
//! | a capability (caps.rs) | `with` | its answer, or `Missing` |
//!
//! Built for wasm32 too (`hl_inbox`, `hl_run`, `hl_outbox`): the same bundle in, the same report out – tests/wasm.rs
//! proves native ≡ wasm32 ≡ wasm32 + simd128, so the browser's bridge adds nothing a scenario can see.
pub mod bones;
pub mod caps;
pub mod deck;
pub mod stage;

use deck::{round, Deck, T0};
use deck_show::conductor::{Conductor, Now, Out};
use deck_space::piece::{Found, What};
use deck_space::rig::address::Place;
use deck_space::quality::{Governor, Level};
use deck_space::resolver::{Found as Opinion, Resolver, Source, Strength};
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

/// the paths of deck/ files a scenario's steps name (`scene = …`, `show = …`, `document = …`, a piece's `scene` / `bundle`) – the CLI and
/// the tests bundle them
pub fn files_of(scenario: &str) -> Vec<String> {
    let Ok(t) = scenario.parse::<toml::Table>() else { return Vec::new() };
    let steps = t.get("step").and_then(|s| s.as_array()).cloned().unwrap_or_default();
    let named = |s: &toml::Value, k: &str| s.get(k).and_then(|v| v.as_str()).map(String::from);
    let pieces = |s: &toml::Value| -> Vec<String> {
        let ps = s.get("pieces").and_then(|p| p.as_table()).map(|t| t.values().cloned().collect::<Vec<_>>()).unwrap_or_default();
        ps.iter().flat_map(|p| [named(p, "scene"), named(p, "bundle")]).flatten().collect()
    };
    // #246: a published scene's scripted answers (`answers = [{ status, file }]`)
    let answers = |s: &toml::Value| -> Vec<String> { s.get("answers").and_then(|a| a.as_array()).into_iter().flatten().filter_map(|a| named(a, "file")).collect() };
    let mut out: Vec<String> = steps.iter().flat_map(|s| [named(s, "scene"), named(s, "show"), named(s, "document"), named(s, "content")].into_iter().flatten().chain(pieces(s)).chain(answers(s))).collect();
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

/// #467: the show a scenario fires at this deck: its conductor, the outlet the deck plays, what the deck has for a piece
struct OnShow {
    conductor: Conductor,
    outlet: String,
    pieces: BTreeMap<String, toml::Value>,
    /// #548: the votes a fired row opened (for the next id: `v1`, `v2` …) and the show's question the last one asks
    votes: u32,
    asked: Option<String>,
}

struct Run<'a> {
    bundle: &'a Bundle,
    deck: Deck,
    show: Option<OnShow>,
    governor: Option<Governor>,
    gov_t: f64,
    marks: BTreeMap<String, String>,
    /// #474: the opinions a `resolve` step gave the resolver
    opinions: Resolver<Value>,
    /// #496: the last scene document resolved (its stage), and its pick once a `pick` step asked
    resolved: Option<Scene>,
    stage: Option<stage::Stage>,
    /// #246: the published scenes this deck keeps (a native deck's `<cache>/scenes/`): scene id → the core's cache entry; a
    /// `reset` (a restart) keeps it
    kept: BTreeMap<String, String>,
    /// #516: the scene document playing (deck-space scene_play.rs), the rigs and scene-content its load named – an address
    /// action's cast word re-resolves it with them
    playing: Option<deck_space::scene_play::Playing>,
    own_rigs: BTreeMap<String, Rig>,
    content: deck_space::push::Content,
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
    let mut r = Run { bundle, deck: Deck::new(rate, channels), show: None, governor: None, gov_t: 0.0, marks: BTreeMap::new(), opinions: Resolver::new(), resolved: None, stage: None, kept: BTreeMap::new(), playing: None, own_rigs: BTreeMap::new(), content: Default::default() };
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

    /// #467: what this deck has for a piece – the bytes the scenario names (the adapter's part: no choice made here)
    fn find(&self, place: &Place) -> Option<Found> {
        let entry = self.show.as_ref()?.pieces.get(&place.to_string())?;
        let s = |k: &str| entry.get(k).and_then(|v| v.as_str());
        if let Some(t) = s("toml") {
            return Some(Found::Scene(t.to_string()));
        }
        if let Some(p) = s("scene") {
            return self.bundle.files.get(p).map(|t| Found::Scene(t.clone()));
        }
        if let Some(p) = s("bundle") {
            return self.bundle.files.get(p).map(|t| Found::Rig(t.clone()));
        }
        let subject = s("rig")?;
        deck_space::rig::bundle::CORE.iter().find(|(c, _)| *c == subject).map(|(_, t)| Found::Rig(t.to_string()))
    }

    /// #467: the conductor's outs → the cell for this deck's outlet, played (the last one, if a row sent several), and the
    /// answer: what the deck did, what plays, why not
    fn deliver(&mut self, outs: Vec<Out>) -> Result<Value, Fault> {
        let mut out = self.deliver_cell(&outs)?;
        // #516: then the row's address actions, to the playing scene document by the same path as the relay's `address`
        // each as a line (`<address> > <action>: <did>[ at <holder>]`), the stage after the last change as `stage`
        // a cell's camera cut only on its own column (the relay's `where.group`)
        let outlet = self.show.as_ref().map(|s| s.outlet.clone()).unwrap_or_default();
        let mut addressed = Vec::new();
        let words = outs.iter().filter_map(|o| match o {
            Out::Address(a) => Some((a.address.as_str(), a.action.as_str())),
            Out::Camera { column, address, action, .. } if *column == outlet => Some((address.as_str(), action.as_str())),
            _ => None,
        });
        for (address, action) in words {
            let answer = self.address(address, action)?;
            let at = answer["holder"].as_str().map(|h| format!(" at {h}")).unwrap_or_default();
            addressed.push(format!("{address} > {action}: {}{at}", answer["did"].as_str().unwrap_or_default()));
            if answer.get("stage").is_some() {
                out["stage"] = answer["stage"].clone();
            }
            if answer.get("viewing").is_some() {
                out["camera"] = answer.clone();
            }
        }
        if !addressed.is_empty() {
            out["addresses"] = json!(addressed);
        }
        // #548: a row's question – the `vote.open` the conductor's subtree sends the relay for it (deck-show `Ask::vote`)
        if let Some(a) = outs.iter().find_map(|o| if let Out::Ask(a) = o { Some(a) } else { None }) {
            let on = self.show.as_mut().ok_or_else(|| Fault::Error("a question without a show".into()))?;
            on.votes += 1;
            let open = a.vote(on.conductor.show(), &format!("v{}", on.votes)).map_err(Fault::Error)?;
            on.asked = Some(a.question.clone());
            out["vote"] = serde_json::to_value(&open).map_err(|e| Fault::Error(e.to_string()))?;
        }
        Ok(out)
    }

    fn deliver_cell(&mut self, outs: &[Out]) -> Result<Value, Fault> {
        let outlet = self.show.as_ref().map(|s| s.outlet.clone()).unwrap_or_default();
        let cells: Vec<protocol::Cell> = outs
            .iter()
            .filter_map(Out::envelope)
            .filter_map(|e| match &e.message {
                protocol::Message::Cell(c) if c.outlet == outlet => Some(c.cell.clone()),
                _ => None,
            })
            .collect();
        let Some(cell) = cells.last() else {
            return Ok(json!({ "do": "none", "playing": self.deck.pieces.piece().unwrap_or_default() }));
        };
        let now = self.deck.now;
        let mut pieces = std::mem::take(&mut self.deck.pieces);
        let answer = pieces.cell(cell, now, |p| self.find(p));
        self.deck.pieces = pieces;
        let mut out = serde_json::to_value(&answer.what).map_err(|e| Fault::Error(e.to_string()))?;
        match &answer.what {
            What::Load { scene, values, ms, .. } => {
                let table = self.deck.crossfade(scene, *ms).map_err(Fault::Error)?;
                for (path, v) in values {
                    self.deck.send(path, *v, live_core::wire::Mode::Force, 1.0).map_err(Fault::Error)?;
                }
                out["nodes"] = json!(table["nodes"].as_array().map_or(0, Vec::len));
                out["values"] = json!(values.iter().map(|(p, v)| (p.clone(), json!(round(f64::from(*v), 4)))).collect::<serde_json::Map<_, _>>());
            }
            What::Black { scene, ms } => {
                self.deck.crossfade(scene, *ms).map_err(Fault::Error)?;
            }
            What::Glide { glides, .. } => out["glides"] = json!(glides.iter().map(|g| g.path.clone()).collect::<Vec<_>>()),
            What::Same | What::Kept => {}
        }
        if let Some(o) = out.as_object_mut() {
            o.remove("scene");
            o.insert("problems".into(), json!(answer.problems));
            o.insert("playing".into(), json!(self.deck.pieces.piece().unwrap_or_default()));
        }
        Ok(out)
    }

    /// #516: an address action (`address > action`, as a show row's `addresses` or the relay's `address` carry it) passed to
    /// the playing scene document (deck-space scene_play.rs); a changed cast is recast in the engine (live-core `recast`).
    /// The answer: did (changed, nothing, not_mine), the holder, the log's last line, the stage after a change
    fn address(&mut self, address: &str, action: &str) -> Result<Value, Fault> {
        use deck_space::scene_play::Recast;
        // a camera word (`shot:<id>`, `move:<id>`) goes to the stage's camera (deck-space Space `camera_word`), as a deck's
        // Space takes the relay's `address`
        if deck_space::rig::address::CameraWord::parse(action).is_some() {
            let Some(scene) = self.resolved.as_ref() else { return Ok(json!({ "did": "not_mine" })) };
            if self.stage.is_none() {
                self.stage = Some(stage::Stage::new(scene).map_err(Fault::Error)?);
            }
            return Ok(self.stage.as_mut().expect("made above").camera(address, action));
        }
        let Some(playing) = self.playing.as_mut() else { return Ok(json!({ "did": "not_mine" })) };
        let (own, content) = (&self.own_rigs, &self.content);
        let rigs = |s: &str, v: Option<&str>| own.get(s).cloned().or_else(|| deck_space::scene_resolve::core_rigs(s, v));
        let sets = |l: &str| content.set_of(l);
        let recast = playing.word(address, action, &rigs, &sets);
        let said = playing.log().last().unwrap_or_default().to_string();
        Ok(match recast {
            Recast::NotMine => json!({ "did": "not_mine" }),
            Recast::Nothing => json!({ "did": "nothing", "said": said }),
            Recast::Changed { holder, ms } => {
                let r = playing.resolved().clone();
                let e = deck_space::rig::engine::lower(&r.scene).map_err(|e| Fault::Error(format!("{e:#}")))?;
                let table = self.deck.recast(&e.scene, ms).map_err(Fault::Error)?;
                (self.resolved, self.stage) = (Some(r.scene.clone()), None);
                let ids: Vec<Value> = table["nodes"].as_array().map(|ns| ns.iter().map(|n| n["id"].clone()).collect()).unwrap_or_default();
                json!({ "did": "changed", "holder": holder, "ms": ms, "said": said, "ids": ids, "stage": stage_facts(&r, &self.content) })
            }
        })
    }

    /// #517: a story event (the VNEvent's JSON, as the relay brings a reader's `story`) passed to the playing scene document
    /// (deck-space scene_play.rs `Playing::story`: its listeners); a changed cast is recast in the engine once. The answer:
    /// did (one line per cast word a listener made: changed at <holder>, nothing), the log's last line, the stage after a change
    fn story(&mut self, event: &Value) -> Result<Value, Fault> {
        use deck_space::scene_play::Recast;
        let Some(playing) = self.playing.as_mut() else { return Ok(json!({ "did": [] })) };
        let (own, content) = (&self.own_rigs, &self.content);
        let rigs = |s: &str, v: Option<&str>| own.get(s).cloned().or_else(|| deck_space::scene_resolve::core_rigs(s, v));
        let sets = |l: &str| content.set_of(l);
        let did = playing.story(event, &rigs, &sets);
        let said = playing.log().last().unwrap_or_default().to_string();
        let words: Vec<String> = did.iter().map(|r| match r {
            Recast::Changed { holder, .. } => format!("changed at {holder}"),
            Recast::Nothing => "nothing".into(),
            Recast::NotMine => "not_mine".into(),
        }).collect();
        let Some(ms) = did.iter().rev().find_map(|r| if let Recast::Changed { ms, .. } = r { Some(*ms) } else { None }) else {
            return Ok(json!({ "did": words, "said": said }));
        };
        let r = playing.resolved().clone();
        let e = deck_space::rig::engine::lower(&r.scene).map_err(|e| Fault::Error(format!("{e:#}")))?;
        let table = self.deck.recast(&e.scene, ms).map_err(Fault::Error)?;
        (self.resolved, self.stage) = (Some(r.scene.clone()), None);
        let ids: Vec<Value> = table["nodes"].as_array().map(|ns| ns.iter().map(|n| n["id"].clone()).collect()).unwrap_or_default();
        Ok(json!({ "did": words, "said": said, "ids": ids, "stage": stage_facts(&r, &self.content) }))
    }

    /// the show's time: the deck's ms since its start
    fn show_now(&self) -> Now {
        Now::at((self.deck.now - T0).max(0.0) as u64)
    }

    /// the scene a `load` / `warm` step names, as live-core gets it (lowered: the browser's split) → (TOML, page ids)
    fn scene(&mut self, step: &toml::Table) -> Result<(String, Vec<String>, Value), Fault> {
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
        // #492: a scene document resolved by the core (scene_resolve.rs) with a use's fillers (`with`) and the rigs – the
        // core's, or bundles written into the scenario (`rigs`: subject → bundle); the stage's cast comes back as facts
        if let Some(path) = step.get("document").and_then(|v| v.as_str()) {
            use deck_space::scene_document::{Fillers, SceneDocument};
            let mut doc = SceneDocument::parse_json(self.file(path)?).map_err(|e| err(format!("{path}: {e:#}")))?;
            // #517: listeners written into the scenario (`listener`, as a document writes them), checked by the core
            // a camera written into the scenario (`camera`, as the DOP panel stores it, scene_camera.rs)
            if let Some(c) = step.get("camera") {
                doc.camera = Some(c.clone().try_into().map_err(|e| err(format!("camera: {e}")))?);
            }
            if let Some(l) = step.get("listener") {
                doc.listeners = l.clone().try_into().map_err(|e| err(format!("listener: {e}")))?;
                let problems = doc.check();
                if !problems.is_empty() {
                    return Err(err(problems.join("; ")));
                }
            }
            // #504: a use `{ scene, with }` (as a group or cell stores it) is checked by the core (`Use::problems`) – in the toon's
            // `book` when given, each filler against its `fill` – before it plays; else the bare `with`
            let with: BTreeMap<String, Fillers> = match (step.get("use"), step.get("with")) {
                (Some(u), _) => {
                    let u: deck_space::scene_resolve::Use = u.clone().try_into().map_err(|e| err(format!("use: {e}")))?;
                    let book: Option<Vec<String>> = step.get("book").map(|b| b.clone().try_into()).transpose().map_err(|e| err(format!("book: {e}")))?;
                    let problems = u.problems(&doc, book.as_deref());
                    if !problems.is_empty() {
                        return Err(err(problems.join("; ")));
                    }
                    u.with
                }
                (None, Some(v)) => v.clone().try_into().map_err(|e| err(format!("with: {e}")))?,
                (None, None) => BTreeMap::new(),
            };
            self.own_rigs = match step.get("rigs") {
                Some(v) => v.clone().try_into().map_err(|e| err(format!("rigs: {e}")))?,
                None => BTreeMap::new(),
            };
            // #512: scene-content's answer (`content`, a file of the bundle) – a location's set, the set's media per rung
            self.content = match step.get("content").and_then(|v| v.as_str()) {
                Some(path) => deck_space::push::Content::parse(self.file(path)?).map_err(|e| err(format!("{path}: {e}")))?,
                None => Default::default(),
            };
            let (own, content) = (&self.own_rigs, &self.content);
            let rigs = |s: &str, v: Option<&str>| own.get(s).cloned().or_else(|| deck_space::scene_resolve::core_rigs(s, v));
            let sets = |l: &str| content.set_of(l);
            // a deck plays without previews: the core's playing scene (#516), which takes cast words while it plays; the
            // editor's previews are a resolve alone
            let previews = step.get("previews").and_then(|v| v.as_bool()).unwrap_or(false);
            let (r, playing) = if previews {
                (deck_space::scene_resolve::resolve(&doc, &with, true, &rigs, &sets).map_err(|e| err(e.join("; ")))?, None)
            } else {
                let p = deck_space::scene_play::Playing::start(doc, with, &rigs, &sets).map_err(|e| err(e.join("; ")))?;
                (p.resolved().clone(), Some(p))
            };
            self.playing = playing;
            let e = deck_space::rig::engine::lower(&r.scene).map_err(|e| err(format!("{e:#}")))?;
            (self.resolved, self.stage) = (Some(r.scene.clone()), None);
            return Ok((e.scene, e.page.iter().map(|n| n.id.clone()).collect(), stage_facts(&r, &self.content)));
        }
        // #246: a published scene (`published = "scene:<id>[@v]"`) by the core's loader (deck-space published.rs): the API's
        // answers scripted in order (`answers`: `{ status, file }`, status 0 = unreachable), the run's cache (`kept`) as a deck's
        if let Some(reference) = step.get("published").and_then(|v| v.as_str()) {
            use deck_space::published::{Load, SceneRef, Step};
            let r = SceneRef::parse(reference).ok_or_else(|| err(format!("'{reference}' is no scene:<id>")))?.map_err(err)?;
            let mut answers = Vec::new();
            for a in step.get("answers").and_then(|v| v.as_array()).into_iter().flatten() {
                let status = a.get("status").and_then(|v| v.as_integer()).unwrap_or(200) as u16;
                let body = match a.get("file").and_then(|v| v.as_str()) {
                    Some(path) => self.file(path)?.to_string(),
                    None => String::new(),
                };
                answers.push((status, body));
            }
            let mut answers = answers.into_iter();
            let (mut load, mut next) = Load::start(r.clone(), self.kept.get(&r.id).map(String::as_str));
            let mut asked = Vec::new();
            let l = loop {
                match next {
                    Step::Ask(path) if asked.len() < 8 => {
                        asked.push(path);
                        let (status, body) = answers.next().unwrap_or((0, String::new()));
                        next = load.answer(status, &body);
                    }
                    Step::Ask(_) => return Err(err(format!("{r}: the loader keeps asking"))),
                    Step::Play(l) => break *l,
                    Step::Fail(why) => return Err(err(why)),
                }
            };
            if let Some(text) = &l.store {
                self.kept.insert(r.id.clone(), text.clone());
            }
            let e = deck_space::rig::engine::lower(&l.scene).map_err(|e| err(format!("{e:#}")))?;
            (self.resolved, self.stage) = (Some(l.scene.clone()), None);
            let set = l.scene.nodes.iter().find(|n| n.kind == "space").and_then(|n| n.params.get("set")).and_then(|v| v.as_str());
            let facts = json!({ "version": l.version, "cached": l.cached, "asked": asked, "notes": l.notes, "set": set, "character": l.scene.being_character() });
            return Ok((e.scene, e.page.iter().map(|n| n.id.clone()).collect(), facts));
        }
        // #425: no rig plays – the deck's own scene, silent (what `Deck::new` and `play_core` of a character without a rig play)
        if step.get("silent").and_then(|v| v.as_bool()).unwrap_or(false) {
            let played = deck_space::rig::Played::silent().map_err(|e| err(format!("{e:#}")))?;
            let page = played.engine.page.iter().map(|n| n.id.clone()).collect();
            return Ok((played.engine.scene, page, json!({ "knobs": played.knobs.len(), "tracks": played.tracks.len(), "character": played.character })));
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
                (self.resolved, self.stage, self.playing) = (None, None, None);
                json!({})
            }
            // #516: an address action as the relay's `address` carries it to this deck (a show row's go through `fire`)
            "address" => {
                let address = text("address").ok_or_else(|| Fault::Error("an address step without `address`".into()))?;
                self.address(&address, &text("action").unwrap_or_default())?
            }
            // #517: a reader's story event (the VNEvent: `type`, `params`, `id`) for the playing scene's listeners
            "story" => {
                let event = step.get("event").ok_or_else(|| Fault::Error("a story step without `event`".into()))?;
                self.story(&serde_json::to_value(event).map_err(|e| Fault::Error(format!("event: {e}")))?)?
            }
            // #496: the stage's pick (stage.rs) – on the last scene document loaded
            "pick" => {
                if self.stage.is_none() {
                    let scene = self.resolved.as_ref().ok_or_else(|| Fault::Error("pick before a scene document's load".into()))?;
                    self.stage = Some(stage::Stage::new(scene).map_err(Fault::Error)?);
                }
                let stage = self.stage.as_mut().expect("made above");
                stage.step(step, &mut self.deck).map_err(Fault::Error)?
            }
            "load" => {
                let (toml, page, facts) = self.scene(step)?;
                let table = self.deck.load(&toml).map_err(Fault::Error)?;
                let ids: Vec<Value> = table["nodes"].as_array().map(|ns| ns.iter().map(|n| n["id"].clone()).collect()).unwrap_or_default();
                // a scene document's facts are its stage's (#492), a rig's the rig's
                let key = if step.contains_key("document") || step.contains_key("published") { "stage" } else { "rig" };
                let mut out = json!({ "nodes": ids.len(), "ids": ids, "page": page, "channels": table["channels"] });
                out[key] = facts;
                out
            }
            "warm" => {
                let (toml, _, _) = self.scene(step)?;
                self.deck.warm(&toml, num("quanta", 256.0) as usize).map_err(Fault::Error)?;
                json!({ "warmed": true })
            }
            "run" => {
                d.tracing = flag("trace", false);
                let seen = d.run(num("ms", 1000.0));
                let mut out = d.state();
                // #467: the piece on ("" = none), how far its crossfade and its glide are (-1 = none runs)
                out.insert("piece".into(), json!(d.pieces.piece().unwrap_or_default()));
                out.insert("crossfade".into(), json!(d.crossfading().map_or(-1.0, |x| round(f64::from(x), 3))));
                out.insert("gliding".into(), json!(d.pieces.gliding(d.now).map_or(-1.0, |x| round(f64::from(x), 3))));
                if d.tracing {
                    out.insert("trace".into(), json!(seen.trace));
                }
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
            "mode" => {
                let m = text("mode").unwrap_or_default();
                let i = deck_core::tempo::ClockMode::ALL.iter().position(|x| x.name() == m).ok_or_else(|| Fault::Error(format!("no clock mode '{m}'")))?;
                d.mode(i);
                json!({})
            }
            "tempo_line" => {
                let mut out = d.state();
                out.insert("line".into(), json!(d.tempo_line()));
                out.insert("status".into(), json!(d.line()));
                out.insert("hud".into(), json!(d.hud_line()));
                Value::Object(out)
            }
            // #387: the sound report's clock line – the last minute's error on the show's line, deck-core's
            "clock_report" => {
                let (line, s) = d.clock_report();
                let s = s.map(|s| (s.n, round(s.p50, 3), round(s.p95, 3), round(s.max, 3), round(s.mean, 3), round(s.seconds, 3)));
                let (n, p50, p95, max, mean, seconds) = s.unwrap_or_default();
                json!({ "line": line, "n": n, "p50": p50, "p95": p95, "max": max, "mean": mean, "seconds": seconds })
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
                d.midi_in(step.get("bpm").map(|_| num("bpm", 120.0)), flag("start", true));
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
            // #360: what of a scene a build with these modules plays – deck-core's carried part
            "carried_part" => {
                let with = step.get("with").and_then(|v| v.as_table()).ok_or_else(|| Fault::Error("carried_part without `with`".into()))?;
                let path = with.get("scene").and_then(|v| v.as_str()).unwrap_or_default();
                let scene = Scene::parse(self.file(path)?).map_err(|e| Fault::Error(format!("{e:#}")))?;
                let modules: Vec<&str> = with.get("modules").and_then(|v| v.as_array()).map(|a| a.iter().filter_map(|m| m.as_str()).collect()).unwrap_or_default();
                let (part, dropped, _) = scene.carried_part(|f| modules.contains(&f));
                json!({ "kept": part.nodes.len(), "dropped": dropped.iter().map(|n| n.id.as_str()).collect::<Vec<_>>() })
            }
            // #374: the HUD's lines and the bar's items – deck-ui's, from this deck's clock and monitor
            "hud" => {
                let with = step.get("with").and_then(|v| v.as_table());
                let get = |k: &str| with.and_then(|w| w.get(k));
                let scene = get("scene").and_then(|v| v.as_str()).unwrap_or_default();
                let fps = get("fps").and_then(|v| v.as_float().or_else(|| v.as_integer().map(|i| i as f64))).unwrap_or(60.0);
                let s = d.hud_status(scene, fps as f32, get("audio").and_then(|v| v.as_bool()).unwrap_or(false));
                let lines = deck_ui::hud_lines(&s);
                let bar = deck_ui::bar_items(&s);
                json!({
                    "title": lines.first(),
                    "lines": lines,
                    "bar": bar.iter().map(|i| i.text.as_str()).collect::<Vec<_>>(),
                    "levels": bar.iter().map(|i| i.level).collect::<Vec<_>>(),
                })
            }
            // #409: the page's deck state – a pinned mood (`pin`, "" = the ladder again), a hand (`touch`) – and the live
            // context the core's engine applied last (it goes in every clock frame of a `run`)
            "context" => {
                if let Some(m) = text("pin") {
                    d.state.ladder.pin(Some(m.as_str()));
                }
                if step.contains_key("touch") {
                    d.touch = num("touch", 0.0) as f32;
                }
                let c = d.applied_context().unwrap_or_default();
                let r = |x: f32| round(f64::from(x), 4);
                json!({
                    "deck": { "mood": d.state.mood(), "energy": r(d.state.ladder.energy()) },
                    "engine": { "mood": c.mood(), "persona": c.persona(), "energy": r(c.energy), "touch": r(c.touch), "strain": r(c.strain) },
                })
            }
            // #467: a show fired at this deck – the conductor's cells for its outlet played by the deck's pieces
            "show" => {
                let path = text("show").ok_or_else(|| Fault::Error("a show step without `show`".into()))?;
                let show = deck_show::document::parse(self.file(&path)?).map_err(|e| Fault::Error(format!("{path}: {e:?}")))?;
                let rows: Vec<String> = show.rows.iter().map(|r| r.id.clone()).collect();
                let pieces = step.get("pieces").and_then(|p| p.as_table()).map(|t| t.clone().into_iter().collect()).unwrap_or_default();
                self.show = Some(OnShow { conductor: Conductor::new(show), outlet: text("outlet").unwrap_or_else(|| "left".into()), pieces, votes: 0, asked: None });
                json!({ "rows": rows })
            }
            "pieces" => {
                let on = self.show.as_mut().ok_or_else(|| Fault::Error("pieces before show".into()))?;
                on.pieces.extend(step.get("pieces").and_then(|p| p.as_table()).cloned().unwrap_or_default());
                json!({})
            }
            "fire" | "go" | "launch" => {
                let now = self.show_now();
                let on = self.show.as_mut().ok_or_else(|| Fault::Error(format!("{what} before show")))?;
                let outs = match what {
                    "fire" => on.conductor.fire(&text("row").unwrap_or_default(), now),
                    "launch" => on.conductor.launch_cell(&text("row").unwrap_or_default(), &text("column").unwrap_or_default(), now),
                    _ => on.conductor.go(now),
                };
                let row = on.conductor.current().map(String::from);
                let mut out = self.deliver(outs)?;
                out["row"] = json!(row);
                out
            }
            // #548: the relay's result of the vote the last row's question opened (`option`: its place from 0, as on the
            // wire) → the show's event (deck-show `vote::result_event`) → the row set to it fires, its cell to this deck
            "vote" => {
                let now = self.show_now();
                let on = self.show.as_mut().ok_or_else(|| Fault::Error("vote before show".into()))?;
                let question = on.asked.take().ok_or_else(|| Fault::Error("vote before a row asked a question".into()))?;
                let option = step.get("option").and_then(|v| v.as_integer()).unwrap_or(0);
                let result = protocol::VoteResult { vote: format!("v{}", on.votes), option, counts: vec![], how: protocol::VoteHow::Rule, groups: None, story: None };
                let event = deck_show::vote::result_event(&question, &result).ok_or_else(|| Fault::Error(format!("no option {option}")))?;
                let outs = on.conductor.on_event(&event, now);
                let row = on.conductor.current().map(String::from);
                let said = on.conductor.log().last().map(|e| deck_show::view::line(&e.what)).unwrap_or_default();
                let mut out = self.deliver(outs)?;
                out["event"] = json!(event);
                out["row"] = json!(row);
                out["said"] = json!(said);
                out
            }
            // #474: the one resolver – opinions given (`set`: strength, id, value; `call`: caller, value, start, length;
            // `clear`: strength, id), then a key asked at `now`: the value, where it came from, what it falls back to
            "resolve" => {
                let rows = |k: &str| step.get(k).and_then(|v| v.as_array()).cloned().unwrap_or_default();
                let word = |t: &toml::Value, k: &str| t.get(k).and_then(|v| v.as_str()).unwrap_or_default().to_string();
                let time = |t: &toml::Value, k: &str| t.get(k).and_then(|v| v.as_float().or_else(|| v.as_integer().map(|i| i as f64))).unwrap_or(0.0);
                let source = |t: &toml::Value| {
                    let s = word(t, "strength");
                    Strength::parse(&s).map(|st| Source::new(st, &word(t, "id"))).ok_or_else(|| Fault::Error(format!("no strength '{s}'")))
                };
                let key = text("key").ok_or_else(|| Fault::Error("a resolve step without `key`".into()))?;
                for t in rows("set") {
                    let value = t.get("value").map(to_json).unwrap_or(Value::Null);
                    self.opinions.set(&key, source(&t)?, value);
                }
                for t in rows("call") {
                    let value = t.get("value").map(to_json).unwrap_or(Value::Null);
                    self.opinions.call(&key, &word(&t, "caller"), value, time(&t, "start"), time(&t, "length"));
                }
                for t in rows("clear") {
                    self.opinions.clear(&key, &source(&t)?);
                }
                let one = |f: Opinion<'_, Value>| json!({ "value": f.value, "origin": f.origin.to_string() });
                match self.opinions.resolve(&key, num("now", 0.0)) {
                    Some(r) => json!({ "found": true, "value": r.value, "origin": r.origin.to_string(), "fallback": r.fallback.map(one) }),
                    None => json!({ "found": false }),
                }
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

/// #492: a resolved scene document's stage as facts – each member's look (#495: its form, the core's cast.rs, as the decks
/// draw it – its holder's choice over its look's own –; in its look, whether its model or its being draws it; and a card's picture: scene-content's entry and field, a member
/// without art the default character's; a character's in its being's mood), the slots, the
/// notes, the location and (#512) the set on the space node with its file per rung at the node's quality
fn stage_facts(r: &deck_space::scene_resolve::Resolved, content: &deck_space::push::Content) -> Value {
    use deck_space::cast::{self, Form, Picture, Who};
    let word = |n: &deck_space::scene::NodeDef, k: &str| n.params.get(k).and_then(|v| v.as_str()).map(String::from);
    let stage_3d = cast::stage_3d(&r.scene);
    let cast: Vec<Value> = r
        .scene
        .nodes
        .iter()
        .filter(|n| n.kind == "look")
        .map(|n| {
            let character = n.params.get("being").and_then(|b| b.get("character")).and_then(|v| v.as_str()).unwrap_or_default();
            let who = word(n, "who").as_deref().and_then(Who::parse);
            let look = who.as_ref().and_then(|w| content.look_form(w.character()));
            let form = who.as_ref().map(|w| cast::form(w, stage_3d, word(n, "form").as_deref().and_then(Form::parse), look));
            let picture = match (&who, form) {
                (Some(Who::Character(c)), Some(Form::Card)) => Some(Picture::character(content.drawn_as(c), cast::CARD_VIEW, "")),
                (Some(Who::Item(i)), Some(Form::Card)) => Some(Picture::item(i)),
                _ => None,
            }
            .map(|p| format!("{}.{}.{}", p.kind, p.id, p.field()));
            // a member in its look: drawn as its model when scene-content names one (a card says so by its form)
            let model = who.as_ref().is_some_and(|w| content.model_of(w.character()).is_some());
            let drawing = form.filter(|f| *f == Form::Look).map(|f| cast::drawing(f, model).word());
            let fact = json!({ "holder": word(n, "holder"), "look": n.id, "character": character, "spot": word(n, "spot"), "who": word(n, "who"), "form": form.map(Form::word), "drawing": drawing, "picture": picture });
            // what a member does not have is left out (a scenario's TOML has no null to expect)
            Value::Object(fact.as_object().into_iter().flatten().filter(|(_, v)| !v.is_null()).map(|(k, v)| (k.clone(), v.clone())).collect())
        })
        .collect();
    let space = r.scene.nodes.iter().find(|n| n.kind == "space");
    let set = space.and_then(|n| word(n, "set"));
    let quality = space.and_then(|n| word(n, "quality")).unwrap_or_else(|| deck_space::push::DEFAULT_QUALITY.to_string());
    let media = set.as_deref().and_then(|s| content.media(s, &quality)).map(|m| {
        let path = |f: Option<&deck_space::push::File>| f.map(|f| f.path.clone());
        let all = [("model", path(m.model)), ("video", path(m.video)), ("still", path(m.still))];
        Value::Object(all.into_iter().filter_map(|(k, p)| p.map(|p| (k.to_string(), json!(p)))).collect())
    });
    json!({ "cast": cast, "stage_3d": stage_3d, "slots": r.cast.iter().map(|m| m.slots.clone()).collect::<Vec<_>>(), "notes": r.notes, "location": r.location, "set": set, "media": media })
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

/// wasm: the questions and the .glb in the inbox's first `len` bytes (bones.rs) answered → the answer's length in the
/// outbox (< 0: an error text)
#[no_mangle]
pub extern "C" fn hl_bones(len: usize) -> i32 {
    IO.with(|io| {
        let mut io = io.borrow_mut();
        let (text, ok) = match bones::answer(&io.0[..len.min(io.0.len())]) {
            Ok(lines) => (lines, true),
            Err(e) => (e, false),
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

/// wasm: the outbox's address (its length is what `hl_run` or `hl_bones` returned)
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
        // no capability is missing today: caps.rs gives the unit test a stand-in
        let r = run(&b("expected_fail = \"#0 a stand-in\"\n[[step]]\ndo = \"test_missing\"\n"));
        assert_eq!(r.status, Status::Xfail, "{}", r.reason);
        assert!(r.reason.contains("missing capability #0"));
        // the wrong ticket, another failure, or a pass: red
        assert_eq!(run(&b("expected_fail = \"#365\"\n[[step]]\ndo = \"test_missing\"\n")).status, Status::Fail);
        assert_eq!(run(&b("expected_fail = \"#0\"\n[[step]]\ndo = \"nonsense\"\n")).status, Status::Fail);
        let passed = run(&b("expected_fail = \"#375\"\n[[step]]\ndo = \"reset\"\n"));
        assert_eq!(passed.status, Status::Fail);
        assert!(passed.reason.contains("remove the line"));
    }
}
