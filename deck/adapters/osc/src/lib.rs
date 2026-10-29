//! `adapters-osc` – OSC in / out over UDP (`rosc`), agents/research/CYBERDECKS.md §4: Ableton, TouchDesigner,
//! Resolume, laser software (#178) speak it. One device = one target (`host`) and / or one listening port (`listen`).
//!
//! Dogma (CYBERDECKS.md §2): the input is one tokio task awaiting `recv_from`; sending is a non-blocking `send_to`.
//! Bundles are flattened; their timetags are ignored for now (the bridge times by the envelope's `at`).

use std::net::{SocketAddr, ToSocketAddrs};
use std::sync::Arc;

use adapters_core::{Adapter, Direction, Inbound, PortInfo};
use anyhow::{anyhow, Context};
use protocol::{DeviceMsg, OscArg, OscMessage};
use rosc::{OscPacket, OscType};
use tokio::net::UdpSocket;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

/// The largest UDP datagram we accept (OSC over UDP has no length header; 64 KiB is the datagram limit).
const MAX_DATAGRAM: usize = 65_507;

/// `OscMessage` ↔ `rosc` – the pure part (tested without sockets).
pub mod codec {
    use super::*;

    pub fn to_rosc(msg: &OscMessage) -> rosc::OscMessage {
        rosc::OscMessage {
            addr: msg.address.clone(),
            args: msg
                .args
                .iter()
                .map(|a| match a {
                    OscArg::Int(i) => OscType::Int(*i as i32),
                    OscArg::Float(f) => OscType::Float(*f as f32),
                    OscArg::String(s) => OscType::String(s.clone()),
                    OscArg::Bool(b) => OscType::Bool(*b),
                })
                .collect(),
        }
    }

    /// `None` for an argument type the protocol does not carry yet (blob, timetag, colour, midi, nil, …).
    fn from_type(t: &OscType) -> Option<OscArg> {
        Some(match t {
            OscType::Int(i) => OscArg::Int(*i as i64),
            OscType::Long(i) => OscArg::Int(*i),
            OscType::Float(f) => OscArg::Float(*f as f64),
            OscType::Double(f) => OscArg::Float(*f),
            OscType::String(s) => OscArg::String(s.clone()),
            OscType::Char(c) => OscArg::String(c.to_string()),
            OscType::Bool(b) => OscArg::Bool(*b),
            _ => return None,
        })
    }

    pub fn from_rosc(msg: &rosc::OscMessage) -> OscMessage {
        OscMessage { address: msg.addr.clone(), args: msg.args.iter().filter_map(from_type).collect() }
    }

    /// Encode one message as a UDP datagram.
    pub fn encode(msg: &OscMessage) -> anyhow::Result<Vec<u8>> {
        rosc::encoder::encode(&OscPacket::Message(to_rosc(msg))).map_err(|e| anyhow!("OSC encode: {e:?}"))
    }

    /// Decode a datagram; a bundle (nested or not) becomes its messages in order.
    pub fn decode(bytes: &[u8]) -> anyhow::Result<Vec<OscMessage>> {
        let (_, packet) = rosc::decoder::decode_udp(bytes).map_err(|e| anyhow!("OSC decode: {e:?}"))?;
        let mut out = Vec::new();
        flatten(packet, &mut out);
        Ok(out)
    }

    fn flatten(packet: OscPacket, out: &mut Vec<OscMessage>) {
        match packet {
            OscPacket::Message(m) => out.push(from_rosc(&m)),
            OscPacket::Bundle(b) => b.content.into_iter().for_each(|p| flatten(p, out)),
        }
    }
}

/// One OSC device of the venue patch.
pub struct OscAdapter {
    outlet: String,
    /// Where `send` goes (`host = "127.0.0.1:7000"`); `None` = input only.
    target: Option<SocketAddr>,
    /// Where we listen (`listen = "0.0.0.0:9000"`); `None` = output only.
    listen: Option<SocketAddr>,
    socket: Option<Arc<UdpSocket>>,
    reader: Option<JoinHandle<()>>,
}

