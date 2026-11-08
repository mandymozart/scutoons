//! The transport bench (#438, feature `doors`): the load of examples/bench.rs – N decks with `deck.values` at 30 Hz to the
//! conductor, M phones getting the conductor's cues at 5 Hz – over any door, under whatever the kernel does to the path
//! (`tc netem` on lo, deck/relay/netem/run.sh). Every door joins one relay, so `--decks-via` and `--phones-via` may be
//! lists (`renet2,wt,ws`, round robin) and the report has each door's own numbers.
//!
//! `cargo run --release -p relay --features doors --example doors_bench -- [--via ws|quinn|wt|renet2] [--decks-via …]
//!  [--conductor-via …] [--phones-via …] [--decks 20] [--listeners 100] [--seconds 30] [--resend 40] [--keys 0]
//!  [--label x] [--serve | --connect] [--count-egress] [--ladder-join native|browser --climb race|turn --joins 40]
//!  [--ladder-show --block-at 8]`
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
use relay::doors::ladder::{Action, Climb, Event, Ladder, Policy, Rung};
use relay::doors::{quic, renet, Doors};
use relay::outbox::Lane;
use tokio::io::{AsyncBufReadExt, AsyncRead, AsyncWrite, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, UdpSocket};
use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::Message as Ws;

const CODE: &str = "BENCH";
const WARMUP: Duration = Duration::from_secs(2);
const KEY: [u8; 32] = [11; 32];
const SHARED: &str = "/tmp/438";

#[derive(Clone, Copy, PartialEq, Debug)]
enum Via {
    Ws = 0,
    Quinn = 1,
    Wt = 2,
    Renet2 = 3,
}
const VIAS: [Via; 4] = [Via::Ws, Via::Quinn, Via::Wt, Via::Renet2];

