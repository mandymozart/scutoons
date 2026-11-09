//! The module node's engine side (#342) with native stand-ins for modules (`Fake`, behind the same `ModuleInstance` seam
//! a wasm instance is driven through – deck/engine/tests/modules_v2.rs runs the real ABI v2 fixture): channel slices over
//! a multichannel graph, control and path outputs one block late, a `domain: control` module in the control half, seeds,
//! events both ways at their frames, a show's restart through the loader lane.
use super::*;
use crate::{Build, Engine, Seams, Sequencing};
use deck_space::graph::Graph;
use deck_space::scene::Scene;
use std::sync::Mutex;

const RATE: f32 = 48_000.0;
const Q: usize = QUANTUM;

/// what one instance saw
#[derive(Default, Debug)]
struct Seen {
    seed: u64,
    /// (frame, frames) per call
    calls: Vec<(f64, u32)>,
    /// the channels audio input 0 carried, per call
    ch_in: Vec<usize>,
    /// control input 0 per call
    control: Vec<f32>,
    /// (the call's frame, the record) per event input record
    events: Vec<(f64, Ev)>,
    late: u32,
    offered: Vec<u32>,
    /// (mood, persona, mood_index, energy, touch, strain) per call – the context the module played in (#409)
    ctx: Vec<(String, String, u32, f32, f32, f32)>,
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum Plays {
    /// audio out c = in c × (c + 1); `level` = push × 2 every call; `drive` = 0.5 written at call 3 only
    Wide,
    /// `cut` = 100 + the call's number, every call (a control-domain module)
    Policy,
    /// call 1: a note on (frame 5, id 7, key 69, len 0.01 beats) and a `story.cue` (frame 3, level 0.25, x 0.75)
    Emitter,
    /// takes events
    Listener,
    /// call 1 traps; the restart's job returns 42; the swapped-in call takes the held notes again
    Restarting,
}

struct Fake {
    plays: Plays,
    seen: Arc<Mutex<Seen>>,
    ch_in: usize,
    audio_in: Vec<Vec<f32>>,
    audio_out: Vec<Vec<f32>>,
    control_in: f32,
    cout: Vec<(f32, bool)>,
    ev_in: Vec<Ev>,
    ev_out: [Vec<Ev>; 2],
    calls: u32,
    trapped: bool,
    job_out: bool,
    swap: bool,
    /// the header's character fields and senses as the engine set them
    mood: String,
    persona: String,
    mood_index: u32,
    senses: (f32, f32, f32),
}

struct Job(u32);

impl Restart for Job {
    fn run(self: Box<Self>) -> Box<dyn Any + Send> {
        Box::new(self.0)
    }
}

impl ModuleInstance for Fake {
    fn silenced(&self) -> bool {
        false
    }
    fn set_character(&mut self, _: &str, persona: &str, mood: &str) {
        (self.persona, self.mood) = (persona.to_string(), mood.to_string());
    }
    fn set_mood_index(&mut self, i: u32) {
        self.mood_index = i;
    }
    fn set_senses(&mut self, energy: f32, touch: f32, strain: f32) {
        self.senses = (energy, touch, strain);
    }
    fn set_vars(&mut self, _: &[(String, f32)]) {}
    fn set_tempo(&mut self, _: &TempoMap) {}
    fn set_param(&mut self, _: usize, _: f32) {}
    fn set_control_in(&mut self, _: usize, value: f32) {
        self.control_in = value;
    }
    fn set_audio_in(&mut self, port: usize, chans: &[&[f32]]) {
        if port != 0 {
            return;
        }
        self.ch_in = chans.len();
        for (c, s) in chans.iter().enumerate() {
            self.audio_in[c][..s.len()].copy_from_slice(s);
        }
    }
    fn process(&mut self, frame: f64, frames: u32) -> bool {
        let mut seen = self.seen.lock().unwrap();
        seen.calls.push((frame, frames));
        seen.ch_in.push(self.ch_in);
        seen.control.push(self.control_in);
        seen.ctx.push((self.mood.clone(), self.persona.clone(), self.mood_index, self.senses.0, self.senses.1, self.senses.2));
        for e in self.ev_in.drain(..) {
            seen.events.push((frame, e));
        }
        self.calls += 1;
        self.ev_out = [Vec::new(), Vec::new()];
        for c in &mut self.cout {
            c.1 = false;
        }
        let n = frames as usize;
        match self.plays {
            Plays::Wide => {
                for (c, out) in self.audio_out.iter_mut().enumerate() {
                    for (o, i) in out[..n].iter_mut().zip(&self.audio_in[c][..n]) {
                        *o = if c < self.ch_in { i * (c + 1) as f32 } else { 0.0 };
                    }
                }
                self.cout[0] = (self.control_in * 2.0, true);
                if self.calls == 3 {
                    self.cout[1] = (0.5, true);
                }
            }
            Plays::Policy => self.cout[0] = (100.0 + self.calls as f32, true),
            Plays::Emitter if self.calls == 1 => {
                self.ev_out[0].push(Ev::note_on(5, 7, f32::NAN, 69.0, 0.5).lasting(0.01));
                self.ev_out[1].push(Ev::event(3, 0).with(0, 0.25).with(1, 0.75));
            }
            Plays::Restarting if self.calls == 1 => {
                self.trapped = true;
                return false;
            }
            _ => {}
        }
        self.swap = false;
        true
    }
    fn read_audio_out(&mut self, port: usize, c: usize, dst: &mut [f32]) {
        match self.audio_out.get(c).filter(|_| port == 0) {
            Some(src) => dst.copy_from_slice(&src[..dst.len()]),
            None => dst.fill(0.0),
        }
    }
    fn control_out(&self, i: usize) -> (f32, bool) {
        self.cout.get(i).copied().unwrap_or((0.0, false))
    }
    fn set_events_in(&mut self, _port: usize, events: &[Ev]) -> usize {
        self.ev_in.extend_from_slice(events);
        events.len()
    }
    fn add_late(&mut self, n: u32) {
        self.seen.lock().unwrap().late += n;
    }
    fn events_out(&self, port: usize) -> &[Ev] {
        self.ev_out.get(port).map_or(&[], |v| v.as_slice())
    }
    fn swapping(&self) -> bool {
        self.swap
    }
    fn take_restart(&mut self) -> Option<Box<dyn Restart>> {
        (self.trapped && !std::mem::replace(&mut self.job_out, true)).then(|| Box::new(Job(42)) as Box<dyn Restart>)
    }
    fn offer(&mut self, done: Box<dyn Any + Send>) {
        if let Ok(v) = done.downcast::<u32>() {
            self.seen.lock().unwrap().offered.push(*v);
            self.swap = true;
        }
    }
}

/// the stand-in modules by name: descriptor JSON (ABI v2) and behaviour
const MODULES: [(&str, &str, Plays); 5] = [
    (
        "t_wide",
        r#"{"abi":2,"name":"t_wide","version":"1","params":[{"name":"p","min":0,"max":1,"default":0.5}],"ports":[
            {"name":"in","dir":"in","type":"audio","channels":"any"},{"name":"push","dir":"in","type":"control"},
            {"name":"out","dir":"out","type":"audio","channels":"any"},{"name":"level","dir":"out","type":"control"},
            {"name":"drive","dir":"out","type":"control","path":"f.gain"}],"moods":["calm","wild"]}"#,
        Plays::Wide,
    ),
    (
        "t_policy",
        r#"{"abi":2,"name":"t_policy","version":"1","domain":"control","params":[],"moods":["thinking","sad","wild"],"ports":[
            {"name":"push","dir":"in","type":"control"},{"name":"cut","dir":"out","type":"control"}]}"#,
        Plays::Policy,
    ),
    (
        "t_emitter",
        r#"{"abi":2,"name":"t_emitter","version":"1","domain":"control","params":[],"ports":[
            {"name":"echo","dir":"out","type":"event","names":["music.note"]},
            {"name":"cues","dir":"out","type":"event","names":["story.cue"],"props":["level","x"]}]}"#,
        Plays::Emitter,
    ),
    (
        "t_listener",
        r#"{"abi":2,"name":"t_listener","version":"1","params":[],"ports":[
            {"name":"notes","dir":"in","type":"event","names":["music.note"],"capacity":16},
            {"name":"cues","dir":"in","type":"event","names":["story.cue"],"props":["x","level"]}]}"#,
        Plays::Listener,
    ),
    (
        "t_restarting",
        r#"{"abi":2,"name":"t_restarting","version":"1","params":[],"ports":[
            {"name":"notes","dir":"in","type":"event","names":["music.note"]}]}"#,
        Plays::Restarting,
    ),
];

