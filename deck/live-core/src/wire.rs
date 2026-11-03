//! The wire format across the page / worker ↔ wasm boundary (#184): binary records, no JSON. The TypeScript twin is
//! shared/live-core/src/wire.ts; `fixtures/wire.json` holds records with their bytes, and both sides' tests encode and
//! decode every one of them (the layout cannot drift).
//!
//! A record, little-endian, 4-byte aligned so the values can be read as a `Float32Array` in place:
//!
//! | offset | type | field |
//! |---|---|---|
//! | 0 | u8 | op – 1 = `send` |
//! | 1 | u8 | mode – 0 force, 1 blend, 2 release, 3 base (#188: a knob turns the input's base, a link flows on top) |
//! | 2 | u16 | n – the number of values |
//! | 4 | u16 | node – its index in the load table (names are resolved once, at load – JSON stays at the edge) |
//! | 6 | u16 | port – the first input the vector lands on (v[0] → port, v[1] → port + 1 …) |
//! | 8 | f32 | amount – blend: 0–1 (force = 1, release = 0) |
//! | 12 | f32 × n | the values (#189's float vectors) |
//!
//! Records follow each other without padding (each is a multiple of 4 bytes). The analysis frame going out is not a
//! record but a block of f32: level, bass, mid, high, then the `SPECTRUM` bins, then the limiter's gain reduction (dB,
//! the deepest since the last frame – #188), then (#282) the number of device channels and `MAX_OUTPUTS` peaks (0..1,
//! each channel's largest sample since the last frame; the slots past the count stay 0), then (P6, #320) the number of
//! values and the values themselves: every load-table row's inputs and outputs of the heard block, from the row's `at`.
//! Device messages leave in their own records (signals.rs).

use deck_core::analysis::SPECTRUM;

pub const HEADER: usize = 12;
pub const OP_SEND: u8 = 1;
/// the analysis frame's slot of the limiter's gain reduction (dB)
pub const REDUCTION: usize = 4 + SPECTRUM;
/// … of the number of device channels the engine renders (#282)
pub const OUTPUT_COUNT: usize = REDUCTION + 1;
/// … of the first channel's peak
pub const PEAKS: usize = REDUCTION + 2;
/// the most channels a frame carries peaks for (deck-engine's device channel limit)
pub const MAX_OUTPUTS: usize = 32;
/// … of the number of values that follow
pub const VALUE_COUNT: usize = PEAKS + MAX_OUTPUTS;
/// … of the first value
pub const VALUES_AT: usize = VALUE_COUNT + 1;
/// the analysis frame's fixed part: 4 bands + the spectrum + the reduction + the channel count + the peaks + the value
/// count; the values follow it
pub const FRAME: usize = VALUES_AT;
/// the most values a frame carries (a row past them reads `at` = -1 in the table)
pub const MAX_FRAME_VALUES: usize = 4096;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Force = 0,
    Blend = 1,
    Release = 2,
    /// the values become the inputs' bases (`Control::set_base`): a linked input keeps its link on top
    Base = 3,
}

impl Mode {
    fn from_u8(b: u8) -> Option<Mode> {
        Some(match b {
            0 => Mode::Force,
            1 => Mode::Blend,
            2 => Mode::Release,
            3 => Mode::Base,
            _ => return None,
        })
    }

    /// what the input keeps of the sent value (`Control::send`'s amount)
    pub fn amount(self, amount: f32) -> f32 {
        match self {
            Mode::Force => 1.0,
            Mode::Blend => amount,
            Mode::Release | Mode::Base => 0.0,
        }
    }
}

/// One decoded `send` record; the values stay in the input bytes.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Send<'a> {
    pub mode: Mode,
    pub node: u16,
    pub port: u16,
    pub amount: f32,
    raw: &'a [u8],
}

impl Send<'_> {
    pub fn len(&self) -> usize {
        self.raw.len() / 4
    }

    pub fn is_empty(&self) -> bool {
        self.raw.is_empty()
    }

    pub fn values(&self) -> impl Iterator<Item = f32> + '_ {
        self.raw.chunks_exact(4).map(|b| f32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    }
}

/// Where a buffer stops being records.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WireError {
    pub at: usize,
    pub what: &'static str,
}

impl std::fmt::Display for WireError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "wire: {} at byte {}", self.what, self.at)
    }
}

/// The records of `bytes`, in order; the first malformed one ends the walk with its error.
pub fn records(bytes: &[u8]) -> Records<'_> {
    Records { bytes, at: 0 }
}

pub struct Records<'a> {
    bytes: &'a [u8],
    at: usize,
}

impl<'a> Iterator for Records<'a> {
    type Item = Result<Send<'a>, WireError>;

