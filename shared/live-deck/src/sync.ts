// sync.ts (#195, #371): the browser deck's fetch loop for the conductor's `scene.sync`. Everything that is decided –
// the items, the request, what to fetch at the deck's quality, each file's check, the progress, the answer
// (`deck.synced`) and the PUSH line – is deck-space's push machine (push.rs), here space-web's `DeckPush`. This file
// only moves bytes: it GETs the URLs the core names (the HTTP cache keeps the CDN's immutable files) and hands each
// reply's status, length and first bytes back. A newer push stops the older one's answers.
import type { DeckSynced, SceneSync } from '@scutoons/live';

/** the wasm-bindgen API of deck/space-web's `DeckPush` */
export interface DeckPushView {
	/** a push (JSON) at the deck's 3D quality ('' = the core's): the URL to ask */
	start(sync: string, quality: string): string;
	/** the API's reply: its HTTP status (0: the network is gone) and body */
	asked(status: number, body: string): void;
	/** the next file to fetch, `[number, url]` (JSON); undefined: none left */
	next_fetch(): string | undefined;
	/** file `n` came: its HTTP status (0: the network is gone), its length and first bytes */
	received(n: number, status: number, len: number, head: Uint8Array): void;
	/** the answer now (`deck.synced`, JSON) */
	answer(): string;
	/** the answer's PUSH line */
	line(): string;
}

/** how many of a file's first bytes the core checks (deck-space `push::HEAD`) – a slice, no rule */
const HEAD = 12;

export interface SyncOptions {
	/** the set's 3D quality this deck shows ('' = the core's default) */
	quality?: string;
	/** every answer, progress and final, with its line (the link sends the answer as `deck.synced`) */
	report: (r: DeckSynced, line: string) => void;
	/** default: the global fetch */
	fetch?: typeof fetch;
}

/** The deck's answers to the conductor's pushes, through the core once it came (`useCore`). */
export class BrowserSync {
	/** the last answer and its line (the page's status) */
	last: DeckSynced | null = null;
	line = '';
	private core: DeckPushView | null = null;
	/** a push that came before the core */
	private waiting: SceneSync | null = null;
	private run = 0;

	constructor(private readonly o: SyncOptions) {}

	/** the core (space-web `DeckPush`); a push that waited for it is answered now */
	useCore(core: DeckPushView): void {
		this.core = core;
		const s = this.waiting;
		this.waiting = null;
		if (s) void this.push(s);
	}

	/** Work on a push: the request the core names, then every file it asks for, reporting after each. Resolves with
	 *  the final answer (null: the core is not there yet – the push waits for it). */
	async push(s: SceneSync): Promise<DeckSynced | null> {
		const run = ++this.run;
		const core = this.core;
		if (!core) {
			this.waiting = s;
			return null;
		}
		const get = this.o.fetch ?? fetch;
		const stale = () => run !== this.run;
		const say = (): DeckSynced => {
			const r = JSON.parse(core.answer()) as DeckSynced;
			if (!stale()) {
				this.last = r;
				this.line = core.line();
				this.o.report(r, this.line);
			}
			return r;
		};
		const url = core.start(JSON.stringify(s), this.o.quality ?? '');
		const [status, body] = await reply(get, url, async (r) => r.text());
		if (stale()) return null;
		core.asked(status, body ?? '');
		let last = say();
		for (let next = core.next_fetch(); next !== undefined; next = core.next_fetch()) {
			const [n, file] = JSON.parse(next) as [number, string];
			const [got, bytes] = await reply(get, file, async (r) => new Uint8Array(await r.arrayBuffer()));
			if (stale()) return null;
			core.received(n, got, bytes?.byteLength ?? 0, bytes?.subarray(0, HEAD) ?? new Uint8Array());
			last = say();
		}
		return last;
	}
}

/** a GET's status and body (status 0: the network is gone; no body unless 2xx) */
async function reply<T>(get: typeof fetch, url: string, body: (r: Response) => Promise<T>): Promise<[number, T | undefined]> {
	try {
		const r = await get(url);
		return r.ok ? [r.status, await body(r)] : [r.status, undefined];
	} catch {
		return [0, undefined];
	}
}