/// the stand-ins registered as node kinds, opened by kind; every instance's `Seen` in open order
#[derive(Default)]
struct Source {
    opened: Mutex<Vec<(String, Arc<Mutex<Seen>>)>>,
}

fn layouts() -> &'static [(ModuleKind, Layout, Plays)] {
    static L: std::sync::OnceLock<Vec<(ModuleKind, Layout, Plays)>> = std::sync::OnceLock::new();
    L.get_or_init(|| {
        MODULES
            .iter()
            .map(|(_, json, plays)| {
                let d: deck_catalog::Descriptor = serde_json::from_str(json).unwrap();
                let l = Layout::of(&d).unwrap();
                (register(&d, &l), l, *plays)
            })
            .collect()
    })
}

impl ModuleSource for Source {
    fn open(&self, kind: ModuleKind, _rate: f64, seed: u64) -> Result<Opened> {
        let (_, layout, plays) = layouts().iter().find(|(k, ..)| *k == kind).ok_or_else(|| anyhow!("not a stand-in"))?;
        let seen = Arc::new(Mutex::new(Seen { seed, ..Seen::default() }));
        self.opened.lock().unwrap().push((kind.spec().kind.to_string(), seen.clone()));
        let controls = layout.outputs.iter().filter(|o| matches!(o, OutBind::Control(_))).count();
        let fake = Fake {
            plays: *plays,
            seen,
            ch_in: 0,
            audio_in: vec![vec![0.0; Q]; MAX_CHANNELS],
            audio_out: vec![vec![0.0; Q]; 8],
            control_in: 0.0,
            cout: vec![(0.0, false); controls],
            ev_in: Vec::new(),
            ev_out: [Vec::new(), Vec::new()],
            calls: 0,
            trapped: false,
            job_out: false,
            swap: false,
            mood: String::new(),
            persona: String::new(),
            mood_index: 0,
            senses: (0.0, 0.0, 0.0),
        };
        let params = layout.inputs.iter().filter(|b| matches!(b, InBind::Param(_))).count();
        Ok(Opened { instance: Box::new(fake), layout: layout.clone(), params })
    }
}