    fn next(&mut self) -> Option<Self::Item> {
        let (b, at) = (self.bytes, self.at);
        if at >= b.len() {
            return None;
        }
        let fail = |what| Some(Err(WireError { at, what }));
        if b.len() - at < HEADER {
            self.at = b.len();
            return fail("a truncated header");
        }
        let h = &b[at..at + HEADER];
        let n = u16::from_le_bytes([h[2], h[3]]) as usize;
        let end = at + HEADER + 4 * n;
        if h[0] != OP_SEND {
            self.at = b.len();
            return fail("an unknown op");
        }
        let Some(mode) = Mode::from_u8(h[1]) else {
            self.at = b.len();
            return fail("an unknown mode");
        };
        if end > b.len() {
            self.at = b.len();
            return fail("truncated values");
        }
        self.at = end;
        Some(Ok(Send {
            mode,
            node: u16::from_le_bytes([h[4], h[5]]),
            port: u16::from_le_bytes([h[6], h[7]]),
            amount: f32::from_le_bytes([h[8], h[9], h[10], h[11]]),
            raw: &b[at + HEADER..end],
        }))
    }
}

/// Appends one `send` record to `out`.
pub fn encode_send(out: &mut Vec<u8>, mode: Mode, node: u16, port: u16, amount: f32, values: &[f32]) {
    let n = u16::try_from(values.len()).expect("at most 65535 values per record");
    out.extend_from_slice(&[OP_SEND, mode as u8]);
    out.extend_from_slice(&n.to_le_bytes());
    out.extend_from_slice(&node.to_le_bytes());
    out.extend_from_slice(&port.to_le_bytes());
    out.extend_from_slice(&amount.to_le_bytes());
    for v in values {
        out.extend_from_slice(&v.to_le_bytes());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    const FIXTURE: &str = include_str!("../fixtures/wire.json");

    fn hex(b: &[u8]) -> String {
        b.iter().map(|x| format!("{x:02x}")).collect()
    }

    fn mode(s: &str) -> Mode {
        match s {
            "force" => Mode::Force,
            "blend" => Mode::Blend,
            "release" => Mode::Release,
            "base" => Mode::Base,
            _ => panic!("mode {s}"),
        }
    }

    /// Every fixture record encodes to its bytes and decodes back to its fields (shared/live-core's test reads the same
    /// file).
    #[test]
    fn fixture_round_trip() {
        let rows: Vec<Value> = serde_json::from_str(FIXTURE).unwrap();
        assert!(rows.len() >= 4);
        let mut all = vec![];
        for row in &rows {
            let r = &row["record"];
            let values: Vec<f32> = r["v"].as_array().unwrap().iter().map(|x| x.as_f64().unwrap() as f32).collect();
            let (m, node, port, amount) =
                (mode(r["mode"].as_str().unwrap()), r["node"].as_u64().unwrap() as u16, r["port"].as_u64().unwrap() as u16, r["amount"].as_f64().unwrap() as f32);
            let mut out = vec![];
            encode_send(&mut out, m, node, port, amount, &values);
            assert_eq!(hex(&out), row["hex"].as_str().unwrap(), "{}", row["name"]);
            let got = records(&out).next().unwrap().unwrap();
            assert_eq!((got.mode, got.node, got.port, got.amount), (m, node, port, amount));
            assert_eq!(got.values().collect::<Vec<_>>(), values);
            all.extend_from_slice(&out);
        }
        // the records back to back: one walk finds them all
        assert_eq!(records(&all).filter(Result::is_ok).count(), rows.len());
    }

    #[test]
    fn malformed_bytes_stop_the_walk() {
        let mut b = vec![];
        encode_send(&mut b, Mode::Force, 1, 0, 1.0, &[0.5, 0.25]);
        let err = |bytes: &[u8]| records(bytes).find_map(Result::err).unwrap();
        assert_eq!(err(&b[..HEADER + 4]), WireError { at: 0, what: "truncated values" });
        assert_eq!(err(&b[..7]).what, "a truncated header");
        let mut bad = b.clone();
        bad[0] = 9;
        assert_eq!(err(&bad).what, "an unknown op");
        bad[0] = OP_SEND;
        bad[1] = 7;
        assert_eq!(err(&bad).what, "an unknown mode");
        // a good record, then garbage: the first comes out, then the error at its end
        let mut two = b.clone();
        two.extend_from_slice(&[1, 0, 0]);
        let walked: Vec<_> = records(&two).collect();
        assert!(walked[0].is_ok());
        assert_eq!(walked[1], Err(WireError { at: b.len(), what: "a truncated header" }));
    }
}
