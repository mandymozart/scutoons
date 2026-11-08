//! #438: the RUDP door – `renet2` over netcode (the netcode.io protocol over UDP). Two channels: 0 reliable and ordered, 1
//! unreliable. renet2 is a tick-driven, synchronous library, so all its clients live in ONE task; the task wakes on the
//! socket, on an outbox, or after `TICK` (retransmits and acks need a clock) instead of spinning.
//!
//! Authentication: netcode's connect token, sealed with the key shared with whoever mints it (the API, as for session
//! tokens). With `ServerAuthentication::Secure` every packet is AEAD-encrypted, so `hello.token` is not in clear.

use std::collections::{HashMap, HashSet};
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use renet2::{ChannelConfig, ClientId, ConnectionConfig, RenetClient, RenetServer, SendType, ServerEvent};
use renet2_netcode::{
    BoxedSocket, ClientAuthentication, ClientSocket, ConnectToken, NetcodeClientTransport, NetcodeServerTransport, NetcodeTransportError, ServerAuthentication, ServerSetupConfig, ServerSocket,
};
use tokio::net::UdpSocket;
use tokio::sync::mpsc;

use super::{Doors, Member, DOOR_IDLE};
use crate::outbox::Lane;
use crate::{Join, PING_EVERY};

pub const RELIABLE: u8 = 0;
pub const UNRELIABLE: u8 = 1;
pub const PROTOCOL_ID: u64 = 0x5343_5554_4f4f_4e53; // "SCUTOONS"
/// the longest the task sleeps: the clock of retransmits and acks
pub const TICK: Duration = Duration::from_millis(2);

/// renet2's retransmit is one fixed timer (no RTT estimate): shorter than the path's RTT sends everything twice, longer
/// is the tail of a lost message. The caller picks it for the path (`--resend`).
pub fn connection_config(resend: Duration) -> ConnectionConfig {
    let channels = vec![
        ChannelConfig { channel_id: RELIABLE, max_memory_usage_bytes: crate::outbox::MAX_BYTES, send_type: SendType::ReliableOrdered { resend_time: resend } },
        ChannelConfig { channel_id: UNRELIABLE, max_memory_usage_bytes: 1 << 20, send_type: SendType::Unreliable { ordered_reliable_substrate: false } },
    ];
    ConnectionConfig::from_shared_channels(channels)
}

fn since_epoch() -> Duration {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default()
}

/// A tokio UDP socket as a netcode socket: readiness wakes the task, the send and receive never block.
#[derive(Debug)]
pub struct TokioUdp(pub Arc<UdpSocket>);

impl ServerSocket for TokioUdp {
    fn is_encrypted(&self) -> bool {
        false
    }
    fn is_reliable(&self) -> bool {
        false
    }
    fn addr(&self) -> std::io::Result<SocketAddr> {
        self.0.local_addr()
    }
    fn is_closed(&mut self) -> bool {
        false
    }
    fn close(&mut self) {}
    fn connection_denied(&mut self, _: SocketAddr) {}
    fn connection_accepted(&mut self, _: u64, _: SocketAddr) {}
    fn disconnect(&mut self, _: SocketAddr) {}
    fn preupdate(&mut self) {}
    fn try_recv(&mut self, buffer: &mut [u8]) -> std::io::Result<(usize, SocketAddr)> {
        self.0.try_recv_from(buffer)
    }
    fn postupdate(&mut self) {}
    fn send(&mut self, addr: SocketAddr, packet: &[u8]) -> Result<(), NetcodeTransportError> {
        send_or_drop(self.0.try_send_to(packet, addr))
    }
}