impl Source {
    /// what the `i`-th opened instance saw
    fn seen(&self, i: usize) -> Arc<Mutex<Seen>> {
        self.opened.lock().unwrap()[i].1.clone()
    }
}

/// `text` built on a `channels`-channel device with the stand-ins, the hosts' generators in the block
fn build(text: &str, channels: usize) -> (Engine, crate::Handles, Arc<Source>) {
    layouts();
    let g = Graph::build(&Scene::parse(text).unwrap()).unwrap();
    let source = Arc::new(Source::default());
    let mut seams = Seams::with_deck(crate::DeckConfig::device(channels));
    seams.modules = Some(source.clone());
    let (e, h) = Engine::build_seams(&g, RATE, 1.0, Build::patcher(), Sequencing::InBlock, &seams).unwrap();
    (e, h, source)
}

/// one quantum on a `channels`-channel device, interleaved
fn quantum(e: &mut Engine, channels: usize) -> Vec<f32> {
    let mut out = vec![0.0; Q * channels];
    e.render(&mut out, channels, |x| x);
    out
}

fn node(text: &str, id: &str) -> usize {
    Graph::build(&Scene::parse(text).unwrap()).unwrap().nodes.iter().position(|n| n.id == id).unwrap()
}

/// an oscillator spread onto five channels by a bus, through the module, onto a five-channel device
const WIDE: &str = "[[node]]\nid=\"o\"\nkind=\"osc\"\nfreq=440\ngain=0.02\n\
    [[node]]\nid=\"b\"\nkind=\"bus\"\nchannels=5\n\
    [[node]]\nid=\"m\"\nkind=\"module:t_wide\"\npush=0.25\n\
    [[node]]\nid=\"dev\"\nkind=\"out\"\n\
    [[link]]\nfrom=\"o\"\nto=\"b.in\"\nmap=[[1, 1], [1, 2], [1, 3], [1, 4], [1, 5]]\n\
    [[link]]\nfrom=\"b\"\nto=\"m.in\"\n\
    [[link]]\nfrom=\"m.out\"\nto=\"dev.in\"\n\
    [[node]]\nid=\"f\"\nkind=\"osc\"\ngain=0.3\n[[link]]\nfrom=\"m.level\"\nto=\"f.freq\"\n";

