// The audience vote screen's link to the relay (#231, #366): deck-space's relay client (relay.rs, relay-web's wasm) under
// @scutoons/live-deck's socket – it joins as an audience phone of its group, hands the screen what it hears (the
// welcome's open vote and group instruction, the votes, a refusal), keeps which vote is open (a late listener gets it at
// once) and addresses a cast to it. This file only wires the screen's `VoteSource` to it. The conductor's side of the
// vote is the deck editor's Perform workspace (#533).
import { RelaySocket, type Platform, type RelayCoreView, type RelayModule, type RelayStatus } from '@scutoons/live-deck';
import type { Envelope } from '@scutoons/live';
import type { VoteSource } from './live-vote.ts';

export type { WebSocketLike as SocketLike } from '@scutoons/live-deck';

export interface VoteLinkOptions extends Platform {
	/** `ws://host:port/live/<code>` */
	relay: string;
	group?: string;
	nick?: string;
	/** relay-web (`loadRelay(WASM_BASE)`); the link joins once it is there */
	core: RelayModule | Promise<RelayModule>;
	/** joining · connected · offline · refused (the core's status) */
	onStatus?: (s: LinkStatus) => void;
}

export type LinkStatus = RelayStatus;

/** relay-web's `RelayClient` with a phone's moves */
interface VoteCore extends RelayCoreView {
	vote_cast(option: number): boolean;
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
		const core = new m.RelayClient(JSON.stringify({ relay: o.relay, role: 'audience', group: o.group, nick: o.nick })) as VoteCore;
		this.s = new RelaySocket(core, { onStatus: (_, s) => o.onStatus?.(s), onVote: (e) => this.emit(e) }, o) as RelaySocket & { core: VoteCore };
		this.s.start();
	}

	get client(): string {
		return this.s?.core.client() ?? '';
	}

	on(_role: 'audience', fn: (e: Envelope) => void): () => void {
		this.listeners.add(fn);
		const open = this.s?.core.current_vote();
		if (open) fn(JSON.parse(open) as Envelope);
		return () => this.listeners.delete(fn);
	}

	cast(_client: string, option: number): void {
		if (!this.s) return;
		this.s.core.vote_cast(option);
		this.s.run();
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
