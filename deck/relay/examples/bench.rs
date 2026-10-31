//! The relay bench (#223): the relay in-process, N simulated decks sending `deck.values` at 30 Hz to the conductor,
//! M audience phones receiving the conductor's cues at 5 Hz – p50 / p99 / max latency (deck → conductor, conductor →
//! phone), the relay's CPU (its own runtime's threads), bytes, the lowest link rate presence showed. No Docker, no
//! external server.
//!
//! `cargo run --release -p relay --example bench -- [--decks 20] [--listeners 100] [--seconds 10] [--port 7070]
//!  [--loss 1 [--rtt 10] [--unsent 4096] [--lossy both|down|up]]`
//!
//! Without `--loss` the clients connect over TCP to 127.0.0.1:`--port`. `--loss <percent>` gives every client its own
//! in-process lossy link instead (`relay::serve_stream` over pipes – Windows' loopback buffers megabytes per socket,
//! which would hide any backpressure). Real packet loss can't be made in-process, so the link models what a lost
//! segment does to a TCP stream: nothing is dropped, but the segment (and – in order – everything behind it) arrives
//! late: one-way `rtt / 2` for every segment, a lost one after its recovery (RACK / fast retransmit or a tail-loss
//! probe: `2.5 × rtt`; a lost retransmit waits the RTO, 200 ms). A link holds at most a congestion window (16 KB) per
//! direction – then it stops reading and the relay's side backs up (`--unsent` bytes: what the relay's kernel keeps
//! unsent – `relay::UNSENT` with TCP_NOTSENT_LOWAT). The real test is Linux `tc netem` (deck/relay/README.md "Bench").

use std::collections::VecDeque;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};

use futures_util::{SinkExt, StreamExt};
use protocol::{Envelope, Message};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite::tungstenite::Message as Ws;
use tokio_tungstenite::{client_async, WebSocketStream};

const CODE: &str = "BENCH";
/// a TCP segment's payload
const SEGMENT: usize = 1460;
/// what one direction of a link holds before it stops reading: a TCP congestion window (IW10 ≈ 15 KB)
const WINDOW: usize = 16 * 1024;
/// a client's receive buffer (it reads at once)
const CLIENT_BUFFER: usize = 64 * 1024;
/// a lost retransmit waits the retransmission timeout (Linux' minimum)
const RTO: Duration = Duration::from_millis(200);
/// the first second is not measured (joins, welcome snapshots)
const WARMUP: Duration = Duration::from_secs(1);

static START: OnceLock<Instant> = OnceLock::new();

fn t_us() -> f64 {
    START.get().expect("started").elapsed().as_micros() as f64
}

#[derive(Debug, Clone)]
struct Args {
    decks: usize,
    listeners: usize,
    seconds: u64,
    rtt_ms: f64,
    loss: f64,
    /// `--loss` given (also `--loss 0`: the link with its delay, nothing lost)
    lossy_link: bool,
    /// the bytes the relay's socket keeps unsent (the kernel: relay::UNSENT with TCP_NOTSENT_LOWAT, ~128 KB without)
    unsent: usize,
    /// which direction loses segments: both, `down` (relay → clients) or `up` (clients → relay)
    lossy: Lossy,
    port: u16,
}

#[derive(Debug, Clone, Copy)]
enum Lossy {
    Both,
    Down,
    Up,
}

fn args() -> Args {
    let mut a = Args { decks: 20, listeners: 100, seconds: 10, rtt_ms: 10.0, loss: 0.0, lossy_link: false, unsent: 4096, lossy: Lossy::Both, port: 7070 };
    let mut it = std::env::args().skip(1);
    while let Some(k) = it.next() {
        let v = it.next().unwrap_or_default();
        match k.as_str() {
            "--decks" => a.decks = v.parse().expect("--decks N"),
            "--listeners" => a.listeners = v.parse().expect("--listeners M"),
            "--seconds" => a.seconds = v.parse().expect("--seconds S"),
            "--rtt" => a.rtt_ms = v.parse().expect("--rtt ms"),
            "--loss" => {
                a.loss = v.parse().expect("--loss percent");
                a.lossy_link = true;
            }
            "--port" => a.port = v.parse().expect("--port P"),
            "--unsent" => a.unsent = v.parse().expect("--unsent bytes"),
            "--lossy" => {
                a.lossy = match v.as_str() {
                    "down" => Lossy::Down,
                    "up" => Lossy::Up,
                    _ => Lossy::Both,
                }
            }
            _ => panic!("unknown argument {k} (see the file's header)"),
        }
    }
    a
}

