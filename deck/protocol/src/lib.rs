//! The Live relay protocol, v1 (agents/research/LIVE.md B.6). The types live in `generated.rs`, written by
//! `deck/protocol/generate.ts` from `deck/protocol/schema.json` – the same schema generates `@scutoons/live`'s
//! `protocol.ts`, so the web clients and the relay never drift. Hand-written here: only helpers.

mod generated;

pub use generated::*;

impl Envelope {
    /// An unstamped envelope (client → relay): `seq` 0, no `at`, no `to`.
    pub fn new(from: impl Into<String>, message: Message) -> Self {
        Envelope { v: PROTOCOL_VERSION, seq: 0, at: None, from: from.into(), to: None, message }
    }

    /// The same envelope addressed to `to`.
    pub fn to(mut self, to: To) -> Self {
        self.to = Some(to);
        self
    }
}

impl To {
    pub fn role(role: Role) -> Self {
        To { role: Some(role), group: None, client: None }
    }
    pub fn group(group: impl Into<String>) -> Self {
        To { role: None, group: Some(group.into()), client: None }
    }
    pub fn client(client: impl Into<String>) -> Self {
        To { role: None, group: None, client: Some(client.into()) }
    }
    /// `role` and `group` together (an outlet's screens, one side of the audience).
    pub fn role_group(role: Role, group: impl Into<String>) -> Self {
        To { role: Some(role), group: Some(group.into()), client: None }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    const FIXTURE: &str = include_str!("../fixtures/messages.json");

    /// Every envelope of the shared fixture (the TypeScript side parses the same file: shared/live/test) survives
    /// deserialize → serialize unchanged.
    #[test]
    fn fixture_round_trip() {
        let raw: Vec<Value> = serde_json::from_str(FIXTURE).unwrap();
        assert!(raw.len() >= 20);
        for value in raw {
            let kind = value["kind"].as_str().unwrap().to_owned();
            let envelope: Envelope = serde_json::from_value(value.clone())
                .unwrap_or_else(|e| panic!("{kind}: {e}\n{value}"));
            assert_eq!(envelope.message.kind(), kind);
            assert_eq!(envelope.v, PROTOCOL_VERSION);
            let back = serde_json::to_value(&envelope).unwrap();
            assert_eq!(back, value, "{kind} changed in the round trip");
        }
    }

    #[test]
    fn typed_payloads() {
        let raw: Vec<Envelope> = serde_json::from_str(FIXTURE).unwrap();
        let device = raw.iter().find(|e| matches!(e.message, Message::Device(_))).unwrap();
        let Message::Device(d) = &device.message else { unreachable!() };
        assert_eq!(d.outlet, "ableton");
        let DeviceMsg::Midi(m) = &d.msgs[0] else { panic!("midi expected") };
        assert_eq!(m.bytes, vec![0x9F, 12, 100]);
        assert_eq!(device.to.as_ref().unwrap().role, Some(Role::Bridge));
    }

    #[test]
    fn unknown_kind_is_an_error() {
        let bad = r#"{"v":1,"seq":0,"from":"x","kind":"teleport","p":{}}"#;
        assert!(serde_json::from_str::<Envelope>(bad).is_err());
    }

    #[test]
    fn builders() {
        let e = Envelope::new("c1", Message::Ready(Empty {})).to(To::role_group(Role::Output, "left"));
        let json = serde_json::to_string(&e).unwrap();
        assert_eq!(json, r#"{"v":1,"seq":0,"from":"c1","to":{"role":"output","group":"left"},"kind":"ready","p":{}}"#);
    }
}
