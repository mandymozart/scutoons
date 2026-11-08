//! The device bridge (agents/research/LIVE.md B.7, Rust per CYBERDECKS.md §5): a venue patch (B.5.4) in TOML names
//! the relay and the devices; each device becomes an adapter (`adapters-core::Adapter`); the bridge joins the relay
//! as `bridge`, reports its ports (`devices`), plays `device` messages at their `at`, and forwards what comes in as
//! `device.in` to the conductor.
//!
//! Timing (B.6 clock): the offset to the relay is the median of 5 ping samples at join; `device` messages wait in a
//! queue until their `at` (a late one plays at once), and `panic` empties the queue and silences every adapter.
//! Not yet: the status window, `dmx-artnet` / `dmx-sacn`.

use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashMap};
use std::time::Duration;

use adapters_core::{Adapter, DeviceConfig, Inbound};
use adapters_dmx_usbpro::DmxUsbProAdapter;
use adapters_midi::MidiAdapter;
use adapters_osc::OscAdapter;
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
/// (CYBERDECKS.md §4: dmx-artnet, dmx-sacn, gpio, audio, video-in, display, laser #178).
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
                "osc" => {
                    let host = d.options.get("host").and_then(|v| v.as_str());
                    let listen = d.options.get("listen").and_then(|v| v.as_str());
                    Box::new(OscAdapter::new(&d.outlet, host, listen).with_context(|| d.outlet.clone())?)
                }
                "dmx-usbpro" => {
                    // optional: without `port` the first widget found (USB ids / names, adapters-dmx-usbpro detect)
                    let port = d.options.get("port").and_then(|v| v.as_str());
                    Box::new(DmxUsbProAdapter::new(&d.outlet, port))
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
    sink.send(Ws::Text(serde_json::to_string(&Envelope::new("", Message::Hello(hello)))?.into())).await?;

    let welcome: Envelope = match source.next().await {
        Some(Ok(Ws::Text(t))) => serde_json::from_str(&t)?,
        other => return Err(anyhow!("no welcome: {other:?}")),
    };
    let Message::Welcome(w) = welcome.message else { return Err(anyhow!("relay refused: {:?}", welcome.message)) };
    // B.6 clock: 5 samples, offset = median of t1 − (t0 + rtt / 2); stored as local − relay
    let mut samples = vec![];
    for _ in 0..SAMPLES {
        let t0 = now_ms();
        sink.send(Ws::Text(serde_json::to_string(&Envelope::new("", Message::Ping(protocol::Ping { t0 })))?.into())).await?;
        loop {
            let Some(Ok(Ws::Text(t))) = source.next().await else { return Err(anyhow!("relay closed during the clock")) };
            if let Ok(Envelope { message: Message::Pong(p), .. }) = serde_json::from_str::<Envelope>(&t) {
                if p.t0 == t0 {
                    samples.push((p.t0, p.t1, now_ms()));
                    break;
                }
            }
        }
    }
    let offset_ms = clock_offset(&samples).unwrap_or_else(|| now_ms() - w.now);
    tracing::info!(client = w.client, offset_ms, "joined the relay");

    sink.send(Ws::Text(serde_json::to_string(&Envelope::new("", Message::Devices(Devices { ports: ports(adapters) })))?.into())).await?;

    let mut by_outlet: HashMap<&str, &mut Box<dyn Adapter>> = adapters.iter_mut().map(|(o, a)| (o.as_str(), a)).collect();
    // device messages waiting for their time (local ms), earliest first; `n` keeps the arrival order on equal times
    let mut queue: BinaryHeap<Reverse<(i64, u64, Timed)>> = BinaryHeap::new();
    let mut n = 0u64;
    let mut keepalive = tokio::time::interval(Duration::from_secs(20));
    keepalive.tick().await;
    loop {
        let next = queue.peek().map(|Reverse((t, _, _))| *t);
        let due = async {
            match next {
                Some(t) => tokio::time::sleep(Duration::from_millis((t - now_ms()).max(0) as u64)).await,
                None => std::future::pending().await,
            }
        };
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
                        // play at `at` on the bridge's own clock (B.7: never a browser tab); without `at` at once
                        let local = d.at.or(envelope.at).map_or(0, |at| at + offset_ms);
                        n += 1;
                        queue.push(Reverse((local, n, Timed(d))));
                    }
                    Message::Panic(_) => {
                        queue.clear();
                        for (outlet, adapter) in by_outlet.iter_mut() {
                            if let Err(e) = adapter.panic() { tracing::warn!(outlet, "panic failed: {e}"); }
                        }
                        tracing::info!("panic: queue cleared, devices silenced");
                    }
                    _ => {}
                }
            }
            _ = due => {
                while queue.peek().is_some_and(|Reverse((t, _, _))| *t <= now_ms()) {
                    let Reverse((_, _, Timed(d))) = queue.pop().expect("peeked");
                    match by_outlet.get_mut(d.outlet.as_str()) {
                        Some(adapter) => for msg in &d.msgs {
                            if let Err(e) = adapter.send(msg) { tracing::warn!(outlet = d.outlet, "send failed: {e}"); }
                        },
                        None => tracing::warn!(outlet = d.outlet, "device for an unpatched outlet"),
                    }
                }
            }
            Some(inbound) = inbound.recv() => {
                let m = Message::DeviceIn(DeviceIn { outlet: inbound.outlet, msg: inbound.msg });
                sink.send(Ws::Text(serde_json::to_string(&Envelope::new("", m).to(To::role(Role::Conductor)))?.into())).await?;
            }
            _ = keepalive.tick() => {
                let ping = Message::Ping(protocol::Ping { t0: now_ms() });
                sink.send(Ws::Text(serde_json::to_string(&Envelope::new("", ping))?.into())).await?;
            }
        }
    }
}

