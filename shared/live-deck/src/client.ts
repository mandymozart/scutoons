// client.ts (#197, #366): the browser's socket under deck-space's relay client (relay.rs, relay-web's wasm), which decides
// all a relay client does (join, hello, keyframe, value stream, backoff, pings, status, what a deck or a vote screen
// hears); this keeps the WebSocket and one timer (`RelaySocket`) and hands the core's words on (`DeckLink`; the viewer's
// vote link). The core's scripted links (test/fixtures/relay.json) run through it in client.test.ts.
import type { DeckGraph, DeckQuality, DeckSynced, Envelope, SceneSync, Tempo } from '@scutoons/live';
import type { PathKind } from './rig.ts';

export interface WebSocketLike {
	readonly readyState: number;
	send(data: string): void;
	close(): void;
	onopen: ((ev: unknown) => void) | null;
	onmessage: ((ev: { data: unknown }) => void) | null;
	onclose: ((ev: unknown) => void) | null;
	onerror: ((ev: unknown) => void) | null;
}

/** relay-web's `RelayClient` (deck-space `relay::Client`): the socket's events and a deck's moves in, `take` out (a vote
 *  screen's moves: the viewer's live-vote-link.ts) */
export interface RelayCoreView {
	start(now: number): void;
	opened(now: number): void;
	closed(now: number, why: string): void;
	text(now: number, data: string): void;
	tick(now: number): void;
	next_wake(): number | undefined;
	take(): string;
	line(): string;
	client(): string;
	joined(): boolean;
	counts(): string;
	stop(): void;
	set_graph(json: string): void;
	set_stream(json: string): void;
	paths(): string[];
	sample(time: number, values: Float64Array, mood: string): void;
	quality(json: string): void;
	synced(now: number, json: string): void;
	tempo(json: string): boolean;
	ping(now: number): boolean;
}

export interface RelayModule {
	RelayClient: new (options: string) => RelayCoreView;
}

/** where a link stands (deck-space `relay::Status`) */
export type RelayStatus = { state: 'joining' | 'connected' | 'offline' | 'refused'; client?: string; why?: string; wait?: number };

/** What the app hears (deck-space `relay::Out` past the socket's own): the core's status line (JOINING · DECK c3 @ CATH · OFFLINE …)
 *  and status; joined at the page's ms; #321 a pong (its ping's `t0` µs, the relay's `t1` ms, back at the page's `t2` ms)
 *  and the show clock's anchor (arrived at the page's ms); #195 the conductor's push; #231 what a vote screen hears;
 *  every envelope (the core's `messages`). */
export interface RelayHooks {
	onStatus?: (line: string, s: RelayStatus) => void;
	onJoined?: (at: number, client: string) => void;
	onPong?: (t0: number, t1: number, t2: number) => void;
	onTempo?: (t: Tempo, arrived: number) => void;
	onSync?: (s: SceneSync) => void;
	onVote?: (e: Envelope) => void;
	onMessage?: (e: Envelope) => void;
}

/** the page's platform – its WebSocket, clock (ms since the epoch) and timer; tests pass their own */
export interface Platform {
	socket?: new (url: string) => WebSocketLike;
	now?: () => number;
	setTimer?: (fn: () => void, ms: number) => unknown;
	clearTimer?: (id: unknown) => void;
}

const modules = new Map<string, Promise<RelayModule>>();
/** relay-web from `base` (relay_web.js + relay_web_bg.wasm: `npm run build:wasm -w shared/live-deck`), once per page;
 *  `wasm`: its bytes (Node can't fetch a file URL) */
export function loadRelay(base: string, wasm?: BufferSource): Promise<RelayModule> {
	let m = modules.get(base);
	if (!m) {
		modules.set(base, (m = load(base, wasm)));
		m.catch(() => modules.delete(base));
	}
	return m;
}

async function load(base: string, wasm?: BufferSource): Promise<RelayModule> {
	const mod = (await import(/* @vite-ignore */ `${base}relay_web.js`)) as RelayModule & { default: (o: { module_or_path: string | BufferSource }) => Promise<unknown> };
	await mod.default({ module_or_path: wasm ?? `${base}relay_web_bg.wasm` });
	return mod;
}

/** The socket and the timer under one core client: after every call into the core, `run` does what it asks. */
export class RelaySocket {
	private ws: WebSocketLike | null = null;
	private timer: unknown = null;

	constructor(
		readonly core: RelayCoreView,
		private readonly hooks: RelayHooks,
		private readonly p: Platform = {}
	) {}

	/** the page's clock: ms since the epoch (the core's, the show clock's, `getOutputTimestamp`'s) */
	now(): number {
		return this.p.now?.() ?? performance.timeOrigin + performance.now();
	}

	start(): void {
		this.core.start(this.now());
		this.run();
	}

	/** call the core's `name` with `args`, then do what it says */
	call<K extends keyof RelayCoreView>(name: K, ...args: Parameters<RelayCoreView[K]>): ReturnType<RelayCoreView[K]> {
		const r = (this.core[name] as (...a: unknown[]) => ReturnType<RelayCoreView[K]>).apply(this.core, args);
		this.run();
		return r;
	}

	/** what the core asks and says, until it says nothing; then the timer for its next wake */
	run(): void {
		for (let outs = JSON.parse(this.core.take()) as Record<string, never>[]; outs.length; outs = JSON.parse(this.core.take())) for (const o of outs) this.act(o);
		(this.p.clearTimer ?? clearTimeout)(this.timer as never);
		const at = this.core.next_wake();
		this.timer = at === undefined ? null : (this.p.setTimer ?? setTimeout)(this.wake.bind(this), Math.max(0, at - this.now()));
	}

	private wake(): void {
		this.core.tick(this.now());
		this.run();
	}

