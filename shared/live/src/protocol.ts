// GENERATED from deck/protocol/schema.json by deck/protocol/generate.ts – edit the schema, run `npm run generate -w shared/live`.
// agents/research/LIVE.md B.6. One envelope per message, JSON over one WebSocket per client. The generator (deck/protocol/generate.ts) reads only the subset used here: object / string / integer / number / boolean / array / $ref / enum / const / oneOf discriminated by `kind` / additionalProperties maps. Edit this file, run `npm run generate -w shared/live`, commit both outputs.
/** LIVE.md B.2. `uplink` is the local relay connected to the hosted one (B.6 Uplink). `reader` (#90, SHOW.md §7) is a phone reading the chapter: all it sends is its `story` events, to the conductor. `seat` (#90, SHOW.md §4) is a child conductor's relay in its parent's session, its group is the seat's id. `handheld` (#469, research/HANDHELD-CAMERA.md) is a phone as a handheld camera: it joins with the token the API hands it for one pairing code, all it sends is its `hand.*` messages, to the conductor (the storyboard editor), and it hears `hand.view`. */
export type Role = 'conductor' | 'output' | 'bridge' | 'performer' | 'musician' | 'audience' | 'uplink' | 'reader' | 'seat' | 'handheld';

/** Who a message reaches. Every field narrows; all missing = everyone the sender may reach. */
export interface To {
	role?: Role;
	/** a group id (B.5.3) */
	group?: string;
	/** one client id (assigned in `welcome`) */
	client?: string;
}

/** B.6: `{ v: 1, seq, at?, from, to?, kind, p }`. `seq` is per session, assigned by the relay; `at` = relay time (ms) when the effect should happen. */
export interface EnvelopeBase {
	v: 1;
	/** 0 before the relay stamped it (client → relay) */
	seq: number;
	/** relay time in ms when the effect should happen */
	at?: number;
	/** client id; `relay` for the relay's own messages */
	from: string;
	to?: To;
}
export type Envelope = EnvelopeBase & Message;

/** `kind` + its payload `p`. One entry per B.6 message; a loose payload (`Loose`) marks a kind whose shape is not fixed yet. */
export type Message =
	| { kind: 'hello'; p: Hello }
	| { kind: 'welcome'; p: Welcome }
	| { kind: 'ping'; p: Ping }
	| { kind: 'pong'; p: Pong }
	| { kind: 'state'; p: State }
	| { kind: 'cue'; p: Cue }
	| { kind: 'vote.open'; p: VoteOpen }
	| { kind: 'vote.close'; p: VoteClose }
	| { kind: 'vote.hold'; p: VoteHold }
	| { kind: 'vote.cast'; p: VoteCast }
	| { kind: 'vote.tally'; p: VoteTally }
	| { kind: 'vote.result'; p: VoteResult }
	| { kind: 'assign'; p: Assign }
	| { kind: 'panic'; p: Panic }
	| { kind: 'media.preload'; p: MediaPreload }
	| { kind: 'media.ready'; p: MediaReady }
	| { kind: 'snapshot'; p: Snapshot }
	| { kind: 'cell'; p: CellChange }
	| { kind: 'say'; p: Loose }
	| { kind: 'line'; p: Loose }
	| { kind: 'fx'; p: Loose }
	| { kind: 'colour'; p: Loose }
	| { kind: 'story'; p: Loose }
	| { kind: 'presence'; p: Presence }
	| { kind: 'ready'; p: Empty }
	| { kind: 'pickup'; p: Pickup }
	| { kind: 'react'; p: React }
	| { kind: 'device'; p: Device }
	| { kind: 'device.in'; p: DeviceIn }
	| { kind: 'devices'; p: Devices }
	| { kind: 'deck.graph'; p: DeckGraph }
	| { kind: 'deck.values'; p: DeckValues }
	| { kind: 'deck.midi'; p: DeckMidi }
	| { kind: 'deck.listen'; p: DeckListen }
	| { kind: 'deck.events'; p: DeckEvents }
	| { kind: 'scene.sync'; p: SceneSync }
	| { kind: 'deck.synced'; p: DeckSynced }
	| { kind: 'deck.quality'; p: DeckQuality }
	| { kind: 'tempo'; p: Tempo }
	| { kind: 'kick'; p: Kick }
	| { kind: 'rename'; p: Rename }
	| { kind: 'seat'; p: SeatWord }
	| { kind: 'seat.state'; p: SeatState }
	| { kind: 'flash'; p: Flash }
	| { kind: 'flash.report'; p: FlashReport }
	| { kind: 'address'; p: Address }
	| { kind: 'address.answer'; p: AddressAnswer }
	| { kind: 'hand.pose'; p: HandPose }
	| { kind: 'hand.state'; p: HandState }
	| { kind: 'hand.record'; p: HandRecord }
	| { kind: 'hand.recenter'; p: Empty }
	| { kind: 'hand.view'; p: HandView }
	| { kind: 'error'; p: Error };

