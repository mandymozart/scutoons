//! Finding the widget without a configured `port`: the serial ports the OS lists, ranked by their USB ids and names.
//! FTDI's 0403:6001 is in every Enttec DMX USB Pro and the older DMXKing ultraDMX Micro / Pro – but also in any
//! FTDI USB-serial cable, so a name that says DMX / Enttec / DMXking ranks higher. The ultraDMX MAX is a USB CDC
//! device (no FTDI); it is found by its name (QLC+ lists its product id 0x0094; the vendor id is not documented).

use serialport::{SerialPortInfo, SerialPortType};

/// USB ids that carry the widget protocol.
pub const KNOWN: &[(u16, u16, &str)] = &[(0x0403, 0x6001, "FTDI FT232R / FT245R (Enttec DMX USB Pro, DMXKing ultraDMX Micro / Pro)")];

/// Words in a USB manufacturer / product string that mean a DMX widget (lower case).
const HINTS: &[&str] = &["dmx", "enttec", "dmxking"];

/// One serial port as the detection sees it (plain data, so the tests need no OS).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Candidate {
    pub name: String,
    pub vid: Option<u16>,
    pub pid: Option<u16>,
    pub manufacturer: Option<String>,
    pub product: Option<String>,
    pub serial: Option<String>,
}

impl Candidate {
    pub fn from_info(info: &SerialPortInfo) -> Candidate {
        let mut c = Candidate { name: info.port_name.clone(), ..Candidate::default() };
        if let SerialPortType::UsbPort(usb) = &info.port_type {
            c.vid = Some(usb.vid);
            c.pid = Some(usb.pid);
            c.manufacturer = usb.manufacturer.clone();
            c.product = usb.product.clone();
            c.serial = usb.serial_number.clone();
        }
        c
    }

    /// 0 = not a widget; +1 a known USB id; +2 a DMX name. macOS lists every device twice (`/dev/tty.*` for
    /// dial-in, `/dev/cu.*` for calling out) – only `cu.` counts.
    pub fn score(&self) -> u8 {
        if self.name.starts_with("/dev/tty.") {
            return 0;
        }
        let known = matches!((self.vid, self.pid), (Some(v), Some(p)) if KNOWN.iter().any(|&(kv, kp, _)| kv == v && kp == p));
        let text = format!("{} {}", self.manufacturer.as_deref().unwrap_or(""), self.product.as_deref().unwrap_or("")).to_lowercase();
        let named = HINTS.iter().any(|h| text.contains(h));
        u8::from(known) + 2 * u8::from(named)
    }

    /// For logs and `deck-check`: `COM3 0403:6001 ENTTEC DMX USB PRO (EN123456)`.
    pub fn describe(&self) -> String {
        let mut s = self.name.clone();
        if let (Some(v), Some(p)) = (self.vid, self.pid) {
            s.push_str(&format!(" {v:04x}:{p:04x}"));
        }
        for part in [&self.manufacturer, &self.product].into_iter().flatten() {
            s.push(' ');
            s.push_str(part);
        }
        if let Some(serial) = &self.serial {
            s.push_str(&format!(" ({serial})"));
        }
        s
    }
}

/// The widgets among `ports`, best first (score, then name).
pub fn rank(ports: &[Candidate]) -> Vec<&Candidate> {
    let mut v: Vec<&Candidate> = ports.iter().filter(|c| c.score() > 0).collect();
    v.sort_by(|a, b| b.score().cmp(&a.score()).then_with(|| a.name.cmp(&b.name)));
    v
}

/// The port to open, if any.
pub fn pick(ports: &[Candidate]) -> Option<&Candidate> {
    rank(ports).into_iter().next()
}

/// The serial ports of this machine (empty if the OS cannot list them).
pub fn candidates() -> Vec<Candidate> {
    match serialport::available_ports() {
        Ok(ports) => ports.iter().map(Candidate::from_info).collect(),
        Err(e) => {
            tracing::debug!("serial ports not listed: {e}");
            Vec::new()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serialport::UsbPortInfo;

    fn usb(name: &str, vid: u16, pid: u16, manufacturer: Option<&str>, product: Option<&str>) -> Candidate {
        let info = SerialPortInfo {
            port_name: name.into(),
            port_type: SerialPortType::UsbPort(UsbPortInfo {
                vid,
                pid,
                serial_number: Some("EN123456".into()),
                manufacturer: manufacturer.map(Into::into),
                product: product.map(Into::into),
            }),
        };
        Candidate::from_info(&info)
    }

    #[test]
    fn ranks_by_id_and_name() {
        let ports = vec![
            Candidate { name: "/dev/ttyS0".into(), ..Candidate::default() },
            usb("/dev/ttyUSB1", 0x0403, 0x6001, Some("FTDI"), Some("FT232R USB UART")), // a plain FTDI cable
            usb("/dev/ttyACM0", 0x2341, 0x0043, Some("Arduino"), None),
            usb("/dev/ttyUSB0", 0x0403, 0x6001, Some("ENTTEC"), Some("DMX USB PRO")),
            usb("/dev/ttyACM1", 0x1fc9, 0x0094, Some("DMXking.com"), Some("ultraDMX MAX")),
        ];
        let ranked: Vec<&str> = rank(&ports).iter().map(|c| c.name.as_str()).collect();
        assert_eq!(ranked, ["/dev/ttyUSB0", "/dev/ttyACM1", "/dev/ttyUSB1"]);
        assert_eq!(pick(&ports).unwrap().name, "/dev/ttyUSB0");
        assert_eq!(ports[3].describe(), "/dev/ttyUSB0 0403:6001 ENTTEC DMX USB PRO (EN123456)");
    }

    #[test]
    fn macos_counts_cu_only_and_nothing_is_none() {
        let ports = vec![
            usb("/dev/tty.usbserial-EN123456", 0x0403, 0x6001, Some("ENTTEC"), Some("DMX USB PRO")),
            usb("/dev/cu.usbserial-EN123456", 0x0403, 0x6001, Some("ENTTEC"), Some("DMX USB PRO")),
        ];
        assert_eq!(rank(&ports).len(), 1);
        assert_eq!(pick(&ports).unwrap().name, "/dev/cu.usbserial-EN123456");
        assert!(pick(&[Candidate { name: "COM1".into(), ..Candidate::default() }]).is_none());
        assert!(pick(&[]).is_none());
    }
}