/// B.6: 5 clock samples at join.
pub const SAMPLES: usize = 5;

/// The clock offset (local − relay, ms) from ping samples `(t0, t1, t2)`: t0 sent and t2 received on the local clock,
/// t1 the relay's time – the median of `t1 − (t0 + rtt / 2)`, negated. None without samples.
pub fn clock_offset(samples: &[(i64, i64, i64)]) -> Option<i64> {
    let mut d: Vec<i64> = samples.iter().map(|&(t0, t1, t2)| t1 - (t0 + (t2 - t0) / 2)).collect();
    if d.is_empty() {
        return None;
    }
    d.sort_unstable();
    Some(-d[d.len() / 2])
}

/// A `device` message in the queue, ordered only by its time (the heap's key ahead of it).
struct Timed(protocol::Device);

impl PartialEq for Timed {
    fn eq(&self, _: &Self) -> bool {
        true
    }
}
impl Eq for Timed {}
impl PartialOrd for Timed {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for Timed {
    fn cmp(&self, _: &Self) -> std::cmp::Ordering {
        std::cmp::Ordering::Equal
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
        assert_eq!(patch.devices.len(), 4);
        let adapters = build_adapters(&patch).unwrap();
        assert_eq!(adapters[0].0, "ableton");
        assert_eq!(adapters[0].1.name(), "midi");
        assert_eq!(adapters[2].0, "resolume");
        assert_eq!(adapters[2].1.name(), "osc");
        assert_eq!(adapters[2].1.ports().len(), 2, "host out + listen in");
        assert_eq!(adapters[3].0, "lights");
        assert_eq!(adapters[3].1.name(), "dmx-usbpro");
    }

    #[test]
    fn unknown_adapter_is_named() {
        let patch = Patch::parse("relay = \"ws://x/live/Q\"\n[[device]]\noutlet = \"lights\"\nadapter = \"dmx-artnet\"\n").unwrap();
        let err = build_adapters(&patch).err().expect("an error").to_string();
        assert!(err.contains("dmx-artnet") && err.contains("lights"), "{err}");
    }

    #[test]
    fn osc_needs_host_or_listen() {
        let patch = Patch::parse("relay = \"ws://x/live/Q\"
[[device]]
outlet = \"td\"
adapter = \"osc\"
").unwrap();
        let err = format!("{:#}", build_adapters(&patch).err().expect("an error"));
        assert!(err.contains("td") && err.contains("host"), "{err}");
    }

    #[test]
    fn midi_needs_a_port() {
        let patch = Patch::parse("relay = \"ws://x/live/Q\"\n[[device]]\noutlet = \"a\"\nadapter = \"midi\"\n").unwrap();
        assert!(build_adapters(&patch).err().expect("an error").to_string().contains("port"));
    }

    #[test]
    fn clock_offset_is_the_median() {
        // the relay is 1000 ms ahead; one sample has a slow return trip (the outlier the median drops)
        let samples = [(0, 1_005, 10), (100, 1_104, 108), (200, 1_210, 220), (300, 1_302, 304), (400, 1_700, 1_400)];
        assert_eq!(clock_offset(&samples), Some(-1_000));
        assert_eq!(clock_offset(&[]), None);
    }
}
