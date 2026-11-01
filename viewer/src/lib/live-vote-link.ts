// The vote screens' link to the relay (#231): one WebSocket, `hello` first, the relay's vote messages to the listeners
// of the screen (the same envelopes the fake relay emits, live-vote-sim.ts). Platform-free – a WebSocket constructor can
// be injected – so Node tests it against a fake socket. Joins `<relay>?role=<role>&group=<group>` like the deck link
// (shared/live-deck client.ts); the conductor says its session token. Reconnects (backoff 1 2 4 8 10 s) and re-asks for
// nothing: the welcome snapshot carries the open vote, which reaches the screen as a `vote.open` again.
import type { Envelope, Message, Role, VoteOpen, Welcome } from '@scutoons/live';
import type { VoteSource } from './live-vote.ts';

const PING_MS = 20_000;
const BACKOFF_S = [1, 2, 4, 8, 10];

export interface SocketLike {
	readonly readyState: number;
	send(data: string): void;
	close(): void;
	onopen: ((ev: unknown) => void) | null;
	onmessage: ((ev: { data: unknown }) => void) | null;
	onclose: ((ev: unknown) => void) | null;
	onerror: ((ev: unknown) => void) | null;
}

export interface VoteLinkOptions {
	/** `ws://host:port/live/<code>` */
	relay: string;
	role: Extract<Role, 'conductor' | 'audience'>;
	group?: string;
	nick?: string;
	/** the conductor's session token (B.6) */
	token?: string;
	socket?: new (url: string) => SocketLike;
	/** JOINING · CONNECTED · OFFLINE – joining again · a refusal */
	onStatus?: (s: LinkStatus) => void;
}

export type LinkStatus = { state: 'joining' | 'connected' | 'offline'; client?: string } | { state: 'refused'; why: string };

/** A client → relay envelope (`seq` 0, `from` empty: the relay stamps both). */
export const envelope = (m: Message): string => JSON.stringify({ v: 1, seq: 0, from: '', ...m });

export function codeOfRelay(relay: string): { base: string; code: string } {
	const m = /^(wss?:\/\/[^/]+)\/live\/([^/?#]+)\/?$/.exec(relay.trim());
	if (!m) throw new Error(`relay URL must look like ws://host:7070/live/CODE, got '${relay}'`);
	return { base: m[1], code: decodeURIComponent(m[2]) };
}

export class RelayVote implements VoteSource {
	client = '';
	private ws: SocketLike | null = null;
	private tries = 0;
	private stopped = false;
	private timer: ReturnType<typeof setTimeout> | null = null;
	private ping: ReturnType<typeof setInterval> | null = null;
	private readonly listeners = new Set<(e: Envelope) => void>();
	private readonly base: string;
	private readonly code: string;
	/** the vote that is open now (a late listener gets it at once) */
	private current: VoteOpen | null = null;

	constructor(private readonly o: VoteLinkOptions) {
		({ base: this.base, code: this.code } = codeOfRelay(o.relay));
		this.connect();
	}

	on(_role: 'conductor' | 'audience', fn: (e: Envelope) => void): () => void {
		this.listeners.add(fn);
		if (this.current) fn(this.synthetic({ kind: 'vote.open', p: this.current }));
		return () => this.listeners.delete(fn);
	}

	cast(_client: string, option: number): void {
		if (this.current) this.raw({ kind: 'vote.cast', p: { vote: this.current.vote, option } });
	}

	open(p: VoteOpen): void {
		this.raw({ kind: 'vote.open', p });
	}

	close(override?: number): void {
		if (this.current) this.raw({ kind: 'vote.close', p: { vote: this.current.vote, ...(override !== undefined ? { override } : {}) } });
	}

	hold(hold: boolean): void {
		if (this.current) this.raw({ kind: 'vote.hold', p: { vote: this.current.vote, hold } });
	}

	dispose(): void {
		this.stopped = true;
		if (this.timer) clearTimeout(this.timer);
		if (this.ping) clearInterval(this.ping);
		this.ws?.close();
		this.listeners.clear();
	}

	private status(s: LinkStatus) {
		this.o.onStatus?.(s);
	}

	private connect() {
		if (this.stopped) return;
		this.status({ state: 'joining' });
		const Socket = this.o.socket ?? (globalThis.WebSocket as unknown as new (url: string) => SocketLike);
		const q = `role=${this.o.role}${this.o.group ? `&group=${encodeURIComponent(this.o.group)}` : ''}`;
		const ws = new Socket(`${this.base}/live/${encodeURIComponent(this.code)}?${q}`);
		this.ws = ws;
		let welcomed = false;
		ws.onopen = () => {
			this.tries = 0;
			ws.send(
				envelope({
					kind: 'hello',
					p: { code: this.code, role: this.o.role, ...(this.o.group ? { group: this.o.group } : {}), ...(this.o.nick ? { nick: this.o.nick } : {}), ...(this.o.token ? { token: this.o.token } : {}) }
				})
			);
			this.ping = setInterval(() => this.raw({ kind: 'ping', p: { t0: 0 } }), PING_MS);
		};
		ws.onmessage = (ev) => {
			let e: Envelope;
			try {
				e = JSON.parse(String(ev.data)) as Envelope;
			} catch {
				return;
			}
			if (e.kind === 'welcome') {
				welcomed = true;
				this.client = (e.p as Welcome).client;
				this.status({ state: 'connected', client: this.client });
				const vote = (e.p as Welcome).snapshot.vote;
				if (vote) this.take({ kind: 'vote.open', p: vote }, e);
				const say = this.o.group ? (e.p as Welcome).snapshot.say?.[this.o.group] : undefined;
				if (say) this.emit(this.synthetic({ kind: 'say', p: say }));
			} else if (e.kind === 'error') {
				if (e.p.code === 'token') this.status({ state: 'refused', why: e.p.message });
				this.emit(e);
			} else if (e.kind === 'vote.open' || e.kind === 'vote.result' || e.kind === 'vote.tally') {
				this.take(e, e);
			} else if (e.kind === 'say') {
				this.emit(e);
			}
		};
		ws.onerror = () => {};
		ws.onclose = () => {
			if (this.ping) clearInterval(this.ping);
			this.ping = null;
			this.ws = null;
			if (this.stopped) return;
			const wait = welcomed ? 0.25 : BACKOFF_S[Math.min(this.tries++, BACKOFF_S.length - 1)];
			this.status({ state: 'offline' });
			this.timer = setTimeout(() => this.connect(), wait * 1000);
		};
	}

	/** keep which vote is open, and pass the message on */
	private take(m: Pick<Envelope, 'kind' | 'p'>, from: Envelope) {
		if (m.kind === 'vote.open') this.current = m.p as VoteOpen;
		else if (m.kind === 'vote.result') this.current = null;
		this.emit(m.kind === from.kind ? from : this.synthetic(m));
	}

	private synthetic(m: Pick<Envelope, 'kind' | 'p'>): Envelope {
		return { v: 1, seq: 0, from: 'relay', ...m } as Envelope;
	}

	private emit(e: Envelope) {
		for (const fn of [...this.listeners]) fn(e);
	}

	private raw(m: Message): boolean {
		if (!this.ws || this.ws.readyState !== 1) return false;
		this.ws.send(envelope(m));
		return true;
	}
}
