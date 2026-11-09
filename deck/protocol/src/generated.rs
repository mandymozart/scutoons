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
	#[serde(rename = "vote.hold")]
	VoteHold(VoteHold),
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
	#[serde(rename = "media.ready")]
	MediaReady(MediaReady),
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
	#[serde(rename = "story")]
	Story(Loose),
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
	#[serde(rename = "deck.graph")]
	DeckGraph(DeckGraph),
	#[serde(rename = "deck.values")]
	DeckValues(DeckValues),
	#[serde(rename = "deck.midi")]
	DeckMidi(DeckMidi),
	#[serde(rename = "scene.sync")]
	SceneSync(SceneSync),
	#[serde(rename = "deck.synced")]
	DeckSynced(DeckSynced),
	#[serde(rename = "deck.quality")]
	DeckQuality(DeckQuality),
	#[serde(rename = "tempo")]
	Tempo(Tempo),
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
			Message::VoteHold(_) => "vote.hold",
			Message::VoteCast(_) => "vote.cast",
			Message::VoteTally(_) => "vote.tally",
			Message::VoteResult(_) => "vote.result",
			Message::Assign(_) => "assign",
			Message::Panic(_) => "panic",
			Message::MediaPreload(_) => "media.preload",
			Message::MediaReady(_) => "media.ready",
			Message::Snapshot(_) => "snapshot",
			Message::Cell(_) => "cell",
			Message::Say(_) => "say",
			Message::Line(_) => "line",
			Message::Fx(_) => "fx",
			Message::Colour(_) => "colour",
			Message::Story(_) => "story",
			Message::Presence(_) => "presence",
			Message::Ready(_) => "ready",
			Message::Pickup(_) => "pickup",
			Message::React(_) => "react",
			Message::Device(_) => "device",
			Message::DeviceIn(_) => "device.in",
			Message::Devices(_) => "devices",
			Message::DeckGraph(_) => "deck.graph",
			Message::DeckValues(_) => "deck.values",
			Message::DeckMidi(_) => "deck.midi",
			Message::SceneSync(_) => "scene.sync",
			Message::DeckSynced(_) => "deck.synced",
			Message::DeckQuality(_) => "deck.quality",
			Message::Tempo(_) => "tempo",
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
	/// a voter sees the tally once they have voted (never before)
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub show: Option<bool>,
	/// the choice event's question by locale (a phone shows its device's language, else the first); texts are locale maps
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub question: Option<std::collections::HashMap<String, String>>,
	/// `options` by locale, same order; `options` is the default
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub labels: Option<std::collections::HashMap<String, Vec<String>>>,
}

/// conductor → relay: close now; `override` = the conductor's option instead of the rule's.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VoteClose {
	pub vote: String,
	#[serde(rename = "override", default, skip_serializing_if = "Option::is_none")]
	pub r#override: Option<i64>,
}

/// conductor → relay: hold the vote – the countdown stops where it is, votes are still accepted; `hold: false` runs it on. The tally says `paused`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VoteHold {
	pub vote: String,
	pub hold: bool,
}

/// relay → conductor every 250 ms while a vote is open (→ the audience too when the vote shows it).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VoteTally {
	pub vote: String,
	pub counts: Vec<i64>,
	pub voters: i64,
	/// ms until the vote closes by itself (stands still while held)
	pub left: i64,
	/// the conductor holds the countdown
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub paused: Option<bool>,
	/// the counts per group (conductor only)
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub groups: Option<std::collections::HashMap<String, Vec<i64>>>,
	/// the connected voting devices; the vote closes when all have voted
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub expected: Option<i64>,
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
	/// the counts per group
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub groups: Option<std::collections::HashMap<String, Vec<i64>>>,
}

/// conductor → relay → the outlet's clients on a row change: the outlet's new cell (B.5.2 `Cell`, resolved by the conductor – a deck renders it natively, #186).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CellChange {
	pub outlet: String,
	pub cell: Cell,
}

