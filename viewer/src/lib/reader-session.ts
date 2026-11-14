// #518 (plan E2b, agents/research/SCENE-EVENTS.md §4; LIVE.md B.2, B.6): a reader joined to a session. A reader page
// with `?live=<code>` (and `?relay=` / `?quic=` as live-deck.ts `relayOf` reads them, a local relay) joins it as role
// `reader` through deck-space's relay client (relay.rs, relay-web's wasm) under live-deck's socket, once for the page's
// life; each story event the reader fires (reader-story.ts) goes to the core as it is, and the core sends it as `story`
// (in order, only while it drives the session's story – #527, relay.rs `Client::story` / `drive`). The relay hands it to
// the conductor and the decks that asked (#517). Without `?live=` this file is never loaded: no link, nothing sent.
// #528: it joins without claiming; the person is asked first (linked-reader.svelte) and only a yes calls `drive` – the
// core's place in the story (`Driving`) comes out as `{ driving }` through `onOther`.
import { RelaySocket, loadRelay, type Platform, type RelayCoreView, type RelayHooks, type RelayModule, type RelayStatus } from '@scutoons/live-deck';
import type { VNEvent } from '@scutoons/content';
import { WASM_BASE, relayOf } from './live-deck.ts';
import { listenSession } from './reader-story.ts';

/** #528: a reader's place in the session's story as the core says it – its JSON (deck-space linked.rs `Driving`) */
export interface Driving {
	claim: 'none' | 'asked' | 'drives' | 'taken_over';
	/** another reader drives now – a claim would take it over (the newcomer's warning) */
	other: boolean;
}

export interface ReaderLinkOptions extends Platform, Pick<RelayHooks, 'onOther'> {
	/** `ws://host:port/live/<code>` */
	relay: string;
	/** relay-web (`loadRelay(WASM_BASE)`); the link joins once it is there */
	core: RelayModule | Promise<RelayModule>;
	/** joining · connected · offline · refused (the core's status) */
	onStatus?: (s: RelayStatus) => void;
}

/** relay-web's `RelayClient` with a reader's moves */
interface ReaderCore extends RelayCoreView {
	story(json: string): boolean;
	drive(): boolean;
}

export class ReaderLink {
	private s: (RelaySocket & { core: ReaderCore }) | null = null;
	/** what fired while relay-web was still loading, handed to the core when it is there */
	private early: string[] | null = [];
	private stopped = false;

	constructor(private readonly o: ReaderLinkOptions) {
		const core = o.core;
		if (core instanceof Promise)
			void core.then(
				(m) => this.begin(m),
				(e) => {
					this.early = null;
					o.onStatus?.({ state: 'refused', why: String(e) });
				}
			);
		else this.begin(core);
	}

	private begin(m: RelayModule): void {
		if (this.stopped) return;
		const core = new m.RelayClient(JSON.stringify({ relay: this.o.relay, role: 'reader' })) as ReaderCore;
		// #528: no claim here – the reader is asked first; the core's `{ driving }` (at the welcome, at each change) goes to `onOther`
		this.s = new RelaySocket(core, { onStatus: (_, s) => this.o.onStatus?.(s), onOther: this.o.onOther }, this.o) as RelaySocket & { core: ReaderCore };
		this.s.start();
		for (const json of this.early ?? []) core.story(json);
		this.early = null;
		this.s.run();
	}

	/** #528: the person said yes (or takes the place back): the core claims the session's story (relay.rs `Client::drive`);
	 *  false: no core yet */
	drive(): boolean {
		if (!this.s) return false;
		const asked = this.s.core.drive();
		this.s.run();
		return asked;
	}

	/** a story event the reader fired */
	tell(event: VNEvent): void {
		const json = JSON.stringify(event);
		if (!this.s) return void this.early?.push(json);
		this.s.core.story(json);
		this.s.run();
	}

	dispose(): void {
		this.stopped = true;
		this.early = null;
		this.s?.close();
	}
}

/** the session a reader page names (`?live=<code>`, the relay as `relayOf` makes it) – none: null */
export function sessionOf(search: URLSearchParams, live: string, quic?: string): string | null {
	const code = search.get('live')?.trim();
	return code ? relayOf(search, live, code, quic) : null;
}

let joined: ReaderLink | null = null;

/** Join the session once for the page's life (a later page of the reading stays in it); the reader's story goes to it. */
export function joinSession(relay: string, o: Partial<ReaderLinkOptions> = {}): ReaderLink {
	if (joined) return joined;
	const link = (joined = new ReaderLink({ ...o, core: o.core ?? loadRelay(WASM_BASE), relay }));
	listenSession((event) => link.tell(event));
	return link;
}

/** out of the session: nothing is sent any more */
export function leaveSession(): void {
	listenSession(null);
	joined?.dispose();
	joined = null;
}
