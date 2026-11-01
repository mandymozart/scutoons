// client.ts (#197, @scutoons/live-deck since #198): the browser deck's relay logic, platform-free (a WebSocket constructor is injected) so Node
// tests it against a real relay. The same contract as a native deck (deck/stage/src/space/link.rs + deck/space's stream.rs):
// - joins `ws://host:port/live/<code>?role=musician&group=decks` and says `hello` first;
// - on `welcome` sends its being (`deck.graph`), then a keyframe of every value sent so far, then only changes;
// - `ValueStream.sample` = stream.rs: at most RATE samples a second, a value only when it moved more than its epsilon,
//   the mood when it changed;
// - a reconnect (backoff 1 2 4 8 10 s) sends the graph and the keyframe again (the relay dropped the being);
// - #195: the conductor's push – the welcome snapshot's `sync` or a `scene.sync` – goes to `onSync` (sync.ts works on
//   it); `synced` sends the answer back as `deck.synced`.
import type { DeckGraph, DeckQuality, DeckSynced, DeckValues, Envelope, Message, SceneSync, Welcome } from '@scutoons/live';

/** at most this many `deck.values` a second (the relay drops more than 30 / s) */
export const RATE = 30;
export const GROUP = 'decks';
const PING_MS = 20_000;
const BACKOFF_S = [1, 2, 4, 8, 10];

