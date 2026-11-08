//! The transport bench (#438, feature `doors`): the load of examples/bench.rs – N decks with `deck.values` at 30 Hz to the
//! conductor, M phones getting the conductor's cues at 5 Hz – over any door, under whatever the kernel does to the path
//! (`tc netem` on lo, deck/relay/netem/run.sh). Every door joins one relay, so `--decks-via`, `--conductor-via` and
//! `--phones-via` may differ.
//!
//! `cargo run --release -p relay --features doors --example doors_bench -- [--via ws|quinn|wt|renet2] [--decks-via …]
//!  [--conductor-via …] [--phones-via …] [--decks 20] [--listeners 100] [--seconds 30] [--resend 40] [--label x]`
//!
//! An unreliable lane always looks fast (what is lost is not counted), so `hold` is reported too: how old a deck's value
//! has become when the next one replaces it – what the conductor really shows, comparable across TCP and UDP.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use futures_util::{SinkExt, StreamExt};
use protocol::{Envelope, Message};
use relay::doors::{quic, renet, Doors};
use relay::outbox::Lane;
use tokio::io::{AsyncBufReadExt, AsyncRead, AsyncWrite, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, UdpSocket};
use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::Message as Ws;

const CODE: &str = "BENCH";
const WARMUP: Duration = Duration::from_secs(2);
const KEY: [u8; 32] = [11; 32];

#[derive(Clone, Copy, PartialEq, Debug)]
enum Via {
    Ws,
    Quinn,
    Wt,
    Renet2,
}

fn via(v: &str) -> Via {
    match v {
        "ws" => Via::Ws,
        "quinn" => Via::Quinn,
        "wt" => Via::Wt,
        "renet2" => Via::Renet2,
        _ => panic!("--via ws|quinn|wt|renet2"),
    }
}

#[derive(Clone, Debug)]
struct Args {
    decks: usize,
    listeners: usize,
    seconds: u64,
    decks_via: Via,
    conductor_via: Via,
    phones_via: Via,
    resend: Duration,
    label: String,
    hash_file: Option<String>,
    ack_delay_ms: u64,
}

fn args() -> Args {
    let mut a = Args { decks: 20, listeners: 100, seconds: 30, decks_via: Via::Ws, conductor_via: Via::Ws, phones_via: Via::Ws, resend: Duration::from_millis(40), label: String::new(), hash_file: None, ack_delay_ms: 0 };
    let mut it = std::env::args().skip(1);
    while let Some(k) = it.next() {
        let v = it.next().unwrap_or_default();
        match k.as_str() {
            "--via" => {
                a.decks_via = via(&v);
                a.conductor_via = via(&v);
                a.phones_via = via(&v);
            }
            "--decks-via" => a.decks_via = via(&v),
            "--conductor-via" => a.conductor_via = via(&v),
            "--phones-via" => a.phones_via = via(&v),
            "--decks" => a.decks = v.parse().unwrap(),
            "--listeners" => a.listeners = v.parse().unwrap(),
            "--seconds" => a.seconds = v.parse().unwrap(),
            "--resend" => a.resend = Duration::from_millis(v.parse().unwrap()),
            "--label" => a.label = v,
            "--hash-file" => a.hash_file = Some(v),
            "--ack-delay" => a.ack_delay_ms = v.parse().unwrap(),
            _ => panic!("unknown argument {k}"),
        }
    }
    a
}

/// microseconds since the epoch: one clock for the bench and a browser on this machine
fn t_us() -> f64 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_micros() as f64
}

#[derive(Default)]
struct Stats {
    values_us: Mutex<Vec<f64>>,
    hold_us: Mutex<Vec<f64>>,
    cues_us: Mutex<Vec<f64>>,
    values_sent: AtomicU64,
    values_got: AtomicU64,
    values_stale: AtomicU64,
    settles: AtomicU64,
    cues_sent: AtomicU64,
    cues_got: AtomicU64,
    cues_bad: AtomicU64,
    bytes_in: AtomicU64,
}

fn percentile(sorted: &[f64], p: f64) -> f64 {
    if sorted.is_empty() {
        return f64::NAN;
    }
    sorted[((sorted.len() as f64 - 1.0) * p).round() as usize]
}

/// CPU time so far of the relay runtime's threads, in clock ticks (10 ms; Linux). Read at the start and the end of the
/// measured window, so the handshakes of 121 joins do not count.
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

/// `tx` bytes of the loopback interface – everything on the wire, headers and retransmits included (Linux only)
fn wire_bytes() -> u64 {
    let Ok(dev) = std::fs::read_to_string("/proc/net/dev") else { return 0 };
    dev.lines().find_map(|l| l.trim().strip_prefix("lo:").map(|r| r.split_whitespace().nth(8).and_then(|x| x.parse().ok()).unwrap_or(0))).unwrap_or(0)
}

