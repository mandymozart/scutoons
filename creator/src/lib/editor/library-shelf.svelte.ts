// The Library's list, one for the whole creator: the server's answer (GET /api/library, `@scutoons/api` library()) for
// the place, the type and From chosen, held per query with its version (a second look asks with If-None-Match), and the
// thing stepped into (`?at=`). The server resolves what is listed; this holds the choices and what came back.
import { library as libraryAnswer, libraryPath, type FileKind, type FileOrigin, type LibraryEntry, type LibraryLevel, type LibraryQuery, type LibraryStep } from '@scutoons/api';
import { api } from '$lib/api.ts';
import { getLocale } from '$lib/paraglide/runtime.js';
import { shared } from '$lib/utils/shared.ts';
import { placeAvailable, placeQuery, startPlace, type PlaceContext, type ShelfGrouping, type ShelfPlace, type ShelfSort } from './library-list.ts';

const VIEW_KEY = 'sc-creator-library-shelf';

interface Held {
	version: string;
	entries: LibraryEntry[];
}

/** a toon's seasons (its step's levels) and which season each chapter hangs under */
export interface ToonSeasons {
	seasons: LibraryLevel[];
	seasonOf: Record<string, string>;
}

async function every(q: LibraryQuery, held?: string): Promise<Held | null> {
	const first = await libraryAnswer(api, { ...q, at: null }, held);
	if (!first) return null;
	const entries = [...first.entries];
	for (let next = first.next; next; ) {
		const page = await libraryAnswer(api, { ...q, at: null, cursor: next });
		if (!page) break;
		entries.push(...page.entries);
		next = page.next;
	}
	return { version: first.version, entries };
}

class Shelf {
	/** the place chosen and the front item it was chosen for (a new front item starts on its own place) */
	#place = $state<{ place: ShelfPlace; for: string | null } | null>(null);
	/** the cast chosen in By cast (`season:<toon>:<n>`); none: the open item's season */
	castSeason = $state<string | null>(null);
	choice = $state('all');
	from = $state<FileOrigin | null>(null);
	query = $state('');
	sort = $state<ShelfSort>('name');
	grouping = $state<ShelfGrouping>('none');

	entries = $state.raw<LibraryEntry[]>([]);
	status = $state<'idle' | 'loading' | 'ready' | 'error'>('idle');
	error = $state('');
	/** the query whose answer `entries` holds */
	shown = $state<string | null>(null);
	#held = new Map<string, Held>();
	#asking: string | null = null;

	/** the trail of things stepped into (the crumb), the chosen level per column of the last */
	trail = $state<string[]>([]);
	chosen = $state<string[]>([]);
	step = $state.raw<LibraryStep | null>(null);
	/** the names of the things stepped into (the crumb) */
	names = $state.raw<Record<string, string>>({});
	stepStatus = $state<'idle' | 'loading' | 'ready' | 'error'>('idle');

	toons = $state.raw<Record<string, ToonSeasons>>({});
	#toonsAsked = new Set<string>();
	/** the toons an open thing belongs to (its entry's `toons`), by address */
	#openToons = $state.raw<Record<string, string[]>>({});
	#openAsked = new Set<string>();

	constructor() {
		try {
			const v = JSON.parse(localStorage.getItem(VIEW_KEY) ?? 'null') ?? {};
			if (['name', 'recent', 'uses'].includes(v.sort)) this.sort = v.sort;
			if (['none', 'type'].includes(v.grouping)) this.grouping = v.grouping;
		} catch {
			// storage blocked or no window: the start view
		}
	}

	remember() {
		try {
			localStorage.setItem(VIEW_KEY, JSON.stringify({ sort: this.sort, grouping: this.grouping }));
		} catch {
			// storage blocked: this visit only
		}
	}

	/** the season whose cast "This season's cast" means: the chosen one, else the open item's, else the toon's first */
	seasonFor(toon: string | null, open: string | null): string | null {
		if (this.castSeason) return this.castSeason;
		if (!toon) return null;
		const parts = this.toons[toon];
		if (open?.startsWith('season:')) return open;
		const chapter = open?.match(/^(?:chapter|storyboard):([^:]+):(\d+)/);
		if (chapter && parts?.seasonOf[`chapter:${chapter[1]}:${chapter[2]}`]) return parts.seasonOf[`chapter:${chapter[1]}:${chapter[2]}`];
		return parts?.seasons[0]?.address ?? null;
	}