impl OscAdapter {
    pub fn new(outlet: impl Into<String>, host: Option<&str>, listen: Option<&str>) -> anyhow::Result<Self> {
        let resolve = |what: &str, s: &str| -> anyhow::Result<SocketAddr> {
            s.to_socket_addrs().with_context(|| format!("osc {what} {s:?}"))?.next().ok_or_else(|| anyhow!("osc {what} {s:?} resolves to nothing"))
        };
        let target = host.map(|h| resolve("host", h)).transpose()?;
        let listen = listen.map(|l| resolve("listen", l)).transpose()?;
        if target.is_none() && listen.is_none() {
            return Err(anyhow!("osc needs `host` (out) and / or `listen` (in)"));
        }
        Ok(OscAdapter { outlet: outlet.into(), target, listen, socket: None, reader: None })
    }

    /// The address the socket is bound to (after `start`; port 0 in `listen` picks a free one).
    pub fn local_addr(&self) -> Option<SocketAddr> {
        self.socket.as_ref().and_then(|s| s.local_addr().ok())
    }
}

impl Adapter for OscAdapter {
    fn name(&self) -> &'static str {
        "osc"
    }

    /// Needs a tokio runtime (the bridge's): binds the socket and, with `listen`, spawns the reader task.
    fn start(&mut self, inbound: mpsc::Sender<Inbound>) -> anyhow::Result<()> {
        let bind = self.listen.unwrap_or_else(|| match self.target {
            Some(SocketAddr::V6(_)) => "[::]:0".parse().unwrap(),
            _ => "0.0.0.0:0".parse().unwrap(),
        });
        let std_socket = std::net::UdpSocket::bind(bind).with_context(|| format!("osc bind {bind}"))?;
        std_socket.set_nonblocking(true)?;
        let socket = Arc::new(UdpSocket::from_std(std_socket).context("osc socket into tokio")?);
        if self.listen.is_some() {
            let outlet = self.outlet.clone();
            let socket = socket.clone();
            self.reader = Some(tokio::spawn(async move {
                let mut buf = vec![0u8; MAX_DATAGRAM];
                loop {
                    let Ok((n, _peer)) = socket.recv_from(&mut buf).await else { break };
                    match codec::decode(&buf[..n]) {
                        Ok(msgs) => {
                            for msg in msgs {
                                if inbound.try_send(Inbound { outlet: outlet.clone(), msg: DeviceMsg::Osc(msg) }).is_err() {
                                    tracing::warn!(outlet, "OSC in dropped: bridge channel full");
                                }
                            }
                        }
                        Err(e) => tracing::debug!(outlet, "OSC in ignored: {e}"),
                    }
                }
            }));
        }
        self.socket = Some(socket);
        Ok(())
    }

    fn send(&mut self, msg: &DeviceMsg) -> anyhow::Result<()> {
        let DeviceMsg::Osc(m) = msg else { return Err(anyhow!("osc adapter got a non-osc message")) };
        let target = self.target.ok_or_else(|| anyhow!("osc device {:?} has no `host`", self.outlet))?;
        let socket = self.socket.as_ref().ok_or_else(|| anyhow!("osc not started"))?;
        let bytes = codec::encode(m)?;
        // `try_send_to` reports WouldBlock until tokio has registered the socket's readiness once; a UDP send never
        // really waits, so hand it to the runtime and log a failure instead of blocking the bridge loop.
        let socket = socket.clone();
        tokio::spawn(async move {
            if let Err(e) = socket.send_to(&bytes, target).await {
                tracing::warn!(%target, "OSC send failed: {e}");
            }
        });
        Ok(())
    }

    fn stop(&mut self) -> anyhow::Result<()> {
        if let Some(reader) = self.reader.take() {
            reader.abort();
        }
        self.socket = None;
        Ok(())
    }

    /// OSC has no port list to discover: the patch's own endpoints are what we "see".
    fn ports(&self) -> Vec<PortInfo> {
        let mut v = Vec::new();
        if let Some(t) = self.target {
            v.push(PortInfo { name: t.to_string(), direction: Direction::Out });
        }
        if let Some(l) = self.listen {
            v.push(PortInfo { name: l.to_string(), direction: Direction::In });
        }
        v
    }
}