/// What a client shows when it joins / reconnects / is moved (B.6): the last state, the current cell per outlet it sees, the open vote, the last instruction (say) per group it sees. The conductor sees every outlet and group.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Snapshot {
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub state: Option<State>,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub cells: Option<std::collections::HashMap<String, Cell>>,
	/// the last `media.preload` per outlet it sees – a reconnecting output resolves its cell's media ids with it
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub media: Option<std::collections::HashMap<String, MediaPreload>>,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub vote: Option<VoteOpen>,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub say: Option<std::collections::HashMap<String, Loose>>,
	/// #188: every deck's being by client id (the conductor's snapshot)
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub decks: Option<std::collections::HashMap<String, DeckState>>,
	/// #195: the last `scene.sync`, if it is addressed to this client (a deck joining later syncs too; the conductor always sees it)
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub sync: Option<SceneSync>,
	/// #195: each deck's last `deck.synced` for that sync, by client id (the conductor's snapshot)
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub synced: Option<std::collections::HashMap<String, DeckSynced>>,
	/// #321: the show clock's last anchor (every client but a bridge)
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub tempo: Option<Tempo>,
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
	/// the `key` of this client's `welcome`: the proof that it is the client it names
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub key: Option<String>,
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
	/// this client's secret for a later `resume`; no other client ever sees it
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub key: Option<String>,
}

/// client → relay. `t0`: the client's own clock when it sent the ping, in a unit of its choosing (the relay only echoes it; a deck sends µs – #321, the show clock's offset filter).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Ping {
	pub t0: i64,
}

/// relay → the client: its `t0` back and the relay's time (ms) when the ping arrived – a sample of the relay's clock against the client's (deck-core `tempo::sync`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Pong {
	pub t0: i64,
	pub t1: i64,
}

/// #321 (plan §3.7): the show clock's anchor – at relay ms `at` the show was at `beat`, running at `bpm` (or stopped: the beat stands), `per_bar` beats a bar. The leading deck (its picker, its scene's `lead`, or a deck on MIDI clock in – the venue's table-top clock) sends it only when its tempo line changed; the relay fills a missing `at` with now + the lead, keeps the last one for the snapshot and routes it to everyone else. The conductor may always lead; a deck leads while no other connected client does. Every deck on the show clock maps the anchor through its offset filter and follows the line.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Tempo {
	pub bpm: f64,
	pub per_bar: u64,
	pub beat: f64,
	pub running: bool,
	/// relay ms; the envelope's `at` when missing
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub at: Option<i64>,
	/// the clock domain (a sub-conductor's own time, later); missing = "show", the one show clock – reserved, nothing sets or routes it yet
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub domain: Option<String>,
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

/// conductor → relay → the outlet's clients before doors (LIVE.md B.5.6, #186): fetch and cache these, then answer `media.ready`. The relay keeps the last one per outlet for the snapshot.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MediaPreload {
	pub outlet: String,
	pub items: Vec<MediaItem>,
}

/// #193: a set's spot carries rendered variants – a still, a rendered video, a 3D model; the Live view picks one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SpotMode {
	#[serde(rename = "still")]
	Still,
	#[serde(rename = "video")]
	Video,
	#[serde(rename = "model3d")]
	Model3d,
}

impl SpotMode {
	pub fn as_str(self) -> &'static str {
		match self {
			SpotMode::Still => "still",
			SpotMode::Video => "video",
			SpotMode::Model3d => "model3d",
		}
	}
}

/// One medium of a preload: `id` (what layers name as `media`) and either a `url` (show media under live/<toon>/ on the CDN, fetched with the deck token) or a set reference `set` + `spot` + `mode` (#193 – the deck resolves it: the API by id once the set model exists, today a local path / URL fallback).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MediaItem {
	pub id: String,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub url: Option<String>,
	/// a set id (#193: one staging of a place, e.g. `cathedral`)
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub set: Option<String>,
	/// a named spot of the set (missing = the set's default view)
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub spot: Option<String>,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub mode: Option<SpotMode>,
	/// the size, if known
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub bytes: Option<u64>,
}

/// an outlet's client → relay → conductor: what of the outlet's preload is cached (`ready`) and what is not (`missing`, the reason per id in `why`) – the conductor's ready light. Sent after each preload and again while it retries.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MediaReady {
	pub outlet: String,
	pub ready: Vec<String>,
	pub missing: Vec<String>,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub why: Option<std::collections::HashMap<String, String>>,
}

