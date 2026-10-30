//! The Enttec DMX USB Pro widget protocol (the "API" Enttec publishes; DMXKing's ultraDMX speak it too): every
//! message is `0x7E, label, length LSB, length MSB, data…, 0xE7`. Pure functions, tested without a port.

/// Start of message.
pub const SOM: u8 = 0x7E;
/// End of message.
pub const EOM: u8 = 0xE7;

/// Get Widget Parameters (request: user config size LSB / MSB; reply: firmware, break, MAB, rate, user config).
pub const LABEL_GET_PARAMS: u8 = 3;
/// Output Only Send DMX Packet: the DMX start code + 24…512 channels; the widget repeats it on the line by itself.
pub const LABEL_SEND_DMX: u8 = 6;
/// Get Widget Serial Number (request: no data; reply: 4 bytes BCD, least significant first).
pub const LABEL_GET_SERIAL: u8 = 10;

/// Channels in a universe.
pub const UNIVERSE: usize = 512;
/// The largest data a message may carry (the API's limit is 600).
pub const MAX_DATA: usize = 600;

/// One framed message.
pub fn frame(label: u8, data: &[u8]) -> Vec<u8> {
    assert!(data.len() <= MAX_DATA, "widget message too long");
    let mut out = Vec::with_capacity(data.len() + 5);
    out.extend_from_slice(&[SOM, label, (data.len() & 0xFF) as u8, (data.len() >> 8) as u8]);
    out.extend_from_slice(data);
    out.push(EOM);
    out
}

/// Label 6 with start code 0 and the whole universe (always 512 channels: fixtures see one stable frame length).
pub fn dmx_packet(universe: &[u8; UNIVERSE]) -> Vec<u8> {
    let mut data = Vec::with_capacity(UNIVERSE + 1);
    data.push(0x00);
    data.extend_from_slice(universe);
    frame(LABEL_SEND_DMX, &data)
}

/// Label 3 request: no user configuration wanted.
pub fn params_request() -> Vec<u8> {
    frame(LABEL_GET_PARAMS, &[0, 0])
}

/// Label 10 request.
pub fn serial_request() -> Vec<u8> {
    frame(LABEL_GET_SERIAL, &[])
}

/// What label 3 answers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Params {
    /// Firmware version (MSB = major).
    pub firmware: u16,
    /// DMX break in units of 10.67 µs.
    pub break_time: u8,
    /// Mark after break in units of 10.67 µs.
    pub mab_time: u8,
    /// Output rate in packets per second (0 = as fast as possible).
    pub rate: u8,
}

impl Params {
    pub fn parse(data: &[u8]) -> Option<Params> {
        let &[lo, hi, break_time, mab_time, rate, ..] = data else { return None };
        Some(Params { firmware: u16::from_le_bytes([lo, hi]), break_time, mab_time, rate })
    }
}

/// Label 10's 4 BCD bytes, least significant first → the serial as printed on the widget ("00123456").
pub fn parse_serial(data: &[u8]) -> Option<String> {
    let bytes: [u8; 4] = data.get(..4)?.try_into().ok()?;
    let mut s = String::with_capacity(8);
    for b in bytes.iter().rev() {
        let (hi, lo) = (b >> 4, b & 0x0F);
        if hi > 9 || lo > 9 {
            return None;
        }
        s.push(char::from(b'0' + hi));
        s.push(char::from(b'0' + lo));
    }
    Some(s)
}

/// Splits a byte stream from the widget into `(label, data)` messages; skips garbage up to the next 0x7E and drops a
/// message whose end byte is wrong (it resyncs on the next start byte).
#[derive(Default)]
pub struct Decoder {
    buf: Vec<u8>,
}

impl Decoder {
    pub fn push(&mut self, bytes: &[u8]) -> Vec<(u8, Vec<u8>)> {
        self.buf.extend_from_slice(bytes);
        let mut out = Vec::new();
        loop {
            match self.buf.iter().position(|&b| b == SOM) {
                Some(0) => {}
                Some(i) => {
                    self.buf.drain(..i);
                }
                None => {
                    self.buf.clear();
                    return out;
                }
            }
            if self.buf.len() < 4 {
                return out;
            }
            let len = usize::from(self.buf[2]) | usize::from(self.buf[3]) << 8;
            if len > MAX_DATA {
                self.buf.drain(..1);
                continue;
            }
            if self.buf.len() < len + 5 {
                return out;
            }
            if self.buf[len + 4] == EOM {
                out.push((self.buf[1], self.buf[4..4 + len].to_vec()));
                self.buf.drain(..len + 5);
            } else {
                self.buf.drain(..1);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dmx_packet_bytes() {
        let mut u = [0u8; UNIVERSE];
        u[0] = 255;
        u[2] = 128;
        u[511] = 7;
        let p = dmx_packet(&u);
        assert_eq!(p.len(), 518, "5 framing bytes + start code + 512");
        // 0x7E, label 6, length 513 = 0x0201 little-endian, start code 0
        assert_eq!(&p[..5], &[0x7E, 6, 0x01, 0x02, 0x00]);
        assert_eq!(&p[5..8], &[255, 0, 128]);
        assert_eq!(p[516], 7);
        assert_eq!(p[517], 0xE7);
    }

    #[test]
    fn requests() {
        assert_eq!(params_request(), vec![0x7E, 3, 2, 0, 0, 0, 0xE7]);
        assert_eq!(serial_request(), vec![0x7E, 10, 0, 0, 0xE7]);
    }

    #[test]
    fn decoder_resyncs_and_splits() {
        let mut d = Decoder::default();
        let params = frame(3, &[0x44, 0x01, 9, 1, 40]);
        let serial = frame(10, &[0x56, 0x34, 0x12, 0x00]);
        let mut stream = vec![0x00, 0x13, 0xE7]; // garbage before
        stream.extend_from_slice(&[0x7E, 3, 1, 0, 9, 0x00]); // a broken message (wrong end byte)
        stream.extend_from_slice(&params);
        stream.extend_from_slice(&serial[..3]); // split across reads
        let first = d.push(&stream);
        assert_eq!(first, vec![(3, vec![0x44, 0x01, 9, 1, 40])]);
        let second = d.push(&serial[3..]);
        assert_eq!(second, vec![(10, vec![0x56, 0x34, 0x12, 0x00])]);
        assert!(d.push(&[0xFF; 3]).is_empty());
    }

    #[test]
    fn params_and_serial_parse() {
        let p = Params::parse(&[0x44, 0x01, 9, 1, 40]).unwrap();
        assert_eq!(p, Params { firmware: 0x0144, break_time: 9, mab_time: 1, rate: 40 });
        assert_eq!(Params::parse(&[1, 2]), None);
        assert_eq!(parse_serial(&[0x56, 0x34, 0x12, 0x00]).as_deref(), Some("00123456"));
        assert_eq!(parse_serial(&[0xFF, 0, 0, 0]), None, "not BCD");
        assert_eq!(parse_serial(&[1, 2]), None);
    }
}
