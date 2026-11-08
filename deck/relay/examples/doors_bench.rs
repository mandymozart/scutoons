//! The transport bench (#438): the load of examples/bench.rs – N decks with `deck.values` at 30 Hz to the conductor, M
//! phones getting the conductor's cues at 5 Hz – over either door, under whatever the kernel does to the path (`tc netem`
//! on lo, deck/relay/netem/run.sh). Both doors join one relay, so `--decks-via` and `--phones-via` may be lists (`wt,ws`,
//! round robin) and the report has each door's own numbers.
//!
//! `cargo run --release -p relay --example doors_bench -- [--via ws|wt] [--decks-via …] [--conductor-via …] [--phones-via …]
//!  [--decks 20] [--listeners 100] [--seconds 30] [--keys 0] [--label x] [--serve | --connect] [--count-egress]`
//!
//! An unreliable lane always looks fast (what is lost is not counted), so `hold` is reported too: how old a deck's value
//! has become when the next one replaces it – what the conductor really shows, comparable across TCP and UDP.
//!
//! `--serve` / `--connect`: the relay and the clients in two processes, so RSS and CPU are the relay's alone.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::path::Path;
use std::process::Command;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use futures_util::{SinkExt, StreamExt};
use protocol::{Envelope, Message};
use relay::doors::{quic, Doors};
use relay::outbox::Lane;
use tokio::io::{AsyncBufReadExt, AsyncRead, AsyncWrite, AsyncWriteExt, BufReader};
use tokio::net::TcpListener;
use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::Message as Ws;

const CODE: &str = "BENCH";
const WARMUP: Duration = Duration::from_secs(2);
const SHARED: &str = "/tmp/438";

#[derive(Clone, Copy, PartialEq, Debug)]
enum Via {
    Ws = 0,
    Wt = 1,
}
const VIAS: [Via; 2] = [Via::Ws, Via::Wt];

fn via(v: &str) -> Via {
    match v {
        "ws" => Via::Ws,
        "wt" => Via::Wt,
        _ => panic!("--via ws|wt"),
    }
}

fn vias(v: &str) -> Vec<Via> {
    v.split(',').map(via).collect()
}

#[derive(Clone, Debug)]
struct Args {
    decks: usize,
    listeners: usize,
    seconds: u64,
    decks_via: Vec<Via>,
    conductor_via: Via,
    phones_via: Vec<Via>,
    label: String,
    hash_file: Option<String>,
    keys: usize,
    serve: bool,
    connect: bool,
    count_egress: bool,
}

fn args() -> Args {
    let mut a = Args {
        decks: 20,
        listeners: 100,
        seconds: 30,
        decks_via: vec![Via::Ws],
        conductor_via: Via::Ws,
        phones_via: vec![Via::Ws],
        label: String::new(),
        hash_file: None,
        keys: 0,
        serve: false,
        connect: false,
        count_egress: false,
    };
    let mut it = std::env::args().skip(1);
    while let Some(k) = it.next() {
        match k.as_str() {
            "--serve" => {
                a.serve = true;
                continue;
            }
            "--connect" => {
                a.connect = true;
                continue;
            }
            "--count-egress" => {
                a.count_egress = true;
                continue;
            }
            _ => {}
        }
        let v = it.next().unwrap_or_default();
        match k.as_str() {
            "--via" => {
                a.decks_via = vias(&v);
                a.conductor_via = via(&v);
                a.phones_via = vias(&v);
            }
            "--decks-via" => a.decks_via = vias(&v),
            "--conductor-via" => a.conductor_via = via(&v),
            "--phones-via" => a.phones_via = vias(&v),
            "--decks" => a.decks = v.parse().unwrap(),
            "--listeners" => a.listeners = v.parse().unwrap(),
            "--seconds" => a.seconds = v.parse().unwrap(),
            "--label" => a.label = v,
            "--hash-file" => a.hash_file = Some(v),
            "--keys" => a.keys = v.parse().unwrap(),
            _ => panic!("unknown argument {k}"),
        }
    }
    a
}

/// microseconds since the epoch: one clock for the bench, its second process and a browser on this machine
fn t_us() -> f64 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_micros() as f64
}

