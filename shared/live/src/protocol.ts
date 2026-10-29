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
	| { kind: 'vote.open'; p: Loose }
	| { kind: 'vote.close'; p: Loose }
	| { kind: 'vote.cast'; p: VoteCast }
	| { kind: 'vote.tally'; p: Loose }
	| { kind: 'vote.result'; p: Loose }
	| { kind: 'assign'; p: Assign }
	| { kind: 'panic'; p: Panic }
	| { kind: 'media.preload'; p: MediaPreload }
	| { kind: 'snapshot'; p: Loose }
	| { kind: 'cell'; p: Loose }
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

/** relay → client after `hello`. `snapshot` is loose until the relay keeps state. */
export interface Welcome {
	client: string;
	group?: string;
	snapshot: Loose;
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

/** One message for a device, resolved against the venue patch. `midi`: raw bytes (status + data). `osc` / `dmx` come with their adapters (CYBERDECKS.md §4). */
export type DeviceMsg =
	| { kind: 'midi'; p: MidiBytes }
	| { kind: 'osc'; p: Loose }
	| { kind: 'dmx'; p: Loose };

export interface MidiBytes {
	bytes: number[];
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