/// B.5.2: own = the cell's layers; mirror = what outlet `of` shows; black = black (a device outlet: all off).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CellMode {
	#[serde(rename = "own")]
	Own,
	#[serde(rename = "mirror")]
	Mirror,
	#[serde(rename = "black")]
	Black,
}

impl CellMode {
	pub fn as_str(self) -> &'static str {
		match self {
			CellMode::Own => "own",
			CellMode::Mirror => "mirror",
			CellMode::Black => "black",
		}
	}
}

/// B.5.2 `Cell` on the wire: what one outlet shows for the current row, resolved by the conductor (toon picks as toon + chapter + slice index, live media by preload id, sets by set + spot + mode). `layers` back → front.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Cell {
	pub mode: CellMode,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub layers: Option<Vec<Layer>>,
	/// mirror: the outlet whose cell this one shows
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub of: Option<String>,
	/// a device outlet: the messages on row enter
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub send: Option<Vec<DeviceMsg>>,
}

/// B.5.2 `Layer`: a source in a box of the outlet. `enter` / `exit` fade it in / out when the cell changes (missing = the outlet's default crossfade); a layer with the same id and source in both cells stays.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Layer {
	pub id: String,
	pub src: LayerSrc,
	#[serde(rename = "box", default, skip_serializing_if = "Option::is_none")]
	pub r#box: Option<LayerBox>,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub fit: Option<Fit>,
	/// 0–1 (default 1)
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub opacity: Option<f64>,
	#[serde(rename = "loop", default, skip_serializing_if = "Option::is_none")]
	pub r#loop: Option<bool>,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub enter: Option<Transition>,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub exit: Option<Transition>,
}

/// A box in % of the outlet (a panel's crop: of the slice); x, y = the top left corner. A layer without one fills the outlet.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LayerBox {
	pub x: f64,
	pub y: f64,
	pub w: f64,
	pub h: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Fit {
	#[serde(rename = "cover")]
	Cover,
	#[serde(rename = "contain")]
	Contain,
}

impl Fit {
	pub fn as_str(self) -> &'static str {
		match self {
			Fit::Cover => "cover",
			Fit::Contain => "contain",
		}
	}
}

/// A layer's fade when the cell changes, `ms` long (0 = a cut).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Transition {
	pub ms: u64,
}

/// What a layer shows (B.5.2 ToonSrc / LiveSrc, resolved). From the toon (the public library): scroll, slice, panel, surtitles. Live: video / image / model3d (a preloaded medium by id), set (a set's spot in a mode, #193), text, colour, camera.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "p")]
pub enum LayerSrc {
	#[serde(rename = "scroll")]
	Scroll(ToonChapter),
	#[serde(rename = "slice")]
	Slice(ToonSlice),
	#[serde(rename = "panel")]
	Panel(ToonPanel),
	#[serde(rename = "surtitles")]
	Surtitles(Surtitles),
	#[serde(rename = "video")]
	Video(MediaSrc),
	#[serde(rename = "image")]
	Image(MediaSrc),
	#[serde(rename = "model3d")]
	Model3d(MediaSrc),
	#[serde(rename = "set")]
	Set(SetSrc),
	#[serde(rename = "text")]
	Text(TextSrc),
	#[serde(rename = "colour")]
	Colour(ColourSrc),
	#[serde(rename = "camera")]
	Camera(CameraSrc),
}

impl LayerSrc {
	/// The `kind` string on the wire.
	pub fn kind(&self) -> &'static str {
		match self {
			LayerSrc::Scroll(_) => "scroll",
			LayerSrc::Slice(_) => "slice",
			LayerSrc::Panel(_) => "panel",
			LayerSrc::Surtitles(_) => "surtitles",
			LayerSrc::Video(_) => "video",
			LayerSrc::Image(_) => "image",
			LayerSrc::Model3d(_) => "model3d",
			LayerSrc::Set(_) => "set",
			LayerSrc::Text(_) => "text",
			LayerSrc::Colour(_) => "colour",
			LayerSrc::Camera(_) => "camera",
		}
	}
}

/// scroll: the chapter's slices running one after the other.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToonChapter {
	pub toon: String,
	pub chapter: u64,
}