/** A payload whose shape B.6 names but this skeleton does not fix yet (say / line / fx / colour, cue actions). */
export type Loose = Record<string, unknown>;

/** B.11 (3): majority = most votes; groups = each group's majority is one vote; first = the first option to reach `n`. */
export type VoteRule = 'majority' | 'groups' | 'first';

/** conductor → relay → the voting groups. `options` are the choice event's labels (index = option). The relay tallies, closes at `seconds` by itself and sends `vote.result`. */
export interface VoteOpen {
	vote: string;
	options: string[];
	rule: VoteRule;
	/** rule first: the votes an option needs */
	n?: number;
	seconds: number;
	/** the option when nobody voted (default 0) */
	fallback?: number;
	/** which groups vote (default: all audience) */
	groups?: string[];
	/** a voter sees the tally once they have voted (never before) */
	show?: boolean;
	/** the choice event's question by locale (a phone shows its device's language, else the first); texts are locale maps */
	question?: Record<string, string>;
	/** `options` by locale, same order; `options` is the default */
	labels?: Record<string, string[]>;
}

/** conductor → relay: close now; `override` = the conductor's option instead of the rule's. */
export interface VoteClose {
	vote: string;
	override?: number;
}

/** conductor → relay: hold the vote – the countdown stops where it is, votes are still accepted; `hold: false` runs it on. The tally says `paused`. */
export interface VoteHold {
	vote: string;
	hold: boolean;
}

/** relay → conductor every 250 ms while a vote is open (→ the audience too when the vote shows it). */
export interface VoteTally {
	vote: string;
	counts: number[];
	voters: number;
	/** ms until the vote closes by itself (stands still while held) */
	left: number;
	/** the conductor holds the countdown */
	paused?: boolean;
	/** the counts per group (conductor only) */
	groups?: Record<string, number[]>;
	/** the connected voting devices; the vote closes when all have voted */
	expected?: number;
}

export type VoteHow = 'rule' | 'fallback' | 'override';

/** relay → everyone: the option the conductor applies as the choice event (B.1). */
export interface VoteResult {
	vote: string;
	option: number;
	counts: number[];
	how: VoteHow;
	/** the counts per group */
	groups?: Record<string, number[]>;
}

/** conductor → relay → the outlet's clients on a row change: the outlet's new cell (B.5.2 `Cell`, resolved by the conductor – a deck renders it natively, #186). */
export interface CellChange {
	outlet: string;
	cell: Cell;
}

/** What a client shows when it joins / reconnects / is moved (B.6): the last state, the current cell per outlet it sees, the open vote, the last instruction (say) per group it sees. The conductor sees every outlet and group. */
export interface Snapshot {
	state?: State;
	cells?: Record<string, Cell>;
	/** the last `media.preload` per outlet it sees – a reconnecting output resolves its cell's media ids with it */
	media?: Record<string, MediaPreload>;
	vote?: VoteOpen;
	say?: Record<string, Loose>;
	/** #188: every deck's being by client id (the conductor's snapshot) */
	decks?: Record<string, DeckState>;
	/** #195: the last `scene.sync`, if it is addressed to this client (a deck joining later syncs too; the conductor always sees it) */
	sync?: SceneSync;
	/** #195: each deck's last `deck.synced` for that sync, by client id (the conductor's snapshot) */
	synced?: Record<string, DeckSynced>;
	/** #321: the show clock's last anchor (every client but a bridge) */
	tempo?: Tempo;
	/** #90: the parent's last word to this subtree (a conductor), or to this seat (a child relay) */
	seat?: SeatWord;
	/** #90: each seat's readout (the conductor's snapshot) */
	seats?: Record<string, SeatState>;
	/** #90: the last flash */
	flash?: Flash;
	/** #471: each deck's and sub conductor's last flash.report, by its path (the conductor's snapshot): the flash list of a conductor that joins later */
	reports?: Record<string, FlashReport>;
	/** the session's seq when the snapshot was taken */
	seq?: number;
}

