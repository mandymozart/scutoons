//! The bridge against the real relay: a `device` message plays at its `at` (not before), a later one waits in the queue
//! without blocking, and `panic` empties the queue and silences the adapters. A recording adapter stands in for MIDI;
//! the DMX test runs the real dmx-usbpro adapter on its in-memory widget.
use std::future::Future;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use adapters_core::{Adapter, Inbound};
use adapters_dmx_usbpro::mock::Widget;
use adapters_dmx_usbpro::DmxUsbProAdapter;
use futures_util::{SinkExt, StreamExt};
use protocol::{Device, DeviceMsg, DmxMessage, Envelope, Hello, MidiBytes, Message, Panic, Role};
use tokio::net::TcpListener;
use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::Message as Ws;

#[derive(Default)]
struct Log {
    sent: Vec<(i64, Vec<u8>)>,
    panics: Vec<i64>,
}

struct Recorder(Arc<Mutex<Log>>);

impl Adapter for Recorder {
    fn name(&self) -> &'static str {
        "midi"
    }
    fn start(&mut self, _: mpsc::Sender<Inbound>) -> anyhow::Result<()> {
        Ok(())
    }
    fn send(&mut self, msg: &DeviceMsg) -> anyhow::Result<()> {
        let DeviceMsg::Midi(m) = msg else { anyhow::bail!("midi only") };
        self.0.lock().unwrap().sent.push((bridge::now_ms(), m.bytes.clone()));
        Ok(())
    }
    fn panic(&mut self) -> anyhow::Result<()> {
        self.0.lock().unwrap().panics.push(bridge::now_ms());
        Ok(())
    }
    fn stop(&mut self) -> anyhow::Result<()> {
        Ok(())
    }
}

/// How long a step may take on a loaded machine before the test gives up on it.
const WAIT: Duration = Duration::from_secs(60);

/// The timing bounds are real time (thread wake-ups, the clock samples at join): one try inside them passes, a
/// scheduler that ignores `at` never does.
const TRIES: usize = 8;

/// Polls `ok` until it holds; false after WAIT.
async fn eventually(ok: impl Fn() -> bool) -> bool {
    let end = Instant::now() + WAIT;
    while !ok() {
        if Instant::now() > end {
            return false;
        }
        tokio::time::sleep(Duration::from_millis(2)).await;
    }
    true
}

async fn until_relay_ms(t: i64) {
    let wait = t - relay::now_ms();
    if wait > 0 {
        tokio::time::sleep(Duration::from_millis(wait as u64)).await;
    }
}

/// Runs `one` until a try comes back inside its bounds; panics with every miss otherwise.
async fn on_time<F: Future<Output = Result<(), String>>>(one: impl Fn() -> F) {
    let mut missed = vec![];
    for _ in 0..TRIES {
        match one().await {
            Ok(()) => return,
            Err(m) => missed.push(m),
        }
    }
    panic!("no try on time: {missed:?}");
}

fn device(at: i64, note: u8) -> Message {
    let msgs = vec![DeviceMsg::Midi(MidiBytes { bytes: vec![0x90, note, 100] })];
    Message::Device(Device { outlet: "synth".into(), msgs, at: Some(at) })
}

type Conductor = tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>;

fn send(m: Message) -> Ws {
    Ws::Text(serde_json::to_string(&Envelope::new("", m)).unwrap().into())
}

/// A relay, a bridge session with `adapters` (started), and a conductor that waits until the bridge has joined.
async fn stage(mut adapters: Vec<(String, Box<dyn Adapter>)>) -> Conductor {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    tokio::spawn(relay::serve(listener, Arc::new(relay::Relay::new())));

    let patch = bridge::Patch::parse(&format!("relay = \"ws://127.0.0.1:{port}/live/BT\"\ntoken = \"dev\"\n")).unwrap();
    tokio::spawn(async move {
        let (tx, mut rx) = mpsc::channel(8);
        for (_, a) in adapters.iter_mut() {
            a.start(tx.clone()).unwrap();
        }
        bridge::session(&patch, &mut adapters, &mut rx).await
    });

    let (mut ws, _) = tokio_tungstenite::connect_async(format!("ws://127.0.0.1:{port}/live/BT")).await.unwrap();
    let hello = Hello { code: "BT".into(), role: Role::Conductor, group: None, nick: None, token: Some("dev".into()), device: None, proof: None, resume: None };
    ws.send(send(Message::Hello(hello))).await.unwrap();
    // presence counts no bridge – its `devices` says it has joined
    tokio::time::timeout(WAIT, async {
        loop {
            let Some(Ok(Ws::Text(t))) = ws.next().await else { panic!("closed") };
            if serde_json::from_str::<Envelope>(&t).unwrap().message.kind() == "devices" {
                break;
            }
        }
    })
    .await
    .expect("the bridge never joined");
    ws
}

