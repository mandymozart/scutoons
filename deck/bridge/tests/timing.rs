//! The bridge against the real relay: a `device` message plays at its `at` (not before), a later one waits in the queue
//! without blocking, and `panic` empties the queue and silences the adapters. A recording adapter stands in for MIDI.
use std::sync::{Arc, Mutex};
use std::time::Duration;

use adapters_core::{Adapter, Inbound};
use futures_util::{SinkExt, StreamExt};
use protocol::{Device, DeviceMsg, Envelope, Hello, MidiBytes, Message, Panic, Role};
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

#[tokio::test]
async fn plays_at_at_and_panics() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    tokio::spawn(relay::serve(listener, Arc::new(relay::Relay::new())));

    let log = Arc::new(Mutex::new(Log::default()));
    let patch = bridge::Patch::parse(&format!("relay = \"ws://127.0.0.1:{port}/live/BT\"\ntoken = \"dev\"\n")).unwrap();
    let rec = log.clone();
    tokio::spawn(async move {
        let mut adapters: Vec<(String, Box<dyn Adapter>)> = vec![("synth".into(), Box::new(Recorder(rec)))];
        let (_tx, mut rx) = mpsc::channel(8);
        bridge::session(&patch, &mut adapters, &mut rx).await
    });

    // the conductor
    let (mut ws, _) = tokio_tungstenite::connect_async(format!("ws://127.0.0.1:{port}/live/BT")).await.unwrap();
    let hello = Hello { code: "BT".into(), role: Role::Conductor, group: None, nick: None, token: Some("dev".into()), resume: None };
    let send = |m: Message| Ws::Text(serde_json::to_string(&Envelope::new("", m)).unwrap());
    ws.send(send(Message::Hello(hello))).await.unwrap();
    // wait until the bridge has joined (presence counts no bridge – wait for its `devices` instead)
    loop {
        let Some(Ok(Ws::Text(t))) = ws.next().await else { panic!("closed") };
        if serde_json::from_str::<Envelope>(&t).unwrap().message.kind() == "devices" {
            break;
        }
    }

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
