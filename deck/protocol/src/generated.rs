// GENERATED from deck/protocol/schema.json by deck/protocol/generate.ts – edit the schema, run `npm run generate -w shared/live`.
// agents/research/LIVE.md B.6. One envelope per message, JSON over one WebSocket per client. The generator (deck/protocol/generate.ts) reads only the subset used here: object / string / integer / number / boolean / array / $ref / enum / const / oneOf discriminated by `kind` / additionalProperties maps. Edit this file, run `npm run generate -w shared/live`, commit both outputs.
#![allow(clippy::all)]
use serde::{Deserialize, Serialize};

/// A payload whose shape is not fixed yet: any JSON object.
pub type Loose = serde_json::Map<String, serde_json::Value>;

/// LIVE.md B.2. `uplink` is the local relay connected to the hosted one (B.6 Uplink).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Role {
	#[serde(rename = "conductor")]
	Conductor,
	#[serde(rename = "output")]
	Output,
	#[serde(rename = "bridge")]
	Bridge,
	#[serde(rename = "performer")]
	Performer,
	#[serde(rename = "musician")]
	Musician,
	#[serde(rename = "audience")]
	Audience,
	#[serde(rename = "uplink")]
	Uplink,
}

impl Role {
	pub fn as_str(self) -> &'static str {
		match self {
			Role::Conductor => "conductor",
			Role::Output => "output",
			Role::Bridge => "bridge",
			Role::Performer => "performer",
			Role::Musician => "musician",
			Role::Audience => "audience",
			Role::Uplink => "uplink",
		}
	}
}

/// Who a message reaches. Every field narrows; all missing = everyone the sender may reach.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct To {
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub role: Option<Role>,
	/// a group id (B.5.3)
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub group: Option<String>,
	/// one client id (assigned in `welcome`)
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub client: Option<String>,
}

/// B.6: `{ v: 1, seq, at?, from, to?, kind, p }`. `seq` is per session, assigned by the relay; `at` = relay time (ms) when the effect should happen.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Envelope {
	pub v: i64,
	/// 0 before the relay stamped it (client → relay)
	pub seq: u64,
	/// relay time in ms when the effect should happen
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub at: Option<i64>,
	/// client id; `relay` for the relay's own messages
	pub from: String,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub to: Option<To>,
	#[serde(flatten)]
	pub message: Message,
}

/// `kind` + its payload `p`. One entry per B.6 message; a loose payload (`Loose`) marks a kind whose shape is not fixed yet.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "p")]
pub enum Message {
	#[serde(rename = "hello")]
	Hello(Hello),
	#[serde(rename = "welcome")]
	Welcome(Welcome),
	#[serde(rename = "ping")]
	Ping(Ping),
	#[serde(rename = "pong")]
	Pong(Pong),
	#[serde(rename = "state")]
	State(State),
	#[serde(rename = "cue")]
	Cue(Cue),
	#[serde(rename = "vote.open")]
	VoteOpen(VoteOpen),
	#[serde(rename = "vote.close")]
	VoteClose(VoteClose),
	#[serde(rename = "vote.cast")]
	VoteCast(VoteCast),
	#[serde(rename = "vote.tally")]
	VoteTally(VoteTally),
	#[serde(rename = "vote.result")]
	VoteResult(VoteResult),
	#[serde(rename = "assign")]
	Assign(Assign),
	#[serde(rename = "panic")]
	Panic(Panic),
	#[serde(rename = "media.preload")]
	MediaPreload(MediaPreload),
	#[serde(rename = "snapshot")]
	Snapshot(Snapshot),
	#[serde(rename = "cell")]
	Cell(CellChange),
	#[serde(rename = "say")]
	Say(Loose),
	#[serde(rename = "line")]
	Line(Loose),
	#[serde(rename = "fx")]
	Fx(Loose),
	#[serde(rename = "colour")]
	Colour(Loose),
	#[serde(rename = "presence")]
	Presence(Presence),
	#[serde(rename = "ready")]
	Ready(Empty),
	#[serde(rename = "pickup")]
	Pickup(Pickup),
	#[serde(rename = "react")]
	React(React),
	#[serde(rename = "device")]
	Device(Device),
	#[serde(rename = "device.in")]
	DeviceIn(DeviceIn),
	#[serde(rename = "devices")]
	Devices(Devices),
	#[serde(rename = "error")]
	Error(Error),
}

