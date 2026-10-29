//! `adapters-midi` – MIDI in / out through `midir` (ALSA on Linux decks, CoreMIDI, WinMM) and the **default map**
//! of agents/research/LIVE.md B.3: no authoring needed, a DAW's MIDI learn picks it up straight away.
//!
//! The clock (MIDI clock / start / stop) and the per-cue messages of the show come with #172's Open list.

use adapters_core::{Adapter, Direction, Inbound, PortInfo};
use anyhow::{anyhow, Context};
use midir::{MidiInput, MidiInputConnection, MidiOutput, MidiOutputConnection};
use protocol::{DeviceMsg, MidiBytes};
use tokio::sync::mpsc;

pub mod default_map {
    //! LIVE.md B.3: slice enter = note on ch 16 (note = slice index mod 128); each event type its own channel –
    //! showTextBox ch 1, sample ch 2, fx ch 3, choice ch 4 (the option index as note), pickup ch 5; variables as CC
    //! (ch 16, controller = variable index mod 128, value clamped to 0..=127).

    use adapters_core::BusEvent;

    pub const CH_TEXT_BOX: u8 = 1;
    pub const CH_SAMPLE: u8 = 2;
    pub const CH_FX: u8 = 3;
    pub const CH_CHOICE: u8 = 4;
    pub const CH_PICKUP: u8 = 5;
    pub const CH_SLICE: u8 = 16;
    pub const CH_VARIABLE: u8 = 16;
    /// The velocity of every default-map note.
    pub const VELOCITY: u8 = 100;

    /// Status byte for note on, `channel` 1-based (1..=16).
    pub fn note_on(channel: u8, note: u8, velocity: u8) -> [u8; 3] {
        [0x90 | ((channel - 1) & 0x0F), note & 0x7F, velocity & 0x7F]
    }

    /// Status byte for note off, `channel` 1-based.
    pub fn note_off(channel: u8, note: u8) -> [u8; 3] {
        [0x80 | ((channel - 1) & 0x0F), note & 0x7F, 0]
    }

    /// Control change, `channel` 1-based.
    pub fn control_change(channel: u8, controller: u8, value: u8) -> [u8; 3] {
        [0xB0 | ((channel - 1) & 0x0F), controller & 0x7F, value & 0x7F]
    }

    /// The note a slice index lands on: `index mod 128`.
    pub fn slice_note(index: u32) -> u8 {
        (index % 128) as u8
    }

    /// The default map: one bus event → the MIDI message a DAW learns. `None` for events the map does not carry.
    pub fn map(event: &BusEvent) -> Option<[u8; 3]> {
        Some(match event {
            BusEvent::SliceEnter { index } => note_on(CH_SLICE, slice_note(*index), VELOCITY),
            BusEvent::ShowTextBox { index } => note_on(CH_TEXT_BOX, slice_note(*index), VELOCITY),
            BusEvent::Sample { index } => note_on(CH_SAMPLE, slice_note(*index), VELOCITY),
            BusEvent::Fx { index } => note_on(CH_FX, slice_note(*index), VELOCITY),
            BusEvent::Choice { option } => note_on(CH_CHOICE, slice_note(*option), VELOCITY),
            BusEvent::Pickup { index } => note_on(CH_PICKUP, slice_note(*index), VELOCITY),
            BusEvent::Variable { index, value } => {
                control_change(CH_VARIABLE, slice_note(*index), value.round().clamp(0.0, 127.0) as u8)
            }
        })
    }
}

const CLIENT: &str = "scutoons-deck";

/// One MIDI device of the venue patch: an output port, optionally an input port of the same name.
pub struct MidiAdapter {
    outlet: String,
    port: String,
    input: bool,
    out: Option<MidiOutputConnection>,
    in_conn: Option<MidiInputConnection<()>>,
}

impl MidiAdapter {
    /// `port`: a substring of the port name as the OS lists it (`ports()`); `input`: also open the input of that name.
    pub fn new(outlet: impl Into<String>, port: impl Into<String>, input: bool) -> Self {
        MidiAdapter { outlet: outlet.into(), port: port.into(), input, out: None, in_conn: None }
    }

    /// The MIDI ports this machine has (both directions), without opening any.
    pub fn list_ports() -> anyhow::Result<Vec<PortInfo>> {
        let mut ports = Vec::new();
        let output = MidiOutput::new(CLIENT).context("MIDI output client")?;
        for p in output.ports() {
            if let Ok(name) = output.port_name(&p) {
                ports.push(PortInfo { name, direction: Direction::Out });
            }
        }
        let input = MidiInput::new(CLIENT).context("MIDI input client")?;
        for p in input.ports() {
            if let Ok(name) = input.port_name(&p) {
                ports.push(PortInfo { name, direction: Direction::In });
            }
        }
        Ok(ports)
    }
}

