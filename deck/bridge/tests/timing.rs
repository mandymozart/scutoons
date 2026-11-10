//! The bridge against the real relay: a `device` message plays at its `at` (not before), a later one waits in the queue
//! without blocking, and `panic` empties the queue and silences the adapters. A recording adapter stands in for MIDI;
//! the DMX test runs the real dmx-usbpro adapter on its in-memory widget.
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
    panics: usize,
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
        self.0.lock().unwrap().panics += 1;
        Ok(())
    }
    fn stop(&mut self) -> anyhow::Result<()> {
        Ok(())
    }
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
    let hello = Hello { code: "BT".into(), role: Role::Conductor, group: None, nick: None, token: Some("dev".into()), device: None, resume: None };
    ws.send(send(Message::Hello(hello))).await.unwrap();
    // wait until the bridge has joined (presence counts no bridge – wait for its `devices` instead)
    loop {
        let Some(Ok(Ws::Text(t))) = ws.next().await else { panic!("closed") };
        if serde_json::from_str::<Envelope>(&t).unwrap().message.kind() == "devices" {
            break;
        }
    }
    ws
}

#[tokio::test]
async fn plays_at_at_and_panics() {
    let log = Arc::new(Mutex::new(Log::default()));
    let mut ws = stage(vec![("synth".into(), Box::new(Recorder(log.clone())))]).await;

    let t0 = relay::now_ms();
    ws.send(send(device(t0 + 2_000, 61))).await.unwrap(); // far: must not block the next one
    ws.send(send(device(t0 + 300, 60))).await.unwrap();
    tokio::time::sleep(Duration::from_millis(150)).await;
    assert!(log.lock().unwrap().sent.is_empty(), "nothing before `at`");
    tokio::time::sleep(Duration::from_millis(350)).await;
    {
        let l = log.lock().unwrap();
        assert_eq!(l.sent.len(), 1, "the near one played, the far one waits");
        let (played, bytes) = &l.sent[0];
        assert_eq!(bytes, &[0x90, 60, 100]);
        let late = played - (t0 + 300);
        assert!((0..60).contains(&late), "played {late} ms after `at`");
    }
    ws.send(send(Message::Panic(Panic { black: None, stop: Some(true) }))).await.unwrap();
    tokio::time::sleep(Duration::from_millis(1_800)).await;
    let l = log.lock().unwrap();
    assert_eq!(l.panics, 1);
    assert_eq!(l.sent.len(), 1, "panic dropped the queued note");
}

/// Waits off the runtime (the mock widget is polled) until its last frame satisfies `ok`; the frame's arrival time.
async fn frame(widget: &Widget, timeout: Duration, ok: impl Fn(&[u8; 512]) -> bool + Send + 'static) -> Option<Instant> {
    let w = widget.clone();
    tokio::task::spawn_blocking(move || w.wait_frame(timeout, ok)).await.unwrap()
}

/// DMX through the queue with the real dmx-usbpro adapter: the look reaches the widget at `at` (not before), a fade
/// runs on the 40 Hz frames, and `panic` blacks the universe out at once.
#[tokio::test]
async fn dmx_plays_at_at_and_blacks_out_on_panic() {
    let widget = Widget::default();
    let dmx = DmxUsbProAdapter::with_opener("lights", None, widget.opener());
    let mut ws = stage(vec![("lights".into(), Box::new(dmx))]).await;
    frame(&widget, Duration::from_secs(2), |_| true).await.expect("the widget is connected");

    let look = |ch, values: &[u8], fade| DeviceMsg::Dmx(DmxMessage { ch, values: values.to_vec(), fade });
    let msgs = vec![look(1, &[255, 0, 128], None), look(10, &[200], Some(400))];
    let t0 = relay::now_ms();
    let sent = Instant::now();
    ws.send(send(Message::Device(Device { outlet: "lights".into(), msgs, at: Some(t0 + 300) }))).await.unwrap();

    let lit = frame(&widget, Duration::from_secs(2), |f| f[..3] == [255, 0, 128]).await.expect("the look");
    // `at` is in whole ms of the relay clock (t0 read just before `sent`): up to a ms early against `sent` is on time
    let late = (lit - sent).as_millis() as i64 - 300;
    assert!((-2..60).contains(&late), "lit {late} ms after `at`");

    // the fade on channel 10: on its way after ~200 ms (neither a jump nor done – how far depends on the runner's
    // load: macOS CI saw 27, run 41128816256), 200 at the end
    tokio::time::sleep(Duration::from_millis(200)).await;
    let mid = widget.last_frame().unwrap()[9];
    assert!((1..200).contains(&mid), "half-way: {mid}");
    frame(&widget, Duration::from_secs(1), |f| f[9] == 200).await.expect("faded in");

    let panic_at = Instant::now();
    ws.send(send(Message::Panic(Panic { black: Some(true), stop: None }))).await.unwrap();
    let black = frame(&widget, Duration::from_millis(500), |f| f.iter().all(|&v| v == 0)).await.expect("black");
    assert!(black - panic_at < Duration::from_millis(100), "black after {:?}", black - panic_at);
}