export type Empty = Record<string, never>;

/** client → relay, the first message. Audience / performer / musician / output join with the session code (+ group from their QR); conductor and bridge with a session token. */
export interface Hello {
	code: string;
	role: Role;
	group?: string;
	/** trimmed to 24 characters by the relay */
	nick?: string;
	/** HMAC session token (conductor, bridge, uplink) */
	token?: string;
	resume?: Resume;
}

export interface Resume {
	client: string;
	seq: number;
	/** the `key` of this client's `welcome`: the proof that it is the client it names */
	key?: string;
}

/** relay → client after `hello`: the snapshot of what this client should show now (B.6). On a `resume` inside the replay buffer the snapshot is empty and the missed messages follow. */
export interface Welcome {
	client: string;
	group?: string;
	snapshot: Snapshot;
	/** relay time, ms */
	now: number;
	/** this client's secret for a later `resume`; no other client ever sees it */
	key?: string;
}

/** client → relay. `t0`: the client's own clock when it sent the ping, in a unit of its choosing (the relay only echoes it; a deck sends µs – #321, the show clock's offset filter). */
export interface Ping {
	t0: number;
}

/** relay → the client: its `t0` back and the relay's time (ms) when the ping arrived – a sample of the relay's clock against the client's (deck-core `tempo::sync`). */
export interface Pong {
	t0: number;
	t1: number;
}

/** #321 (plan §3.7): the show clock's anchor – at relay ms `at` the show was at `beat`, running at `bpm` (or stopped: the beat stands), `per_bar` beats a bar. The leading deck (its picker, its scene's `lead`, or a deck on MIDI clock in – the venue's table-top clock) sends it only when its tempo line changed; the relay fills a missing `at` with now + the lead, keeps the last one for the snapshot and routes it to everyone else. The conductor may always lead; a deck leads while no other connected client does. Every deck on the show clock maps the anchor through its offset filter and follows the line. */
export interface Tempo {
	bpm: number;
	per_bar: number;
	beat: number;
	running: boolean;
	/** relay ms; the envelope's `at` when missing */
	at?: number;
	/** #90: the clock domain, a sub conductor's own time; missing = "show", the top conductor's clock. A domain stays inside its subtree: each subtree is a session of its own and a parent's anchor only reaches the child's conductor, which takes it or keeps its own */
	domain?: string;
}

/** conductor → relay on every change; folded into the snapshot. */
export interface State {
	row: number;
	slice: number;
	vars: Record<string, number>;
	inventory?: string[];
}

/** conductor → relay; the relay splits `actions` per outlet → `to`. Actions are loose until the show schema (B.5) lands. */
export interface Cue {
	id: string;
	actions: Loose[];
}

export interface VoteCast {
	vote: string;
	option: number;
}

export interface Assign {
	client: string;
	group: string;
}

export interface Panic {
	black?: boolean;
	stop?: boolean;
}

/** conductor → relay → the outlet's clients before doors (LIVE.md B.5.6, #186): fetch and cache these, then answer `media.ready`. The relay keeps the last one per outlet for the snapshot. */
export interface MediaPreload {
	outlet: string;
	items: MediaItem[];
}

/** #193: a set's spot carries rendered variants – a still, a rendered video, a 3D model; the Live view picks one. */
export type SpotMode = 'still' | 'video' | 'model3d';