#[tokio::test]
async fn plays_at_at_and_panics() {
    on_time(|| async {
        let log = Arc::new(Mutex::new(Log::default()));
        let mut ws = stage(vec![("synth".into(), Box::new(Recorder(log.clone())))]).await;

        let t0 = relay::now_ms();
        let far = t0 + 2_000;
        ws.send(send(device(far, 61))).await.unwrap(); // must not block the next one
        ws.send(send(device(t0 + 300, 60))).await.unwrap();
        assert!(eventually(|| !log.lock().unwrap().sent.is_empty()).await, "the near one never played");
        let (played, bytes) = log.lock().unwrap().sent[0].clone();
        assert_eq!(bytes, [0x90, 60, 100], "the near one plays first, the far one waits");
        let late = played - (t0 + 300);

        ws.send(send(Message::Panic(Panic { black: None, stop: Some(true) }))).await.unwrap();
        assert!(eventually(|| !log.lock().unwrap().panics.is_empty()).await, "no panic");
        let panicked = log.lock().unwrap().panics[0];
        if panicked < far {
            until_relay_ms(far + 300).await;
            let l = log.lock().unwrap();
            assert_eq!(l.panics.len(), 1);
            assert_eq!(l.sent.len(), 1, "panic dropped the queued note");
        }
        if !(0..60).contains(&late) {
            return Err(format!("played {late} ms after `at`"));
        }
        if panicked >= far {
            return Err(format!("the panic came {} ms after the far note was due", panicked - far));
        }
        Ok(())
    })
    .await;
}

/// Waits off the runtime (the mock widget is polled) until its last frame satisfies `ok`.
async fn frame(widget: &Widget, ok: impl Fn(&[u8; 512]) -> bool + Send + 'static) -> Option<Instant> {
    let w = widget.clone();
    tokio::task::spawn_blocking(move || w.wait_frame(WAIT, ok)).await.unwrap()
}

/// When the first frame since `since` that satisfies `ok` arrived.
fn first_frame(widget: &Widget, since: Instant, ok: impl Fn(&[u8; 512]) -> bool) -> Instant {
    widget.frames().into_iter().find(|(t, f)| *t >= since && ok(f)).expect("a frame that was seen").0
}

/// DMX through the queue with the real dmx-usbpro adapter: the look reaches the widget at `at` (not before), a fade
/// runs on the 40 Hz frames, and `panic` blacks the universe out at once.
#[tokio::test]
async fn dmx_plays_at_at_and_blacks_out_on_panic() {
    on_time(|| async {
        let widget = Widget::default();
        let dmx = DmxUsbProAdapter::with_opener("lights", None, widget.opener());
        let mut ws = stage(vec![("lights".into(), Box::new(dmx))]).await;
        frame(&widget, |_| true).await.expect("the widget is connected");

        let look = |ch, values: &[u8], fade| DeviceMsg::Dmx(DmxMessage { ch, values: values.to_vec(), fade });
        let msgs = vec![look(1, &[255, 0, 128], None), look(10, &[200], Some(400))];
        let t0 = relay::now_ms();
        let sent = Instant::now();
        ws.send(send(Message::Device(Device { outlet: "lights".into(), msgs, at: Some(t0 + 300) }))).await.unwrap();

        frame(&widget, |f| f[..3] == [255, 0, 128]).await.expect("the look");
        let lit = first_frame(&widget, sent, |f| f[..3] == [255, 0, 128]);
        // `at` is in whole ms of the relay clock (t0 read just before `sent`): up to a ms early against `sent` is on time
        let late = (lit - sent).as_millis() as i64 - 300;

        frame(&widget, |f| f[9] == 200).await.expect("faded in");
        let fade: Vec<u8> = widget.frames().iter().filter(|(t, _)| *t >= lit).map(|(_, f)| f[9]).collect();
        assert!(fade.windows(2).all(|w| w[0] <= w[1]), "the fade only rises: {fade:?}");

        let panic_at = Instant::now();
        ws.send(send(Message::Panic(Panic { black: Some(true), stop: None }))).await.unwrap();
        frame(&widget, |f| f.iter().all(|&v| v == 0)).await.expect("black");
        let black = first_frame(&widget, panic_at, |f| f.iter().all(|&v| v == 0)) - panic_at;

        if !(-2..60).contains(&late) {
            return Err(format!("lit {late} ms after `at`"));
        }
        if !fade.iter().any(|v| (1..200).contains(v)) {
            return Err(format!("no frame on the way: {fade:?}"));
        }
        if black >= Duration::from_millis(100) {
            return Err(format!("black after {black:?}"));
        }
        Ok(())
    })
    .await;
}