/// Where the doors listen.
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
            let (ws, _) = tokio_tungstenite::connect_async(format!("ws://{}{path}", net.ws)).await.expect("ws");
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
            let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
            tokio::spawn(renet::client(net.renet, KEY, id, net.resend, out, inc));
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

fn main() {
    let a = args();
    relay::doors::init_crypto();
    quic::ACK_DELAY_MS.store(a.ack_delay_ms, Ordering::Relaxed);
    let relay_rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .thread_name("relay")
        .enable_all()
        .build()
        .expect("relay runtime");
    let identity = wtransport::Identity::self_signed(["localhost", "127.0.0.1"]).unwrap();
    let hash = identity.certificate_chain().as_slice()[0].hash();
    if let Some(f) = &a.hash_file {
        std::fs::write(f, hash.fmt(wtransport::tls::Sha256DigestFmt::BytesArray)).unwrap();
    }
    let chain: quinn::rustls::pki_types::CertificateDer<'static> = identity.certificate_chain().as_slice()[0].der().to_vec().into();
    let key = quinn::rustls::pki_types::PrivateKeyDer::Pkcs8(identity.private_key().secret_der().to_vec().into());

    let net = relay_rt.block_on(async {
        let relay = Arc::new(relay::Relay::new());
        let doors = Doors::new(relay.clone());
        let ws = TcpListener::bind("127.0.0.1:7070").await.expect("7070");
        let ws_addr = ws.local_addr().unwrap();
        tokio::spawn(relay::serve(ws, relay));
        let quinn_addr: SocketAddr = "127.0.0.1:7071".parse().unwrap();
        let wt_addr: SocketAddr = "127.0.0.1:7072".parse().unwrap();
        tokio::spawn(quic::quinn_door(doors.clone(), quinn_addr, vec![chain.clone()], key).unwrap());
        tokio::spawn(quic::wt_door(doors.clone(), wt_addr, identity).unwrap());
        let udp = Arc::new(UdpSocket::bind("127.0.0.1:7073").await.expect("7073"));
        let renet_addr = udp.local_addr().unwrap();
        let resend = a.resend;
        tokio::spawn(async move {
            if let Err(e) = renet::netcode_door(doors, udp, vec![], KEY, resend).await {
                eprintln!("netcode door: {e}");
            }
        });
        Net { ws: ws_addr, quinn: quinn_addr, wt: wt_addr, renet: renet_addr, chain, hash, resend: a.resend }
    });

    let clients = tokio::runtime::Builder::new_multi_thread().worker_threads(8).enable_all().build().expect("client runtime");
    let stats = Arc::new(Stats::default());
    let measured = Arc::new(AtomicU64::new(0));
    let wire0 = Arc::new(AtomicU64::new(0));
    let ticks0 = Arc::new(AtomicU64::new(0));
    let wall = clients.block_on(run(a.clone(), net, stats.clone(), measured.clone(), wire0.clone(), ticks0.clone()));
    let wire1 = wire_bytes();
    let cpu = Duration::from_millis(relay_ticks().saturating_sub(ticks0.load(Ordering::Relaxed)) * 10);
    relay_rt.shutdown_timeout(Duration::from_secs(2));
    clients.shutdown_timeout(Duration::from_secs(1));
    report(&a, &stats, wall, cpu, wire1.saturating_sub(wire0.load(Ordering::Relaxed)));
}