#[cfg(test)]
mod tests {
    use super::codec::*;
    use super::*;
    use std::time::Duration;

    fn sample() -> OscMessage {
        OscMessage {
            address: "/composition/layers/1/opacity".into(),
            args: vec![OscArg::Float(0.75), OscArg::Int(3), OscArg::String("go".into()), OscArg::Bool(true)],
        }
    }

    #[test]
    fn encode_decode_round_trip() {
        let bytes = encode(&sample()).unwrap();
        assert_eq!(bytes.len() % 4, 0, "OSC packets are 4-byte aligned");
        assert!(bytes.starts_with(b"/composition/layers/1/opacity\0"));
        let back = decode(&bytes).unwrap();
        assert_eq!(back, vec![sample()]);
    }

    #[test]
    fn bundles_flatten_in_order() {
        let a = rosc::OscMessage { addr: "/a".into(), args: vec![OscType::Int(1)] };
        let b = rosc::OscMessage { addr: "/b".into(), args: vec![OscType::Double(2.5), OscType::Blob(vec![1, 2])] };
        let inner = OscPacket::Bundle(rosc::OscBundle { timetag: rosc::OscTime { seconds: 0, fractional: 1 }, content: vec![OscPacket::Message(b)] });
        let bundle = OscPacket::Bundle(rosc::OscBundle { timetag: rosc::OscTime { seconds: 0, fractional: 1 }, content: vec![OscPacket::Message(a), inner] });
        let bytes = rosc::encoder::encode(&bundle).unwrap();
        let msgs = decode(&bytes).unwrap();
        assert_eq!(msgs.len(), 2);
        assert_eq!(msgs[0].address, "/a");
        assert_eq!(msgs[0].args, vec![OscArg::Int(1)]);
        // the blob is dropped, the double narrows to a float arg
        assert_eq!(msgs[1].args, vec![OscArg::Float(2.5)]);
    }

    #[test]
    fn garbage_is_an_error() {
        assert!(decode(b"not osc").is_err());
    }

    #[test]
    fn needs_host_or_listen() {
        assert!(OscAdapter::new("x", None, None).is_err());
        assert!(OscAdapter::new("x", Some("127.0.0.1:7000"), None).is_ok());
        assert!(OscAdapter::new("x", Some("no such host at all:7000"), None).is_err());
    }

    /// in: a datagram to the listening port arrives as `Inbound`; out: `send` reaches a plain UDP socket.
    #[tokio::test]
    async fn udp_in_and_out() {
        let peer = tokio::net::UdpSocket::bind("127.0.0.1:0").await.unwrap();
        let peer_addr = peer.local_addr().unwrap();
        let mut adapter = OscAdapter::new("resolume", Some(&peer_addr.to_string()), Some("127.0.0.1:0")).unwrap();
        let (tx, mut rx) = mpsc::channel(8);
        adapter.start(tx).unwrap();
        let listen = adapter.local_addr().unwrap();
        assert_ne!(listen.port(), 0);
        assert_eq!(adapter.ports().len(), 2);

        // out
        adapter.send(&DeviceMsg::Osc(sample())).unwrap();
        let mut buf = [0u8; 1024];
        let (n, from) = tokio::time::timeout(Duration::from_secs(2), peer.recv_from(&mut buf)).await.unwrap().unwrap();
        assert_eq!(from, listen);
        assert_eq!(decode(&buf[..n]).unwrap(), vec![sample()]);

        // in
        let pedal = OscMessage { address: "/scutoons/next".into(), args: vec![] };
        peer.send_to(&encode(&pedal).unwrap(), listen).await.unwrap();
        let inbound = tokio::time::timeout(Duration::from_secs(2), rx.recv()).await.unwrap().unwrap();
        assert_eq!(inbound.outlet, "resolume");
        assert_eq!(inbound.msg, DeviceMsg::Osc(pedal));

        adapter.stop().unwrap();
        assert!(adapter.send(&DeviceMsg::Osc(sample())).is_err(), "stopped");
    }
}
