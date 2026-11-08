//! The MIDI clock's test bench on one PC (#321) – a loopback MIDI port (loopMIDI on Windows, `snd-virmidi` on Linux)
//! instead of an Elektron, the relay to see what the show makes of it. Both directions:
//!
//!   cargo run --release -p deck-native --example midi-clock -- ports
//!   cargo run --release -p deck-native --example midi-clock -- send  "loopMIDI" ws://127.0.0.1:7070/live/CLOCK
//!   cargo run --release -p deck-native --example midi-clock -- listen "loopMIDI" ws://127.0.0.1:7070/live/CLOCK [seconds]
//!
//! `send` plays an Elektron's clock into the port (start, 120 BPM, a ramp to 140, a dropped tick, stop and silence – it
//! sends clock only while playing –, song position + continue, the cable pulled for 2 s and plugged back without a
//! start) while a deck follows that port as the show's leader (a scene with `source = "midi"`); it watches the relay's
//! `tempo` anchors and says how far each is off the clock it sent (the same PC, the same clock – the relay's ms).
//! `listen` reads a clock out from the port (a deck's `midi_out`, the leader on the relay) and says how far each tick
//! is off the leader's anchor, its tempo and its transport.
use deck_native::live::config::{dial, timed_out};
use midir::{Ignore, MidiInput, MidiOutput};
use protocol::{Envelope, Hello, Message, Role};
use std::sync::mpsc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tungstenite::Message as Ws;

const PPQN: f64 = 24.0;

/// the relay's clock: ms since the epoch
fn now_ms() -> f64 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs_f64() * 1e3
}

fn sleep_until(ms: f64) {
    loop {
        let left = ms - now_ms();
        if left <= 0.0 {
            return;
        }
        if left > 2.0 {
            std::thread::sleep(Duration::from_secs_f64((left - 1.5) / 1e3));
        } else {
            std::hint::spin_loop();
        }
    }
}

/// the relay's anchors (`tempo`) as they arrive: (at ms, beat, bpm, running)
fn watch(relay: String, tx: mpsc::Sender<(f64, f64, f64, bool)>) {
    std::thread::spawn(move || {
        let Some((base, code)) = relay.rsplit_once("/live/").map(|(b, c)| (b.to_string(), c.to_string())) else {
            eprintln!("relay URL must be ws://host:port/live/CODE");
            return;
        };
        let url = format!("{base}/live/{code}?role=output&group=clock-check");
        let mut ws = match dial(&url, Duration::from_millis(100)) {
            Ok(ws) => ws,
            Err(e) => return eprintln!("relay: {e}"),
        };
        let hello = Message::Hello(Hello { code, role: Role::Output, group: Some("clock-check".into()), nick: Some("midi-clock".into()), token: Some("dev".into()), resume: None });
        let _ = ws.send(Ws::Text(serde_json::to_string(&Envelope::new("", hello)).unwrap().into()));
        loop {
            match ws.read() {
                Ok(Ws::Text(t)) => {
                    let Ok(e) = serde_json::from_str::<Envelope>(&t) else { continue };
                    let tempo = match e.message {
                        Message::Tempo(t) => Some(t),
                        Message::Welcome(w) => w.snapshot.tempo,
                        _ => None,
                    };
                    if let Some(t) = tempo.filter(|t| t.at.is_some()) {
                        let _ = tx.send((t.at.unwrap() as f64, t.beat, t.bpm, t.running));
                    }
                }
                Ok(_) => {}
                Err(tungstenite::Error::Io(e)) if timed_out(&e) => {}
                Err(e) => return eprintln!("relay: {e}"),
            }
        }
    });
}

fn pick_out(want: &str) -> midir::MidiOutputConnection {
    let out = MidiOutput::new("midi-clock").unwrap();
    let port = out.ports().into_iter().find(|p| out.port_name(p).unwrap_or_default().to_lowercase().contains(&want.to_lowercase())).expect("no such MIDI out port (try `ports`)");
    println!("out: {}", out.port_name(&port).unwrap());
    out.connect(&port, "midi-clock").unwrap()
}

