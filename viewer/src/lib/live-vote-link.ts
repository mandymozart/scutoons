// The vote screens' link to the relay (#231, #366): deck-space's relay client (relay.rs, relay-web's wasm) under
// @scutoons/live-deck's socket – it joins as the screen's role (audience / conductor, the conductor with its session
// token), hands the screen what it hears (the welcome's open vote and group instruction, the votes, a refusal), keeps
// which vote is open (a late listener gets it at once) and addresses cast / hold / close to it. This file only wires the
// screen's `VoteSource` to it.
import { RelaySocket, type Platform, type RelayCoreView, type RelayModule, type RelayStatus } from '@scutoons/live-deck';
import type { Envelope, Role, VoteOpen } from '@scutoons/live';
import type { VoteSource } from './live-vote.ts';

export type { WebSocketLike as SocketLike } from '@scutoons/live-deck';

export interface VoteLinkOptions extends Platform {
	/** `ws://host:port/live/<code>` */
	relay: string;
	role: Extract<Role, 'conductor' | 'audience'>;
	group?: string;
	nick?: string;
	/** the conductor's session token (B.6) */
	token?: string;
	/** relay-web (`loadRelay(WASM_BASE)`); the link joins once it is there */
	core: RelayModule | Promise<RelayModule>;
	/** joining · connected · offline · refused (the core's status) */
	onStatus?: (s: LinkStatus) => void;
}

export type LinkStatus = RelayStatus;

/** relay-web's `RelayClient` with a vote screen's moves */
interface VoteCore extends RelayCoreView {
	vote_open(json: string): boolean;
	vote_cast(option: number): boolean;
	vote_close(over?: number): boolean;
	vote_hold(hold: boolean): boolean;
	current_vote(): string;
}

export class RelayVote implements VoteSource {
	private s: (RelaySocket & { core: VoteCore }) | null = null;
	private stopped = false;
	private readonly listeners = new Set<(e: Envelope) => void>();

	constructor(private readonly o: VoteLinkOptions) {
		const core = o.core;
		if (core instanceof Promise) void core.then((m) => this.begin(m), (e) => o.onStatus?.({ state: 'refused', why: String(e) }));
		else this.begin(core);
	}

	private begin(m: RelayModule): void {
		if (this.stopped) return;
		const o = this.o;
		const core = new m.RelayClient(JSON.stringify({ relay: o.relay, role: o.role, group: o.group, nick: o.nick, token: o.token })) as VoteCore;
		this.s = new RelaySocket(core, { onStatus: (_, s) => o.onStatus?.(s), onVote: (e) => this.emit(e) }, o) as RelaySocket & { core: VoteCore };
		this.s.start();
	}

	get client(): string {
		return this.s?.core.client() ?? '';
	}

	on(_role: 'conductor' | 'audience', fn: (e: Envelope) => void): () => void {
		this.listeners.add(fn);
		const open = this.s?.core.current_vote();
		if (open) fn(JSON.parse(open) as Envelope);
		return () => this.listeners.delete(fn);
	}

	/** call into the core, then do what it says */
	private with(fn: (c: VoteCore) => unknown): void {
		if (!this.s) return;
		fn(this.s.core);
		this.s.run();
	}

	cast(_client: string, option: number): void {
		this.with((c) => c.vote_cast(option));
	}

	open(p: VoteOpen): void {
		this.with((c) => c.vote_open(JSON.stringify(p)));
	}

	close(override?: number): void {
		this.with((c) => c.vote_close(override));
	}

	hold(hold: boolean): void {
		this.with((c) => c.vote_hold(hold));
	}

	dispose(): void {
		this.stopped = true;
		this.s?.close();
		this.listeners.clear();
	}

	private emit(e: Envelope) {
		for (const fn of [...this.listeners]) fn(e);
	}
}