#[test]
fn a_module_takes_and_gives_channel_slices_of_a_multichannel_graph() {
    let (mut e, _h, src) = build(WIDE, 5);
    let mut heard = 0;
    for _ in 0..8 {
        let out = quantum(&mut e, 5);
        for f in out.chunks(5).filter(|f| f[0].abs() > 1e-4) {
            for c in 1..5 {
                assert!((f[c] / f[0] - (c + 1) as f32).abs() < 1e-3, "channel {c} = in × {}: {f:?}", c + 1);
            }
            heard += 1;
        }
    }
    assert!(heard > 100, "the module's five channels reach the device ({heard} frames)");
    let seen = src.seen(0);
        let seen = seen.lock().unwrap();
    assert!(seen.ch_in.iter().all(|&c| c == 5), "the cable's five channels go in: {:?}", seen.ch_in);
    assert_eq!(seen.calls[..3], [(0.0, Q as u32), (Q as f64, Q as u32), (2.0 * Q as f64, Q as u32)], "one call a quantum, at its frame");
}

#[test]
fn control_and_path_outputs_act_one_block_late_and_hold() {
    let (mut e, _h, _src) = build(WIDE, 5);
    let f = node(WIDE, "f");
    let mut level = Vec::new();
    let mut gain = Vec::new();
    for _ in 0..6 {
        quantum(&mut e, 5);
        level.push(e.control().input(f, "freq", -1.0));
        gain.push(e.control().input(f, "gain", -1.0));
    }
    // `level` (a cable) = push × 2, written from the first call on: its consumer reads it from the second block on
    assert_eq!(level, [0.0, 0.5, 0.5, 0.5, 0.5, 0.5]);
    // `drive` (path `f.gain`, set): the target's own 0.3 until the module writes 0.5 in its third call, then held
    assert_eq!(gain, [0.3, 0.3, 0.3, 0.5, 0.5, 0.5]);
}

const POLICY: &str = "[[node]]\nid=\"p\"\nkind=\"module:t_policy\"\npush=0.7\n\
    [[node]]\nid=\"o\"\nkind=\"osc\"\n[[node]]\nid=\"out\"\nkind=\"audio_out\"\n\
    [[link]]\nfrom=\"p.cut\"\nto=\"o.freq\"\n[[link]]\nfrom=\"o\"\nto=\"out.in\"\n";

#[test]
fn a_control_module_steps_in_the_control_half() {
    let (mut e, _h, src) = build(POLICY, 2);
    assert_eq!(e.shape().0, 2, "not an audio step: the osc and the sink only");
    let o = node(POLICY, "o");
    let mut freq = Vec::new();
    for _ in 0..4 {
        quantum(&mut e, 2);
        freq.push(e.control().input(o, "freq", -1.0));
    }
    // call k writes 100 + k; the oscillator reads it at the next block
    assert_eq!(freq, [0.0, 101.0, 102.0, 103.0]);
    let seen = src.seen(0);
        let seen = seen.lock().unwrap();
    assert_eq!(seen.calls, [(0.0, Q as u32), (Q as f64, Q as u32), (2.0 * Q as f64, Q as u32), (3.0 * Q as f64, Q as u32)], "once per block");
    assert!(seen.control.iter().all(|&v| v == 0.7), "its control input from the control half's table: {:?}", seen.control);
}

