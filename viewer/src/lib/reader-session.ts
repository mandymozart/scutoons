// #518 (plan E2b, agents/research/SCENE-EVENTS.md §4; LIVE.md B.2, B.6): a reader joined to a session. A reader page
// with `?live=<code>` (and `?relay=` / `?quic=` as live-deck.ts `relayOf` reads them, a local relay) joins it as role
// `reader` through deck-space's relay client (relay.rs, relay-web's wasm) under live-deck's socket, once for the page's
// life; each story event the reader fires (reader-story.ts) goes to the core as it is, and the core sends it as `story`
// (in order, only while it drives the session's story – #527, relay.rs `Client::story` / `drive`). The relay hands it to
// the conductor and the decks that asked (#517). Without `?live=` this file is never loaded: no link, nothing sent.
// #528: it joins without claiming; the person is asked first (linked-reader.svelte) and only a yes calls `drive` – the
// core's place in the story (`Driving`) comes out as `{ driving }` through `onOther`.
// #552 (SHOW.md §15 3): a story question's vote result – the core matches it to where the reader stands (deck-space
// linked.rs `Place`, fed by every event told here and `closed`) and says `{ choice }`: a `pick` is handed to the choice
// buttons (reader-story.ts `pickChoice`), as a tap; an `ignored` goes on to `onOther` for the linked sign to say.
import { RelaySocket, loadRelay, type Platform, type RelayCoreView, type RelayHooks, type RelayModule, type RelayStatus } from '@scutoons/live-deck';
import type { VNEvent } from '@scutoons/content';
import { WASM_BASE, relayOf } from './live-deck.ts';
import { listenSession, pickChoice, type SessionEar } from './reader-story.ts';

/** #528: a reader's place in the session's story as the core says it – its JSON (deck-space linked.rs `Driving`) */
export interface Driving {
	claim: 'none' | 'asked' | 'drives' | 'taken_over';
	/** another reader drives now – a claim would take it over (the newcomer's warning) */
	other: boolean;
}

/** #552: a story question's result as the core says it (deck-space linked.rs `Choice`): the `showChoices`' id, the voted
 *  option's `addChoice` id; `pick` = its place among the options this reader shows, or why it is `ignored` */
export interface VoteChoice {
	event: string;
	option: string;
	pick?: number;
	ignored?: 'elsewhere' | 'not_offered';
}

export interface ReaderLinkOptions extends Platform, Pick<RelayHooks, 'onOther'> {
	/** `ws://host:port/live/<code>` */
	relay: string;
	/** relay-web (`loadRelay(WASM_BASE)`); the link joins once it is there */
	core: RelayModule | Promise<RelayModule>;
	/** joining · connected · offline · refused (the core's status) */
	onStatus?: (s: RelayStatus) => void;
	/** #552: pick the open choice at this place (default: the page's choice buttons, reader-story.ts `pickChoice`) */
	pick?: (index: number) => boolean;
}

/** relay-web's `RelayClient` with a reader's moves */
interface ReaderCore extends RelayCoreView {
	story(json: string): boolean;
	drive(): boolean;
	choices_closed(): void;
}

/** the session's ear (reader-story.ts): what the reader fires and when its choices close go to the core */
export class ReaderLink implements SessionEar {
	private s: (RelaySocket & { core: ReaderCore }) | null = null;
	/** what fired while relay-web was still loading (null: #552 its choices closed), handed to the core when it is there */
	private early: (string | null)[] | null = [];
	private stopped = false;

	constructor(private readonly o: ReaderLinkOptions) {
		const core = o.core;
		if (core instanceof Promise)
			void core.then(
				this.begin.bind(this),
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
		this.s = new RelaySocket(core, { onStatus: (_, s) => this.o.onStatus?.(s), onOther: this.heard.bind(this) }, this.o) as RelaySocket & { core: ReaderCore };
		this.s.start();
		for (const json of this.early ?? []) void (json === null ? core.choices_closed() : core.story(json));
		this.early = null;
		this.s.run();
	}

	/** #552: the core's `{ choice }` – its `pick` performed (the core decided it); every word on to `onOther` */
	private heard(o: Record<string, never>): void {
		const choice = 'choice' in o ? (o.choice as VoteChoice) : null;
		if (choice?.pick !== undefined) (this.o.pick ?? pickChoice)(choice.pick);
		this.o.onOther?.(o);
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
	fired(event: VNEvent): void {
		const json = JSON.stringify(event);
		if (!this.s) return void this.early?.push(json);
		this.s.core.story(json);
		this.s.run();
	}

	/** #552: a `showChoices` is over (picked, or nothing to ask) – the core's place moves on */
	closed(): void {
		if (!this.s) return void this.early?.push(null);
		this.s.core.choices_closed();
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
	listenSession(link);
	return link;
}

/** out of the session: nothing is sent any more */
export function leaveSession(): void {
	listenSession(null);
	joined?.dispose();
	joined = null;
}
