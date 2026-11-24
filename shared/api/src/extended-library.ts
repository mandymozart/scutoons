// The extended Library's reader (#864, GET /api/library, server/docs/API.md "The extended Library"): things and files in
// one list, a thing stepped into. The server resolves everything (types, names, pictures, places, used in); this only asks.
import { ApiError, type ApiClient } from './client.ts';
import type { FileKind, FileOrigin, ItemSubtype, LibraryEntry, LibraryList, LibraryStep, ThingType } from './library-types.ts';

/** a place: This toon · Everything · Recently added · Unused · This item (`item:<address>`) · a season's cast (`cast:<season>`) */
export type LibraryPlace = 'toon' | 'all' | 'recent' | 'unused' | `item:${string}` | `cast:${number}`;

export interface LibraryQuery {
	/** the toon; none = the universe */
	toon?: string | null;
	/** a thing's (or a file's) address: the answer is it stepped into */
	at?: string | null;
	place?: LibraryPlace;
	type?: ThingType | readonly ThingType[];
	subtype?: ItemSubtype;
	kind?: FileKind | readonly FileKind[];
	from?: FileOrigin;
	locale?: string;
	cursor?: string | null;
	limit?: number;
}

/** `/library?…` for a query (only what is set; lists comma-joined) */
export function libraryPath(q: LibraryQuery = {}): string {
	const params = new URLSearchParams();
	const set = (name: string, value: string | number | readonly string[] | null | undefined) => {
		if (value === null || value === undefined) return;
		const text = typeof value === 'object' ? value.join(',') : String(value);
		if (text !== '') params.set(name, text);
	};
	set('toon', q.toon);
	set('at', q.at);
	set('place', q.place);
	set('type', q.type as string | readonly string[] | undefined);
	set('subtype', q.subtype);
	set('kind', q.kind as string | readonly string[] | undefined);
	set('from', q.from);
	set('locale', q.locale);
	set('cursor', q.cursor);
	set('limit', q.limit);
	const query = params.toString();
	return query ? `/library?${query}` : '/library';
}

/**
 * One answer: a place's page, or – with `at` – the thing stepped into. `held` = the version (ETag) already held: null when
 * it is still current (304), else the new answer.
 */
export function library(client: ApiClient, q: LibraryQuery & { at: string }, held?: string): Promise<LibraryStep | null>;
export function library(client: ApiClient, q?: LibraryQuery & { at?: null }, held?: string): Promise<LibraryList | null>;
export function library(client: ApiClient, q: LibraryQuery, held?: string): Promise<LibraryList | LibraryStep | null>;
export async function library(client: ApiClient, q: LibraryQuery = {}, held?: string): Promise<LibraryList | LibraryStep | null> {
	try {
		const response = await client.fetchRaw(libraryPath(q), held ? { headers: { 'If-None-Match': `"${held}"` } } : {});
		return (await response.json()) as LibraryList | LibraryStep;
	} catch (error) {
		if (error instanceof ApiError && error.status === 304) return null;
		throw error;
	}
}

/** Every entry of a place, page after page (`next` until null) */
export async function libraryEntries(client: ApiClient, q: Omit<LibraryQuery, 'at' | 'cursor'> = {}): Promise<LibraryEntry[]> {
	const out: LibraryEntry[] = [];
	let cursor: string | null = null;
	do {
		const page: LibraryList | null = await library(client, { ...q, at: null, cursor });
		if (!page) break;
		out.push(...page.entries);
		cursor = page.next;
	} while (cursor);
	return out;
}