impl Message {
	/// The `kind` string on the wire.
	pub fn kind(&self) -> &'static str {
		match self {
			Message::Hello(_) => "hello",
			Message::Welcome(_) => "welcome",
			Message::Ping(_) => "ping",
			Message::Pong(_) => "pong",
			Message::State(_) => "state",
			Message::Cue(_) => "cue",
			Message::VoteOpen(_) => "vote.open",
			Message::VoteClose(_) => "vote.close",
			Message::VoteCast(_) => "vote.cast",
			Message::VoteTally(_) => "vote.tally",
			Message::VoteResult(_) => "vote.result",
			Message::Assign(_) => "assign",
			Message::Panic(_) => "panic",
			Message::MediaPreload(_) => "media.preload",
			Message::Snapshot(_) => "snapshot",
			Message::Cell(_) => "cell",
			Message::Say(_) => "say",
			Message::Line(_) => "line",
			Message::Fx(_) => "fx",
			Message::Colour(_) => "colour",
			Message::Presence(_) => "presence",
			Message::Ready(_) => "ready",
			Message::Pickup(_) => "pickup",
			Message::React(_) => "react",
			Message::Device(_) => "device",
			Message::DeviceIn(_) => "device.in",
			Message::Devices(_) => "devices",
			Message::Error(_) => "error",
		}
	}
}

/// B.11 (3): majority = most votes; groups = each group's majority is one vote; first = the first option to reach `n`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum VoteRule {
	#[serde(rename = "majority")]
	Majority,
	#[serde(rename = "groups")]
	Groups,
	#[serde(rename = "first")]
	First,
}

impl VoteRule {
	pub fn as_str(self) -> &'static str {
		match self {
			VoteRule::Majority => "majority",
			VoteRule::Groups => "groups",
			VoteRule::First => "first",
		}
	}
}

/// conductor → relay → the voting groups. `options` are the choice event's labels (index = option). The relay tallies, closes at `seconds` by itself and sends `vote.result`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VoteOpen {
	pub vote: String,
	pub options: Vec<String>,
	pub rule: VoteRule,
	/// rule first: the votes an option needs
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub n: Option<i64>,
	pub seconds: i64,
	/// the option when nobody voted (default 0)
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub fallback: Option<i64>,
	/// which groups vote (default: all audience)
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub groups: Option<Vec<String>>,
	/// the audience sees the tally too
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub show: Option<bool>,
}

/// conductor → relay: close now; `override` = the conductor's option instead of the rule's.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VoteClose {
	pub vote: String,
	#[serde(rename = "override", default, skip_serializing_if = "Option::is_none")]
	pub r#override: Option<i64>,
}

/// relay → conductor every 250 ms while a vote is open (→ the audience too when the vote shows it).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VoteTally {
	pub vote: String,
	pub counts: Vec<i64>,
	pub voters: i64,
	/// ms until the vote closes by itself
	pub left: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum VoteHow {
	#[serde(rename = "rule")]
	Rule,
	#[serde(rename = "fallback")]
	Fallback,
	#[serde(rename = "override")]
	Override,
}

impl VoteHow {
	pub fn as_str(self) -> &'static str {
		match self {
			VoteHow::Rule => "rule",
			VoteHow::Fallback => "fallback",
			VoteHow::Override => "override",
		}
	}
}

/// relay → everyone: the option the conductor applies as the choice event (B.1).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VoteResult {
	pub vote: String,
	pub option: i64,
	pub counts: Vec<i64>,
	pub how: VoteHow,
}

/// conductor → relay → the outlet's clients on a row change: the outlet's new cell (B.5.2 `Cell`, loose until the show schema lands).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CellChange {
	pub outlet: String,
	pub cell: Loose,
}

/// What a client shows when it joins / reconnects / is moved (B.6): the last state, the current cell per outlet it sees, the open vote, the last instruction (say) per group it sees. The conductor sees every outlet and group.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Snapshot {
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub state: Option<State>,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub cells: Option<std::collections::HashMap<String, Loose>>,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub vote: Option<VoteOpen>,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub say: Option<std::collections::HashMap<String, Loose>>,
	/// the session's seq when the snapshot was taken
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub seq: Option<i64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Empty {}