/// what the clients saw after the warm-up
#[derive(Default)]
struct Stats {
    values_us: Mutex<Vec<f64>>,
    cues_us: Mutex<Vec<f64>>,
    values_sent: AtomicU64,
    values_got: AtomicU64,
    cues_sent: AtomicU64,
    cues_got: AtomicU64,
    /// cues that came out of order or with a gap
    cues_bad: AtomicU64,
    bytes_in: AtomicU64,
    reconnects: AtomicU64,
    /// the lowest link rate presence showed (#223), u64::MAX = none
    lowest_hz: AtomicU64,
}

fn measuring() -> bool {
    START.get().is_some_and(|s| s.elapsed() >= WARMUP)
}

fn percentile(sorted: &[f64], p: f64) -> f64 {
    if sorted.is_empty() {
        return f64::NAN;
    }
    let i = ((sorted.len() as f64 - 1.0) * p).round() as usize;
    sorted[i]
}

/// The CPU time this thread used so far.
fn thread_cpu() -> Duration {
    #[cfg(unix)]
    {
        let mut ts = libc::timespec { tv_sec: 0, tv_nsec: 0 };
        // SAFETY: a valid out pointer; CLOCK_THREAD_CPUTIME_ID exists on Linux and macOS
        unsafe { libc::clock_gettime(libc::CLOCK_THREAD_CPUTIME_ID, &mut ts) };
        Duration::new(ts.tv_sec as u64, ts.tv_nsec as u32)
    }
    #[cfg(windows)]
    {
        #[repr(C)]
        #[derive(Default, Clone, Copy)]
        struct FileTime {
            low: u32,
            high: u32,
        }
        #[link(name = "kernel32")]
        extern "system" {
            fn GetCurrentThread() -> isize;
            fn GetThreadTimes(h: isize, c: *mut FileTime, e: *mut FileTime, k: *mut FileTime, u: *mut FileTime) -> i32;
        }
        let (mut c, mut e, mut k, mut u) = (FileTime::default(), FileTime::default(), FileTime::default(), FileTime::default());
        // SAFETY: the pseudo handle of the calling thread, valid out pointers
        unsafe { GetThreadTimes(GetCurrentThread(), &mut c, &mut e, &mut k, &mut u) };
        let t100 = |f: FileTime| ((f.high as u64) << 32) | f.low as u64;
        Duration::from_nanos((t100(k) + t100(u)) * 100)
    }
    #[cfg(not(any(unix, windows)))]
    Duration::ZERO
}

/// the relay's threads add their CPU time here when they stop
static RELAY_CPU_NS: AtomicU64 = AtomicU64::new(0);

/// Windows' timers tick every 15.6 ms unless a process asks for 1 ms – the link's delays would round up to that.
fn fine_timers() {
    #[cfg(windows)]
    {
        #[link(name = "winmm")]
        extern "system" {
            fn timeBeginPeriod(ms: u32) -> u32;
        }
        // SAFETY: a plain call; the period ends with the process
        unsafe { timeBeginPeriod(1) };
    }
}

fn main() {
    let a = args();
    fine_timers();
    let relay_rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .thread_name("relay")
        .enable_all()
        .on_thread_stop(|| {
            RELAY_CPU_NS.fetch_add(thread_cpu().as_nanos() as u64, Ordering::Relaxed);
        })
        .build()
        .expect("relay runtime");
    let relay = Arc::new(relay::Relay::new());
    let net = if a.lossy_link {
        Net::Lossy { relay, at: relay_rt.handle().clone(), rtt_ms: a.rtt_ms, loss: a.loss / 100.0, unsent: a.unsent, lossy: a.lossy }
    } else {
        let addr = format!("127.0.0.1:{}", a.port);
        let listener = relay_rt.block_on(TcpListener::bind(&addr)).expect("the relay's port");
        relay_rt.spawn(relay::serve(listener, relay));
        Net::Direct(addr)
    };

    let clients = tokio::runtime::Builder::new_multi_thread().enable_all().build().expect("client runtime");
    let stats = Arc::new(Stats { lowest_hz: AtomicU64::new(u64::MAX), ..Stats::default() });
    let wall = clients.block_on(run(a.clone(), Arc::new(net), stats.clone()));
    relay_rt.shutdown_timeout(Duration::from_secs(2));
    clients.shutdown_timeout(Duration::from_secs(1));
    report(&a, &stats, wall, Duration::from_nanos(RELAY_CPU_NS.load(Ordering::Relaxed)));
}

