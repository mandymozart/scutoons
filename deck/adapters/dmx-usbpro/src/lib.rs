//! `adapters-dmx-usbpro` – DMX out over a USB widget that speaks the Enttec DMX USB Pro protocol (#187, owner
//! 2026-10-30): the Enttec DMX USB Pro and compatibles (DMXKing ultraDMX Micro / Pro / MAX). The widget keeps the
//! DMX line refreshed itself; the adapter only hands it a new universe when something changed (≤ 40 Hz) and a
//! keep-alive frame every second.
//!
//! One device = one widget = one universe. Its own thread owns the serial port (CYBERDECKS.md §4: serial timing
//! never async): it waits on its command channel until the next frame is due, never polls. A widget that is missing
//! or pulled out is opened again every 2 s; the universe survives and is sent at once on reconnect.
//!
//! Venue patch: `adapter = "dmx-usbpro"`, `port = "/dev/ttyUSB0"` | `"COM3"` optional (else the first widget found,
//! `detect`). Messages: `dmx { ch, values, fade? }` (protocol `DmxMessage`). `panic` = blackout at once.

pub mod detect;
pub mod mock;
pub mod universe;
pub mod widget;

use std::io::{self, Read, Write};
use std::sync::mpsc as std_mpsc;
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use adapters_core::{Adapter, Direction, Inbound, PortInfo};
use anyhow::{anyhow, Context};
use protocol::{DeviceMsg, DmxMessage};
use tokio::sync::mpsc;

use crate::universe::Universe;
use crate::widget::{Decoder, Params};

/// How long a missing widget waits before the next try.
pub const RETRY: Duration = Duration::from_secs(2);
/// How long the device check waits for the widget's answers.
pub const CHECK_TIMEOUT: Duration = Duration::from_millis(500);

/// A serial port as the adapter needs it (`serialport`'s ports, a mock in the tests).
pub trait Port: Read + Write + Send {}
impl<T: Read + Write + Send + ?Sized> Port for T {}

/// Opens the widget: its name and the port. Called again after the port went away.
pub type Opener = Box<dyn FnMut() -> anyhow::Result<(String, Box<dyn Port>)> + Send>;

/// What the device check found (both answers are optional: clones may not answer label 3 / 10).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct WidgetInfo {
    pub params: Option<Params>,
    pub serial: Option<String>,
}

enum Cmd {
    Set(DmxMessage),
    Now(DmxMessage),
    Panic,
    Stop,
}

pub struct DmxUsbProAdapter {
    outlet: String,
    port: Option<String>,
    opener: Option<Opener>,
    tx: Option<std_mpsc::Sender<Cmd>>,
    worker: Option<JoinHandle<()>>,
}

impl DmxUsbProAdapter {
    /// `port`: the serial port's name; `None` = detect.
    pub fn new(outlet: impl Into<String>, port: Option<&str>) -> Self {
        let port = port.map(str::to_owned);
        let opener = serial_opener(port.clone());
        Self::with_opener(outlet, port, opener)
    }

    /// With any opener (the tests' mock port).
    pub fn with_opener(outlet: impl Into<String>, port: Option<String>, opener: Opener) -> Self {
        DmxUsbProAdapter { outlet: outlet.into(), port, opener: Some(opener), tx: None, worker: None }
    }

    fn command(&self, cmd: Cmd) -> anyhow::Result<()> {
        let tx = self.tx.as_ref().ok_or_else(|| anyhow!("dmx-usbpro {:?} not started", self.outlet))?;
        tx.send(cmd).map_err(|_| anyhow!("dmx-usbpro {:?}: its thread is gone", self.outlet))
    }
}