/** One medium of a preload: `id` (what layers name as `media`) and either a `url` (show media under live/<toon>/ on the CDN, fetched with the deck token) or a set reference `set` + `spot` + `mode` (#193 – the deck resolves it: the API by id once the set model exists, today a local path / URL fallback). */
export interface MediaItem {
	id: string;
	url?: string;
	/** a set id (#193: one staging of a place, e.g. `cathedral`) */
	set?: string;
	/** a named spot of the set (missing = the set's default view) */
	spot?: string;
	mode?: SpotMode;
	/** the size, if known */
	bytes?: number;
}

/** an outlet's client → relay → conductor: what of the outlet's preload is cached (`ready`) and what is not (`missing`, the reason per id in `why`) – the conductor's ready light. Sent after each preload and again while it retries. */
export interface MediaReady {
	outlet: string;
	ready: string[];
	missing: string[];
	why?: Record<string, string>;
}

/** B.5.2: own = the cell's layers; mirror = what outlet `of` shows; black = black (a device outlet: all off). */
export type CellMode = 'own' | 'mirror' | 'black';

/** B.5.2 `Cell` on the wire: what one outlet shows for the current row, resolved by the conductor (toon picks as toon + chapter + slice index, live media by preload id, sets by set + spot + mode). `layers` back → front. */
export interface Cell {
	mode: CellMode;
	layers?: Layer[];
	/** mirror: the outlet whose cell this one shows */
	of?: string;
	/** a device outlet: the messages on row enter */
	send?: DeviceMsg[];
}

/** B.5.2 `Layer`: a source in a box of the outlet. `enter` / `exit` fade it in / out when the cell changes (missing = the outlet's default crossfade); a layer with the same id and source in both cells stays. */
export interface Layer {
	id: string;
	src: LayerSrc;
	box?: LayerBox;
	fit?: Fit;
	/** 0–1 (default 1) */
	opacity?: number;
	loop?: boolean;
	enter?: Transition;
	exit?: Transition;
}

/** A box in % of the outlet (a panel's crop: of the slice); x, y = the top left corner. A layer without one fills the outlet. */
export interface LayerBox {
	x: number;
	y: number;
	w: number;
	h: number;
}

export type Fit = 'cover' | 'contain';

/** A layer's fade when the cell changes, `ms` long (0 = a cut). */
export interface Transition {
	ms: number;
}

/** What a layer shows (B.5.2 ToonSrc / LiveSrc, resolved). From the toon (the public library): scroll, slice, panel, surtitles. Live: video / image / model3d (a preloaded medium by id), set (a set's spot in a mode, #193), text, colour, camera. A show's cell: piece (#89). */
export type LayerSrc =
	| { kind: 'scroll'; p: ToonChapter }
	| { kind: 'slice'; p: ToonSlice }
	| { kind: 'panel'; p: ToonPanel }
	| { kind: 'surtitles'; p: Surtitles }
	| { kind: 'video'; p: MediaSrc }
	| { kind: 'image'; p: MediaSrc }
	| { kind: 'model3d'; p: MediaSrc }
	| { kind: 'set'; p: SetSrc }
	| { kind: 'text'; p: TextSrc }
	| { kind: 'colour'; p: ColourSrc }
	| { kind: 'camera'; p: CameraSrc }
	| { kind: 'piece'; p: PieceSrc };

/** SHOW.md §5, a cell is a piece plus values: the piece (a place, `scene:nave`, `rig:storm`, `character:yellowmouse`) and all its values, resolved by the conductor along the path really taken (keep, values). The deck compares: same piece, the values glide; other piece, load and crossfade. */
export interface PieceSrc {
	piece: string;
	values?: Loose;
}

/** scroll: the chapter's slices running one after the other. */
export interface ToonChapter {
	toon: string;
	chapter: number;
}

/** slice: one slice; `slice` = its index in the chapter's reading order (the slices with an image, 0-based – the conductor resolves the show's pick to it). */
export interface ToonSlice {
	toon: string;
	chapter: number;
	slice: number;
}

/** panel: a crop of one slice (`crop` in % of the slice). */
export interface ToonPanel {
	toon: string;
	chapter: number;
	slice: number;
	crop: LayerBox;
}

