//! `live-cue` – a stand-in conductor for trying a deck outlet (#186) before the conductor web app exists: joins a relay
//! as the conductor (a dev relay takes any token), preloads the outlet, waits for the deck's `media.ready`, then sends
//! the script's cells one after the other, each held its `hold` seconds.
//!
//!   cargo run -p deck-native --example live-cue -- ws://127.0.0.1:7700/live/DECK native/examples/sermon.json [--token T] [--loop]
//!
//! The script: `{ "outlet": "left", "preload": [MediaItem…], "cells": [{ "label": "…", "hold": 8, "cell": Cell }…] }` –
//! the protocol's types (deck/protocol/schema.json). `sermon.json` is research/LIVE-LENSES.md §5's worked example.
use protocol::{CellChange, Envelope, Hello, MediaItem, MediaPreload, Message, Role};
use serde::Deserialize;
use std::time::{Duration, Instant};
use tungstenite::stream::MaybeTlsStream;
use tungstenite::Message as Ws;

#[derive(Deserialize)]
struct Script {
    outlet: String,
    #[serde(default)]
    preload: Vec<MediaItem>,
    cells: Vec<Step>,
}

#[derive(Deserialize)]
struct Step {
    #[serde(default)]
    label: String,
    hold: f32,
    cell: protocol::Cell,
}

type Socket = tungstenite::WebSocket<MaybeTlsStream<std::net::TcpStream>>;

fn send(ws: &mut Socket, m: Message) -> anyhow::Result<()> {
    ws.send(Ws::Text(serde_json::to_string(&Envelope::new("", m))?.into()))?;
    Ok(())
}

/// read for `d`, printing what the conductor hears about the outlet; true once `media.ready` came
fn listen(ws: &mut Socket, d: Duration, outlet: &str) -> anyhow::Result<bool> {
    let until = Instant::now() + d;
    let mut ready = false;
    while Instant::now() < until {
        match ws.read() {
            Ok(Ws::Text(t)) => match serde_json::from_str::<Envelope>(&t)?.message {
                Message::MediaReady(r) if r.outlet == outlet => {
                    println!("media.ready: ready {:?} missing {:?} {}", r.ready, r.missing, r.why.map(|w| format!("{w:?}")).unwrap_or_default());
                    ready = true;
                }
                Message::Presence(p) => println!("presence: {:?} ready {:?}", p.groups, p.ready),
                Message::Error(e) => println!("relay: {} {}", e.code, e.message),
                _ => {}
            },
            Ok(_) => {}
            Err(tungstenite::Error::Io(e)) if matches!(e.kind(), std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut) => {}
            Err(e) => return Err(e.into()),
        }
    }
    Ok(ready)
}

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (Some(relay), Some(file)) = (args.first(), args.get(1)) else {
        anyhow::bail!("live-cue <ws://host:port/live/CODE> <script.json> [--token T] [--loop]");
    };
    let token = args.iter().position(|a| a == "--token").and_then(|i| args.get(i + 1)).cloned().unwrap_or_else(|| "dev".into());
    let looping = args.iter().any(|a| a == "--loop");
    let script: Script = serde_json::from_str(&std::fs::read_to_string(file)?)?;
    let code = relay.rsplit('/').next().unwrap_or_default().to_string();
    let (mut ws, _) = tungstenite::connect(relay.as_str())?;
    if let MaybeTlsStream::Plain(s) = ws.get_ref() {
        s.set_read_timeout(Some(Duration::from_millis(100)))?;
    }
    send(&mut ws, Message::Hello(Hello { code, role: Role::Conductor, group: None, nick: Some("live-cue".into()), token: Some(token), device: None, proof: None, resume: None }))?;
    listen(&mut ws, Duration::from_millis(500), &script.outlet)?;
    if !script.preload.is_empty() {
        send(&mut ws, Message::MediaPreload(MediaPreload { outlet: script.outlet.clone(), items: script.preload.clone() }))?;
        println!("media.preload: {} items – waiting up to 30 s for the deck", script.preload.len());
        let until = Instant::now() + Duration::from_secs(30);
        while Instant::now() < until && !listen(&mut ws, Duration::from_millis(500), &script.outlet)? {}
    }
    loop {
        for (i, step) in script.cells.iter().enumerate() {
            println!("cell {} / {}: {} ({} s)", i + 1, script.cells.len(), step.label, step.hold);
            send(&mut ws, Message::Cell(CellChange { outlet: script.outlet.clone(), cell: step.cell.clone() }))?;
            listen(&mut ws, Duration::from_secs_f32(step.hold.max(0.1)), &script.outlet)?;
        }
        if !looping {
            return Ok(());
        }
    }
}