fn stats(name: &str, mut v: Vec<f64>) {
    if v.is_empty() {
        return println!("{name}: none");
    }
    let n = v.len();
    let mean = v.iter().sum::<f64>() / n as f64;
    v.iter_mut().for_each(|x| *x = x.abs());
    v.sort_by(|a, b| a.partial_cmp(b).unwrap());
    println!("{name}: n {n} · mean {mean:+.2} ms · |p50| {:.2} · |p95| {:.2} · |max| {:.2} ms", v[n / 2], v[(n * 95 / 100).min(n - 1)], v[n - 1]);
}

/// the clock to send: every message's time (ms from the start) and bytes, and the true beat at any time
struct Script {
    events: Vec<(f64, Vec<u8>)>,
    /// (ms, beat, running) at every tick moment – sent or not – and every transport change
    truth: Vec<(f64, f64, bool)>,
}

impl Script {
    fn elektron() -> Script {
        let mut s = Script { events: vec![], truth: vec![] };
        let mut t = 1000.0; // ms after the start: a second to settle
        let mut tick = 0i64;
        // `beats` beats of ticks at `bpm(beat)`, ticks for which `skip(beat)` holds not sent
        let run = |s: &mut Script, t: &mut f64, tick: &mut i64, beats: f64, bpm: &dyn Fn(f64) -> f64, skip: &dyn Fn(f64) -> bool| {
            for _ in 0..(beats * PPQN) as i64 {
                let b = *tick as f64 / PPQN;
                if !skip(b) {
                    s.events.push((*t, vec![0xF8]));
                }
                s.truth.push((*t, b, true));
                *tick += 1;
                *t += 60_000.0 / bpm(b) / PPQN;
            }
        };
        s.events.push((t - 2.0, vec![0xFA]));
        run(&mut s, &mut t, &mut tick, 16.0, &|_| 120.0, &|_| false); // 8 s at 120
        run(&mut s, &mut t, &mut tick, 12.0, &|b| 120.0 + (b - 16.0) / 12.0 * 20.0, &|_| false); // a ramp to 140
        run(&mut s, &mut t, &mut tick, 12.0, &|_| 140.0, &|b| (b - 34.0).abs() < 0.01); // one dropped tick at beat 34
        // stop on beat 40, silence 3 s (no clock while stopped)
        s.events.push((t - 2.0, vec![0xFC]));
        s.truth.push((t - 2.0, tick as f64 / PPQN, false));
        t += 3000.0;
        // song position beat 48 (bar 13 = 192 sixteenths) + continue
        s.events.push((t - 4.0, vec![0xF2, (192 & 0x7F) as u8, (192 >> 7) as u8]));
        s.events.push((t - 2.0, vec![0xFB]));
        tick = 48 * PPQN as i64;
        s.truth.push((t - 2.0, 48.0, true));
        run(&mut s, &mut t, &mut tick, 12.0, &|_| 140.0, &|_| false);
        // the cable pulled for 2 s (the clock plays on), plugged back – no start
        run(&mut s, &mut t, &mut tick, 16.0, &|_| 140.0, &|b| (60.0..60.0 + 140.0 / 30.0).contains(&b));
        s.events.push((t - 2.0, vec![0xFC]));
        s.truth.push((t - 2.0, tick as f64 / PPQN, false));
        s
    }

    /// the true beat at `ms` (from the start) and whether it runs
    fn beat(&self, ms: f64) -> Option<(f64, bool)> {
        let i = self.truth.partition_point(|&(t, ..)| t <= ms);
        let &(t0, b0, run) = self.truth.get(i.checked_sub(1)?)?;
        if !run {
            return Some((b0, false));
        }
        let &(t1, b1, _) = self.truth.get(i)?;
        Some((b0 + (b1 - b0) * (ms - t0) / (t1 - t0), true))
    }
}