/** `ws://host:port/live/<code>` → the base and the session code (link.rs reads it the same way) */
export function parseRelay(url: string): { base: string; code: string } {
	const m = /^(wss?:\/\/[^/]+)\/live\/([^/?#]+)\/?$/.exec(url.trim());
	if (!m) throw new Error(`relay URL must look like ws://host:7070/live/CODE, got '${url}'`);
	return { base: m[1], code: decodeURIComponent(m[2]) };
}

/** A client → relay envelope (`seq` 0, `from` empty: the relay stamps both). */
export const envelope = (m: Message): string => JSON.stringify({ v: 1, seq: 0, from: '', ...m });

/** What a value's path is, for its epsilon (values.rs `epsilon`). */
export type PathKind = { input: [min: number, max: number] } | 'audio' | 'control' | 'shape';

export const epsilon = (k: PathKind): number =>
	k === 'audio' ? 0.01 : k === 'control' ? 2e-3 : k === 'shape' ? 0.5 : Math.max(Math.abs(k.input[1] - k.input[0]) * 1e-3, 1e-5);

/** stream.rs: one sample's changes, or null between samples / when nothing moved. */
export class ValueStream {
	private readonly last = new Map<string, number>();
	private readonly eps: Map<string, number>;
	private next = 0;
	private mood: string | undefined;

	constructor(paths: Record<string, PathKind>) {
		this.eps = new Map(Object.entries(paths).map(([p, k]) => [p, epsilon(k)]));
	}

	get paths(): string[] {
		return [...this.eps.keys()];
	}

	/** At `time` (s): null before the next slot, else what moved since it was last sent (the first sample: all). */
	sample(time: number, read: (path: string) => number, mood: string | undefined): DeckValues | null {
		// a frame may come a hair early; a long gap starts the schedule anew
		if (time + 1e-3 < this.next) return null;
		this.next = time - this.next > 1 / RATE ? time + 1 / RATE : this.next + 1 / RATE;
		const values: Record<string, number[]> = {};
		let any = false;
		for (const [path, eps] of this.eps) {
			const v = read(path);
			if (!Number.isFinite(v)) continue;
			const was = this.last.get(path);
			if (was === undefined || Math.abs(v - was) > eps) {
				this.last.set(path, v);
				values[path] = [v];
				any = true;
			}
		}
		const out: DeckValues = { values };
		if (mood !== undefined && mood !== this.mood) {
			this.mood = mood;
			out.mood = mood;
			any = true;
		}
		return any ? out : null;
	}
}

export interface WebSocketLike {
	readonly readyState: number;
	send(data: string): void;
	close(): void;
	onopen: ((ev: unknown) => void) | null;
	onmessage: ((ev: { data: unknown }) => void) | null;
	onclose: ((ev: unknown) => void) | null;
	onerror: ((ev: unknown) => void) | null;
}

export interface LinkOptions {
	relay: string;
	graph: DeckGraph;
	nick?: string;
	/**
	 * The relay session token, once there is one (#180 / #186: the member's API session → a relay token). A dev relay
	 * and today's decks join without; `hello` carries it when given.
	 */
	token?: string;
	/** default: the global WebSocket */
	socket?: new (url: string) => WebSocketLike;
	/** the status line (JOINING · DECK c3 @ CATH · OFFLINE … · NO RELAY …) */
	onStatus?: (s: string) => void;
	/** every envelope the relay sends (tests, the HUD) */
	onMessage?: (e: Envelope) => void;
	/** #195: the conductor's push (at the welcome when the session has one, then each new one) */
	onSync?: (s: SceneSync) => void;
}

/** link.rs's deck side: join, being, keyframe, changes; reconnects until `close()`. */
export class DeckLink {
	readonly base: string;
	readonly code: string;
	client = '';
	joined = false;
	/** every value sent (the keyframe after a reconnect) */
	readonly sent: Record<string, number[]> = {};
	mood: string | undefined;
	/** messages sent since the start (tests, the HUD) */
	count = { graph: 0, values: 0, synced: 0, quality: 0 };
	/** #220: the last render quality (sent again after a rejoin – the relay forgets a deck that left) */
	private q: DeckQuality | null = null;
	private graph: DeckGraph;
	private ws: WebSocketLike | null = null;
	private tries = 0;
	private stopped = false;
	private timer: ReturnType<typeof setTimeout> | null = null;
	private ping: ReturnType<typeof setInterval> | null = null;

	constructor(private readonly o: LinkOptions) {
		({ base: this.base, code: this.code } = parseRelay(o.relay));
		this.graph = o.graph;
		this.connect();
	}

	private status(s: string) {
		this.o.onStatus?.(s);
	}

	private connect() {
		if (this.stopped) return;
		this.joined = false;
		this.status(`JOINING ${this.code}`);
		const Socket = this.o.socket ?? (globalThis.WebSocket as unknown as new (url: string) => WebSocketLike);
		const ws = new Socket(`${this.base}/live/${encodeURIComponent(this.code)}?role=musician&group=${GROUP}`);
		this.ws = ws;
		ws.onopen = () => {
			this.tries = 0;
			ws.send(envelope({ kind: 'hello', p: { code: this.code, role: 'musician', group: GROUP, ...(this.o.nick ? { nick: this.o.nick } : {}), ...(this.o.token ? { token: this.o.token } : {}) } }));
			this.ping = setInterval(() => this.raw({ kind: 'ping', p: { t0: 0 } }), PING_MS);
		};
		ws.onmessage = (ev) => this.receive(String(ev.data));
		ws.onerror = () => {};
		ws.onclose = () => {
			if (this.ping) clearInterval(this.ping);
			this.ping = null;
			this.ws = null;
			const was = this.joined;
			this.joined = false;
			if (this.stopped) return;
			const wait = was ? 0.25 : BACKOFF_S[Math.min(this.tries++, BACKOFF_S.length - 1)];
			this.status(was ? 'OFFLINE – joining again' : `NO RELAY – again in ${wait} s`);
			this.timer = setTimeout(() => this.connect(), wait * 1000);
		};
	}

	private receive(text: string) {
		let e: Envelope;
		try {
			e = JSON.parse(text) as Envelope;
		} catch {
			return;
		}
		this.o.onMessage?.(e);
		if (e.kind === 'welcome') {
			this.client = (e.p as Welcome).client;
			this.status(`DECK ${this.client} @ ${this.code}`);
			// the being, then every value it showed so far (a keyframe)
			this.raw({ kind: 'deck.graph', p: this.graph });
			this.count.graph++;
			if (Object.keys(this.sent).length || this.mood !== undefined) {
				this.raw({ kind: 'deck.values', p: { values: { ...this.sent }, ...(this.mood !== undefined ? { mood: this.mood } : {}) } });
				this.count.values++;
			}
			if (this.q) {
				this.raw({ kind: 'deck.quality', p: this.q });
				this.count.quality++;
			}
			this.joined = true;
			const sync = (e.p as Welcome).snapshot.sync;
			if (sync) this.o.onSync?.(sync);
		} else if (e.kind === 'scene.sync') {
			this.o.onSync?.(e.p);
		} else if (e.kind === 'error') {
			this.status(`RELAY ${e.p.code}: ${e.p.message}`);
		}
	}

	private raw(m: Message): boolean {
		if (!this.ws || this.ws.readyState !== 1) return false;
		this.ws.send(envelope(m));
		return true;
	}

	/** a sample's changes: folded always, sent once welcomed (offline the latest values wait for the keyframe) */
	send(v: DeckValues) {
		Object.assign(this.sent, v.values);
		if (v.mood !== undefined) this.mood = v.mood;
		if (this.joined && this.raw({ kind: 'deck.values', p: v })) this.count.values++;
	}

	/** #195: the answer to the conductor's push (progress and final); dropped while not joined – a rejoin re-runs the push */
	synced(r: DeckSynced) {
		if (this.joined && this.raw({ kind: 'deck.synced', p: r })) this.count.synced++;
	}

	/** #220: the render quality (the 3D's governor reports a change at once, the frame time at most every 2 s); kept for a rejoin */
	quality(q: DeckQuality) {
		this.q = q;
		if (this.joined && this.raw({ kind: 'deck.quality', p: q })) this.count.quality++;
	}

	/** the being changed (another character): the relay keeps the last graph per client */
	setGraph(g: DeckGraph) {
		this.graph = g;
		if (this.joined && this.raw({ kind: 'deck.graph', p: g })) this.count.graph++;
	}

	close() {
		this.stopped = true;
		if (this.timer) clearTimeout(this.timer);
		if (this.ping) clearInterval(this.ping);
		this.ws?.close();
		this.ws = null;
		this.joined = false;
	}
}
