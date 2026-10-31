//! Stand-in decks for the Cathedral of decks (#188 phase 4): `n` decks join a session as the bundled cathedral scene
//! (scenes/cathedral.toml, `join = "deck"`), each another character, and stream their beings – no window, no sound. Their
//! values move by themselves (hue, glow, the FM voice, the LFO) and each walks through its moods at its own pace, so the
//! conductor (`deck-stage --scene conductor --relay …`) has a congregation to show. Each deck's knobs sit elsewhere
//! (#188's tethered buttons: a deck turned up wide sprawls, a quiet one stays compact) and one of them turns slowly.
//!
//!   cargo run --release -p deck-stage --example decks -- ws://127.0.0.1:7700/live/CATH 4 [seconds]
use deck_stage::control::Controls;
use deck_stage::graph::{Graph, Kind};
use deck_stage::scene::{Scene, CATHEDRAL_SCENE};
use deck_stage::space::portrait::Portraits;
use deck_stage::space::{Connect, Space};
use deck_stage::tempo;
use std::path::Path;
use std::time::{Duration, Instant};

const CHARACTERS: [&str; 6] = ["catpope", "yellowmouse", "billthefrog", "crows", "catpope-biker", "catpope-human"];

fn main() {
    tracing_subscriber::fmt().with_env_filter(tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into())).init();
    let mut args = std::env::args().skip(1);
    let relay = args.next().unwrap_or_else(|| "ws://127.0.0.1:7700/live/CATH".into());
    let n: usize = args.next().and_then(|a| a.parse().ok()).unwrap_or(4);
    let seconds: f32 = args.next().and_then(|a| a.parse().ok()).unwrap_or(0.0);
    let mut decks = Vec::new();
    for i in 0..n {
        let mut scene = deck_stage::modules::carried_part(&Scene::parse(CATHEDRAL_SCENE).unwrap());
        scene.being.character = CHARACTERS[i % CHARACTERS.len()].into();
        let graph = Graph::build(&scene).unwrap();
        let node = graph.first(Kind::Space).unwrap();
        let nowhere = Path::new("/nowhere");
        let mut space = Space::new(&graph, node, &scene.being, nowhere, Portraits::local(nowhere));
        space.connect(Some(&relay), None, Some(format!("stand-in {}", i + 1)));
        let ctl = Controls::new(&graph, None);
        decks.push((graph, ctl, space));
    }
    println!("{n} stand-in decks on {relay}{}", if seconds > 0.0 { format!(" for {seconds} s") } else { " – Ctrl+C ends".into() });
    let start = Instant::now();
    let mut next = start;
    loop {
        let t = start.elapsed().as_secs_f32();
        if seconds > 0.0 && t > seconds {
            break;
        }
        let beat = tempo::Now { bpm: 84.0, pos: tempo::position(t as f64 * 1.4, 4), per_bar: 4, running: true };
        for (i, (graph, ctl, space)) in decks.iter_mut().enumerate() {
            let k = i as f32;
            let sp = graph.first(Kind::Space).unwrap();
            let set = |ctl: &mut Controls, node: usize, port: &str, v: f32| {
                if let Some(p) = graph.nodes[node].kind.input(port) {
                    ctl.set(node, p, v);
                }
            };
            ctl.update(1.0 / 60.0, 0, &beat);
            set(ctl, sp, "hue", (0.52 + k * 0.17 + 0.03 * (t * 0.2).sin()).fract());
            set(ctl, sp, "glow", 0.6 + 0.4 * (t * 0.7 + k).sin());
            if let Some(v) = graph.nodes.iter().position(|n| n.id == "voice") {
                set(ctl, v, "ratio", 1.5 + (t * 0.3 + k).sin());
            }
            // each deck its own settings: deck 0 quiet and compact … the last turned up wide; one knob turns slowly
            let wide = if n > 1 { k / (n - 1) as f32 } else { 0.5 };
            let at = |id: &str| graph.nodes.iter().position(|x| x.id == id);
            let knobs: [(&str, &str, f32); 7] = [
                ("voice", "index", 0.5 + 7.0 * wide),
                ("drone", "freq", 40.0 + 300.0 * wide),
                ("sway", "depth", 0.2 + 1.6 * (1.0 - wide)),
                ("mix", "gain1", 0.2 + 1.2 * wide),
                ("mix", "gain2", 1.3 - 1.1 * wide),
                ("mix", "gain4", 0.1 + 1.3 * wide),
                ("space", "light", 0.3 + 1.1 * wide),
            ];
            for (j, (id, port, v)) in knobs.iter().enumerate() {
                if let Some(node) = at(id) {
                    let slow = if j == i % knobs.len() { 0.35 * (t * 0.5).sin() } else { 0.0 };
                    set(ctl, node, port, v * (1.0 + slow));
                }
            }
            // each deck's energy: a mood of its own, changing every few seconds
            space.being.head.fixed = Some(((t / (3.0 + k)) as usize + i) % space.being.moods.len());
            space.update(1.0 / 60.0, t, ctl, &beat, &|n, p| 0.4 + 0.4 * ((t * 3.0 + (n * 3 + p) as f32).sin()));
        }
        next += Duration::from_millis(16);
        std::thread::sleep(next.saturating_duration_since(Instant::now()));
    }
}