fn send(port: &str, relay: Option<String>) {
    let mut out = pick_out(port);
    let (tx, rx) = mpsc::channel();
    if let Some(r) = relay {
        watch(r, tx);
    }
    let s = Script::elektron();
    let start = now_ms() + 500.0;
    println!("sending {:.0} s of an Elektron's clock: start · 120 · ramp to 140 · a dropped tick · stop + 3 s silence · SPP + continue · 2 s cable out · stop", s.events.last().unwrap().0 / 1e3);
    for (ms, bytes) in &s.events {
        sleep_until(start + ms);
        let _ = out.send(bytes);
    }
    std::thread::sleep(Duration::from_millis(1500));
    // every anchor against the clock that was sent: how far its line is off at its moment
    let mut off = vec![];
    let mut late = vec![];
    for (at, beat, bpm, running) in rx.try_iter() {
        let Some((b, run)) = s.beat(at - start) else { continue };
        if running != run {
            println!("  transport: an anchor says {} at {:.0} ms, the clock {}", if running { "running" } else { "stopped" }, at - start, if run { "runs" } else { "stopped" });
            continue;
        }
        let e = if running { (beat - b) / (bpm / 60.0) * 1e3 } else { (beat - b) / (140.0 / 60.0) * 1e3 };
        // the cable out: the leader holds the tempo, its anchors are no MIDI measurement
        if (60.0..64.67).contains(&b) {
            late.push(e);
        } else {
            off.push(e);
        }
    }
    stats("leader's anchors vs the MIDI clock", off);
    stats("  … while the cable was out (the held tempo)", late);
}

fn listen(port: &str, relay: Option<String>, seconds: f64) {
    let input = {
        let mut i = MidiInput::new("midi-clock").unwrap();
        i.ignore(Ignore::SysexAndActiveSense);
        i
    };
    let p = input.ports().into_iter().find(|p| input.port_name(p).unwrap_or_default().to_lowercase().contains(&port.to_lowercase())).expect("no such MIDI in port (try `ports`)");
    println!("in: {}", input.port_name(&p).unwrap());
    let (mtx, mrx) = mpsc::channel::<(f64, Vec<u8>)>();
    let _conn = input.connect(&p, "midi-clock", move |_, m, _| {
        let _ = mtx.send((now_ms(), m.to_vec()));
    }, ()).unwrap();
    let (tx, rx) = mpsc::channel();
    if let Some(r) = relay {
        watch(r, tx);
    }
    println!("listening {seconds} s …");
    std::thread::sleep(Duration::from_secs_f64(seconds));
    let anchors: Vec<_> = rx.try_iter().collect();
    let msgs: Vec<_> = mrx.try_iter().collect();
    let ticks: Vec<f64> = msgs.iter().filter(|(_, m)| m[0] == 0xF8).map(|(t, _)| *t).collect();
    for (t, m) in msgs.iter().filter(|(_, m)| m[0] != 0xF8) {
        println!("  {:>9.1} ms  {:02X?}", t - msgs[0].0, m);
    }
    let gaps: Vec<f64> = ticks.windows(2).map(|w| w[1] - w[0]).collect();
    if gaps.is_empty() {
        return println!("no clock");
    }
    let mean = gaps.iter().sum::<f64>() / gaps.len() as f64;
    println!("ticks {} · {:.2} BPM from their spacing", ticks.len(), 60_000.0 / mean / PPQN);
    stats("tick spacing − its mean", gaps.iter().map(|g| g - mean).collect());
    // each tick against the leader's newest anchor before it: whole ticks on the anchor's line
    let mut off = vec![];
    for &t in &ticks {
        let Some(&(at, beat, bpm, true)) = anchors.iter().rev().find(|a| a.0 <= t + 300.0) else { continue };
        let b = beat + (t - at) / 60_000.0 * bpm;
        let k = (b * PPQN).round() / PPQN;
        off.push((b - k) / (bpm / 60.0) * 1e3);
    }
    stats("ticks vs the leader's line (to the nearest tick)", off);
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("send") => send(args.get(1).map_or("loopMIDI", |s| s), args.get(2).cloned()),
        Some("listen") => listen(args.get(1).map_or("loopMIDI", |s| s), args.get(2).cloned(), args.get(3).and_then(|s| s.parse().ok()).unwrap_or(30.0)),
        _ => {
            let i = MidiInput::new("midi-clock").unwrap();
            let o = MidiOutput::new("midi-clock").unwrap();
            println!("in:  {:?}", i.ports().iter().map(|p| i.port_name(p).unwrap_or_default()).collect::<Vec<_>>());
            println!("out: {:?}", o.ports().iter().map(|p| o.port_name(p).unwrap_or_default()).collect::<Vec<_>>());
        }
    }
}