impl ClientSocket for TokioUdp {
    fn is_encrypted(&self) -> bool {
        false
    }
    fn is_reliable(&self) -> bool {
        false
    }
    fn addr(&self) -> std::io::Result<SocketAddr> {
        self.0.local_addr()
    }
    fn is_closed(&mut self) -> bool {
        false
    }
    fn close(&mut self) {}
    fn preupdate(&mut self) {}
    fn try_recv(&mut self, buffer: &mut [u8]) -> std::io::Result<(usize, SocketAddr)> {
        self.0.try_recv_from(buffer)
    }
    fn postupdate(&mut self) {}
    fn send(&mut self, addr: SocketAddr, packet: &[u8]) -> Result<(), NetcodeTransportError> {
        send_or_drop(self.0.try_send_to(packet, addr))
    }
}

/// a full socket buffer is a lost packet, which this transport is built for
fn send_or_drop(r: std::io::Result<usize>) -> Result<(), NetcodeTransportError> {
    match r {
        Ok(_) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => Ok(()),
        Err(e) => Err(e.into()),
    }
}

struct Peer {
    member: Option<Arc<Member>>,
    forward: Option<tokio::task::JoinHandle<()>>,
    until: Option<Instant>,
}

/// The door over `socket` plus any `extra` sockets (a WebTransport or WebSocket `ServerSocket`) – every client of one
/// netcode server, whatever its socket. Runs until the transport fails.
pub async fn netcode_door(doors: Arc<Doors>, socket: Arc<UdpSocket>, extra: Vec<(BoxedSocket, SocketAddr)>, key: [u8; 32], resend: Duration) -> anyhow::Result<()> {
    let mut sockets = vec![BoxedSocket::new(TokioUdp(socket.clone()))];
    let mut addresses = vec![vec![socket.local_addr()?]];
    for (s, addr) in extra {
        sockets.push(s);
        addresses.push(vec![addr]);
    }
    let setup = ServerSetupConfig {
        current_time: since_epoch(),
        max_token_lifetime: 3600,
        max_clients: crate::MAX_CLIENTS,
        protocol_id: PROTOCOL_ID,
        socket_addresses: addresses,
        authentication: ServerAuthentication::Secure { private_key: key },
    };
    let mut transport = NetcodeServerTransport::new_with_sockets(setup, sockets)?;
    let mut server = RenetServer::new(connection_config(resend));
    let (dirty_tx, mut dirty_rx) = mpsc::unbounded_channel::<ClientId>();
    let mut peers: HashMap<ClientId, Peer> = HashMap::new();
    let mut dirty: HashSet<ClientId> = HashSet::new();
    let mut batch: Vec<(Lane, String)> = Vec::with_capacity(64);
    let mut last = Instant::now();
    let mut next_rtt = Instant::now() + PING_EVERY;

    loop {
        tokio::select! {
            _ = socket.readable() => {}
            Some(id) = dirty_rx.recv() => { dirty.insert(id); }
            _ = tokio::time::sleep(TICK) => {}
        }
        while let Ok(id) = dirty_rx.try_recv() {
            dirty.insert(id);
        }
        let now = Instant::now();
        let dt = now - last;
        last = now;
        server.update(dt);
        transport.update(dt, &mut server).map_err(|e| anyhow::anyhow!("netcode: {e:?}"))?;

        while let Some(event) = server.get_event() {
            match event {
                ServerEvent::ClientConnected { client_id } => {
                    peers.insert(client_id, Peer { member: None, forward: None, until: None });
                }
                ServerEvent::ClientDisconnected { client_id, .. } => {
                    if let Some(p) = peers.remove(&client_id) {
                        if let Some(f) = p.forward {
                            f.abort();
                        }
                        if let Some(m) = p.member {
                            m.leave();
                        }
                    }
                }
            }
        }

        for (&id, peer) in peers.iter_mut() {
            while let Some(bytes) = server.receive_message(id, RELIABLE) {
                let Ok(text) = std::str::from_utf8(&bytes) else { continue };
                match &peer.member {
                    Some(m) => m.text(text),
                    None => match doors.admit(text, Join::default()) {
                        Ok(m) => {
                            let m = Arc::new(m);
                            let (outbox, tx) = (m.outbox.clone(), dirty_tx.clone());
                            peer.forward = Some(tokio::spawn(async move {
                                loop {
                                    outbox.pushed().await;
                                    if tx.send(id).is_err() {
                                        return;
                                    }
                                }
                            }));
                            peer.member = Some(m);
                            dirty.insert(id);
                        }
                        Err(refusal) => {
                            server.send_message(id, RELIABLE, refusal.into_bytes());
                            server.disconnect(id);
                        }
                    },
                }
            }
            while let Some(bytes) = server.receive_message(id, UNRELIABLE) {
                if let Some(m) = &peer.member {
                    m.datagram(&bytes);
                }
            }
        }

        let rtt_due = now >= next_rtt;
        if rtt_due {
            next_rtt = now + PING_EVERY;
        }
        for (&id, peer) in peers.iter_mut() {
            let Some(m) = peer.member.clone() else { continue };
            if rtt_due {
                m.rtt(Duration::from_secs_f64(server.rtt(id).max(0.0)));
            }
            if !dirty.contains(&id) && peer.until.map_or(true, |u| u > now) {
                continue;
            }
            let took = m.outbox.take_lanes(now, &mut batch);
            peer.until = took.until;
            let any = !batch.is_empty();
            for (lane, text) in batch.drain(..) {
                let channel = if lane == Lane::Reliable { RELIABLE } else { UNRELIABLE };
                if channel == RELIABLE && !server.can_send_message(id, RELIABLE, text.len()) {
                    server.disconnect(id); // too far behind: it reconnects and resyncs from its snapshot
                    break;
                }
                server.send_message(id, channel, text.into_bytes());
            }
            if any && m.outbox.sent(now, Duration::ZERO, took.waited) {
                m.tell_presence();
            }
        }
        dirty.clear();
        transport.send_packets(&mut server);
    }
}