/// #409: the deck's live context reaches every module node at the next block's start – the audio half's block and the
/// control half's stepper alike, through the face's command ring – each counting the mood in its own `moods` list (A1)
#[test]
fn the_live_context_reaches_every_module_node_at_the_next_block() {
    let text = format!("{WIDE}[[node]]\nid=\"p\"\nkind=\"module:t_policy\"\nmood=\"sad\"\npersona=\"day\"\n");
    let (mut e, mut h, src) = build(&text, 5);
    let g = Graph::build(&Scene::parse(&text).unwrap()).unwrap();
    let mut face = crate::control::Controls::new(&g, Some(&mut h));
    let ctx = |s: &Arc<Mutex<Seen>>| s.lock().unwrap().ctx.clone();
    let row = |m: &str, p: &str, i: u32, e: f32| (m.to_string(), p.to_string(), i, e, 0.25, 0.5);
    quantum(&mut e, 5);
    face.set_context(Context::new("wild", "night", 0.75, 0.25, 0.5));
    quantum(&mut e, 5);
    face.set_context(Context::new("sad", "night", 0.5, 0.25, 0.5));
    face.set_context(Context::new("sad", "night", 0.5, 0.25, 0.5)); // unchanged: not sent again
    quantum(&mut e, 5);
    face.set_context(Context::new("neutral", "", 0.0, 0.25, 0.5));
    quantum(&mut e, 5);
    let (wide, policy) = (ctx(&src.seen(0)), ctx(&src.seen(1)));
    // before the first context: the node's params (t_wide has none; t_policy's `sad` is its list's second)
    assert_eq!(wide[0], (String::new(), String::new(), 0, 0.0, 0.0, 0.0));
    assert_eq!(policy[0], ("sad".to_string(), "day".to_string(), 2, 0.0, 0.0, 0.0));
    // then the deck's, from the block after each send; each module counts in its own list, 0 = a mood it does not know
    assert_eq!(wide[1..], [row("wild", "night", 2, 0.75), row("sad", "night", 0, 0.5), row("neutral", "", 0, 0.0)]);
    assert_eq!(policy[1..], [row("wild", "night", 3, 0.75), row("sad", "night", 2, 0.5), row("neutral", "", 0, 0.0)]);
    assert_eq!(e.control().context().mood(), "neutral", "what the engine applied last");
}

#[test]
fn every_node_starts_with_seed_one_and_its_seed_param_changes_it() {
    let text = "[[node]]\nid=\"a\"\nkind=\"module:t_policy\"\n[[node]]\nid=\"b\"\nkind=\"module:t_policy\"\n\
                [[node]]\nid=\"c\"\nkind=\"module:t_policy\"\nseed=3\n";
    let (_e, _h, src) = build(text, 2);
    let seeds: Vec<u64> = (0..3).map(|i| src.seen(i).lock().unwrap().seed).collect();
    assert_eq!(seeds, [1, 1, 3]);
}

#[cfg(feature = "events")]
mod events {
    use super::*;
    use deck_events::{Event, Time};
    use deck_module_host::EvKind;

    const PAIR: &str = "[[node]]\nid=\"em\"\nkind=\"module:t_emitter\"\n[[node]]\nid=\"li\"\nkind=\"module:t_listener\"\n\
                        [[link]]\nfrom=\"em.echo\"\nto=\"li.notes\"\n[[link]]\nfrom=\"em.cues\"\nto=\"li.cues\"\n";
    /// a world node feeding the listener: MIDI notes, the MIDI adapter's name, on the `notes` input
    const WORLD_NOTES: &str = "[[node]]\nid=\"w\"\nkind=\"world\"\nsource=\"midi\"\nlisten=[\"midi.note\"]\n\
                               [[node]]\nid=\"li\"\nkind=\"module:t_listener\"\n[[link]]\nfrom=\"w.events\"\nto=\"li.notes\"\n";