impl Adapter for MidiAdapter {
    fn name(&self) -> &'static str {
        "midi"
    }

    fn start(&mut self, inbound: mpsc::Sender<Inbound>) -> anyhow::Result<()> {
        let output = MidiOutput::new(CLIENT).context("MIDI output client")?;
        let port = output
            .ports()
            .into_iter()
            .find(|p| output.port_name(p).map(|n| n.contains(&self.port)).unwrap_or(false))
            .ok_or_else(|| anyhow!("MIDI out port containing {:?} not found", self.port))?;
        self.out = Some(output.connect(&port, &self.outlet).map_err(|e| anyhow!("MIDI out connect: {e}"))?);

        if self.input {
            let input = MidiInput::new(CLIENT).context("MIDI input client")?;
            let port = input
                .ports()
                .into_iter()
                .find(|p| input.port_name(p).map(|n| n.contains(&self.port)).unwrap_or(false))
                .ok_or_else(|| anyhow!("MIDI in port containing {:?} not found", self.port))?;
            let outlet = self.outlet.clone();
            // The callback runs on midir's thread: a bounded channel, `try_send` – a slow bridge drops, never blocks the
            // MIDI thread (dogma: no busy loops, no back-pressure into a realtime callback).
            let conn = input
                .connect(
                    &port,
                    &self.outlet,
                    move |_stamp, bytes, _| {
                        let msg = DeviceMsg::Midi(MidiBytes { bytes: bytes.to_vec() });
                        if inbound.try_send(Inbound { outlet: outlet.clone(), msg }).is_err() {
                            tracing::warn!(outlet, "MIDI in dropped: bridge channel full");
                        }
                    },
                    (),
                )
                .map_err(|e| anyhow!("MIDI in connect: {e}"))?;
            self.in_conn = Some(conn);
        }
        Ok(())
    }

    fn send(&mut self, msg: &DeviceMsg) -> anyhow::Result<()> {
        let DeviceMsg::Midi(m) = msg else {
            return Err(anyhow!("midi adapter got a {} message", kind(msg)));
        };
        let out = self.out.as_mut().ok_or_else(|| anyhow!("MIDI out not started"))?;
        out.send(&m.bytes).map_err(|e| anyhow!("MIDI send: {e}"))
    }

    fn stop(&mut self) -> anyhow::Result<()> {
        if let Some(c) = self.in_conn.take() {
            c.close();
        }
        if let Some(c) = self.out.take() {
            c.close();
        }
        Ok(())
    }

    fn ports(&self) -> Vec<PortInfo> {
        Self::list_ports().unwrap_or_default()
    }
}

fn kind(msg: &DeviceMsg) -> &'static str {
    match msg {
        DeviceMsg::Midi(_) => "midi",
        DeviceMsg::Osc(_) => "osc",
        DeviceMsg::Dmx(_) => "dmx",
    }
}

#[cfg(test)]
mod tests {
    use super::default_map::*;
    use adapters_core::BusEvent;

    #[test]
    fn slice_enter_is_note_on_channel_16() {
        assert_eq!(map(&BusEvent::SliceEnter { index: 0 }), Some([0x9F, 0, VELOCITY]));
        assert_eq!(map(&BusEvent::SliceEnter { index: 12 }), Some([0x9F, 12, VELOCITY]));
        // mod 128
        assert_eq!(map(&BusEvent::SliceEnter { index: 130 }), Some([0x9F, 2, VELOCITY]));
        assert_eq!(slice_note(127), 127);
        assert_eq!(slice_note(128), 0);
    }

    #[test]
    fn event_types_have_their_channels() {
        assert_eq!(map(&BusEvent::ShowTextBox { index: 3 }).unwrap()[0], 0x90); // ch 1
        assert_eq!(map(&BusEvent::Sample { index: 3 }).unwrap()[0], 0x91); // ch 2
        assert_eq!(map(&BusEvent::Fx { index: 3 }).unwrap()[0], 0x92); // ch 3
        assert_eq!(map(&BusEvent::Choice { option: 1 }), Some([0x93, 1, VELOCITY])); // ch 4, option as note
        assert_eq!(map(&BusEvent::Pickup { index: 9 }).unwrap()[0], 0x94); // ch 5
    }

    #[test]
    fn variables_are_control_changes() {
        assert_eq!(map(&BusEvent::Variable { index: 2, value: 64.0 }), Some([0xBF, 2, 64]));
        assert_eq!(map(&BusEvent::Variable { index: 2, value: 999.0 }), Some([0xBF, 2, 127]));
        assert_eq!(map(&BusEvent::Variable { index: 2, value: -5.0 }), Some([0xBF, 2, 0]));
    }

    #[test]
    fn status_bytes() {
        assert_eq!(note_on(1, 60, 100), [0x90, 60, 100]);
        assert_eq!(note_off(16, 60), [0x8F, 60, 0]);
        assert_eq!(control_change(16, 7, 127), [0xBF, 7, 127]);
    }
}