/// A client's end (the bench, tests): `(lane, text)` in, texts out, until `outgoing` closes.
pub async fn client(server: SocketAddr, key: [u8; 32], client_id: u64, resend: Duration, mut outgoing: mpsc::UnboundedReceiver<(Lane, String)>, incoming: mpsc::UnboundedSender<String>) -> anyhow::Result<()> {
    let socket = Arc::new(UdpSocket::bind("127.0.0.1:0").await?);
    let now = since_epoch();
    let token = ConnectToken::generate(now, PROTOCOL_ID, 3600, client_id, DOOR_IDLE.as_secs() as i32, 0, vec![server], None, &key)?;
    let mut transport = NetcodeClientTransport::new(now, ClientAuthentication::Secure { connect_token: token }, TokioUdp(socket.clone()))?;
    let mut client = RenetClient::new(connection_config(resend), false);
    let mut waiting: Vec<(Lane, String)> = vec![];
    let mut told = false;
    let mut last = Instant::now();
    loop {
        tokio::select! {
            _ = socket.readable() => {}
            next = outgoing.recv() => match next {
                Some(m) => waiting.push(m),
                None => return Ok(()),
            },
            _ = tokio::time::sleep(TICK) => {}
        }
        while let Ok(m) = outgoing.try_recv() {
            waiting.push(m);
        }
        let now = Instant::now();
        let dt = now - last;
        last = now;
        client.update(dt);
        transport.update(dt, &mut client)?;
        if client.is_connected() {
            if !told {
                told = true;
                // the ladder waits for the handshake before it says hello; an empty text is no message
                incoming.send(String::new()).ok();
            }
            for (lane, text) in waiting.drain(..) {
                client.send_message(if lane == Lane::Reliable { RELIABLE } else { UNRELIABLE }, text.into_bytes());
            }
            for channel in [RELIABLE, UNRELIABLE] {
                while let Some(bytes) = client.receive_message(channel) {
                    if let Ok(text) = String::from_utf8(bytes.to_vec()) {
                        if incoming.send(text).is_err() {
                            return Ok(());
                        }
                    }
                }
            }
        }
        transport.send_packets(&mut client)?;
    }
}
