//! The device monitor's input as the page hands it over (#373): a device message in the shape the engine's signal
//! records decode to in the browser – `{"kind":"midi","port":"out"|"conductor","bytes":[…]}`, `{"kind":"dmx","ch":1,
//! "values":[…],"flash":false}`, `{"kind":"osc","address":"/a","args":[{"kind":"float","p":0.5}]}` – pushed into deck-core's
//! `DeviceMonitor`. The browser (space-web's `DeckMonitor.signal`) and the headless host's scenarios both come through
//! here, so the words and the lamp are one path. The monitor itself (the lines, the activity) is deck-core's.
use deck_core::devices::{DeviceMonitor, OscArg, Signal};
use serde_json::Value;

/// Push the message `v` (as above) into `monitor` at `now_ms`. False when `v` is not a device message (nothing pushed).
pub fn push_json(monitor: &mut DeviceMonitor, v: &Value, now_ms: f64) -> bool {
    let bytes = |k: &str| v.get(k).and_then(Value::as_array).into_iter().flatten().filter_map(Value::as_u64).map(|b| b as u8).collect::<Vec<u8>>();
    match v.get("kind").and_then(Value::as_str) {
        Some("midi") => monitor.push(&Signal::Midi { conductor: v.get("port").and_then(Value::as_str) == Some("conductor"), bytes: &bytes("bytes") }, now_ms),
        Some("dmx") => {
            let ch = v.get("ch").and_then(Value::as_u64).unwrap_or(1) as u16;
            monitor.push(&Signal::Dmx { ch, values: &bytes("values"), flash: v.get("flash").and_then(Value::as_bool).unwrap_or(false) }, now_ms);
        }
        Some("osc") => {
            let args: Vec<OscArg> = v
                .get("args")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .filter_map(|a| {
                    let p = &a["p"];
                    OscArg::named(a["kind"].as_str()?, p.as_f64().or_else(|| p.as_bool().map(f64::from)).unwrap_or(0.0), p.as_str().unwrap_or_default())
                })
                .collect();
            monitor.push(&Signal::Osc { address: v.get("address").and_then(Value::as_str).unwrap_or_default(), args: &args }, now_ms);
        }
        _ => return false,
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use deck_core::devices::Kind;
    use serde_json::json;

    #[test]
    fn a_message_as_the_page_sends_it_is_worded_by_the_core() {
        let mut m = DeviceMonitor::new();
        assert!(push_json(&mut m, &json!({ "kind": "midi", "port": "conductor", "bytes": [176, 74, 127], "at": 5 }), 10.0));
        assert!(push_json(&mut m, &json!({ "kind": "dmx", "ch": 20, "values": [255, 255], "flash": false }), 10.0));
        assert!(push_json(&mut m, &json!({ "kind": "osc", "address": "/fx", "args": [{ "kind": "float", "p": 0.5 }, { "kind": "bool", "p": true }, { "kind": "blob", "p": 1 }] }), 10.0));
        assert!(!push_json(&mut m, &json!({ "kind": "sysex" }), 10.0));
        assert_eq!((m.text(Kind::Midi), m.text(Kind::Dmx), m.text(Kind::Osc)), ("CC 74 = 127 · ch 1 → conductor", "DMX 20–21 = 255 255", "/fx 0.5 true"));
    }
}