async fn run(a: Args, net: Net, stats: Arc<Stats>, measuring_flag: Arc<AtomicU64>, wire0: Arc<AtomicU64>, ticks0: Arc<AtomicU64>) -> Duration {
    let path = |role: &str, group: &str| format!("/live/{CODE}?role={role}&group={group}");
    let start_measuring = measuring_flag.clone();
    let measuring = move || measuring_flag.load(Ordering::Relaxed) == 1;
    let welcomed = Arc::new(AtomicU64::new(0));
    let stop = Arc::new(AtomicBool::new(false));
    let is_welcome = |t: &str| t.contains("\"kind\":\"welcome\"");

    let mut conductor = dial(&net, a.conductor_via, &path("conductor", "")).await;
    conductor.tx.send((Lane::Reliable, format!(r#"{{"v":1,"seq":0,"from":"","kind":"hello","p":{{"code":"{CODE}","role":"conductor","token":"bench"}}}}"#))).unwrap();
    let cond_tx = conductor.tx.clone();
    {
        let s = stats.clone();
        let m = measuring.clone();
        let welcomed = welcomed.clone();
        tokio::spawn(async move {
            // per deck: the seq and the send time of the value shown now
            let mut shown: HashMap<String, (u64, f64)> = HashMap::new();
            while let Some(text) = conductor.rx.recv().await {
                let now = t_us();
                if is_welcome(&text) {
                    welcomed.fetch_add(1, Ordering::Relaxed);
                }
                if !m() {
                    continue;
                }
                s.bytes_in.fetch_add(text.len() as u64, Ordering::Relaxed);
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
                if let Some((_, old)) = prev {
                    s.hold_us.lock().unwrap().push(now - old);
                }
            }
        });
    }

    for i in 0..a.listeners {
        let mut w = dial(&net, a.phones_via, &path("audience", "a")).await;
        let s = stats.clone();
        let m = measuring.clone();
        let hello = format!(r#"{{"v":1,"seq":0,"from":"","kind":"hello","p":{{"code":"{CODE}","role":"audience","group":"a","nick":"p{i}"}}}}"#);
        w.tx.send((Lane::Reliable, hello)).unwrap();
        let welcomed = welcomed.clone();
        tokio::spawn(async move {
            let _keep = w.tx;
            let mut last: Option<u64> = None;
            while let Some(text) = w.rx.recv().await {
                let now = t_us();
                if is_welcome(&text) {
                    welcomed.fetch_add(1, Ordering::Relaxed);
                }
                if !m() {
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
            }
        });
    }

    for d in 0..a.decks {
        let mut w = dial(&net, a.decks_via, &path("musician", "decks")).await;
        let s = stats.clone();
        let m = measuring.clone();
        let decks = a.decks;
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
                let msg = format!(r#"{{"v":1,"seq":{seq},"from":"","kind":"deck.values","p":{{"values":{{"bench.t":[{}],"lfo.rate":[{x:.3}],"space.hue":[{x:.3}]}}}}}}"#, t_us());
                if tx.send((Lane::Unreliable, msg)).is_err() {
                    break;
                }
                if m() {
                    s.values_sent.fetch_add(1, Ordering::Relaxed);
                }
            }
        });
    }

    // measure only once every client has its welcome, after a warm-up of the running load
    let all = (a.listeners + a.decks + 1) as u64;
    let deadline = Instant::now() + Duration::from_secs(60);
    while welcomed.load(Ordering::Relaxed) < all && Instant::now() < deadline {
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    let joined = welcomed.load(Ordering::Relaxed);
    if joined < all {
        eprintln!("only {joined} of {all} clients joined within 60 s");
    }
    tokio::time::sleep(WARMUP).await;
    wire0.store(wire_bytes(), Ordering::Relaxed);
    ticks0.store(relay_ticks(), Ordering::Relaxed);
    start_measuring.store(1, Ordering::Relaxed);
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
    stop.store(true, Ordering::Relaxed);
    tokio::time::sleep(Duration::from_millis(500)).await;
    wall_start.elapsed()
}

fn report(a: &Args, s: &Stats, wall: Duration, relay_cpu: Duration, wire: u64) {
    let sorted = |m: &Mutex<Vec<f64>>| {
        let mut v = std::mem::take(&mut *m.lock().unwrap());
        v.sort_by(f64::total_cmp);
        v
    };
    let (v, h, c) = (sorted(&s.values_us), sorted(&s.hold_us), sorted(&s.cues_us));
    let ms = |x: f64| (x / 100.0).round() / 10.0;
    let secs = wall.as_secs_f64();
    let values_sent = s.values_sent.load(Ordering::Relaxed);
    let cues_expected = s.cues_sent.load(Ordering::Relaxed) * a.listeners as u64;
    let record = serde_json::json!({
        "label": a.label,
        "decks_via": format!("{:?}", a.decks_via), "conductor_via": format!("{:?}", a.conductor_via), "phones_via": format!("{:?}", a.phones_via),
        "decks": a.decks, "listeners": a.listeners, "seconds": a.seconds, "resend_ms": a.resend.as_millis() as u64, "ack_delay_ms": a.ack_delay_ms,
        "values": { "p50": ms(percentile(&v, 0.5)), "p99": ms(percentile(&v, 0.99)), "max": ms(v.last().copied().unwrap_or(f64::NAN)),
            "sent": values_sent, "got": s.values_got.load(Ordering::Relaxed), "stale": s.values_stale.load(Ordering::Relaxed), "settles": s.settles.load(Ordering::Relaxed) },
        "hold": { "p50": ms(percentile(&h, 0.5)), "p99": ms(percentile(&h, 0.99)), "max": ms(h.last().copied().unwrap_or(f64::NAN)) },
        "cues": { "p50": ms(percentile(&c, 0.5)), "p99": ms(percentile(&c, 0.99)), "max": ms(c.last().copied().unwrap_or(f64::NAN)),
            "expected": cues_expected, "got": s.cues_got.load(Ordering::Relaxed), "bad": s.cues_bad.load(Ordering::Relaxed) },
        "relay_cpu_pct": (relay_cpu.as_secs_f64() / secs * 1000.0).round() / 10.0,
        "payload_kbps": (s.bytes_in.load(Ordering::Relaxed) as f64 / 1024.0 / secs).round(),
        "wire_kbps": (wire as f64 / 1024.0 / secs).round(),
    });
    println!("{record}");
}