    #[test]
    fn module_events_reach_another_module_one_block_late_at_their_frame_with_the_length_expanded() {
        let (mut e, _h, src) = build(PAIR, 2);
        for _ in 0..4 {
            quantum(&mut e, 2);
        }
        let seen = src.seen(1);
        let seen = seen.lock().unwrap();
        let q = Q as f64;
        let notes: Vec<(f64, u32, Option<EvKind>)> = seen.events.iter().filter(|(_, ev)| ev.kind() != Some(EvKind::Event)).map(|(at, ev)| (*at, ev.frame, ev.kind())).collect();
        // emitted at frame 5 of block 0 → frame 5 of block 1; 0.01 beats at 120 bpm = 240 frames → off at 128 + 5 + 240
        assert_eq!(notes, [(q, 5, Some(EvKind::NoteOn)), (2.0 * q, (Q + 5 + 240 - 2 * Q) as u32, Some(EvKind::NoteOff))]);
        let on = seen.events.iter().find(|(_, ev)| ev.kind() == Some(EvKind::NoteOn)).unwrap().1;
        assert_eq!((on.v[3], on.v[0], on.v[1]), (69.0, 440.0, 0.5), "the key it gave, the Hz filled from the tuning");
        let off = seen.events.iter().find(|(_, ev)| ev.kind() == Some(EvKind::NoteOff)).unwrap().1;
        assert_eq!(off.id, on.id, "on and off share the host's id");
        assert_ne!(on.id, 7, "re-tagged per link");
        // the named event: props by key – the listener's slot 0 is `x`, slot 1 `level`
        let cue = seen.events.iter().find(|(_, ev)| ev.kind() == Some(EvKind::Event)).unwrap();
        assert_eq!((cue.0, cue.1.frame, cue.1.name, cue.1.v[0], cue.1.v[1]), (q, 3, 0, 0.75, 0.25));
    }

    /// #430: the event host joins by cable: two sequencer + synth pairs in one graph stay apart, a listener with no cable
    /// hears nothing, and a module's outputs stay out of the dispatcher's by-name routes
    #[test]
    fn two_pairs_stay_apart_and_an_input_with_no_cable_hears_nothing() {
        let two = "[[node]]\nid=\"a\"\nkind=\"module:t_emitter\"\n[[node]]\nid=\"la\"\nkind=\"module:t_listener\"\n\
                   [[node]]\nid=\"lone\"\nkind=\"module:t_listener\"\n[[node]]\nid=\"b\"\nkind=\"module:t_emitter\"\n\
                   [[node]]\nid=\"lb\"\nkind=\"module:t_listener\"\n[[link]]\nfrom=\"a.echo\"\nto=\"la.notes\"\n[[link]]\nfrom=\"b.echo\"\nto=\"lb.notes\"\n";
        let (mut e, h, src) = build(two, 2);
        assert!(!h.events.has_routes(), "no cable names the world, so no route exists");
        for _ in 0..4 {
            quantum(&mut e, 2);
        }
        // the listeners are opened 2nd, 3rd and 5th: a's, the lone one, b's
        let notes = |i: usize| src.seen(i).lock().unwrap().events.iter().filter(|(_, ev)| ev.kind() == Some(EvKind::NoteOn)).count();
        assert_eq!((notes(1), notes(2), notes(4)), (1, 0, 1), "each its own emitter's one note, and silence for the uncabled");
    }

    /// #430: an event output cabled to two inputs reaches both; one input takes two cables
    #[test]
    fn event_cables_are_many_to_many() {
        let text = "[[node]]\nid=\"a\"\nkind=\"module:t_emitter\"\n[[node]]\nid=\"b\"\nkind=\"module:t_emitter\"\n\
                    [[node]]\nid=\"l1\"\nkind=\"module:t_listener\"\n[[node]]\nid=\"l2\"\nkind=\"module:t_listener\"\n\
                    [[link]]\nfrom=\"a.echo\"\nto=\"l1.notes\"\n[[link]]\nfrom=\"a.echo\"\nto=\"l2.notes\"\n[[link]]\nfrom=\"b.echo\"\nto=\"l2.notes\"\n";
        let (mut e, _h, src) = build(text, 2);
        for _ in 0..4 {
            quantum(&mut e, 2);
        }
        let notes = |i: usize| src.seen(i).lock().unwrap().events.iter().filter(|(_, ev)| ev.kind() == Some(EvKind::NoteOn)).count();
        assert_eq!((notes(2), notes(3)), (1, 2));
    }

