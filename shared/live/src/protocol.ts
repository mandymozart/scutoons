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
	| { kind: 'snapshot'; p: Snapshot }
	| { kind: 'cell'; p: CellChange }
	| { kind: 'say'; p: Loose }
	| { kind: 'line'; p: Loose }
	| { kind: 'fx'; p: Loose }
	| { kind: 'colour'; p: Loose }
	| { kind: 'presence'; p: Presence }
	| { kind: 'ready'; p: Empty }
	| { kind: 'pickup'; p: Pickup }
	| { kind: 'react'; p: React }
	| { kind: 'device'; p: Device }
	| { kind: 'device.in'; p: DeviceIn }
	| { kind: 'devices'; p: Devices }
	| { kind: 'error'; p: Error };

/** A payload whose shape B.6 names but this skeleton does not fix yet (votes, snapshot, cells, say / line / fx / colour). */
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

/** conductor → relay → the outlet's clients on a row change: the outlet's new cell (B.5.2 `Cell`, loose until the show schema lands). */
export interface CellChange {
	outlet: string;
	cell: Loose;
}

/** What a client shows when it joins / reconnects / is moved (B.6): the last state, the current cell per outlet it sees, the open vote, the last instruction (say) per group it sees. The conductor sees every outlet and group. */
export interface Snapshot {
	state?: State;
	cells?: Record<string, Loose>;
	vote?: VoteOpen;
	say?: Record<string, Loose>;
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

export interface MediaPreload {
	outlet: string;
	urls: string[];
}

/** relay → conductor. */
export interface Presence {
	groups: Record<string, number>;
	ready: string[];
}

export interface Pickup {
	item: string;
	placement: string;
}

export interface React {
	kind: string;
}

/** One message for a device, resolved against the venue patch. `midi`: raw bytes (status + data). `osc`: address + typed args. `dmx` comes with its adapters (CYBERDECKS.md §4). */
export type DeviceMsg =
	| { kind: 'midi'; p: MidiBytes }
	| { kind: 'osc'; p: OscMessage }
	| { kind: 'dmx'; p: Loose };

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