async fn run(a: Args, net: Arc<Net>, stats: Arc<Stats>) -> Duration {
    START.get_or_init(Instant::now);
    let url = |role: &str, group: &str| format!("/live/{CODE}?role={role}&group={group}");

    // the conductor first (it gets the values), then the phones, then the decks
    let conductor = dial(&net, &url("conductor", "")).await;
    let (mut cond_tx, mut cond_rx) = conductor.split();
    cond_tx
        .send(Ws::Text(format!(r#"{{"v":1,"seq":0,"from":"","kind":"hello","p":{{"code":"{CODE}","role":"conductor","token":"bench"}}}}"#)))
        .await
        .unwrap();
    let s = stats.clone();
    tokio::spawn(async move {
        while let Some(Ok(frame)) = cond_rx.next().await {
            let Ws::Text(text) = frame else { continue };
            let now = t_us();
            if !measuring() {
                continue;
            }
            s.bytes_in.fetch_add(text.len() as u64, Ordering::Relaxed);
            if text.contains("\"kind\":\"presence\"") {
                // #223: the lowest rate the relay gave any link (presence `links`, read loosely: an older relay has none)
                let p: serde_json::Value = serde_json::from_str(&text).unwrap_or_default();
                for hz in p["p"]["links"].as_object().into_iter().flat_map(|l| l.values()).filter_map(|l| l["hz"].as_u64()) {
                    s.lowest_hz.fetch_min(hz, Ordering::Relaxed);
                }
                continue;
            }
            if !text.contains("\"deck.values\"") {
                continue;
            }
            let Ok(Envelope { message: Message::DeckValues(v), .. }) = serde_json::from_str::<Envelope>(&text) else { continue };
            if let Some(t) = v.values.get("bench.t").and_then(|x| x.first()) {
                s.values_got.fetch_add(1, Ordering::Relaxed);
                s.values_us.lock().unwrap().push(now - t);
            }
        }
    });

    for i in 0..a.listeners {
        let ws = dial(&net, &url("audience", "a")).await;
        let s = stats.clone();
        tokio::spawn(async move {
            let (mut tx, mut rx) = ws.split();
            let hello = format!(r#"{{"v":1,"seq":0,"from":"","kind":"hello","p":{{"code":"{CODE}","role":"audience","group":"a","nick":"p{i}"}}}}"#);
            tx.send(Ws::Text(hello)).await.unwrap();
            let mut last: Option<u64> = None;
            while let Some(Ok(frame)) = rx.next().await {
                let Ws::Text(text) = frame else { continue };
                let now = t_us();
                if !measuring() {
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

    let end = Instant::now() + WARMUP + Duration::from_secs(a.seconds);
    for d in 0..a.decks {
        let ws = dial(&net, &url("musician", "decks")).await;
        let s = stats.clone();
        tokio::spawn(async move {
            let (mut tx, mut rx) = ws.split();
            tokio::spawn(async move { while let Some(Ok(_)) = rx.next().await {} });
            let hello = format!(r#"{{"v":1,"seq":0,"from":"","kind":"hello","p":{{"code":"{CODE}","role":"musician","group":"decks"}}}}"#);
            tx.send(Ws::Text(hello)).await.unwrap();
            let graph = r#"{"v":1,"seq":0,"from":"","kind":"deck.graph","p":{"nodes":[{"id":"lfo","kind":"lfo"},{"id":"space","kind":"space"}],"links":[{"from":"lfo.out","to":"space.hue"}],"character":"catpope"}}"#;
            tx.send(Ws::Text(graph.into())).await.unwrap();
            // 30 Hz, the decks out of phase
            tokio::time::sleep(Duration::from_micros(33_333 * d as u64 / a.decks.max(1) as u64)).await;
            let mut every = tokio::time::interval(Duration::from_micros(33_334));
            every.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
            let mut x = 0.0f64;
            while Instant::now() < end {
                every.tick().await;
                x = (x + 0.01) % 1.0;
                let msg = format!(
                    r#"{{"v":1,"seq":0,"from":"","kind":"deck.values","p":{{"values":{{"bench.t":[{}],"lfo.rate":[{x:.3}],"space.hue":[{x:.3}]}}}}}}"#,
                    t_us()
                );
                if tx.send(Ws::Text(msg)).await.is_err() {
                    s.reconnects.fetch_add(1, Ordering::Relaxed);
                    break;
                }
                if measuring() {
                    s.values_sent.fetch_add(1, Ordering::Relaxed);
                }
            }
        });
    }

    // the conductor's cues to the phones, 5 Hz
    let wall_start = Instant::now();
    let mut every = tokio::time::interval(Duration::from_millis(200));
    let mut n = 0u64;
    while Instant::now() < end {
        every.tick().await;
        if !measuring() {
            continue;
        }
        let cue = format!(r#"{{"v":1,"seq":0,"from":"","to":{{"group":"a"}},"kind":"cue","p":{{"id":"{n}:{}","actions":[]}}}}"#, t_us());
        n += 1;
        stats.cues_sent.fetch_add(1, Ordering::Relaxed);
        if cond_tx.send(Ws::Text(cue)).await.is_err() {
            stats.reconnects.fetch_add(1, Ordering::Relaxed);
            break;
        }
    }
    // the last messages in flight
    tokio::time::sleep(Duration::from_millis(500)).await;
    wall_start.elapsed()
}

fn report(a: &Args, s: &Stats, wall: Duration, relay_cpu: Duration) {
    let mut v = std::mem::take(&mut *s.values_us.lock().unwrap());
    let mut c = std::mem::take(&mut *s.cues_us.lock().unwrap());
    v.sort_by(f64::total_cmp);
    c.sort_by(f64::total_cmp);
    let ms = |x: f64| x / 1000.0;
    let secs = wall.as_secs_f64();
    println!(
        "decks {} · listeners {} · {} s · link: {}",
        a.decks,
        a.listeners,
        a.seconds,
        if a.lossy_link { format!("rtt {} ms, loss {} %", a.rtt_ms, a.loss) } else { "direct".into() }
    );
    println!(
        "values deck → conductor: p50 {:.1} ms · p99 {:.1} ms · max {:.1} ms · {} sent, {} arrived",
        ms(percentile(&v, 0.5)),
        ms(percentile(&v, 0.99)),
        ms(v.last().copied().unwrap_or(f64::NAN)),
        s.values_sent.load(Ordering::Relaxed),
        s.values_got.load(Ordering::Relaxed)
    );
    println!(
        "cues conductor → phone: p50 {:.1} ms · p99 {:.1} ms · max {:.1} ms · {} × {} sent, {} arrived, {} out of order / gaps",
        ms(percentile(&c, 0.5)),
        ms(percentile(&c, 0.99)),
        ms(c.last().copied().unwrap_or(f64::NAN)),
        s.cues_sent.load(Ordering::Relaxed),
        a.listeners,
        s.cues_got.load(Ordering::Relaxed),
        s.cues_bad.load(Ordering::Relaxed)
    );
    println!(
        "relay CPU {:.1} % of one core · clients received {:.0} KB/s · broken connections {}",
        relay_cpu.as_secs_f64() / (secs + WARMUP.as_secs_f64()) * 100.0,
        s.bytes_in.load(Ordering::Relaxed) as f64 / 1024.0 / secs,
        s.reconnects.load(Ordering::Relaxed)
    );
    if let hz @ 0..=999 = s.lowest_hz.load(Ordering::Relaxed) {
        println!("lowest link rate presence showed: {hz} / s");
    }
    println!(
        "| {} | {} | {} | {:.1} | {:.1} | {:.1} | {:.1} | {:.1} | {:.0} |",
        a.decks,
        a.listeners,
        if a.lossy_link { format!("{} % / {} ms", a.loss, a.rtt_ms) } else { "direct".into() },
        ms(percentile(&v, 0.5)),
        ms(percentile(&v, 0.99)),
        ms(percentile(&c, 0.5)),
        ms(percentile(&c, 0.99)),
        relay_cpu.as_secs_f64() / (secs + WARMUP.as_secs_f64()) * 100.0,
        s.bytes_in.load(Ordering::Relaxed) as f64 / 1024.0 / secs,
    );
}

// --- the lossy link -------------------------------------------------------------------------------------------------

/// xorshift64* – no dependency for a coin flip
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> f64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        (self.0.wrapping_mul(0x2545_f491_4f6c_dd1d) >> 11) as f64 / (1u64 << 53) as f64
    }
}

static SEED: AtomicU64 = AtomicU64::new(0x9e37_79b9_7f4a_7c15);

/// any byte stream a client can speak WebSocket over
trait Io: AsyncRead + AsyncWrite + Unpin + Send {}
impl<T: AsyncRead + AsyncWrite + Unpin + Send> Io for T {}

/// How clients reach the relay: its TCP port, or each over its own in-process lossy link.
enum Net {
    Direct(String),
    Lossy { relay: Arc<relay::Relay>, at: tokio::runtime::Handle, rtt_ms: f64, loss: f64, unsent: usize, lossy: Lossy },
}

/// Connect a client to `path` (`/live/<code>?role=…`).
async fn dial(net: &Net, path: &str) -> WebSocketStream<Box<dyn Io>> {
    let io: Box<dyn Io> = match net {
        Net::Direct(addr) => {
            let tcp = TcpStream::connect(addr).await.expect("the relay's port");
            tcp.set_nodelay(true).unwrap();
            Box::new(tcp)
        }
        Net::Lossy { relay, at, rtt_ms, loss, unsent, lossy } => {
            // relay ⇄ its socket (what the kernel keeps unsent) ⇄ the link ⇄ the client's socket ⇄ client
            let (relay_end, link_up) = tokio::io::duplex(*unsent);
            let (link_down, client_end) = tokio::io::duplex(CLIENT_BUFFER);
            at.spawn(relay::serve_stream(relay_end, relay.clone()));
            let (ur, uw) = tokio::io::split(link_up);
            let (dr, dw) = tokio::io::split(link_down);
            let seed = SEED.fetch_add(0x9e37_79b9_7f4a_7c15, Ordering::Relaxed);
            let down = if matches!(lossy, Lossy::Up) { 0.0 } else { *loss };
            let up = if matches!(lossy, Lossy::Down) { 0.0 } else { *loss };
            tokio::spawn(pipe(ur, dw, *rtt_ms, down, seed));
            tokio::spawn(pipe(dr, uw, *rtt_ms, up, seed ^ 0xdead_beef));
            Box::new(client_end)
        }
    };
    client_async(format!("ws://relay{path}"), io).await.expect("the WebSocket handshake").0
}

/// What one direction of the link carries: segments with their arrival time, in order
#[derive(Default)]
struct InFlight {
    q: Mutex<Segments>,
    /// a segment was queued
    ready: tokio::sync::Notify,
    /// a segment arrived (room in the window)
    room: tokio::sync::Notify,
}

#[derive(Default)]
struct Segments {
    segments: VecDeque<(Instant, Vec<u8>)>,
    bytes: usize,
    closed: bool,
}

/// One direction of a lossy link: each segment arrives `rtt / 2` later, a lost one after its recovery, in order.
async fn pipe(mut r: impl AsyncRead + Unpin + Send + 'static, mut w: impl AsyncWrite + Unpin + Send + 'static, rtt_ms: f64, loss: f64, seed: u64) {
    let queue = Arc::new(InFlight::default());
    let q = queue.clone();
    let writer = tokio::spawn(async move {
        enum Next {
            Write(Vec<u8>),
            Sleep(Instant),
            Wait,
            Done,
        }
        loop {
            let next = {
                let mut g = q.q.lock().unwrap();
                match g.segments.front() {
                    Some((due, _)) if *due <= Instant::now() => {
                        let (_, bytes) = g.segments.pop_front().unwrap();
                        g.bytes -= bytes.len();
                        Next::Write(bytes)
                    }
                    Some((due, _)) => Next::Sleep(*due),
                    None if g.closed => Next::Done,
                    None => Next::Wait,
                }
            };
            match next {
                Next::Write(bytes) => {
                    q.room.notify_one();
                    if w.write_all(&bytes).await.is_err() {
                        return;
                    }
                }
                Next::Sleep(due) => tokio::time::sleep_until(due.into()).await,
                Next::Wait => q.ready.notified().await,
                Next::Done => return,
            }
        }
    });
    let mut rng = Rng(seed | 1);
    let one_way = Duration::from_secs_f64(rtt_ms / 2000.0);
    let recover = Duration::from_secs_f64(rtt_ms * 2.5 / 1000.0);
    let mut last_due = Instant::now();
    let mut buf = vec![0u8; SEGMENT];
    loop {
        // a full window: wait until the link delivered some (the relay's socket backs up meanwhile)
        while queue.q.lock().unwrap().bytes >= WINDOW {
            queue.room.notified().await;
        }
        let n = match r.read(&mut buf).await {
            Ok(0) | Err(_) => break,
            Ok(n) => n,
        };
        let now = Instant::now();
        let mut due = now + one_way;
        if rng.next() < loss {
            due += recover;
            if rng.next() < loss {
                due += RTO;
            }
        }
        // TCP delivers in order: nothing overtakes a late segment
        due = due.max(last_due);
        last_due = due;
        let mut g = queue.q.lock().unwrap();
        g.segments.push_back((due, buf[..n].to_vec()));
        g.bytes += n;
        drop(g);
        queue.ready.notify_one();
    }
    queue.q.lock().unwrap().closed = true;
    queue.ready.notify_one();
    let _ = writer.await;
}