#[derive(Default)]
struct Stats {
    values_us: Mutex<Vec<f64>>,
    hold_us: Mutex<Vec<f64>>,
    cues_us: Mutex<Vec<f64>>,
    values_by: [Mutex<Vec<f64>>; 2],
    cues_by: [Mutex<Vec<f64>>; 2],
    values_sent: AtomicU64,
    values_got: AtomicU64,
    values_stale: AtomicU64,
    settles: AtomicU64,
    cues_sent: AtomicU64,
    cues_got: AtomicU64,
    cues_bad: AtomicU64,
    bytes_in: AtomicU64,
    lowest_hz: AtomicU64,
}

fn percentile(sorted: &[f64], p: f64) -> f64 {
    if sorted.is_empty() {
        return f64::NAN;
    }
    sorted[((sorted.len() as f64 - 1.0) * p).round() as usize]
}

/// CPU time so far of the relay runtime's threads, in clock ticks (10 ms; Linux). Read at the start and the end of the
/// measured window, so the handshakes of the joins do not count.
fn relay_ticks() -> u64 {
    let mut ticks = 0;
    let Ok(tasks) = std::fs::read_dir("/proc/self/task") else { return 0 };
    for t in tasks.flatten() {
        let name = std::fs::read_to_string(t.path().join("comm")).unwrap_or_default();
        if name.trim() != "relay" {
            continue;
        }
        let stat = std::fs::read_to_string(t.path().join("stat")).unwrap_or_default();
        // after the ")" of the comm: utime and stime are fields 12 and 13 of what is left
        let rest = stat.rsplit(')').next().unwrap_or("");
        let f: Vec<&str> = rest.split_whitespace().collect();
        ticks += f.get(11).and_then(|x| x.parse::<u64>().ok()).unwrap_or(0) + f.get(12).and_then(|x| x.parse::<u64>().ok()).unwrap_or(0);
    }
    ticks
}

fn rss_kb() -> u64 {
    std::fs::read_to_string("/proc/self/status").unwrap_or_default().lines().find_map(|l| l.strip_prefix("VmRSS:").and_then(|r| r.split_whitespace().next()).and_then(|x| x.parse().ok())).unwrap_or(0)
}

/// `tx` bytes of the loopback interface – everything on the wire, headers and retransmits included (Linux only)
fn wire_bytes() -> u64 {
    let Ok(dev) = std::fs::read_to_string("/proc/net/dev") else { return 0 };
    dev.lines().find_map(|l| l.trim().strip_prefix("lo:").map(|r| r.split_whitespace().nth(8).and_then(|x| x.parse().ok()).unwrap_or(0))).unwrap_or(0)
}

/// the relay's egress, IP headers included: iptables counters on the doors' source ports (run.sh adds the rules)
fn egress_bytes(zero: bool) -> u64 {
    if zero {
        let _ = Command::new("iptables").args(["-Z", "OUTPUT"]).output();
        return 0;
    }
    let Ok(o) = Command::new("iptables").args(["-L", "OUTPUT", "-v", "-x", "-n"]).output() else { return 0 };
    String::from_utf8_lossy(&o.stdout).lines().skip(2).filter_map(|l| l.split_whitespace().nth(1).and_then(|b| b.parse::<u64>().ok())).sum()
}

#[derive(Clone)]
struct Net {
    ws: SocketAddr,
    wt: SocketAddr,
    hash: wtransport::tls::Sha256Digest,
}

/// A client whichever door it came through: `(lane, text)` out, texts in.
struct Wire {
    tx: mpsc::UnboundedSender<(Lane, String)>,
    rx: mpsc::UnboundedReceiver<String>,
}