/** surtitles: the dialog of the slice the show is on (`state.slice`, an index like ToonSlice's) in `locale`; `of` = one character's lines only. */
export interface Surtitles {
	toon: string;
	chapter: number;
	locale: string;
	of?: string;
}

/** a medium of the outlet's preload, by its id */
export interface MediaSrc {
	media: string;
}

/** #193: a set's spot in a mode – a use names set + spot, never a file. still / video = the spot's rendered variant as a picture; model3d = the set as the deck's 3D space, the camera at the spot. */
export interface SetSrc {
	set: string;
	spot?: string;
	mode: SpotMode;
}

/** text: a locale map (the deck shows its locale, else the first); `size` = the line height in % of the outlet height (default 6). */
export interface TextSrc {
	text: Record<string, string>;
	size?: number;
	colour?: string;
}

export interface ColourSrc {
	/** #rrggbb or #rrggbbaa */
	colour: string;
}

/** camera: a video input on the output machine, by name (the deck's video-in – not built yet). */
export interface CameraSrc {
	input: string;
}

/** B.6: conductor → relay: the client is told (`error` code `kicked`) and its link ends; it joins again only with a new `hello`, which the conductor can answer the same way. */
export interface Kick {
	client: string;
}

/** B.6: conductor → relay → that client. The relay cleans the nick (24 characters, the blocklist); empty = none. */
export interface Rename {
	client: string;
	nick: string;
}

/** #90, SHOW.md §4b: how a child takes its parent's rows, a child treats its parent like a reader. follow = the row fires, suggest = its operator decides, free = ignored. The local GO always works. */
export type Word = 'follow' | 'suggest' | 'free';

/** #90: what a parent's row says to its seat: `go` to a row of the child's, or `free`, it is left to itself; `none` only carries the word and the lock. */
export type Says = 'go' | 'free' | 'none';

/** #90: conductor → its relay → the child relay in that seat (`to`: role seat, group the seat's id) → the child's conductor. The last one per seat is kept: a child conductor that joins later finds it in its snapshot. */
export interface SeatWord {
	seat: string;
	word: Word;
	says: Says;
	/** with `says: go`: the child's row */
	row?: string;
	/** a lock from above: the parent's word wins over the child's own (SHOW.md §4b); only ever sent for a seat that may be locked */
	lock?: boolean;
}

/** #90: a child conductor's readout, up to its parent's relay, which keeps it per seat for the parent's presence. */
export interface SeatState {
	show?: Pinned;
	/** the row that fired last */
	row?: string;
	/** clients ready in its subtree */
	ready?: number;
	/** clients connected in its subtree */
	clients?: number;
	/** the word in force */
	word?: Word;
	locked?: boolean;
	/** the clock domain it runs on; missing = the parent's */
	clock?: string;
	/** set by the parent's relay: false = the seat's link went away */
	online?: boolean;
}

/** #90: a show at an exact version. */
export interface Pinned {
	id: string;
	version: number;
}

/** #90, SHOW.md §4b: one assignment, the tree resolved to exact versions. Goes down the tree seat by seat. A lock from above wins, otherwise the nearest wins, a deck may refuse; every deck reports what it runs. */
export interface Flash {
	root: Pinned;
	/** the root first, each show once */
	shows: Pinned[];
	/** a lock from above */
	lock?: boolean;
	/** how many conductors it came through; 0 = from this conductor's own operator */
	hops: number;
	/** #91: the show of the conductor whose session it is in now, at the version the flash pins; a deck takes its outlet's cells of that one */
	here?: Pinned;
	/** #471: this flash's identity; a deck takes one flash once (a rejoin hands it the same one again) */
	id?: string;
	/** #471: who flashed, for the deck's notice */
	by?: string;
	/** #471: the paths the operator left out, relative to the session it is in now (a client id; `<show>/…` for a sub conductor's session, the show its seat sits). A deck not named here takes it, also one that joins later */
	skip?: string[];
	/** #471: the paths the prompt warned of and the operator accepted (asked not to be flashed, unsaved configuration): flashed anyway */
	force?: string[];
}

