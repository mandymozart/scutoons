//! OSC as a tempo source (#181): a remote for the deck's own clock – /tempo/bpm f, /tempo/beat f (the beat heard
//! now), /tempo/start, /tempo/stop, /tempo/tap. Numbers may be int, float or double; bundles are unpacked (their time
//! tags ignored – a message counts when it arrives). A UDP thread stamps each message on arrival. The commands are the
//! steerable clock's (keys and taps use them too); the socket and the decoder (rosc) are the `osc` module's (#185).
#[cfg(feature = "osc")]
pub use io::{listen, parse, Listener};

pub use deck_core::tempo::Cmd;

#[cfg(feature = "osc")]
mod io {
    use super::super::{Event, What};
    use super::Cmd;
    use crate::clock::now_ns;
    use anyhow::{Context, Result};
    use rosc::{OscMessage, OscPacket, OscType};
    use std::net::{SocketAddr, UdpSocket};
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::mpsc::Sender;
    use std::sync::Arc;
    use std::time::Duration;

    /// the tempo commands in one datagram (anything else is ignored)
    pub fn parse(datagram: &[u8]) -> Vec<Cmd> {
        let mut out = Vec::new();
        if let Ok((_, p)) = rosc::decoder::decode_udp(datagram) {
            walk(&p, &mut out);
        }
        out
    }

    fn walk(p: &OscPacket, out: &mut Vec<Cmd>) {
        match p {
            OscPacket::Message(m) => out.extend(cmd(m)),
            OscPacket::Bundle(b) => b.content.iter().for_each(|p| walk(p, out)),
        }
    }

    fn cmd(m: &OscMessage) -> Option<Cmd> {
        let num = || match m.args.first()? {
            OscType::Float(f) => Some(*f as f64),
            OscType::Double(d) => Some(*d),
            OscType::Int(i) => Some(*i as f64),
            OscType::Long(l) => Some(*l as f64),
            _ => None,
        };
        Some(match m.addr.as_str() {
            "/tempo/bpm" => Cmd::Bpm(num()?),
            "/tempo/beat" => Cmd::Beat(num()?),
            "/tempo/start" => Cmd::Start,
            "/tempo/stop" => Cmd::Stop,
            "/tempo/tap" => Cmd::Tap,
            _ => return None,
        })
    }

    /// The listening thread; dropping it stops it (within its 200 ms read timeout).
    pub struct Listener {
        stop: Arc<AtomicBool>,
        pub addr: SocketAddr,
    }

    impl Drop for Listener {
        fn drop(&mut self) {
            self.stop.store(true, Ordering::Relaxed);
        }
    }

    pub fn listen(bind: &str, tx: Sender<Event>) -> Result<Listener> {
        let socket = UdpSocket::bind(bind).with_context(|| format!("OSC bind {bind}"))?;
        socket.set_read_timeout(Some(Duration::from_millis(200)))?;
        let addr = socket.local_addr()?;
        let stop = Arc::new(AtomicBool::new(false));
        let flag = stop.clone();
        std::thread::Builder::new().name("osc-tempo".into()).spawn(move || {
            let mut buf = [0u8; 1536];
            while !flag.load(Ordering::Relaxed) {
                let Ok((n, _)) = socket.recv_from(&mut buf) else { continue };
                let at_ns = now_ns();
                for c in parse(&buf[..n]) {
                    if tx.send(Event { at_ns, what: What::Osc(c) }).is_err() {
                        return;
                    }
                }
            }
        })?;
        Ok(Listener { stop, addr })
    }
}

#[cfg(all(test, feature = "osc"))]
mod tests {
    use super::*;
    use super::super::What;
    use crate::clock::now_ns;
    use rosc::{encoder, OscBundle, OscMessage, OscPacket, OscTime, OscType};
    use std::net::UdpSocket;
    use std::time::Duration;

    fn msg(addr: &str, args: Vec<OscType>) -> OscPacket {
        OscPacket::Message(OscMessage { addr: addr.into(), args })
    }

    #[test]
    fn tempo_messages_parse() {
        let one = |p: OscPacket| parse(&encoder::encode(&p).unwrap());
        assert_eq!(one(msg("/tempo/bpm", vec![OscType::Float(128.0)])), vec![Cmd::Bpm(128.0)]);
        assert_eq!(one(msg("/tempo/bpm", vec![OscType::Int(90)])), vec![Cmd::Bpm(90.0)]);
        assert_eq!(one(msg("/tempo/beat", vec![OscType::Double(16.0)])), vec![Cmd::Beat(16.0)]);
        assert_eq!(one(msg("/tempo/bpm", vec![])), vec![], "a bpm without a number is ignored");
        assert_eq!(one(msg("/other", vec![])), vec![]);
        let bundle = OscPacket::Bundle(OscBundle {
            timetag: OscTime { seconds: 0, fractional: 1 },
            content: vec![msg("/tempo/stop", vec![]), msg("/tempo/tap", vec![]), msg("/tempo/start", vec![])],
        });
        assert_eq!(one(bundle), vec![Cmd::Stop, Cmd::Tap, Cmd::Start]);
        assert!(parse(b"not osc").is_empty());
    }

    #[test]
    fn the_listener_stamps_datagrams() {
        let (tx, rx) = std::sync::mpsc::channel();
        let l = listen("127.0.0.1:0", tx).unwrap();
        let s = UdpSocket::bind("127.0.0.1:0").unwrap();
        let before = now_ns();
        s.send_to(&encoder::encode(&msg("/tempo/bpm", vec![OscType::Float(100.0)])).unwrap(), l.addr).unwrap();
        let e = rx.recv_timeout(Duration::from_secs(2)).unwrap();
        assert!(e.at_ns >= before);
        assert!(matches!(e.what, What::Osc(Cmd::Bpm(b)) if b == 100.0));
    }
}