/// The real serial port: the configured name, else the best widget `detect` finds.
fn serial_opener(configured: Option<String>) -> Opener {
    Box::new(move || {
        let name = match &configured {
            Some(n) => n.clone(),
            None => {
                let ports = detect::candidates();
                let found = detect::pick(&ports).ok_or_else(|| anyhow!("no USB-DMX widget found (serial ports: {})", ports.len()))?;
                tracing::info!("dmx-usbpro: found {}", found.describe());
                found.name.clone()
            }
        };
        // The widget's FTDI / CDC link ignores the baud rate; 115200 like OLA's usbpro plugin.
        let port = serialport::new(&name, 115_200).timeout(Duration::from_millis(50)).open().with_context(|| format!("open {name}"))?;
        Ok((name, Box::new(port) as Box<dyn Port>))
    })
}

/// Asks the widget for its parameters (label 3) and serial (label 10); waits up to `timeout` for the answers.
/// An error only when the port itself fails; no answer is `WidgetInfo::default()`.
pub fn check_widget(port: &mut dyn Port, timeout: Duration) -> io::Result<WidgetInfo> {
    port.write_all(&widget::params_request())?;
    port.write_all(&widget::serial_request())?;
    port.flush()?;
    let deadline = Instant::now() + timeout;
    let mut decoder = Decoder::default();
    let mut info = WidgetInfo::default();
    let mut buf = [0u8; 256];
    while Instant::now() < deadline && (info.params.is_none() || info.serial.is_none()) {
        let n = match port.read(&mut buf) {
            Ok(0) => continue,
            Ok(n) => n,
            Err(e) if matches!(e.kind(), io::ErrorKind::TimedOut | io::ErrorKind::WouldBlock | io::ErrorKind::Interrupted) => continue,
            Err(e) => return Err(e),
        };
        for (label, data) in decoder.push(&buf[..n]) {
            match label {
                widget::LABEL_GET_PARAMS => info.params = Params::parse(&data),
                widget::LABEL_GET_SERIAL => info.serial = widget::parse_serial(&data),
                _ => {}
            }
        }
    }
    Ok(info)
}

/// The adapter's thread: connect (again), apply commands, send due frames.
fn run(outlet: String, mut opener: Opener, rx: std_mpsc::Receiver<Cmd>) {
    let mut universe = Universe::default();
    let mut port: Option<Box<dyn Port>> = None;
    let mut next_try = Instant::now();
    let mut warned = false;
    loop {
        let now = Instant::now();
        if port.is_none() && now >= next_try {
            match opener() {
                Ok((name, mut p)) => match check_widget(p.as_mut(), CHECK_TIMEOUT) {
                    Ok(info) => {
                        match (&info.params, &info.serial) {
                            (None, None) => tracing::warn!(outlet, name, "dmx-usbpro: no answer to the device check – not a widget? sending anyway"),
                            (params, serial) => tracing::info!(
                                outlet, name, firmware = params.map(|p| format!("{}.{}", p.firmware >> 8, p.firmware & 0xFF)), serial,
                                rate = params.map(|p| p.rate), "dmx-usbpro: widget ready"
                            ),
                        }
                        let frame = universe.force(Instant::now());
                        port = write_frame(&outlet, p, &frame);
                        warned = false;
                    }
                    Err(e) => tracing::warn!(outlet, name, "dmx-usbpro: device check failed: {e}"),
                },
                Err(e) => {
                    if !warned {
                        tracing::warn!(outlet, "dmx-usbpro: {e:#} – retrying every {} s", RETRY.as_secs());
                        warned = true;
                    }
                }
            }
            if port.is_none() {
                next_try = Instant::now() + RETRY;
            }
        }

        let deadline = if port.is_some() { universe.next_due(Instant::now()) } else { next_try };
        match rx.recv_timeout(deadline.saturating_duration_since(Instant::now())) {
            Ok(Cmd::Set(msg)) => {
                if let Err(e) = universe.apply(&msg, Instant::now()) {
                    tracing::warn!(outlet, "dmx-usbpro: {e}");
                }
            }
            Ok(Cmd::Now(msg)) => {
                if let Err(e) = universe.apply_now(&msg, Instant::now()) {
                    tracing::warn!(outlet, "dmx-usbpro: {e}");
                }
            }
            Ok(Cmd::Panic) => {
                universe.blackout();
                let frame = universe.force(Instant::now());
                if let Some(p) = port.take() {
                    port = write_frame(&outlet, p, &frame);
                }
            }
            // Stop leaves the last look standing (the widget repeats it): a restarting bridge must not flash black.
            Ok(Cmd::Stop) | Err(std_mpsc::RecvTimeoutError::Disconnected) => return,
            Err(std_mpsc::RecvTimeoutError::Timeout) => {}
        }

        if let Some(p) = port.take() {
            port = match universe.frame(Instant::now()) {
                Some(frame) => write_frame(&outlet, p, &frame),
                None => Some(p),
            };
        }
        if port.is_none() && next_try < Instant::now() {
            // just lost: try again at once (a short USB hiccup), then every RETRY
            next_try = Instant::now();
        }
    }
}