/** #90: what a deck or conductor runs after a flash, to its conductor and on up. `from` of the envelope is the path it came along. */
export interface FlashReport {
	running: Pinned;
	accepted: boolean;
	/** why it refused */
	why?: string;
	/** #91: a deck's outlet (none: a conductor) */
	outlet?: string;
	/** #91: a deck has everything its outlet's cells name (the show and each piece); never blocks GO */
	ready?: boolean;
	/** #91: what it lacks, in words (deck-show's), one line each */
	lacks?: string[];
	/** #471: the deck asks not to be flashed, and why (a request, not a lock) */
	keep?: string;
	/** #471: the deck has configuration a flash would replace */
	unsaved?: boolean;
	/** #471: how many local mappings the deck has (they stay through a flash) */
	mappings?: number;
	/** #471: native, browser, phone … (the flash list's filter) */
	device?: string;
	/** #471: the deck's own version, kept when a flash replaced it */
	kept?: Pinned;
	/** #471: the deck runs its own kept version, not the flashed one */
	own?: boolean;
}

/** #90, SHOW.md §9, for the workshop group and device. The receiver tests itself; a missing fact fails the filter. */
export interface AddressFilter {
	group?: string;
	/** browser, native, phone … */
	device?: string;
}

/** #90, SHOW.md §8: `kind:id > action`, down the tree. Whoever plays it picks up; finding nobody stays silent and is logged, it never goes up to a wider address. */
export interface Address {
	/** the sender's, the answers carry it */
	id: string;
	address: string;
	action: string;
	where?: AddressFilter;
}

/** #90: one receiver's count, up to the conductor that asked: it picked up (`reached`), passed the filter (`passed`), a handler took it (`answered`). A receiver the address does not name sends nothing. */
export interface AddressAnswer {
	id: string;
	reached: number;
	passed: number;
	answered: number;
}

/** relay → conductor: clients per group, who is ready; the decks that stream a being (#188 – a deck that left is gone from `decks`) and who sends values to whom (`routes`); each client's link (`links`, #223 – sent again when a client's rate changes); each deck's render quality (`quality`, #220 – sent again when a deck reports a change). */
export interface Presence {
	groups: Record<string, number>;
	ready: string[];
	/** client ids with a `deck.graph`, sorted */
	decks?: string[];
	/** every sender → receiver pair `deck.values` went along, sorted */
	routes?: Route[];
	/** #223: each client's link as the relay measures it, by client id (audience phones left out) */
	links?: Record<string, LinkQuality>;
	/** #220: each deck's last `deck.quality`, by client id (a client that left is gone) */
	quality?: Record<string, DeckQuality>;
	/** #90: each seat's readout, by seat id */
	seats?: Record<string, SeatState>;
	/** #90: this relay's link to its parent; missing = it has none */
	parent?: boolean;
}

/** #223: one client's link: the smoothed WebSocket ping round trip, the rate the relay sends it state-like messages at (`deck.values`, `presence`, `vote.tally`, `state` – 30 = as fast as they come, down to 5 on a link that can't carry more), and how many of them it replaced before they went out (a stalled client gets the newest, not a backlog). */
export interface LinkQuality {
	/** ms; missing before the first pong */
	rtt?: number;
	hz: number;
	skipped?: number;
}

/** #188: a deck's values flow from client `from` to client `to` (the conductor's scene draws a hose between their beings). */
export interface Route {
	from: string;
	to: string;
}

export interface DeckNode {
	id: string;
	/** a deck-native node kind (`fm`, `lfo`, `space` …) */
	kind: string;
}

export interface DeckLink {
	/** `node.port` (an output) */
	from: string;
	/** `node.port` (an input) */
	to: string;
	scale?: number;
	offset?: number;
}

/** #188 phase 4: a deck → relay → the conductor, on joining and when its scene changes: its being – the stage graph's nodes and links (the body), the head's character, the body's size (m), the moods calm → wild, the exposed parameters. The relay keeps the last one per client for the conductor's snapshot. */
export interface DeckGraph {
	nodes: DeckNode[];
	links: DeckLink[];
	character?: string;
	size?: number;
	moods?: string[];
	expose?: string[];
}

