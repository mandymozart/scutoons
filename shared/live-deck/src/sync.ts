// sync.ts (#195): the browser deck's answer to the conductor's `scene.sync`. The same contract as the native deck's sync
// (deck/native/src/sync.rs), kept minimal and honest: the browser has no cache of its own to fill – it fetches the
// files from the CDN (immutable paths; the HTTP cache keeps them), so an item is "ready" once its files were fetched
// whole (the size scene-content names, when it names one). One request to `GET /library/scene-content` for the push's
// sets and characters; a set's 3D at the deck's quality (else `low`), a character's every portrait. Items are
// `set:<id>` / `character:<id>`; progress after every file, then the final state (`deck.synced`). A newer push stops
// the older one's answers.
import type { DeckSynced, SceneSync, SyncState } from '@scutoons/live';

/** a file in scene-content's answer */
type File = { path: string; size?: number };

type Content = {
	sets?: Record<string, { model?: Record<string, File> }>;
	characters?: Record<string, { portraits?: Record<string, File> }>;
	missing?: { sets?: string[]; characters?: string[] };
};

export interface SyncOptions {
	/** the library API (scene-content), no trailing slash */
	api: string;
	/** the CDN root, ending with / */
	cdn: string;
	/** the set's 3D quality this deck shows (default low) */
	quality?: string;
	/** every answer, progress and final (the link sends it as `deck.synced`) */
	report: (r: DeckSynced) => void;
	/** default: the global fetch */
	fetch?: typeof fetch;
}

/** the items a push names, in its order: `set:<id>`, then `character:<id>` */
export const itemsOf = (s: SceneSync): string[] => [...(s.sets ?? []).map((id) => `set:${id}`), ...(s.characters ?? []).map((id) => `character:${id}`)];

/** the status line (as the native HUD's): PUSH 2/5 CATHEDRAL · PUSH READY CATHEDRAL · PUSH MISSING 1 … · PUSH OFFLINE … */
export function pushLine(r: DeckSynced): string {
	const state = r.state === 'syncing' ? `${r.done ?? 0}/${r.total ?? 0}` : r.state === 'missing' ? `MISSING ${r.missing.length}` : r.state.toUpperCase();
	return `PUSH ${state} ${r.scene}`.toUpperCase();
}

/** The deck's answers to the conductor's pushes. */
export class BrowserSync {
	/** the last answer (the page's status) */
	last: DeckSynced | null = null;
	private run = 0;

	constructor(private readonly o: SyncOptions) {}

	/** Work on a push: ask scene-content once, fetch every file, report after each one. Resolves with the final answer. */
	async push(s: SceneSync): Promise<DeckSynced> {
		const run = ++this.run;
		const get = this.o.fetch ?? fetch;
		const items = itemsOf(s);
		const files = new Map<string, File[]>(items.map((i) => [i, []]));
		const why: Record<string, string> = {};
		const ready = new Set<string>();
		let done = 0;
		let bytes = 0;
		let offline = false;
		const answer = (state: SyncState): DeckSynced => {
			const missing = items.filter((i) => i in why);
			const r: DeckSynced = { scene: s.scene, state, done, total: [...files.values()].reduce((n, f) => n + f.length, 0), bytes, ready: items.filter((i) => ready.has(i)), missing };
			if (missing.length) r.why = Object.fromEntries(missing.map((i) => [i, why[i]]));
			return r;
		};
		const say = (state: SyncState) => {
			if (run !== this.run) return;
			this.last = answer(state);
			this.o.report(this.last);
		};

		let content: Content = {};
		const q = `sets=${(s.sets ?? []).map(encodeURIComponent).join(',')}&characters=${(s.characters ?? []).map(encodeURIComponent).join(',')}`;
		try {
			const r = await get(`${this.o.api}/library/scene-content?${q}`);
			if (!r.ok) throw new Error(`API HTTP ${r.status}`);
			content = (await r.json()) as Content;
		} catch (e) {
			offline = e instanceof TypeError; // fetch's network failure
			for (const i of items) why[i] = offline ? 'offline' : (e as Error).message;
		}
		if (!Object.keys(why).length) {
			const quality = this.o.quality ?? 'low';
			for (const id of s.sets ?? []) {
				const model = content.sets?.[id]?.model;
				const f = model?.[quality] ?? model?.low;
				if (f) files.get(`set:${id}`)!.push(f);
				else why[`set:${id}`] = content.missing?.sets?.includes(id) ? `set ${id} is unknown to the API` : `set ${id} has no 3D`;
			}
			for (const id of s.characters ?? []) {
				const c = content.characters?.[id];
				if (c) files.get(`character:${id}`)!.push(...Object.values(c.portraits ?? {}));
				else why[`character:${id}`] = content.missing?.characters?.includes(id) ? `character ${id} is unknown to the API` : `character ${id} is not in the answer`;
			}
		}
		say('syncing');
		for (const item of items) {
			if (item in why) continue;
			for (const f of files.get(item)!) {
				if (run !== this.run) return answer('syncing');
				try {
					const r = await get(this.o.cdn + f.path.replace(/^\//, ''));
					if (!r.ok) throw new Error(`${f.path}: HTTP ${r.status}`);
					const n = (await r.arrayBuffer()).byteLength;
					if (f.size !== undefined && n !== f.size) throw new Error(`${f.path}: ${n} bytes, the API said ${f.size}`);
					bytes += n;
				} catch (e) {
					offline ||= e instanceof TypeError;
					why[item] ??= e instanceof TypeError ? 'offline' : (e as Error).message;
				}
				done++;
				say('syncing');
			}
			if (!(item in why)) ready.add(item);
		}
		const missing = items.some((i) => i in why);
		const final: SyncState = !missing ? 'ready' : offline ? 'offline' : 'missing';
		say(final);
		return answer(final);
	}
}
