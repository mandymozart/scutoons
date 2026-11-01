// GENERATED from deck/protocol/schema.json by deck/protocol/generate.ts – edit the schema, run `npm run generate -w shared/live`.
// agents/research/LIVE.md B.6. One envelope per message, JSON over one WebSocket per client. The generator (deck/protocol/generate.ts) reads only the subset used here: object / string / integer / number / boolean / array / $ref / enum / const / oneOf discriminated by `kind` / additionalProperties maps. Edit this file, run `npm run generate -w shared/live`, commit both outputs.
/** LIVE.md B.2. `uplink` is the local relay connected to the hosted one (B.6 Uplink). */
export type Role = 'conductor' | 'output' | 'bridge' | 'performer' | 'musician' | 'audience' | 'uplink';

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
	| { kind: 'scene.sync'; p: SceneSync }
	| { kind: 'deck.synced'; p: DeckSynced }
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
	/** the audience sees the tally too */
	show?: boolean;
}

/** conductor → relay: close now; `override` = the conductor's option instead of the rule's. */
export interface VoteClose {
	vote: string;
	override?: number;
}

/** relay → conductor every 250 ms while a vote is open (→ the audience too when the vote shows it). */
export interface VoteTally {
	vote: string;
	counts: number[];
	voters: number;
	/** ms until the vote closes by itself */
	left: number;
}

export type VoteHow = 'rule' | 'fallback' | 'override';

/** relay → everyone: the option the conductor applies as the choice event (B.1). */
export interface VoteResult {
	vote: string;
	option: number;
	counts: number[];
	how: VoteHow;
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
}

/** relay → client after `hello`: the snapshot of what this client should show now (B.6). On a `resume` inside the replay buffer the snapshot is empty and the missed messages follow. */
export interface Welcome {
	client: string;
	group?: string;
	snapshot: Snapshot;
	/** relay time, ms */
	now: number;
}

export interface Ping {
	t0: number;
}

export interface Pong {
	t0: number;
	t1: number;
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

/** What a layer shows (B.5.2 ToonSrc / LiveSrc, resolved). From the toon (the public library): scroll, slice, panel, surtitles. Live: video / image / model3d (a preloaded medium by id), set (a set's spot in a mode, #193), text, colour, camera. */
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
	| { kind: 'camera'; p: CameraSrc };

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

/** relay → conductor: clients per group, who is ready; the decks that stream a being (#188 – a deck that left is gone from `decks`) and who sends values to whom (`routes`). */
export interface Presence {
	groups: Record<string, number>;
	ready: string[];
	/** client ids with a `deck.graph`, sorted */
	decks?: string[];
	/** every sender → receiver pair `deck.values` went along, sorted */
	routes?: Route[];
}

/** #188: a deck's values flow from client `from` to client `to` (the conductor's scene draws a hose between their beings). */
export interface Route {
	from: string;
	to: string;
}

export interface DeckNode {
	id: string;
	/** a deck-stage node kind (`fm`, `lfo`, `space` …) */
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