    /// #430: a `generator` world node runs its patch and sends its notes down its cables only; a MIDI world node's cable
    /// joins the dispatcher (its route), and without a cable the same event name reaches nobody
    #[test]
    fn a_generator_world_node_plays_down_its_cables_only() {
        const GENERATOR: &str = "source=\"generator\"\nlisten=[\"music.note\"]\npatch = { out = \"g.notes\", generator = [{ id = \"g\", kind = \"random_walk\", seed = 3, spread = 1.0, rate = 16.0 }] }\n";
        let text = format!(
            "[[node]]\nid=\"w\"\nkind=\"world\"\n{GENERATOR}[[node]]\nid=\"li\"\nkind=\"module:t_listener\"\n\
             [[node]]\nid=\"off\"\nkind=\"module:t_listener\"\n[[link]]\nfrom=\"w.events\"\nto=\"li.notes\"\n"
        );
        let (mut e, h, src) = build(&text, 2);
        assert!(!h.events.has_routes(), "a generator goes straight down its cables");
        for _ in 0..64 {
            quantum(&mut e, 2);
        }
        let seen = src.seen(0);
        let seen = seen.lock().unwrap();
        let ons = seen.events.iter().filter(|(_, ev)| ev.kind() == Some(EvKind::NoteOn)).count();
        assert!(ons > 0, "the generator's notes arrive");
        assert!(seen.events.iter().any(|(_, ev)| ev.kind() == Some(EvKind::NoteOff)), "their offs follow from their lengths");
        assert!(src.seen(1).lock().unwrap().events.is_empty(), "the uncabled listener hears nothing");
    }

    #[test]
    fn module_events_go_out_to_the_world_as_deck_events_with_their_duration() {
        let (mut e, mut h, _src) = build(PAIR, 2);
        quantum(&mut e, 2);
        let mut outbox = h.events.take_outbox();
        assert_eq!(outbox.len(), 2, "one ring per event output");
        let note = outbox[0].pop().expect("the note");
        let names = &h.events.names;
        assert_eq!(names.name(note.name), "music.note");
        // 120 bpm at 48 kHz: frame 5 is beat 5 / 24 000
        let Time::Beats(start) = note.start else { panic!("on the beat: {:?}", note.start) };
        assert!((start - 5.0 / 24_000.0).abs() < 1e-12, "{start}");
        assert_eq!(note.dur, Some(Time::Beats(f64::from(0.01f32))));
        let get = |k: &str| note.props.get(names.get(k).unwrap());
        assert_eq!((get("pitch"), get("key"), get("vel")), (Some(440.0), Some(69.0), Some(0.5)));
        let cue = outbox[1].pop().expect("the cue");
        assert_eq!(names.name(cue.name), "story.cue");
        assert_eq!((cue.props.get(names.get("level").unwrap()), cue.props.get(names.get("x").unwrap())), (Some(0.25), Some(0.75)));
    }

    /// #442: a far world node's cable gets a ring of its own, handed out with its address and filter; what the link
    /// pushes there plays like a MIDI world node's events – and the outbox rings come with their slots
    #[test]
    fn a_far_world_node_has_its_own_ring_and_the_outbox_names_its_slots() {
        let text = "[[node]]\nid=\"w\"\nkind=\"world\"\nsource=\"far\"\naddress=\"/deck:left/scene:nave/character:catpope/analog:seq.notes\"\nlisten=[\"music.note\"]\n\
                    [[node]]\nid=\"li\"\nkind=\"module:t_listener\"\n[[node]]\nid=\"e\"\nkind=\"module:t_emitter\"\n[[link]]\nfrom=\"w.events\"\nto=\"li.notes\"\n";
        let (mut e, mut h, src) = build(text, 2);
        assert!(!h.events.has_routes(), "a far node joins no dispatcher route");
        let mut far = h.events.take_far();
        assert_eq!(far.len(), 1);
        assert_eq!((far[0].0.as_str(), far[0].1.as_slice()), ("/deck:left/scene:nave/character:catpope/analog:seq.notes", &["music.note".to_string()][..]));
        let slots: Vec<String> = h.events.take_outbox_slots().into_iter().map(|(s, _)| s).collect();
        assert_eq!(slots, ["e.echo", "e.cues"], "each outbox ring with its slot");
        let note = h.events.names.get("music.note").unwrap();
        let pitch = h.events.names.get("pitch").unwrap();
        far[0].2.push(Event::now(note).lasting(Time::Seconds(0.001)).with(pitch, 440.0)).unwrap();
        for _ in 0..3 {
            quantum(&mut e, 2);
        }
        let seen = src.seen(0);
        let seen = seen.lock().unwrap();
        let kinds: Vec<Option<EvKind>> = seen.events.iter().map(|(_, ev)| ev.kind()).collect();
        assert_eq!(kinds, [Some(EvKind::NoteOn), Some(EvKind::NoteOff)], "the far note plays at once, its off from its length");
        assert_eq!(seen.events[0].1.frame, 0);
    }

