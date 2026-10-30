//! A widget in memory, for the tests (this crate's and the bridge's): it records every frame it gets, answers the
//! device check like an Enttec firmware 1.44 (serial 00123456), and can be "unplugged" (writes and opens fail).

use std::collections::VecDeque;
use std::io::{self, Read, Write};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crate::widget::{self, Decoder, UNIVERSE};
use crate::Opener;

#[derive(Default)]
pub struct State {
    /// Frames received (label 6), with the time they arrived.
    pub frames: Vec<(Instant, [u8; UNIVERSE])>,
    /// Labels of every message received, in order.
    pub labels: Vec<u8>,
    /// Writes and opens fail.
    pub unplugged: bool,
    /// Answer labels 3 and 10 (a clone may not).
    pub silent: bool,
    /// How often the port was opened.
    pub opens: usize,
    decoder: Decoder,
    reply: VecDeque<u8>,
}

#[derive(Clone, Default)]
pub struct Widget(pub Arc<Mutex<State>>);

impl Widget {
    pub fn opener(&self) -> Opener {
        let w = self.clone();
        Box::new(move || {
            let mut s = w.0.lock().unwrap();
            if s.unplugged {
                anyhow::bail!("mock: no such port");
            }
            s.opens += 1;
            Ok(("mock".to_owned(), Box::new(Port(w.clone())) as Box<dyn crate::Port>))
        })
    }

    pub fn set_unplugged(&self, unplugged: bool) {
        self.0.lock().unwrap().unplugged = unplugged;
    }

    pub fn frames(&self) -> Vec<(Instant, [u8; UNIVERSE])> {
        self.0.lock().unwrap().frames.clone()
    }

    pub fn last_frame(&self) -> Option<[u8; UNIVERSE]> {
        self.0.lock().unwrap().frames.last().map(|(_, f)| *f)
    }

    /// Waits (polling, tests only) until the last frame satisfies `ok`.
    pub fn wait_frame(&self, timeout: Duration, ok: impl Fn(&[u8; UNIVERSE]) -> bool) -> Option<Instant> {
        let end = Instant::now() + timeout;
        while Instant::now() < end {
            if let Some((t, f)) = self.0.lock().unwrap().frames.last() {
                if ok(f) {
                    return Some(*t);
                }
            }
            std::thread::sleep(Duration::from_millis(2));
        }
        None
    }
}

struct Port(Widget);

impl Write for Port {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let mut s = self.0 .0.lock().unwrap();
        if s.unplugged {
            return Err(io::Error::new(io::ErrorKind::BrokenPipe, "mock: device disconnected"));
        }
        for (label, data) in s.decoder.push(buf) {
            s.labels.push(label);
            match label {
                widget::LABEL_SEND_DMX => {
                    assert_eq!(data.len(), UNIVERSE + 1, "start code + 512");
                    assert_eq!(data[0], 0, "start code 0");
                    let frame: [u8; UNIVERSE] = data[1..].try_into().unwrap();
                    s.frames.push((Instant::now(), frame));
                }
                widget::LABEL_GET_PARAMS if !s.silent => s.reply.extend(widget::frame(3, &[0x44, 0x01, 9, 1, 40])),
                widget::LABEL_GET_SERIAL if !s.silent => s.reply.extend(widget::frame(10, &[0x56, 0x34, 0x12, 0x00])),
                _ => {}
            }
        }
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl Read for Port {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let mut s = self.0 .0.lock().unwrap();
        if s.reply.is_empty() {
            drop(s);
            std::thread::sleep(Duration::from_millis(5));
            return Err(io::Error::new(io::ErrorKind::TimedOut, "mock: nothing to read"));
        }
        let n = buf.len().min(s.reply.len());
        for (i, b) in s.reply.drain(..n).enumerate() {
            buf[i] = b;
        }
        Ok(n)
    }
}