/** #188 phase 4 (the stream #189's sends build on): a deck → relay → the conductor (and `to`, if set), only what changed, at most 30 / s (the relay drops the rest). `values`: a path – `node.port` (a control input, a control output, an audio output's block peak) or `node.shape` (an LFO's waveform) – → a float vector (1 … n components; one today). `mood`: the head's mood when it changed. Not kept for replay: the relay folds them into the snapshot's `decks`. */
export interface DeckValues {
	values: Record<string, number[]>;
	mood?: string;
}

/** #211: a deck → relay → the conductor (and `to`, if set): MIDI its parameters send as outputs (a mapping's target `port = "conductor"` – CC / note / pitch bend, status byte first), for the conductor to route on like device input. Only decks send it; not kept for replay (a value stream, not state). */
export interface DeckMidi {
	msgs: MidiBytes[];
}

/** #469: a handheld → relay → the conductor: the phone's own pose in its world (the editor maps it onto the shot camera). `t` = the phone's clock (ms), `p` = position (metres, 3 numbers), `q` = rotation (quaternion x y z w, 4 numbers). Latest wins: on QUIC a datagram; a pose not newer than the last one from that phone is dropped (by the relay and by the receiving client); never logged or replayed. */
export interface HandPose {
	t: number;
	p: number[];
	q: number[];
}

/** #469: what the phone tracks – WebXR (position + rotation), the orientation sensor (rotation only), nothing yet. */
export type HandMode = 'xr' | 'orientation' | 'none';

/** #469: a handheld → the conductor: what it tracks and whether tracking holds; `ua` its browser. Reliable; sent again after every welcome. */
export interface HandState {
	mode: HandMode;
	tracking: boolean;
	/** at most 300 characters (the client trims it) */
	ua?: string;
}

/** #469: a handheld → the conductor: Record (`on`) or Stop. Reliable (`hand.recenter`: Recenter, no payload). */
export interface HandRecord {
	on: boolean;
}

/** #469: the conductor (the storyboard editor) → its handhelds: the shot it films, whether the handheld camera is on for it, recording and for how long (s). Reliable; sent again after every welcome. */
export interface HandView {
	shot?: string;
	on: boolean;
	recording: boolean;
	seconds: number;
}

/** #442: a deck → relay → the group `decks`: the absolute addresses its far world nodes name (deck-space rig/address.rs: `/deck:<name>/scene:<id>/character:<id>/<track>:<node>.<port>`), sent on each welcome and again every 10 s while it has any (a subscription not refreshed for 30 s lapses). The deck whose name and rig the address names answers with `deck.events` to this client; the relay never parses an address. Ordered, not kept for replay. */
export interface DeckListen {
	addresses: string[];
}

/** #442: a deck → relay → one subscriber (`to.client`): what left the event outputs it asked for, in order, as deck-events events by name – the world node's `listen` filters them on arrival. Ordered, never merged or replayed (a value stream, not state). */
export interface DeckEvents {
	events: DeckEvent[];
}

export interface DeckEvent {
	/** the absolute address the subscriber asked for */
	address: string;
	/** the event's name (`music.note`, `story.cue`) */
	name: string;
	/** its numeric props by key (`pitch`, `vel`, `key`, `glide` …) */
	props: Record<string, number>;
	/** a note's length in seconds (beats become seconds on the sender's tempo map); missing = a moment */
	dur_s?: number;
}

/** #195: conductor → relay → the decks (the group `decks`, or only the client ids in `decks`): fetch what this scene needs into the cache – the same scene-content request (`GET /api/library/scene-content`) and cache as a deck's own scene, skipping what is there – and answer `deck.synced`. Only the conductor sends it; the relay logs it and keeps the last one for the snapshot, so a deck that joins later syncs too. */
export interface SceneSync {
	/** the scene / show id the sync is for (`deck.synced` names it back) */
	scene: string;
	/** set ids – each deck fetches the set's 3D at its own quality */
	sets?: string[];
	/** character ids – every portrait the API lists */
	characters?: string[];
	/** #191: lent beings – reserved, no deck reads it yet */
	beings?: string[];
	/** only these client ids (missing = every deck) */
	decks?: string[];
}