	private act(o: Record<string, never>): void {
		const h = this.hooks;
		if ('dial' in o) this.dial(o.dial);
		else if ('send' in o) this.ws?.send(o.send);
		// the socket's onclose tells the core
		else if ('close' in o) this.ws?.close();
		else if ('status' in o) h.onStatus?.(this.core.line(), o.status);
		else if ('joined' in o) h.onJoined?.(o.joined['at'], o.joined['client']);
		else if ('pong' in o) h.onPong?.(o.pong['t0'], o.pong['t1'], o.pong['t2']);
		else if ('tempo' in o) h.onTempo?.(o.tempo['tempo'], o.tempo['arrived']);
		else if ('sync' in o) h.onSync?.(o.sync);
		else if ('vote' in o) h.onVote?.(o.vote);
		else if ('message' in o) h.onMessage?.(o.message);
	}

	private dial(url: string): void {
		try {
			this.ws = new (this.p.socket ?? (globalThis.WebSocket as unknown as new (url: string) => WebSocketLike))(url);
		} catch (e) {
			this.ws = null;
			this.core.closed(this.now(), String(e));
			return;
		}
		this.ws.onopen = this.on.bind(this, this.ws, 'open');
		this.ws.onmessage = this.on.bind(this, this.ws, 'text');
		this.ws.onclose = this.on.bind(this, this.ws, 'close');
		this.ws.onerror = null;
	}

	/** a socket's event – only the current socket's count */
	private on(ws: WebSocketLike, what: 'open' | 'text' | 'close', ev: unknown): void {
		if (this.ws !== ws) return;
		const e = ev as { data?: unknown; reason?: unknown } | null;
		if (what === 'open') this.core.opened(this.now());
		else if (what === 'text') this.core.text(this.now(), String(e?.data));
		else {
			this.ws = null;
			this.core.closed(this.now(), String(e?.reason ?? ''));
		}
		this.run();
	}

	close(): void {
		this.call('stop');
		this.ws = null;
	}
}

/** a deck's link: `token` = the relay session token once there is one (#180 / #186; a dev relay and today's decks join
 *  without), `core` = relay-web (`loadRelay`; the link joins once it is there), the hooks as `RelayHooks` */
export interface LinkOptions extends Platform, Pick<RelayHooks, 'onPong' | 'onTempo' | 'onSync' | 'onMessage'> {
	relay: string;
	graph: DeckGraph;
	nick?: string;
	token?: string;
	core: RelayModule | Promise<RelayModule>;
	onStatus?: (line: string) => void;
	onJoined?: (at: number) => void;
}

/** A deck on the relay (a `musician` of the decks): its being, stream, quality and sync answers go to the core; joins
 *  once relay-web is there, until `close()`. */
export class DeckLink {
	socket: RelaySocket | null = null;
	private stopped = false;
	private graph: DeckGraph;
	private streamed: [string, PathKind][] = [];
	private q: DeckQuality | null = null;
	private order: string[] = [];

	constructor(private readonly o: LinkOptions) {
		this.graph = o.graph;
		void this.begin(o.core);
	}

	private async begin(core: RelayModule | Promise<RelayModule>): Promise<void> {
		const o = this.o;
		try {
			const m = core instanceof Promise ? await core : core;
			if (this.stopped) return;
			const c = new m.RelayClient(JSON.stringify({ relay: o.relay, role: 'musician', group: 'decks', nick: o.nick, token: o.token, messages: !!o.onMessage }));
			c.set_graph(JSON.stringify(this.graph));
			c.set_stream(JSON.stringify(this.streamed));
			this.order = c.paths();
			if (this.q) c.quality(JSON.stringify(this.q));
			this.socket = new RelaySocket(c, o, o);
			this.socket.start();
		} catch (e) {
			o.onStatus?.(`relay client: ${(e as Error).message ?? e}`);
		}
	}

	get client(): string {
		return this.socket?.core.client() ?? '';
	}

	get joined(): boolean {
		return this.socket?.core.joined() ?? false;
	}

	/** the paths the deck streams with how each moves (the played scene's `streamed`) */
	setStream(streamed: [string, PathKind][]): void {
		this.streamed = streamed;
		this.socket?.call('set_stream', JSON.stringify(streamed));
		this.order = this.socket?.core.paths() ?? [];
	}

	/** One frame at `time` (s): each streamed path's value now and the mood – the core sends what moved, at its pace. */
	sample(time: number, read: (path: string) => number, mood: string | undefined): void {
		this.socket?.call('sample', time, Float64Array.from(this.order, read), mood ?? '');
	}

	/** #195: the answer to the conductor's push (progress and final; the core keeps it for a rejoin) */
	synced(r: DeckSynced): void {
		this.socket?.call('synced', this.socket.now(), JSON.stringify(r));
	}

	/** #220: the render quality (the 3D's governor reports a change at once, the frame time at most every 2 s) */
	quality(q: DeckQuality): void {
		this.q = q;
		this.socket?.call('quality', JSON.stringify(q));
	}

	/** #321: a ping now (the clock's), its `t0` the page's clock in µs */
	sendPing(): boolean {
		return this.socket?.call('ping', this.socket.now()) ?? false;
	}

	/** #321: a leading deck's anchor (dropped while not joined – the core's clock sends again after a join) */
	tempo(t: Tempo): boolean {
		return this.socket?.call('tempo', JSON.stringify(t)) ?? false;
	}

	/** the being changed (another character): the relay keeps the last graph per client */
	setGraph(g: DeckGraph): void {
		this.graph = g;
		this.socket?.call('set_graph', JSON.stringify(g));
	}

	close(): void {
		this.stopped = true;
		this.socket?.close();
	}
}