/// slice: one slice; `slice` = its index in the chapter's reading order (the slices with an image, 0-based – the conductor resolves the show's pick to it).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToonSlice {
	pub toon: String,
	pub chapter: u64,
	pub slice: u64,
}

/// panel: a crop of one slice (`crop` in % of the slice).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToonPanel {
	pub toon: String,
	pub chapter: u64,
	pub slice: u64,
	pub crop: LayerBox,
}

/// surtitles: the dialog of the slice the show is on (`state.slice`, an index like ToonSlice's) in `locale`; `of` = one character's lines only.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Surtitles {
	pub toon: String,
	pub chapter: u64,
	pub locale: String,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub of: Option<String>,
}

/// a medium of the outlet's preload, by its id
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MediaSrc {
	pub media: String,
}

/// #193: a set's spot in a mode – a use names set + spot, never a file. still / video = the spot's rendered variant as a picture; model3d = the set as the deck's 3D space, the camera at the spot.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SetSrc {
	pub set: String,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub spot: Option<String>,
	pub mode: SpotMode,
}

/// text: a locale map (the deck shows its locale, else the first); `size` = the line height in % of the outlet height (default 6).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TextSrc {
	pub text: std::collections::HashMap<String, String>,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub size: Option<f64>,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub colour: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ColourSrc {
	/// #rrggbb or #rrggbbaa
	pub colour: String,
}

/// camera: a video input on the output machine, by name (the deck's video-in – not built yet).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CameraSrc {
	pub input: String,
}

/// relay → conductor: clients per group, who is ready; the decks that stream a being (#188 – a deck that left is gone from `decks`) and who sends values to whom (`routes`); each client's link (`links`, #223 – sent again when a client's rate changes); each deck's render quality (`quality`, #220 – sent again when a deck reports a change).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Presence {
	pub groups: std::collections::HashMap<String, i64>,
	pub ready: Vec<String>,
	/// client ids with a `deck.graph`, sorted
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub decks: Option<Vec<String>>,
	/// every sender → receiver pair `deck.values` went along, sorted
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub routes: Option<Vec<Route>>,
	/// #223: each client's link as the relay measures it, by client id (audience phones left out)
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub links: Option<std::collections::HashMap<String, LinkQuality>>,
	/// #220: each deck's last `deck.quality`, by client id (a client that left is gone)
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub quality: Option<std::collections::HashMap<String, DeckQuality>>,
}

/// #223: one client's link: the smoothed WebSocket ping round trip, the rate the relay sends it state-like messages at (`deck.values`, `presence`, `vote.tally`, `state` – 30 = as fast as they come, down to 5 on a link that can't carry more), and how many of them it replaced before they went out (a stalled client gets the newest, not a backlog).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LinkQuality {
	/// ms; missing before the first pong
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub rtt: Option<u64>,
	pub hz: u64,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub skipped: Option<u64>,
}

/// #188: a deck's values flow from client `from` to client `to` (the conductor's scene draws a hose between their beings).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Route {
	pub from: String,
	pub to: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DeckNode {
	pub id: String,
	/// a deck-native node kind (`fm`, `lfo`, `space` …)
	pub kind: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DeckLink {
	/// `node.port` (an output)
	pub from: String,
	/// `node.port` (an input)
	pub to: String,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub scale: Option<f64>,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub offset: Option<f64>,
}

/// #188 phase 4: a deck → relay → the conductor, on joining and when its scene changes: its being – the stage graph's nodes and links (the body), the head's character, the body's size (m), the moods calm → wild, the exposed parameters. The relay keeps the last one per client for the conductor's snapshot.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DeckGraph {
	pub nodes: Vec<DeckNode>,
	pub links: Vec<DeckLink>,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub character: Option<String>,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub size: Option<f64>,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub moods: Option<Vec<String>>,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub expose: Option<Vec<String>>,
}

/// #188 phase 4 (the stream #189's sends build on): a deck → relay → the conductor (and `to`, if set), only what changed, at most 30 / s (the relay drops the rest). `values`: a path – `node.port` (a control input, a control output, an audio output's block peak) or `node.shape` (an LFO's waveform) – → a float vector (1 … n components; one today). `mood`: the head's mood when it changed. Not kept for replay: the relay folds them into the snapshot's `decks`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DeckValues {
	pub values: std::collections::HashMap<String, Vec<f64>>,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub mood: Option<String>,
}