async fn dial(net: &Net, via: Via, path: &str) -> Wire {
    let (tx, mut out) = mpsc::unbounded_channel::<(Lane, String)>();
    let (inc, rx) = mpsc::unbounded_channel::<String>();
    match via {
        Via::Ws => {
            // Nagle off, as in examples/bench.rs and every browser
            let (ws, _) = tokio_tungstenite::connect_async_with_config(format!("ws://{}{path}", net.ws), None, true).await.expect("ws");
            let (mut w, mut r) = ws.split();
            tokio::spawn(async move {
                while let Some((_, text)) = out.recv().await {
                    if w.send(Ws::Text(text)).await.is_err() {
                        return;
                    }
                }
            });
            tokio::spawn(async move {
                while let Some(Ok(frame)) = r.next().await {
                    if let Ws::Text(t) = frame {
                        if inc.send(t).is_err() {
                            return;
                        }
                    }
                }
            });
        }
        Via::Wt => {
            let config = wtransport::ClientConfig::builder().with_bind_default().with_server_certificate_hashes([net.hash.clone()]).build();
            let endpoint = wtransport::Endpoint::client(config).expect("wt endpoint");
            let conn = Arc::new(endpoint.connect(format!("https://127.0.0.1:{}{path}", net.wt.port())).await.expect("wt"));
            let (send, recv) = conn.open_bi().await.expect("wt open").await.expect("wt bi");
            let c2 = conn.clone();
            tokio::spawn(async move {
                let _keep = (endpoint, c2.clone());
                pump_out(send, &mut out, |t| c2.send_datagram(t.into_bytes()).is_ok()).await;
            });
            let inc2 = inc.clone();
            tokio::spawn(async move {
                while let Ok(d) = conn.receive_datagram().await {
                    if inc2.send(String::from_utf8_lossy(d.payload().as_ref()).into_owned()).is_err() {
                        return;
                    }
                }
            });
            tokio::spawn(pump_in(recv, inc));
        }
    }
    Wire { tx, rx }
}

async fn pump_out<W: AsyncWrite + Unpin>(mut send: W, out: &mut mpsc::UnboundedReceiver<(Lane, String)>, dgram: impl Fn(String) -> bool) {
    while let Some((lane, text)) = out.recv().await {
        let reliable = lane == Lane::Reliable || !dgram(text.clone());
        if reliable && send.write_all(format!("{text}\n").as_bytes()).await.is_err() {
            return;
        }
    }
}

async fn pump_in<R: AsyncRead + Unpin>(recv: R, inc: mpsc::UnboundedSender<String>) {
    let mut lines = BufReader::new(recv).lines();
    while let Ok(Some(line)) = lines.next_line().await {
        if inc.send(line).is_err() {
            return;
        }
    }
}

fn path(role: &str, group: &str) -> String {
    format!("/live/{CODE}?role={role}&group={group}")
}

fn is_welcome(t: &str) -> bool {
    t.contains("\"kind\":\"welcome\"")
}

fn main() {
    let a = args();
    let _ = std::fs::create_dir_all(SHARED);
    let clients = tokio::runtime::Builder::new_multi_thread().worker_threads(8).enable_all().build().expect("client runtime");

    let (net, relay_rt) = if a.connect {
        let hash: [u8; 32] = std::fs::read(format!("{SHARED}/hash")).expect("--serve first").try_into().unwrap();
        (net_of(wtransport::tls::Sha256Digest::new(hash)), None)
    } else {
        let rt = tokio::runtime::Builder::new_multi_thread().worker_threads(2).thread_name("relay").enable_all().build().expect("relay runtime");
        let net = start_doors(&rt, &a);
        (net, Some(rt))
    };

    if a.serve {
        serve_loop(&a.label);
        return;
    }
    let stats = Arc::new(Stats { lowest_hz: AtomicU64::new(u64::MAX), ..Stats::default() });
    let w = clients.block_on(run(a.clone(), net, stats.clone()));
    report(&a, &stats, w);
    let _ = (&relay_rt, &clients);
    std::process::exit(0);
}

fn net_of(hash: wtransport::tls::Sha256Digest) -> Net {
    Net { ws: "127.0.0.1:7070".parse().unwrap(), wt: "127.0.0.1:7072".parse().unwrap(), hash }
}

fn start_doors(rt: &tokio::runtime::Runtime, a: &Args) -> Net {
    let identity = wtransport::Identity::self_signed(["localhost", "127.0.0.1"]).unwrap();
    let hash = identity.certificate_chain().as_slice()[0].hash();
    if let Some(f) = &a.hash_file {
        std::fs::write(f, hash.fmt(wtransport::tls::Sha256DigestFmt::BytesArray)).unwrap();
    }
    std::fs::write(format!("{SHARED}/hash"), hash.as_ref()).unwrap();
    let files = relay::tls::TlsFiles { cert: format!("{SHARED}/cert.pem").into(), key: format!("{SHARED}/key.pem").into() };
    std::fs::write(&files.cert, identity.certificate_chain().as_slice()[0].to_pem()).unwrap();
    std::fs::write(&files.key, identity.private_key().to_secret_pem()).unwrap();
    let net = net_of(hash);
    let n = net.clone();
    rt.block_on(async move {
        let relay = Arc::new(relay::Relay::new());
        let doors = Doors::new(relay.clone());
        let ws = TcpListener::bind(n.ws).await.expect("7070");
        tokio::spawn(relay::serve(ws, relay));
        tokio::spawn(quic::door(doors, n.wt, files).unwrap());
    });
    net
}

