//! The device bridge (agents/research/LIVE.md B.7, Rust per CYBERDECKS.md §5): a venue patch (B.5.4) in TOML names
//! the relay and the devices; each device becomes an adapter (`adapters-core::Adapter`); the bridge joins the relay
//! as `bridge`, reports its ports (`devices`), plays `device` messages at their `at`, and forwards what comes in as
//! `device.in` to the conductor.
//!
//! Not yet (each a `TODO` below): the clock offset from 5 ping samples, the status window, osc / dmx adapters.

use std::collections::HashMap;
use std::time::Duration;

use adapters_core::{Adapter, DeviceConfig, Inbound};
use adapters_midi::MidiAdapter;
use anyhow::{anyhow, Context};
use futures_util::{SinkExt, StreamExt};
use protocol::{Devices, Envelope, Hello, Message, Port, Role, To, DeviceIn};
use serde::Deserialize;
use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::Message as Ws;

/// The venue patch file (B.5.4). Example: deck/bridge/venue.example.toml.
#[derive(Debug, Clone, Deserialize)]
pub struct Patch {
    /// `ws://<relay>:7700/live/<code>`
    pub relay: String,
    /// The bridge's session token (B.6). `BRIDGE_TOKEN` env overrides.
    #[serde(default)]
    pub token: Option<String>,
    #[serde(default, rename = "device")]
    pub devices: Vec<DeviceConfig>,
}

impl Patch {
    pub fn parse(text: &str) -> anyhow::Result<Patch> {
        toml::from_str(text).context("venue patch")
    }

    /// The session code is the last path segment of `relay`.
    pub fn code(&self) -> anyhow::Result<String> {
        let path = self.relay.split('?').next().unwrap_or_default();
        path.rsplit('/').next().filter(|c| !c.is_empty()).map(str::to_owned).ok_or_else(|| anyhow!("relay URL without /live/<code>"))
    }
}

/// Build (not start) an adapter per device. Adapters that are not built yet are an error that names them
/// (CYBERDECKS.md §4: osc, dmx-artnet, dmx-sacn, dmx-usb, gpio, audio, video-in, display).
pub fn build_adapters(patch: &Patch) -> anyhow::Result<Vec<(String, Box<dyn Adapter>)>> {
    patch
        .devices
        .iter()
        .map(|d| {
            let adapter: Box<dyn Adapter> = match d.adapter.as_str() {
                "midi" => {
                    let port = d.options.get("port").and_then(|v| v.as_str()).ok_or_else(|| anyhow!("{}: midi needs `port`", d.outlet))?;
                    let input = d.options.get("input").and_then(|v| v.as_bool()).unwrap_or(false);
                    Box::new(MidiAdapter::new(&d.outlet, port, input))
                }
                other => return Err(anyhow!("{}: adapter {other:?} is not built yet (#172 Open)", d.outlet)),
            };
            Ok((d.outlet.clone(), adapter))
        })
        .collect()
}

fn ports(adapters: &[(String, Box<dyn Adapter>)]) -> Vec<Port> {
    let mut seen = std::collections::HashSet::new();
    adapters
        .iter()
        .flat_map(|(_, a)| a.ports().into_iter().map(move |p| (a.name(), p)))
        .filter(|(adapter, p)| seen.insert((adapter.to_string(), p.name.clone(), p.direction.to_string())))
        .map(|(adapter, p)| Port { adapter: adapter.into(), name: p.name, direction: p.direction.to_string() })
        .collect()
}