	place(ctx: PlaceContext): ShelfPlace {
		const p = this.#place;
		return p && p.for === ctx.open && placeAvailable(p.place, ctx) ? p.place : startPlace(ctx);
	}

	setPlace(place: ShelfPlace, open: string | null, castSeason: string | null = null) {
		this.#place = { place, for: open };
		this.castSeason = castSeason;
		this.leave();
	}

	/** the server's query for what is chosen */
	queryFor(ctx: PlaceContext, pickKinds: readonly FileKind[] | null): LibraryQuery {
		return placeQuery(this.place(ctx), ctx, this.choice, this.from, pickKinds, getLocale());
	}

	/** show the answer for `q`: held at once, asked again with its version; a failed ask waits for the next change */
	async show(q: LibraryQuery, again = false) {
		const key = libraryPath(q);
		if (!again && (key === this.#asking || (key === this.shown && this.status !== 'idle'))) return;
		this.#asking = key;
		const held = this.#held.get(key);
		this.shown = key;
		if (held) ((this.entries = held.entries), (this.status = 'ready'));
		else this.status = 'loading';
		try {
			const answer = await every(q, held?.version);
			if (answer) this.#held.set(key, answer);
			if (this.shown !== key) return;
			this.entries = (answer ?? held)!.entries;
			this.status = 'ready';
		} catch (e) {
			if (this.shown !== key) return;
			this.error = e instanceof Error ? e.message : String(e);
			this.status = 'error';
		} finally {
			if (this.#asking === key) this.#asking = null;
		}
	}

	/** something changed on the server (an upload, a still): every held answer is asked again */
	refresh(q: LibraryQuery) {
		void this.show(q, true);
		if (this.trail.length) void this.#stepTo(this.trail.at(-1)!);
	}

	/** the toon an open thing belongs to, as the server answers it (its first); asked once per address */
	toonOf(address: string | null): string | null {
		if (!address) return null;
		const known = this.#openToons[address];
		if (known) return known[0] ?? null;
		if (!this.#openAsked.has(address)) {
			this.#openAsked.add(address);
			libraryAnswer(api, { at: address, locale: getLocale() })
				.then((step) => step && (this.#openToons = { ...this.#openToons, [address]: step.at.toons }))
				.catch(() => undefined);
		}
		return null;
	}

	/** a toon's seasons, read once (its step) */
	loadToon(toon: string) {
		if (this.#toonsAsked.has(toon)) return;
		this.#toonsAsked.add(toon);
		libraryAnswer(api, { at: `toon:${toon}`, locale: getLocale() })
			.then((step) => {
				if (!step) return;
				const seasons = step.levels.filter((l) => l.level === 'season');
				const seasonOf = Object.fromEntries(step.levels.filter((l) => l.level === 'chapter' && l.parent.startsWith('season:')).map((l) => [l.address, l.parent]));
				this.toons = { ...this.toons, [toon]: { seasons, seasonOf } };
			})
			.catch(() => this.#toonsAsked.delete(toon));
	}

	/** step into a thing (from the list: a new trail; from a thing stepped into: one deeper) */
	stepInto(address: string, deeper = false) {
		this.trail = deeper ? [...this.trail, address] : [address];
		this.chosen = [];
		void this.#stepTo(address);
	}

	/** back to the trail's `i`-th thing (its levels as they were left: the top) */
	backTo(i: number) {
		if (i < 0) return this.leave();
		this.trail = this.trail.slice(0, i + 1);
		this.chosen = [];
		void this.#stepTo(this.trail[i]);
	}

	leave() {
		this.trail = [];
		this.chosen = [];
		this.step = null;
		this.stepStatus = 'idle';
	}

	/** a level chosen in column `i`: the columns after it follow it */
	choose(i: number, key: string) {
		this.chosen = [...this.chosen.slice(0, i), key];
	}

	async #stepTo(address: string) {
		this.stepStatus = 'loading';
		try {
			const step = await libraryAnswer(api, { at: address, locale: getLocale() });
			if (this.trail.at(-1) !== address) return;
			if (step) ((this.step = step), (this.names = { ...this.names, [address]: step.at.name }));
			this.stepStatus = 'ready';
		} catch {
			if (this.trail.at(-1) === address) this.stepStatus = 'error';
		}
	}
}

export const shelf = shared('creator-library-shelf', () => new Shelf());