/// The relay alone: what it holds and burns while another process plays the clients.
fn serve_loop(label: &str) {
    let (measure, stop) = (format!("{SHARED}/measure"), format!("{SHARED}/stop"));
    let _ = std::fs::remove_file(&measure);
    let _ = std::fs::remove_file(&stop);
    let idle = rss_kb();
    std::fs::write(format!("{SHARED}/ready"), "1").unwrap();
    let deadline = Instant::now() + Duration::from_secs(300);
    while !Path::new(&measure).exists() && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(20));
    }
    let (t0, start, joined) = (relay_ticks(), Instant::now(), rss_kb());
    while !Path::new(&stop).exists() && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(100));
    }
    let cpu = (relay_ticks().saturating_sub(t0) * 10) as f64 / 1000.0 / start.elapsed().as_secs_f64() * 100.0;
    println!("{}", serde_json::json!({ "label": label, "serve": true, "rss_idle_kb": idle, "rss_joined_kb": joined, "rss_end_kb": rss_kb(), "relay_cpu_pct": (cpu * 10.0).round() / 10.0 }));
    std::process::exit(0);
}

struct Window {
    wire: u64,
    ticks: u64,
    wall: Duration,
    egress: u64,
}

async fn run(a: Args, net: Net, stats: Arc<Stats>) -> Window {
    let flag = Arc::new(AtomicBool::new(false));
    let welcomed = Arc::new(AtomicU64::new(0));
    let stop = Arc::new(AtomicBool::new(false));

    let mut conductor = dial(&net, a.conductor_via, &path("conductor", "")).await;
    conductor.tx.send((Lane::Reliable, format!(r#"{{"v":1,"seq":0,"from":"","kind":"hello","p":{{"code":"{CODE}","role":"conductor","token":"bench"}}}}"#))).unwrap();
    let cond_tx = conductor.tx.clone();
    {
        let (s, m, welcomed) = (stats.clone(), flag.clone(), welcomed.clone());
        tokio::spawn(async move {
            // per deck: the seq and the send time of the value shown now
            let mut shown: HashMap<String, (u64, f64)> = HashMap::new();
            while let Some(text) = conductor.rx.recv().await {
                let now = t_us();
                if is_welcome(&text) {
                    welcomed.fetch_add(1, Ordering::Relaxed);
                }
                if !m.load(Ordering::Relaxed) {
                    continue;
                }
                s.bytes_in.fetch_add(text.len() as u64, Ordering::Relaxed);
                if text.contains("\"kind\":\"presence\"") {
                    let p: serde_json::Value = serde_json::from_str(&text).unwrap_or_default();
                    for hz in p["p"]["links"].as_object().into_iter().flat_map(|l| l.values()).filter_map(|l| l["hz"].as_u64()) {
                        s.lowest_hz.fetch_min(hz, Ordering::Relaxed);
                    }
                    continue;
                }
                if !text.contains("\"deck.values\"") {
                    continue;
                }
                let Ok(Envelope { seq, from, to, message: Message::DeckValues(v), .. }) = serde_json::from_str::<Envelope>(&text) else { continue };
                let Some(t) = v.values.get("bench.t").and_then(|x| x.first()).copied() else { continue };
                if to.is_some() {
                    s.settles.fetch_add(1, Ordering::Relaxed);
                    continue;
                }
                let prev = shown.get(&from).copied();
                if prev.is_some_and(|(q, _)| seq <= q) {
                    s.values_stale.fetch_add(1, Ordering::Relaxed); // overtaken: a real receiver ignores it
                    continue;
                }
                shown.insert(from, (seq, t));
                s.values_got.fetch_add(1, Ordering::Relaxed);
                s.values_us.lock().unwrap().push(now - t);
                if let Some(door) = v.values.get("bench.via").and_then(|x| x.first()) {
                    s.values_by[*door as usize].lock().unwrap().push(now - t);
                }
                if let Some((_, old)) = prev {
                    s.hold_us.lock().unwrap().push(now - old);
                }
            }
        });
    }

    for i in 0..a.listeners {
        let door = a.phones_via[i % a.phones_via.len()];
        let mut w = dial(&net, door, &path("audience", "a")).await;
        let (s, m, welcomed) = (stats.clone(), flag.clone(), welcomed.clone());
        let hello = format!(r#"{{"v":1,"seq":0,"from":"","kind":"hello","p":{{"code":"{CODE}","role":"audience","group":"a","nick":"p{i}"}}}}"#);
        w.tx.send((Lane::Reliable, hello)).unwrap();
        tokio::spawn(async move {
            let _keep = w.tx;
            let mut last: Option<u64> = None;
            while let Some(text) = w.rx.recv().await {
                let now = t_us();
                if is_welcome(&text) {
                    welcomed.fetch_add(1, Ordering::Relaxed);
                }
                if !m.load(Ordering::Relaxed) {
                    continue;
                }
                s.bytes_in.fetch_add(text.len() as u64, Ordering::Relaxed);
                let Ok(Envelope { message: Message::Cue(c), .. }) = serde_json::from_str::<Envelope>(&text) else { continue };
                let Some((n, t)) = c.id.split_once(':') else { continue };
                let (n, t): (u64, f64) = (n.parse().unwrap(), t.parse().unwrap());
                if last.is_some_and(|l| n != l + 1) {
                    s.cues_bad.fetch_add(1, Ordering::Relaxed);
                }
                last = Some(n);
                s.cues_got.fetch_add(1, Ordering::Relaxed);
                s.cues_us.lock().unwrap().push(now - t);
                s.cues_by[door as usize].lock().unwrap().push(now - t);
            }
        });
    }

    let extra: String = (0..a.keys).map(|k| format!(r#","k{k}.value":[0.123]"#)).collect();
    for d in 0..a.decks {
        let door = a.decks_via[d % a.decks_via.len()];
        let mut w = dial(&net, door, &path("musician", "decks")).await;
        let (s, m, decks, extra) = (stats.clone(), flag.clone(), a.decks, extra.clone());
        let (stop, welcomed) = (stop.clone(), welcomed.clone());
        w.tx.send((Lane::Reliable, format!(r#"{{"v":1,"seq":0,"from":"","kind":"hello","p":{{"code":"{CODE}","role":"musician","group":"decks"}}}}"#))).unwrap();
        w.tx.send((Lane::Reliable, r#"{"v":1,"seq":0,"from":"","kind":"deck.graph","p":{"nodes":[{"id":"lfo","kind":"lfo"},{"id":"space","kind":"space"}],"links":[{"from":"lfo.out","to":"space.hue"}],"character":"catpope"}}"#.to_owned())).unwrap();
        tokio::spawn(async move {
            let tx = w.tx;
            tokio::spawn(async move {
                while let Some(text) = w.rx.recv().await {
                    if is_welcome(&text) {
                        welcomed.fetch_add(1, Ordering::Relaxed);
                    }
                }
            });
            tokio::time::sleep(Duration::from_micros(33_333 * d as u64 / decks.max(1) as u64)).await;
            let mut every = tokio::time::interval(Duration::from_micros(33_334));
            every.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
            let (mut x, mut seq) = (0.0f64, 0u64);
            while !stop.load(Ordering::Relaxed) {
                every.tick().await;
                x = (x + 0.01) % 1.0;
                seq += 1;
                let msg = format!(r#"{{"v":1,"seq":{seq},"from":"","kind":"deck.values","p":{{"values":{{"bench.t":[{}],"bench.via":[{}],"lfo.rate":[{x:.3}],"space.hue":[{x:.3}]{extra}}}}}}}"#, t_us(), door as u8);
                if tx.send((Lane::Unreliable, msg)).is_err() {
                    break;
                }
                if m.load(Ordering::Relaxed) {
                    s.values_sent.fetch_add(1, Ordering::Relaxed);
                }
            }
        });
    }

    // measure only once every client has its welcome, after a warm-up of the running load
    let all = (a.listeners + a.decks + 1) as u64;
    let deadline = Instant::now() + Duration::from_secs(25);
    while welcomed.load(Ordering::Relaxed) < all && Instant::now() < deadline {
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    let joined = welcomed.load(Ordering::Relaxed);
    if joined < all {
        eprintln!("only {joined} of {all} clients joined within 25 s");
    }
    tokio::time::sleep(WARMUP).await;
    let wire0 = wire_bytes();
    let ticks0 = relay_ticks();
    if a.count_egress {
        egress_bytes(true);
    }
    if a.connect {
        std::fs::write(format!("{SHARED}/measure"), "1").unwrap();
    }
    flag.store(true, Ordering::Relaxed);
    let wall_start = Instant::now();
    let mut every = tokio::time::interval(Duration::from_millis(200));
    let mut n = 0u64;
    let end = Instant::now() + Duration::from_secs(a.seconds);
    while Instant::now() < end {
        every.tick().await;
        let cue = format!(r#"{{"v":1,"seq":0,"from":"","to":{{"group":"a"}},"kind":"cue","p":{{"id":"{n}:{}","actions":[]}}}}"#, t_us());
        n += 1;
        stats.cues_sent.fetch_add(1, Ordering::Relaxed);
        if cond_tx.send((Lane::Reliable, cue)).is_err() {
            break;
        }
    }
    let egress = if a.count_egress { egress_bytes(false) } else { 0 };
    stop.store(true, Ordering::Relaxed);
    let wall = wall_start.elapsed();
    let window = Window { wire: wire_bytes().saturating_sub(wire0), ticks: relay_ticks().saturating_sub(ticks0), wall, egress };
    if a.connect {
        std::fs::write(format!("{SHARED}/stop"), "1").unwrap();
    }
    tokio::time::sleep(Duration::from_millis(500)).await;
    window
}

fn report(a: &Args, s: &Stats, w: Window) {
    let sorted = |m: &Mutex<Vec<f64>>| {
        let mut v = std::mem::take(&mut *m.lock().unwrap());
        v.sort_by(f64::total_cmp);
        v
    };
    let ms = |x: f64| (x / 100.0).round() / 10.0;
    let trio = |v: &[f64]| serde_json::json!({ "p50": ms(percentile(v, 0.5)), "p99": ms(percentile(v, 0.99)), "max": ms(v.last().copied().unwrap_or(f64::NAN)), "n": v.len() });
    let (v, h, c) = (sorted(&s.values_us), sorted(&s.hold_us), sorted(&s.cues_us));
    let mut by = serde_json::Map::new();
    for door in VIAS {
        let (dv, dc) = (sorted(&s.values_by[door as usize]), sorted(&s.cues_by[door as usize]));
        if !dv.is_empty() || !dc.is_empty() {
            by.insert(format!("{door:?}"), serde_json::json!({ "values": trio(&dv), "cues": trio(&dc) }));
        }
    }
    let secs = w.wall.as_secs_f64();
    let lowest = s.lowest_hz.load(Ordering::Relaxed);
    let record = serde_json::json!({
        "label": a.label,
        "decks_via": format!("{:?}", a.decks_via), "conductor_via": format!("{:?}", a.conductor_via), "phones_via": format!("{:?}", a.phones_via),
        "decks": a.decks, "listeners": a.listeners, "seconds": a.seconds, "keys": a.keys,
        "values": trio(&v), "values_sent": s.values_sent.load(Ordering::Relaxed), "values_got": s.values_got.load(Ordering::Relaxed), "values_stale": s.values_stale.load(Ordering::Relaxed), "settles": s.settles.load(Ordering::Relaxed),
        "hold": trio(&h),
        "cues": trio(&c), "cues_expected": s.cues_sent.load(Ordering::Relaxed) * a.listeners as u64, "cues_got": s.cues_got.load(Ordering::Relaxed), "cues_bad": s.cues_bad.load(Ordering::Relaxed),
        "by_door": by,
        "lowest_hz": if lowest < 1000 { serde_json::json!(lowest) } else { serde_json::Value::Null },
        "relay_cpu_pct": ((w.ticks * 10) as f64 / 1000.0 / secs * 1000.0).round() / 10.0,
        "payload_kbps": (s.bytes_in.load(Ordering::Relaxed) as f64 / 1024.0 / secs).round(),
        "wire_kbps": (w.wire as f64 / 1024.0 / secs).round(),
        "egress_kbps": (w.egress as f64 / 1024.0 / secs).round(),
    });
    println!("{record}");
}