/// One relay session: connect, hello, devices, then route until the socket closes. The caller reconnects.
pub async fn session(patch: &Patch, adapters: &mut [(String, Box<dyn Adapter>)], inbound: &mut mpsc::Receiver<Inbound>) -> anyhow::Result<()> {
    let code = patch.code()?;
    let token = std::env::var("BRIDGE_TOKEN").ok().or_else(|| patch.token.clone()).unwrap_or_default();
    let (ws, _) = tokio_tungstenite::connect_async(&patch.relay).await.context("relay")?;
    let (mut sink, mut source) = ws.split();
    let hello = Hello { code, role: Role::Bridge, group: None, nick: None, token: Some(token), resume: None };
    sink.send(Ws::Text(serde_json::to_string(&Envelope::new("", Message::Hello(hello)))?)).await?;

    let welcome: Envelope = match source.next().await {
        Some(Ok(Ws::Text(t))) => serde_json::from_str(&t)?,
        other => return Err(anyhow!("no welcome: {other:?}")),
    };
    let Message::Welcome(w) = welcome.message else { return Err(anyhow!("relay refused: {:?}", welcome.message)) };
    // TODO(B.6 clock): 5 ping samples at join, offset = median of t1 − (t0 + rtt / 2); one sample for now.
    let offset_ms = now_ms() - w.now;
    tracing::info!(client = w.client, offset_ms, "joined the relay");

    sink.send(Ws::Text(serde_json::to_string(&Envelope::new("", Message::Devices(Devices { ports: ports(adapters) })))?)).await?;

    let mut by_outlet: HashMap<&str, &mut Box<dyn Adapter>> = adapters.iter_mut().map(|(o, a)| (o.as_str(), a)).collect();
    let mut keepalive = tokio::time::interval(Duration::from_secs(20));
    keepalive.tick().await;
    loop {
        tokio::select! {
            frame = source.next() => {
                let text = match frame {
                    Some(Ok(Ws::Text(t))) => t,
                    Some(Ok(Ws::Close(_))) | None => return Err(anyhow!("relay closed")),
                    Some(Err(e)) => return Err(e.into()),
                    Some(Ok(_)) => continue,
                };
                let Ok(envelope) = serde_json::from_str::<Envelope>(&text) else { continue };
                match envelope.message {
                    Message::Device(d) => {
                        // Play at `at` on the bridge's own clock (B.7: never a browser tab). A late message plays at once.
                        if let Some(at) = d.at {
                            let local = at + offset_ms;
                            let wait = local - now_ms();
                            if wait > 0 { tokio::time::sleep(Duration::from_millis(wait as u64)).await; }
                        }
                        match by_outlet.get_mut(d.outlet.as_str()) {
                            Some(adapter) => for msg in &d.msgs {
                                if let Err(e) = adapter.send(msg) { tracing::warn!(outlet = d.outlet, "send failed: {e}"); }
                            },
                            None => tracing::warn!(outlet = d.outlet, "device for an unpatched outlet"),
                        }
                    }
                    Message::Pong(_) => {}
                    Message::Panic(_) => { /* TODO(B.6 panic): all notes off / DMX black on every adapter */ }
                    _ => {}
                }
            }
            Some(inbound) = inbound.recv() => {
                let m = Message::DeviceIn(DeviceIn { outlet: inbound.outlet, msg: inbound.msg });
                sink.send(Ws::Text(serde_json::to_string(&Envelope::new("", m).to(To::role(Role::Conductor)))?)).await?;
            }
            _ = keepalive.tick() => {
                let ping = Message::Ping(protocol::Ping { t0: now_ms() });
                sink.send(Ws::Text(serde_json::to_string(&Envelope::new("", ping))?)).await?;
            }
        }
    }
}

pub fn now_ms() -> i64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis() as i64).unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    const EXAMPLE: &str = include_str!("../venue.example.toml");

    #[test]
    fn example_patch_parses_and_builds() {
        let patch = Patch::parse(EXAMPLE).unwrap();
        assert_eq!(patch.code().unwrap(), "A7K2");
        assert_eq!(patch.devices.len(), 2);
        let adapters = build_adapters(&patch).unwrap();
        assert_eq!(adapters[0].0, "ableton");
        assert_eq!(adapters[0].1.name(), "midi");
    }

    #[test]
    fn unknown_adapter_is_named() {
        let patch = Patch::parse("relay = \"ws://x/live/Q\"\n[[device]]\noutlet = \"lights\"\nadapter = \"dmx-artnet\"\n").unwrap();
        let err = build_adapters(&patch).err().expect("an error").to_string();
        assert!(err.contains("dmx-artnet") && err.contains("lights"), "{err}");
    }

    #[test]
    fn midi_needs_a_port() {
        let patch = Patch::parse("relay = \"ws://x/live/Q\"\n[[device]]\noutlet = \"a\"\nadapter = \"midi\"\n").unwrap();
        assert!(build_adapters(&patch).err().expect("an error").to_string().contains("port"));
    }
}
