//! `adapters-core` – one trait for every device adapter (agents/research/CYBERDECKS.md §4: midi, dmx-*, osc, gpio,
//! audio …). The bridge (`deck/bridge`) loads adapters from the venue patch (LIVE.md B.5.4), hands each one the
//! `device` messages addressed to its outlet and forwards what comes back in as `device.in`.
//!
//! Dogma (CYBERDECKS.md §2): no busy loops – an adapter waits on its hardware callback or a tokio timer, never polls.

use std::fmt;

use protocol::DeviceMsg;
use serde::Deserialize;
use tokio::sync::mpsc;

/// Something a device sent in (a pedal, a DAW's clock, an OSC controller). The bridge wraps it in `device.in`.
#[derive(Debug, Clone, PartialEq)]
pub struct Inbound {
    pub outlet: String,
    pub msg: DeviceMsg,
}

/// One logical device of the venue patch (LIVE.md B.5.4), the adapter-independent part. Each adapter reads its own
/// options (`port`, `host`, `universe` …) from `options`.
#[derive(Debug, Clone, Deserialize)]
pub struct DeviceConfig {
    /// The outlet name the show uses (`device { outlet }` is routed by it).
    pub outlet: String,
    /// Which adapter: `midi` · `osc` · `dmx-artnet` · …
    pub adapter: String,
    #[serde(flatten)]
    pub options: toml::Table,
}

/// A port an adapter can see (for the patch UI, `devices { ports }`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PortInfo {
    pub name: String,
    pub direction: Direction,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    In,
    Out,
}

impl fmt::Display for Direction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Direction::In => "in",
            Direction::Out => "out",
        })
    }
}

/// The adapter contract. Sync by design: adapters own their threads (a MIDI callback, the Open DMX bit-banger) or
/// spawn tokio tasks in `start`; the bridge never blocks on them.
pub trait Adapter: Send {
    /// The adapter's name as the venue patch spells it (`midi`, `osc` …).
    fn name(&self) -> &'static str;

    /// Open the device; from now on inbound messages go to `inbound` (tagged with the device's outlet).
    fn start(&mut self, inbound: mpsc::Sender<Inbound>) -> anyhow::Result<()>;

    /// Play one message now. Timing (`at`) is the bridge's job: it holds the message until `at − offset`.
    fn send(&mut self, msg: &DeviceMsg) -> anyhow::Result<()>;

    /// Close the device. Called once; dropping without `stop` must not panic.
    fn stop(&mut self) -> anyhow::Result<()>;

    /// The ports this adapter kind can see on this machine (not only the one it opened).
    fn ports(&self) -> Vec<PortInfo> {
        Vec::new()
    }
}

/// The reader's bus events the default maps translate to device messages (LIVE.md B.1, B.3). Only what the MIDI
/// default map needs for now; grows with the conductor.
#[derive(Debug, Clone, PartialEq)]
pub enum BusEvent {
    SliceEnter { index: u32 },
    ShowTextBox { index: u32 },
    Sample { index: u32 },
    Fx { index: u32 },
    Choice { option: u32 },
    Pickup { index: u32 },
    Variable { index: u32, value: f64 },
}