/** #195: syncing = files still to settle; ready = every item is in the cache; missing = settled, but some item is not there (`why`); offline = the API can't be reached and some item is not in the cache. */
export type SyncState = 'syncing' | 'ready' | 'missing' | 'offline';

/** #220: a deck's render quality level, cheapest first – the hardware tiers S / M / L / XL (deck/HARDWARE.md). */
export type QualityLevel = 'low' | 'medium' | 'high' | 'ultra';

/** #220: a deck → relay, when its quality changes (at once) and its frame time moved (at most every 2 s): the level it renders at now, the render scale (× the screen; > 1 supersamples), whether the governor steps it (`auto`) and the mean frame interval (ms). The relay keeps the last one per client and hands it to the conductor in presence (`quality`), next to the link. */
export interface DeckQuality {
	level: QualityLevel;
	scale: number;
	auto: boolean;
	/** ms, the mean of the last half second */
	frame: number;
	/** the level chosen (the governor steps at most up to it); missing = `level` */
	ceiling?: QualityLevel;
}

/** #195: a deck → relay → the conductor, while it works on a `scene.sync` (progress) and when it is done – like `media.ready`. Items are `set:<id>` and `character:<id>`; one in neither list is still syncing. The relay keeps each deck's last one for the conductor's snapshot (`synced`) and marks a deck ready in presence once its state is ready. */
export interface DeckSynced {
	scene: string;
	state: SyncState;
	/** files settled (fetched, cached or failed) */
	done?: number;
	/** files the items need */
	total?: number;
	/** bytes downloaded for this sync */
	bytes?: number;
	ready: string[];
	missing: string[];
	/** the reason per missing item */
	why?: Record<string, string>;
}

/** A deck's being in the conductor's snapshot: its graph and its latest values (all of them, folded). */
export interface DeckState {
	graph: DeckGraph;
	values: Record<string, number[]>;
	mood?: string;
}

export interface Pickup {
	item: string;
	placement: string;
}

export interface React {
	kind: string;
}

/** One message for a device, resolved against the venue patch. `midi`: raw bytes (status + data). `osc`: address + typed args. `dmx`: channel values of the outlet's universe (a look resolves to one per fixture). */
export type DeviceMsg =
	| { kind: 'midi'; p: MidiBytes }
	| { kind: 'osc'; p: OscMessage }
	| { kind: 'dmx'; p: DmxMessage };

export interface MidiBytes {
	bytes: number[];
}

/** One OSC argument, typed (JSON alone cannot tell 2 from 2.0). On the wire: int32, float32, string, bool. Blobs / timetags come when something needs them. */
export type OscArg =
	| { kind: 'int'; p: number }
	| { kind: 'float'; p: number }
	| { kind: 'string'; p: string }
	| { kind: 'bool'; p: boolean };

/** An OSC message (in or out): the address pattern and its arguments. Bundles are flattened by the adapter. */
export interface OscMessage {
	/** `/like/this` */
	address: string;
	args: OscArg[];
}

/** DMX out (#187): `values` from channel `ch` on (1-based; ch + values − 1 ≤ 512) in the outlet's universe; `fade` ms from the current values (missing / 0 = at once). Several ranges = several messages in `msgs`. */
export interface DmxMessage {
	ch: number;
	values: number[];
	fade?: number;
}

/** relay → bridge: play `msgs` on `outlet` at `at`. */
export interface Device {
	outlet: string;
	msgs: DeviceMsg[];
	at?: number;
}

/** bridge → conductor: MIDI / OSC in, mapped to actions there. */
export interface DeviceIn {
	outlet: string;
	msg: DeviceMsg;
}

export interface Port {
	/** midi · osc · dmx-artnet · … */
	adapter: string;
	name: string;
	direction: 'in' | 'out';
}

/** bridge → relay: the ports the bridge sees, for the patch UI. */
export interface Devices {
	ports: Port[];
}

/** relay → client (not in B.6; the relay's refusals: bad hello, unknown kind, rate limit). */
export interface Error {
	code: string;
	message: string;
}

export const PROTOCOL_VERSION = 1 as const;