/// #211: a deck → relay → the conductor (and `to`, if set): MIDI its parameters send as outputs (a mapping's target `port = "conductor"` – CC / note / pitch bend, status byte first), for the conductor to route on like device input. Only decks send it; not kept for replay (a value stream, not state).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DeckMidi {
	pub msgs: Vec<MidiBytes>,
}

/// #195: conductor → relay → the decks (the group `decks`, or only the client ids in `decks`): fetch what this scene needs into the cache – the same scene-content request (`GET /api/library/scene-content`) and cache as a deck's own scene, skipping what is there – and answer `deck.synced`. Only the conductor sends it; the relay logs it and keeps the last one for the snapshot, so a deck that joins later syncs too.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SceneSync {
	/// the scene / show id the sync is for (`deck.synced` names it back)
	pub scene: String,
	/// set ids – each deck fetches the set's 3D at its own quality
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub sets: Option<Vec<String>>,
	/// character ids – every portrait the API lists
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub characters: Option<Vec<String>>,
	/// #191: lent beings – reserved, no deck reads it yet
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub beings: Option<Vec<String>>,
	/// only these client ids (missing = every deck)
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub decks: Option<Vec<String>>,
}

/// #195: syncing = files still to settle; ready = every item is in the cache; missing = settled, but some item is not there (`why`); offline = the API can't be reached and some item is not in the cache.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SyncState {
	#[serde(rename = "syncing")]
	Syncing,
	#[serde(rename = "ready")]
	Ready,
	#[serde(rename = "missing")]
	Missing,
	#[serde(rename = "offline")]
	Offline,
}

impl SyncState {
	pub fn as_str(self) -> &'static str {
		match self {
			SyncState::Syncing => "syncing",
			SyncState::Ready => "ready",
			SyncState::Missing => "missing",
			SyncState::Offline => "offline",
		}
	}
}

/// #220: a deck's render quality level, cheapest first – the hardware tiers S / M / L / XL (deck/HARDWARE.md).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum QualityLevel {
	#[serde(rename = "low")]
	Low,
	#[serde(rename = "medium")]
	Medium,
	#[serde(rename = "high")]
	High,
	#[serde(rename = "ultra")]
	Ultra,
}

impl QualityLevel {
	pub fn as_str(self) -> &'static str {
		match self {
			QualityLevel::Low => "low",
			QualityLevel::Medium => "medium",
			QualityLevel::High => "high",
			QualityLevel::Ultra => "ultra",
		}
	}
}

/// #220: a deck → relay, when its quality changes (at once) and its frame time moved (at most every 2 s): the level it renders at now, the render scale (× the screen; > 1 supersamples), whether the governor steps it (`auto`) and the mean frame interval (ms). The relay keeps the last one per client and hands it to the conductor in presence (`quality`), next to the link.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DeckQuality {
	pub level: QualityLevel,
	pub scale: f64,
	pub auto: bool,
	/// ms, the mean of the last half second
	pub frame: f64,
	/// the level chosen (the governor steps at most up to it); missing = `level`
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub ceiling: Option<QualityLevel>,
}

/// #195: a deck → relay → the conductor, while it works on a `scene.sync` (progress) and when it is done – like `media.ready`. Items are `set:<id>` and `character:<id>`; one in neither list is still syncing. The relay keeps each deck's last one for the conductor's snapshot (`synced`) and marks a deck ready in presence once its state is ready.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DeckSynced {
	pub scene: String,
	pub state: SyncState,
	/// files settled (fetched, cached or failed)
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub done: Option<u64>,
	/// files the items need
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub total: Option<u64>,
	/// bytes downloaded for this sync
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub bytes: Option<u64>,
	pub ready: Vec<String>,
	pub missing: Vec<String>,
	/// the reason per missing item
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub why: Option<std::collections::HashMap<String, String>>,
}

/// A deck's being in the conductor's snapshot: its graph and its latest values (all of them, folded).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DeckState {
	pub graph: DeckGraph,
	pub values: std::collections::HashMap<String, Vec<f64>>,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub mood: Option<String>,
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