/// client → relay, the first message. Audience / performer / musician / output join with the session code (+ group from their QR); conductor and bridge with a session token.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Hello {
	pub code: String,
	pub role: Role,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub group: Option<String>,
	/// trimmed to 24 characters by the relay
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub nick: Option<String>,
	/// HMAC session token (conductor, bridge, uplink)
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub token: Option<String>,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub resume: Option<Resume>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Resume {
	pub client: String,
	pub seq: i64,
}

/// relay → client after `hello`: the snapshot of what this client should show now (B.6). On a `resume` inside the replay buffer the snapshot is empty and the missed messages follow.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Welcome {
	pub client: String,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub group: Option<String>,
	pub snapshot: Snapshot,
	/// relay time, ms
	pub now: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Ping {
	pub t0: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Pong {
	pub t0: i64,
	pub t1: i64,
}

/// conductor → relay on every change; folded into the snapshot.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct State {
	pub row: i64,
	pub slice: i64,
	pub vars: std::collections::HashMap<String, f64>,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub inventory: Option<Vec<String>>,
}

/// conductor → relay; the relay splits `actions` per outlet → `to`. Actions are loose until the show schema (B.5) lands.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Cue {
	pub id: String,
	pub actions: Vec<Loose>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VoteCast {
	pub vote: String,
	pub option: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Assign {
	pub client: String,
	pub group: String,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Panic {
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub black: Option<bool>,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub stop: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MediaPreload {
	pub outlet: String,
	pub urls: Vec<String>,
}

/// relay → conductor.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Presence {
	pub groups: std::collections::HashMap<String, i64>,
	pub ready: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Pickup {
	pub item: String,
	pub placement: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct React {
	pub kind: String,
}

/// One message for a device, resolved against the venue patch. `midi`: raw bytes (status + data). `osc`: address + typed args. `dmx`: channel values of the outlet's universe (a look resolves to one per fixture).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "p")]
pub enum DeviceMsg {
	#[serde(rename = "midi")]
	Midi(MidiBytes),
	#[serde(rename = "osc")]
	Osc(OscMessage),
	#[serde(rename = "dmx")]
	Dmx(DmxMessage),
}

impl DeviceMsg {
	/// The `kind` string on the wire.
	pub fn kind(&self) -> &'static str {
		match self {
			DeviceMsg::Midi(_) => "midi",
			DeviceMsg::Osc(_) => "osc",
			DeviceMsg::Dmx(_) => "dmx",
		}
	}
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MidiBytes {
	pub bytes: Vec<u8>,
}

/// One OSC argument, typed (JSON alone cannot tell 2 from 2.0). On the wire: int32, float32, string, bool. Blobs / timetags come when something needs them.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "p")]
pub enum OscArg {
	#[serde(rename = "int")]
	Int(i64),
	#[serde(rename = "float")]
	Float(f64),
	#[serde(rename = "string")]
	String(String),
	#[serde(rename = "bool")]
	Bool(bool),
}

impl OscArg {
	/// The `kind` string on the wire.
	pub fn kind(&self) -> &'static str {
		match self {
			OscArg::Int(_) => "int",
			OscArg::Float(_) => "float",
			OscArg::String(_) => "string",
			OscArg::Bool(_) => "bool",
		}
	}
}

/// An OSC message (in or out): the address pattern and its arguments. Bundles are flattened by the adapter.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OscMessage {
	/// `/like/this`
	pub address: String,
	pub args: Vec<OscArg>,
}

/// DMX out (#187): `values` from channel `ch` on (1-based; ch + values − 1 ≤ 512) in the outlet's universe; `fade` ms from the current values (missing / 0 = at once). Several ranges = several messages in `msgs`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DmxMessage {
	pub ch: u64,
	pub values: Vec<u8>,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub fade: Option<u64>,
}

/// relay → bridge: play `msgs` on `outlet` at `at`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Device {
	pub outlet: String,
	pub msgs: Vec<DeviceMsg>,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub at: Option<i64>,
}

/// bridge → conductor: MIDI / OSC in, mapped to actions there.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DeviceIn {
	pub outlet: String,
	pub msg: DeviceMsg,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Port {
	/// midi · osc · dmx-artnet · …
	pub adapter: String,
	pub name: String,
	pub direction: String,
}

/// bridge → relay: the ports the bridge sees, for the patch UI.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Devices {
	pub ports: Vec<Port>,
}

/// relay → client (not in B.6; the relay's refusals: bad hello, unknown kind, rate limit).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Error {
	pub code: String,
	pub message: String,
}

pub const PROTOCOL_VERSION: i64 = 1;