/// Writes one frame; `None` when the port failed (the widget is gone – the caller reconnects).
fn write_frame(outlet: &str, mut port: Box<dyn Port>, frame: &[u8; widget::UNIVERSE]) -> Option<Box<dyn Port>> {
    match port.write_all(&widget::dmx_packet(frame)).and_then(|()| port.flush()) {
        Ok(()) => Some(port),
        Err(e) => {
            tracing::warn!(outlet, "dmx-usbpro: the widget went away ({e}) – reconnecting");
            None
        }
    }
}

impl Adapter for DmxUsbProAdapter {
    fn name(&self) -> &'static str {
        "dmx-usbpro"
    }

    /// Starts the thread; a missing widget is not an error here (it is looked for every 2 s).
    fn start(&mut self, _inbound: mpsc::Sender<Inbound>) -> anyhow::Result<()> {
        let opener = self.opener.take().ok_or_else(|| anyhow!("dmx-usbpro {:?} started twice", self.outlet))?;
        let (tx, rx) = std_mpsc::channel();
        let outlet = self.outlet.clone();
        let worker = std::thread::Builder::new().name(format!("dmx-usbpro {}", self.outlet)).spawn(move || run(outlet, opener, rx))?;
        self.tx = Some(tx);
        self.worker = Some(worker);
        Ok(())
    }

    fn send(&mut self, msg: &DeviceMsg) -> anyhow::Result<()> {
        let DeviceMsg::Dmx(m) = msg else { return Err(anyhow!("dmx-usbpro got a non-dmx message")) };
        universe::check(m)?;
        self.command(Cmd::Set(m.clone()))
    }

    /// A change in its own frame at once (≥ 4 ms after the last – a strobe's flash, #211), not on the 40 Hz grid.
    fn send_now(&mut self, msg: &DeviceMsg) -> anyhow::Result<()> {
        let DeviceMsg::Dmx(m) = msg else { return Err(anyhow!("dmx-usbpro got a non-dmx message")) };
        universe::check(m)?;
        self.command(Cmd::Now(m.clone()))
    }

    fn panic(&mut self) -> anyhow::Result<()> {
        self.command(Cmd::Panic)
    }

    fn stop(&mut self) -> anyhow::Result<()> {
        if let Some(tx) = self.tx.take() {
            let _ = tx.send(Cmd::Stop);
        }
        if let Some(worker) = self.worker.take() {
            worker.join().map_err(|_| anyhow!("dmx-usbpro {:?}: its thread panicked", self.outlet))?;
        }
        Ok(())
    }

    /// The widgets this machine sees (the configured port too, plugged in or not).
    fn ports(&self) -> Vec<PortInfo> {
        let found = detect::candidates();
        let mut names: Vec<String> = detect::rank(&found).into_iter().map(|c| c.name.clone()).collect();
        if let Some(p) = &self.port {
            if !names.contains(p) {
                names.push(p.clone());
            }
        }
        names.into_iter().map(|name| PortInfo { name, direction: Direction::Out }).collect()
    }
}

impl Drop for DmxUsbProAdapter {
    fn drop(&mut self) {
        let _ = self.stop();
    }
}

#[cfg(test)]
mod tests;
