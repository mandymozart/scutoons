//! #374: the HUD's lines and the bar's items for a scripted set of states, against tests/golden/hud.txt – the native
//! HUD's output byte for byte (checked against main.rs's own builders in a differential run before they left it).
//! `DECK_UI_GOLDEN=write cargo test -p deck-ui --test golden` writes the file anew (a change of wording is a reviewed diff).
use deck_core::monitor::{AudioWindow, Monitor, Report};
use deck_ui::{alert_row, bar_items, beat_row, hud_lines, Audio, Gpu, Quality, Source, Space, Status};

/// deck-core's monitor fed like an adapter feeds it: `frames` at `frame_ms` with `work_ms` each, underruns at `glitches_at_s`,
/// an audio window of `blocks` blocks at `load` of their budget
fn report(frame_ms: f64, frames: u32, work_ms: f32, glitches_at_s: &[f64], load: f32) -> Report {
    let mut m = Monitor::new();
    m.target(60.0);
    let mut t = 0.0;
    for _ in 0..=frames {
        m.frame(t, work_ms, true);
        t += frame_ms;
    }
    let mut totals = [0u64; 3];
    for at in glitches_at_s {
        totals[2] += 1;
        m.glitches(t.min(at * 1000.0), totals);
    }
    if load > 0.0 {
        let mut w = AudioWindow::with_lead(512, 48_000.0);
        (0..375).for_each(|_| w.block(load, 1.0));
        w.limiter(-1.5);
        m.audio(t, w);
    }
    m.report(t)
}

fn states() -> Vec<(&'static str, Status)> {
    let source = |name: &str, status: &str, midi_out: bool| Source { name: name.into(), status: status.into(), midi_out };
    let boot = Status {
        scene: "cathedral".into(),
        report: report(16.667, 120, 3.0, &[], 0.0),
        tempo: "BPM 120.0  BAR 1 BEAT 1".into(),
        source: source("internal", "", false),
        ..Status::default()
    };
    let full = Status {
        scene: "show clock – lead".into(),
        gpu: Some(Gpu { name: "NVIDIA GeForce RTX 3060".into(), backend: "Vulkan".into() }),
        quality: Some(Quality { level: "high".into(), ceiling: "ultra".into(), auto: true, scale: 0.85, render: 0.85 }),
        audio: Some(Audio { name: "PipeWire".into(), rate: 48_000, buffer: 256, lead: 512, clock_rate: 48_000.0 }),
        report: report(16.667, 600, 4.0, &[9.5], 0.35),
        events: Some([1234, 2]),
        tempo: "BPM 120.0  BAR 2 BEAT 1 CLK".into(),
        source: source("midi", "LEADS", true),
        outputs: Some("dmx 2 lamps".into()),
        sync: Some("SYNC 2/5 SETS".into()),
        push: Some("PUSH CATH OK".into()),
        space: Some(Space { mood: "MOOD HAPPY < ENERGY 0.62".into(), line: "VOICE.INDEX 2.00".into(), session: vec!["DECK c1 @ CATH".into(), "FACE plf 0/5 MOODS".into()], joined: true }),
        fft: Some([0.5, 0.25, 0.125, 0.0625]),
        music: vec!["track 1 playing".into()],
        slices: vec!["1 / 12 ready".into()],
        ..Status::default()
    };
    let broken = Status {
        error: Some("this build of the stage does not carry the module 'space' (node space) – build deck-native with --features space".into()),
        live: vec!["LIVE CATH – NO RELAY YET".into()],
        live_note: Some("the outlet has no relay — set one in deck.toml".into()),
        scene: "broken".into(),
        gpu: Some(Gpu { name: "Intel(R) UHD Graphics".into(), backend: "Dx12".into() }),
        quality: Some(Quality { level: "medium".into(), ceiling: "high".into(), auto: false, scale: 0.7, render: 0.7 }),
        report: report(40.0, 100, 20.0, &[], 0.9),
        tempo: "BPM 98.5  BAR 7 BEAT 3 STOP".into(),
        source: source("relay", "SHOW CLOCK SYNCING", false),
        outputs_note: Some("no DMX interface – plug one in…".into()),
        sync: Some("SYNC OK 5/5".into()),
        space: Some(Space { mood: "MOOD CALM".into(), line: "SPACE".into(), session: vec!["ALONE".into()], joined: false }),
        ..Status::default()
    };
    let outlet = Status {
        live: vec!["LIVE CATH – CELL 3 OF 9".into(), "MEDIA 4/4".into()],
        scene: "outlet".into(),
        report: report(16.667, 60, 2.0, &[], 0.0),
        tempo: "BPM 120.0  BAR 1 BEAT 2".into(),
        source: source("link", "3 PEERS", false),
        ..Status::default()
    };
    // the lower lines (the full HUD cuts them at 13): the outputs, the sync, the push, the space; the bands, music, slices
    let stage = Status {
        scene: "cathedral".into(),
        report: report(16.667, 60, 2.0, &[], 0.0),
        tempo: "BPM 84.0  BAR 3 BEAT 4".into(),
        source: source("internal", "", false),
        outputs: Some("dmx 2 lamps".into()),
        sync: Some("SYNC 2/5 SETS".into()),
        push: Some("PUSH CATH OK".into()),
        space: Some(Space { mood: "MOOD HAPPY < ENERGY 0.62".into(), line: "VOICE.INDEX 2.00".into(), session: vec!["DECK c1 @ CATH".into()], joined: false }),
        ..Status::default()
    };
    let media = Status {
        scene: "madness".into(),
        report: report(16.667, 60, 2.0, &[], 0.0),
        tempo: "BPM 96.0  BAR 2 BEAT 2".into(),
        source: source("internal", "", false),
        outputs_note: Some("no DMX interface – plug one in…".into()),
        fft: Some([0.891, 0.75, 0.805, 0.4849]),
        music: vec!["'calmness' 30 s (online, 3.2 s to load) – Leugnen".into()],
        slices: vec!["1 / 12 ready".into()],
        ..Status::default()
    };
    vec![("boot", boot), ("full", full), ("broken", broken), ("outlet", outlet), ("stage", stage), ("media", media)]
}

fn render() -> String {
    let mut out = String::new();
    for (name, s) in states() {
        let lines = hud_lines(&s);
        out.push_str(&format!("== {name} (alert row {:?}, beat row {:?})\n", alert_row(&lines), beat_row(&lines)));
        for l in &lines {
            out.push_str(&format!("L {l}\n"));
        }
        for i in bar_items(&s) {
            out.push_str(&format!("B {:?} {}\n", i.level, i.text));
        }
    }
    out
}

#[test]
fn the_hud_says_what_the_golden_says() {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/golden/hud.txt");
    let got = render();
    if std::env::var("DECK_UI_GOLDEN").as_deref() == Ok("write") {
        std::fs::write(path, &got).unwrap();
    }
    let want = std::fs::read_to_string(path).unwrap().replace("\r\n", "\n");
    assert_eq!(got, want, "the HUD's words changed – DECK_UI_GOLDEN=write and review the diff");
}