    #[test]
    fn world_events_land_on_their_frame_with_the_off_from_their_duration_and_late_ones_at_zero() {
        let (mut e, mut h, src) = build(WORLD_NOTES, 2);
        let routing = h.events.take_routing();
        let note = h.events.names.get("midi.note").unwrap();
        let pitch = h.events.names.get("pitch").unwrap();
        let mut outlets = routing.outlets;
        let send = |outlets: &mut Vec<rtrb::Producer<Event>>, ev: Event| {
            for &t in routing.table.targets(ev.name) {
                outlets[t as usize].push(ev.clone()).unwrap();
            }
        };
        // 0.01 beats = 240 frames: block 1, frame 112; lasting 0.01 beats: the off at 480 = block 3, frame 96
        send(&mut outlets, Event::beats(note, 0.01).lasting(Time::Beats(0.01)).with(pitch, 440.0));
        for _ in 0..4 {
            quantum(&mut e, 2);
        }
        // a note already past when it arrives plays at frame 0 and is counted late
        send(&mut outlets, Event::beats(note, 0.0).with(pitch, 220.0));
        quantum(&mut e, 2);
        let seen = src.seen(0);
        let seen = seen.lock().unwrap();
        let got: Vec<(f64, u32, Option<EvKind>, f32)> = seen.events.iter().map(|(at, ev)| (*at, ev.frame, ev.kind(), ev.v[3])).collect();
        let q = Q as f64;
        assert_eq!(got[0], (q, 112, Some(EvKind::NoteOn), 69.0), "the key filled from the Hz");
        assert_eq!((got[1].0, got[1].1, got[1].2), (3.0 * q, 96, Some(EvKind::NoteOff)));
        assert_eq!((got[2].0, got[2].1, got[2].2, got[2].3), (4.0 * q, 0, Some(EvKind::NoteOn), 57.0));
        assert_eq!(seen.events[0].1.v[1], 1.0, "a velocity the source lacks is 1");
        assert_eq!(seen.late, 1);
    }

    #[test]
    fn a_show_restart_goes_through_the_loader_lane_and_gets_the_held_notes_again() {
        let (mut e, mut h, src) = build("[[node]]\nid=\"w\"\nkind=\"world\"\nsource=\"midi\"\nlisten=[\"midi.note\"]\n[[node]]\nid=\"r\"\nkind=\"module:t_restarting\"\n[[link]]\nfrom=\"w.events\"\nto=\"r.notes\"\n", 2);
        let routing = h.events.take_routing();
        let note = h.events.names.get("midi.note").unwrap();
        let mut outlets = routing.outlets;
        for &t in routing.table.targets(note) {
            outlets[t as usize].push(Event::new(note, Time::Seconds(10.0 / f64::from(RATE)))).unwrap();
        }
        quantum(&mut e, 2); // the note on at frame 10, held; the call traps; the rebuild goes out
        assert_eq!(h.restarts.len(), 1);
        assert_eq!(h.restarts[0].serve(), 1, "the platform's loader runs the rebuild");
        assert_eq!(h.restarts[0].serve(), 0);
        quantum(&mut e, 2); // swapped in: the held note again at frame 0
        let seen = src.seen(0);
        let seen = seen.lock().unwrap();
        assert_eq!(seen.offered, [42]);
        let ons: Vec<(f64, u32)> = seen.events.iter().filter(|(_, ev)| ev.kind() == Some(EvKind::NoteOn)).map(|(at, ev)| (*at, ev.frame)).collect();
        assert_eq!(ons, [(0.0, 10), (Q as f64, 0)]);
        assert_eq!(seen.events[0].1.id, seen.events[1].1.id, "the same note");
    }
}