fn via(v: &str) -> Via {
    match v {
        "ws" => Via::Ws,
        "quinn" => Via::Quinn,
        "wt" => Via::Wt,
        "renet2" => Via::Renet2,
        _ => panic!("--via ws|quinn|wt|renet2"),
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
    resend: Duration,
    label: String,
    hash_file: Option<String>,
    ack_delay_ms: u64,
    keys: usize,
    serve: bool,
    connect: bool,
    count_egress: bool,
    ladder_join: Option<String>,
    climb: Climb,
    joins: usize,
    ladder_show: bool,
    block_at: u64,
}

fn args() -> Args {
    let mut a = Args {
        decks: 20,
        listeners: 100,
        seconds: 30,
        decks_via: vec![Via::Ws],
        conductor_via: Via::Ws,
        phones_via: vec![Via::Ws],
        resend: Duration::from_millis(40),
        label: String::new(),
        hash_file: None,
        ack_delay_ms: 0,
        keys: 0,
        serve: false,
        connect: false,
        count_egress: false,
        ladder_join: None,
        climb: Climb::Race,
        joins: 40,
        ladder_show: false,
        block_at: 8,
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
            "--ladder-show" => {
                a.ladder_show = true;
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
            "--resend" => a.resend = Duration::from_millis(v.parse().unwrap()),
            "--label" => a.label = v,
            "--hash-file" => a.hash_file = Some(v),
            "--ack-delay" => a.ack_delay_ms = v.parse().unwrap(),
            "--keys" => a.keys = v.parse().unwrap(),
            "--ladder-join" => a.ladder_join = Some(v),
            "--climb" => a.climb = if v == "turn" { Climb::InTurn } else { Climb::Race },
            "--joins" => a.joins = v.parse().unwrap(),
            "--block-at" => a.block_at = v.parse().unwrap(),
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
    values_by: [Mutex<Vec<f64>>; 4],
    cues_by: [Mutex<Vec<f64>>; 4],
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
    quinn: SocketAddr,
    wt: SocketAddr,
    renet: SocketAddr,
    chain: quinn::rustls::pki_types::CertificateDer<'static>,
    hash: wtransport::tls::Sha256Digest,
    resend: Duration,
}

/// A client whichever door it came through: `(lane, text)` out, texts in.
struct Wire {
    tx: mpsc::UnboundedSender<(Lane, String)>,
    rx: mpsc::UnboundedReceiver<String>,
}

static NEXT_ID: AtomicU64 = AtomicU64::new(1);

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
        Via::Quinn => {
            let (conn, send, recv) = quic::quinn_client(net.quinn, net.chain.clone()).await.expect("quinn");
            let c2 = conn.clone();
            tokio::spawn(async move {
                let _keep = c2.clone();
                pump_out(send, &mut out, |t| c2.send_datagram(t.into_bytes().into()).is_ok()).await;
            });
            let inc2 = inc.clone();
            tokio::spawn(async move {
                while let Ok(d) = conn.read_datagram().await {
                    if inc2.send(String::from_utf8_lossy(&d).into_owned()).is_err() {
                        return;
                    }
                }
            });
            tokio::spawn(pump_in(recv, inc));
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
        Via::Renet2 => {
            let id = NEXT_ID.fetch_add(1, Ordering::Relaxed) + ((std::process::id() as u64) << 20);
            let (server, resend) = (net.renet, net.resend);
            tokio::spawn(async move {
                if let Err(e) = renet::client(server, KEY, id, resend, out, inc).await {
                    eprintln!("renet2 client {id}: {e}");
                }
            });
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
    relay::doors::init_crypto();
    quic::ACK_DELAY_MS.store(a.ack_delay_ms, Ordering::Relaxed);
    let _ = std::fs::create_dir_all(SHARED);
    let clients = tokio::runtime::Builder::new_multi_thread().worker_threads(8).enable_all().build().expect("client runtime");

    let (net, relay_rt) = if a.connect {
        let chain: quinn::rustls::pki_types::CertificateDer<'static> = std::fs::read(format!("{SHARED}/chain.der")).expect("--serve first").into();
        let hash: [u8; 32] = std::fs::read(format!("{SHARED}/hash")).unwrap().try_into().unwrap();
        (net_of(chain, wtransport::tls::Sha256Digest::new(hash), a.resend), None)
    } else {
        let rt = tokio::runtime::Builder::new_multi_thread().worker_threads(2).thread_name("relay").enable_all().build().expect("relay runtime");
        let net = start_doors(&rt, &a);
        (net, Some(rt))
    };

    if a.serve {
        serve_loop(&a.label);
        return;
    }
    if let Some(stack) = a.ladder_join.clone() {
        clients.block_on(ladder_join(&a, &net, &stack));
    } else if a.ladder_show {
        clients.block_on(ladder_show(&a, &net));
    } else {
        let stats = Arc::new(Stats { lowest_hz: AtomicU64::new(u64::MAX), ..Stats::default() });
        let w = clients.block_on(run(a.clone(), net, stats.clone()));
        report(&a, &stats, w);
    }
    let _ = (&relay_rt, &clients);
    std::process::exit(0);
}

fn net_of(chain: quinn::rustls::pki_types::CertificateDer<'static>, hash: wtransport::tls::Sha256Digest, resend: Duration) -> Net {
    Net { ws: "127.0.0.1:7070".parse().unwrap(), quinn: "127.0.0.1:7071".parse().unwrap(), wt: "127.0.0.1:7072".parse().unwrap(), renet: "127.0.0.1:7073".parse().unwrap(), chain, hash, resend }
}

fn start_doors(rt: &tokio::runtime::Runtime, a: &Args) -> Net {
    let identity = wtransport::Identity::self_signed(["localhost", "127.0.0.1"]).unwrap();
    let hash = identity.certificate_chain().as_slice()[0].hash();
    if let Some(f) = &a.hash_file {
        std::fs::write(f, hash.fmt(wtransport::tls::Sha256DigestFmt::BytesArray)).unwrap();
    }
    let chain: quinn::rustls::pki_types::CertificateDer<'static> = identity.certificate_chain().as_slice()[0].der().to_vec().into();
    std::fs::write(format!("{SHARED}/chain.der"), chain.as_ref()).unwrap();
    std::fs::write(format!("{SHARED}/hash"), hash.as_ref()).unwrap();
    let key = quinn::rustls::pki_types::PrivateKeyDer::Pkcs8(identity.private_key().secret_der().to_vec().into());
    let net = net_of(chain.clone(), hash, a.resend);
    let n = net.clone();
    let resend = a.resend;
    rt.block_on(async move {
        let relay = Arc::new(relay::Relay::new());
        let doors = Doors::new(relay.clone());
        let ws = TcpListener::bind(n.ws).await.expect("7070");
        tokio::spawn(relay::serve(ws, relay));
        tokio::spawn(quic::quinn_door(doors.clone(), n.quinn, vec![chain], key).unwrap());
        tokio::spawn(quic::wt_door(doors.clone(), n.wt, identity).unwrap());
        let udp = Arc::new(UdpSocket::bind(n.renet).await.expect("7073"));
        tokio::spawn(async move {
            if let Err(e) = renet::netcode_door(doors, udp, vec![], KEY, resend).await {
                eprintln!("netcode door: {e}");
            }
        });
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
        "decks": a.decks, "listeners": a.listeners, "seconds": a.seconds, "resend_ms": a.resend.as_millis() as u64, "ack_delay_ms": a.ack_delay_ms, "keys": a.keys,
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

fn via_of(r: Rung) -> Via {
    match r {
        Rung::Rudp => Via::Renet2,
        Rung::Quic => Via::Wt,
        Rung::WebSocket => Via::Ws,
    }
}

fn hello_text(role: &str, nick: &str, resume: Option<&(String, i64)>) -> String {
    let resume = resume.map_or(String::new(), |(client, seq)| format!(r#","resume":{{"client":"{client}","seq":{seq}}}"#));
    let group = if role == "audience" { "a" } else { "decks" };
    format!(r#"{{"v":1,"seq":0,"from":"","kind":"hello","p":{{"code":"{CODE}","role":"{role}","group":"{group}","nick":"{nick}"{resume}}}}}"#)
}

/// One client on the ladder: `Ladder` decides, this only dials and moves texts (what a platform adapter would do). Returns
/// the rung in use when `on_text` says stop.
async fn ladder_client(net: Net, policy: Policy, role: &'static str, init: Vec<String>, mut out: mpsc::UnboundedReceiver<(Lane, String)>, mut on_text: impl FnMut(&str, Rung) -> bool) -> Rung {
    let mut ladder = Ladder::new(policy, Instant::now());
    let (up_tx, mut up_rx) = mpsc::unbounded_channel::<(Rung, Wire)>();
    let mut tasks: HashMap<Rung, tokio::task::JoinHandle<()>> = HashMap::new();
    let mut wires: HashMap<Rung, Wire> = HashMap::new();
    let mut active: Option<Rung> = None;
    let mut me: Option<(String, i64)> = None;
    let mut ping = tokio::time::interval(Duration::from_secs(1));
    let nick = format!("l{}", NEXT_ID.fetch_add(1, Ordering::Relaxed));
    let mut actions: Vec<Action> = vec![];
    loop {
        actions.extend(ladder.poll(Instant::now()));
        for action in actions.drain(..) {
            match action {
                Action::Dial(r) => {
                    let (net, up) = (net.clone(), up_tx.clone());
                    let p = path(role, if role == "audience" { "a" } else { "decks" });
                    tasks.insert(
                        r,
                        tokio::spawn(async move {
                            let mut w = dial(&net, via_of(r), &p).await;
                            if r == Rung::Rudp {
                                // renet2's client reports its handshake with an empty text
                                while let Some(t) = w.rx.recv().await {
                                    if t.is_empty() {
                                        break;
                                    }
                                }
                            }
                            up.send((r, w)).ok();
                        }),
                    );
                }
                Action::Close(r) => {
                    if let Some(t) = tasks.remove(&r) {
                        t.abort();
                    }
                    wires.remove(&r);
                    if active == Some(r) {
                        active = None;
                    }
                }
                Action::Join { rung, resume } => {
                    active = Some(rung);
                    let w = wires.get(&rung).expect("a rung that came up");
                    w.tx.send((Lane::Reliable, hello_text(role, &nick, if resume { me.as_ref() } else { None }))).ok();
                    for text in &init {
                        w.tx.send((Lane::Reliable, text.clone())).ok();
                    }
                }
            }
        }
        let wake = ladder.next_wake(Instant::now());
        let text = async {
            match active {
                Some(r) => wires.get_mut(&r).expect("the active rung").rx.recv().await,
                None => std::future::pending().await,
            }
        };
        tokio::select! {
            Some((r, w)) = up_rx.recv() => {
                wires.insert(r, w);
                actions = ladder.on(Event::Connected(r), Instant::now());
            }
            t = text => match t {
                Some(t) if t.is_empty() => {}
                Some(t) => {
                    ladder.on(Event::Inbound, Instant::now());
                    if let Ok(e) = serde_json::from_str::<Envelope>(&t) {
                        if let Message::Welcome(w) = &e.message {
                            let seq = me.as_ref().map_or(0, |m| m.1);
                            me = Some((w.client.clone(), seq));
                        } else if !matches!(e.message, Message::DeckValues(_)) && e.seq > 0 {
                            // the resume point: only messages of the reliable lane count
                            if let Some(m) = &mut me {
                                m.1 = m.1.max(e.seq as i64);
                            }
                        }
                    }
                    if on_text(&t, active.expect("a text from the active rung")) {
                        for t in tasks.values() { t.abort(); }
                        return active.expect("the active rung");
                    }
                }
                None => {
                    let r = active.expect("the active rung");
                    actions = ladder.on(Event::Failed(r), Instant::now());
                }
            },
            Some(m) = out.recv(), if active.is_some() => {
                if let Some(w) = active.and_then(|r| wires.get(&r)) { w.tx.send(m).ok(); }
            }
            _ = ping.tick(), if active.is_some() => {
                let ping = format!(r#"{{"v":1,"seq":0,"from":"","kind":"ping","p":{{"t0":{}}}}}"#, (t_us() / 1000.0) as i64);
                if let Some(w) = active.and_then(|r| wires.get(&r)) { w.tx.send((Lane::Reliable, ping)).ok(); }
            }
            _ = tokio::time::sleep_until(wake.into()) => {}
        }
    }
}

/// How long a client needs from "go" to its `welcome`, with whatever iptables blocks now (run.sh `ladder`).
async fn ladder_join(a: &Args, net: &Net, stack: &str) {
    let policy = if stack == "browser" { Policy::browser(a.climb) } else { Policy::native(a.climb) };
    let mut times = vec![];
    let mut rungs: HashMap<String, u64> = HashMap::new();
    for _ in 0..a.joins {
        let start = Instant::now();
        let (_keep, out) = mpsc::unbounded_channel();
        let rung = tokio::time::timeout(Duration::from_secs(20), ladder_client(net.clone(), policy.clone(), "audience", vec![], out, |t, _| is_welcome(t))).await;
        match rung {
            Ok(r) => {
                times.push(start.elapsed().as_secs_f64() * 1000.0);
                *rungs.entry(format!("{r:?}")).or_default() += 1;
            }
            Err(_) => *rungs.entry("timeout".into()).or_default() += 1,
        }
        tokio::time::sleep(Duration::from_millis(30)).await;
    }
    times.sort_by(f64::total_cmp);
    let r = |x: f64| (x * 10.0).round() / 10.0;
    println!("{}", serde_json::json!({ "label": a.label, "stack": stack, "climb": format!("{:?}", a.climb), "joins": times.len(), "p50_ms": r(percentile(&times, 0.5)), "p99_ms": r(percentile(&times, 0.99)), "max_ms": r(times.last().copied().unwrap_or(f64::NAN)), "rungs": rungs }));
}

/// UDP dies mid-show: a deck and a phone on the ladder, a conductor on WebSocket (it stays). What did they lose?
async fn ladder_show(a: &Args, net: &Net) {
    let mut conductor = dial(net, Via::Ws, &path("conductor", "")).await;
    conductor.tx.send((Lane::Reliable, format!(r#"{{"v":1,"seq":0,"from":"","kind":"hello","p":{{"code":"{CODE}","role":"conductor","token":"bench"}}}}"#))).unwrap();
    let mut blocked_at: Option<Instant> = None;

    // the phone
    let phone_log = Arc::new(Mutex::new((Vec::<(u64, Rung)>::new(), Vec::<String>::new())));
    let (_keep_phone, phone_out) = mpsc::unbounded_channel();
    let log = phone_log.clone();
    tokio::spawn(ladder_client(net.clone(), Policy::native(Climb::Race), "audience", vec![], phone_out, move |t, r| {
        if let Ok(e) = serde_json::from_str::<Envelope>(t) {
            let mut g = log.lock().unwrap();
            match e.message {
                Message::Welcome(w) => g.1.push(w.client),
                Message::Cue(c) => {
                    if let Some(n) = c.id.split_once(':').and_then(|(n, _)| n.parse().ok()) {
                        g.0.push((n, r));
                    }
                }
                _ => {}
            }
        }
        false
    }));
    // the deck: its graph after every (re)join, values at 30 Hz through whichever rung is up
    let (deck_tx, deck_out) = mpsc::unbounded_channel();
    let graph = r#"{"v":1,"seq":0,"from":"","kind":"deck.graph","p":{"nodes":[{"id":"lfo","kind":"lfo"}],"links":[]}}"#.to_owned();
    let deck_rung = Arc::new(Mutex::new(None::<Rung>));
    let rung_log = deck_rung.clone();
    tokio::spawn(ladder_client(net.clone(), Policy::native(Climb::Race), "musician", vec![graph], deck_out, move |_, r| {
        *rung_log.lock().unwrap() = Some(r);
        false
    }));
    tokio::spawn(async move {
        let mut every = tokio::time::interval(Duration::from_micros(33_334));
        let mut seq = 0u64;
        loop {
            every.tick().await;
            seq += 1;
            let msg = format!(r#"{{"v":1,"seq":{seq},"from":"","kind":"deck.values","p":{{"values":{{"bench.t":[{}]}}}}}}"#, t_us());
            if deck_tx.send((Lane::Unreliable, msg)).is_err() {
                return;
            }
        }
    });

    // the conductor's cues and what it sees
    let cond_tx = conductor.tx.clone();
    tokio::spawn(async move {
        let mut every = tokio::time::interval(Duration::from_millis(200));
        let mut n = 0u64;
        loop {
            every.tick().await;
            let cue = format!(r#"{{"v":1,"seq":0,"from":"","to":{{"group":"a"}},"kind":"cue","p":{{"id":"{n}:{}","actions":[]}}}}"#, t_us());
            n += 1;
            if cond_tx.send((Lane::Reliable, cue)).is_err() {
                return;
            }
        }
    });
    let (mut last_value, mut gap_ms, mut values_after) = (None::<Instant>, 0f64, 0u64);
    let end = Instant::now() + Duration::from_secs(a.block_at + a.seconds);
    let block_time = Instant::now() + Duration::from_secs(a.block_at);
    let mut first_after: Option<f64> = None;
    loop {
        let wake = if blocked_at.is_some() { end } else { block_time };
        let next = tokio::select! {
            t = conductor.rx.recv() => t,
            _ = tokio::time::sleep_until(wake.into()) => None,
        };
        let now = Instant::now();
        if blocked_at.is_none() && now >= block_time {
            let _ = Command::new("iptables").args(["-I", "INPUT", "-p", "udp", "-j", "DROP"]).output();
            blocked_at = Some(now);
        }
        if now >= end {
            break;
        }
        let Some(t) = next else { continue };
        if !t.contains("\"deck.values\"") || !t.contains("bench.t") {
            continue;
        }
        if let Some(l) = last_value {
            gap_ms = f64::max(gap_ms, (now - l).as_secs_f64() * 1000.0);
        }
        last_value = Some(now);
        if let Some(at) = blocked_at {
            values_after += 1;
            if first_after.is_none() && now - at > Duration::from_millis(500) {
                first_after = Some((now - at).as_secs_f64() * 1000.0);
            }
        }
    }
    let g = phone_log.lock().unwrap();
    let numbers: Vec<u64> = g.0.iter().map(|(n, _)| *n).collect();
    let (first, last) = (numbers.first().copied().unwrap_or(0), numbers.last().copied().unwrap_or(0));
    let unique: std::collections::BTreeSet<u64> = numbers.iter().copied().collect();
    println!(
        "{}",
        serde_json::json!({ "label": a.label, "value_gap_ms": gap_ms.round(), "first_value_after_block_ms": first_after, "values_after_block": values_after,
            "deck_rung_at_end": format!("{:?}", *deck_rung.lock().unwrap()),
            "phone_rung_at_end": format!("{:?}", g.0.last().map(|x| x.1)), "phone_client_ids": g.1, "cues_seen": numbers.len(), "cues_range": [first, last], "cues_missing": (last - first + 1).saturating_sub(unique.len() as u64) })
    );
    let _ = Command::new("iptables").args(["-D", "INPUT", "-p", "udp", "-j", "DROP"]).output();
}
