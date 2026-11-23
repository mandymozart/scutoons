//! A deck's MIDI inputs as ports: which device is which port, and their bytes through one MIDI adapter each (midi.rs).
//! The platform only lists the devices it sees and hands over their bytes; the rules are here:
//! - a device is known by its label: the system's name without ALSA's changing `client:port` suffix; ALSA's loopback
//!   (`Midi Through`) is no input; two devices of the same name are `name`, `name #2` in the system's order
//! - the devices present at the first listing are numbered 1 … in their labels' order; one plugged in later takes the next
//!   number never given; one that comes back takes its own number again (a number is never reused for another device)
//! - a device that goes away ends its held notes on its port, nothing else
//!
//! Live events carry no source (port and channel are props): nothing on them for the audio thread to free.
use super::midi::Midi;
use crate::event::Event;
use crate::names::Names;
use crate::Time;

/// what a listing changed: a device to open (its index in the listing) or one to close
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Change {
    Opened { port: u8, label: String, index: usize },
    Closed { port: u8, label: String },
}

struct Port {
    label: String,
    present: bool,
    midi: Midi,
}

pub struct Inputs {
    names: Names,
    /// port n = ports[n - 1]
    ports: Vec<Port>,
    listed: bool,
}

/// a device's label from its system name; None = not an input to play (ALSA's loopback)
pub fn label(raw: &str) -> Option<String> {
    let name = raw.trim();
    if name.is_empty() || name.to_lowercase().contains("midi through") {
        return None;
    }
    let alsa_suffix = |tail: &str| tail.split_once(':').is_some_and(|(a, b)| !a.is_empty() && !b.is_empty() && a.bytes().chain(b.bytes()).all(|c| c.is_ascii_digit()));
    let name = match name.rsplit_once(' ') {
        Some((head, tail)) if alsa_suffix(tail) => head.trim_end(),
        _ => name,
    };
    Some(name.to_string())
}

/// the labels of a listing, a repeated one numbered `#2`, `#3` … in the listing's order
pub fn labels(listing: &[String]) -> Vec<Option<String>> {
    let mut seen: Vec<String> = Vec::new();
    listing
        .iter()
        .map(|raw| {
            let l = label(raw)?;
            let n = seen.iter().filter(|s| **s == l).count();
            seen.push(l.clone());
            Some(if n == 0 { l } else { format!("{l} #{}", n + 1) })
        })
        .collect()
}

impl Inputs {
    /// `names`: the engine's dictionary (its event and key names are the ones the routes know)
    pub fn new(names: Names) -> Inputs {
        Inputs { names, ports: Vec::new(), listed: false }
    }

    /// The devices present now, as the system names them (any order). What changed comes back; a device gone ends its
    /// held notes at `at` into `out`.
    pub fn sync(&mut self, listing: &[String], at: Time, out: &mut Vec<Event>) -> Vec<Change> {
        let labels = labels(listing);
        let mut changes = Vec::new();
        for (k, p) in self.ports.iter_mut().enumerate() {
            if p.present && !labels.iter().flatten().any(|l| *l == p.label) {
                p.present = false;
                p.midi.release_all_at(at, out);
                changes.push(Change::Closed { port: k as u8 + 1, label: p.label.clone() });
            }
        }
        let mut order: Vec<usize> = (0..listing.len()).filter(|&i| labels[i].is_some()).collect();
        if !self.listed {
            order.sort_by(|&a, &b| labels[a].cmp(&labels[b]));
            self.listed = true;
        }
        for index in order {
            let l = labels[index].clone().expect("filtered above");
            let port = match self.ports.iter().position(|p| p.label == l) {
                Some(k) if self.ports[k].present => continue,
                Some(k) => k,
                None if self.ports.len() < usize::from(u8::MAX) => {
                    let n = self.ports.len() as u8 + 1;
                    self.ports.push(Port { label: l.clone(), present: false, midi: Midi::new(&mut self.names, None, n) });
                    self.ports.len() - 1
                }
                None => continue,
            };
            self.ports[port].present = true;
            changes.push(Change::Opened { port: port as u8 + 1, label: l, index });
        }
        changes
    }

    /// One message from `port` (1 …) at `at`; its events go to `out`. False: not a note or controller, or no such device.
    pub fn feed(&mut self, port: u8, bytes: &[u8], at: Time, out: &mut Vec<Event>) -> bool {
        match self.ports.get_mut(usize::from(port).wrapping_sub(1)) {
            Some(p) if p.present => p.midi.feed_at(bytes, at, out),
            _ => false,
        }
    }

    /// the devices present: (port, label)
    pub fn present(&self) -> impl Iterator<Item = (u8, &str)> {
        self.ports.iter().enumerate().filter(|(_, p)| p.present).map(|(k, p)| (k as u8 + 1, p.label.as_str()))
    }
}
