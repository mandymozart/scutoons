// The reader's entities (server `/api/library/*`, `/api/events/`; agents/DATA.md §3, docs/API.md "Library"): composed
// from the content store, one request per toon page / chapter, ETag + a 60 s cache. Public answers through a header-free
// client (simple GET, no CORS preflight); `draft: true` (creators' preview) needs the session client.
import type { ApiClient } from './client.ts';
import {
	resolveCast,
	type Chapter,
	type Character,
	type CharacterRegistry,
	type Event,
	type PeopleRegistry,
	type ReaderMap,
	type RoleVocabulary,
	type SeasonOverview,
	type ToonBundle,
	type ToonCast,
	type ToonCredits,
	type ToonIndex
} from '@scutoons/content';

export interface LibraryOptions {
	/** the newest versions without status gates (role creator; uses the session client) */
	draft?: boolean;
	signal?: AbortSignal;
}

export interface LibraryApiOptions {
	/** header-free client for the public answers */
	publicClient: ApiClient;
	/** session client for `draft` (creators); default = publicClient */
	sessionClient?: ApiClient;
	/**
	 * Keep public answers in memory for this many ms (identical requests share one fetch while it runs and reuse its
	 * result afterwards). 0 / unset = no cache.
	 */
	cacheMs?: number;
}

export type LibraryApi = ReturnType<typeof createLibraryApi>;

export function createLibraryApi({ publicClient, sessionClient, cacheMs = 0 }: LibraryApiOptions) {
	const cache = new Map<string, { at: number; value: Promise<unknown> }>();

	function get<T>(path: string, options: LibraryOptions = {}): Promise<T> {
		if (options.draft) {
			const sep = path.includes('?') ? '&' : '?';
			return (sessionClient ?? publicClient).get<T>(`${path}${sep}draft=1`, { signal: options.signal });
		}
		if (cacheMs <= 0) return publicClient.get<T>(path, { signal: options.signal });
		const hit = cache.get(path);
		if (hit && Date.now() - hit.at <= cacheMs) return hit.value as Promise<T>;
		const value = publicClient.get<T>(path, { signal: options.signal });
		const entry = { at: Date.now(), value };
		cache.set(path, entry);
		// failures are not kept: the next call tries again
		value.catch(() => cache.get(path) === entry && cache.delete(path));
		return value;
	}

	const q = (locale?: string) => (locale ? `?locale=${encodeURIComponent(locale)}` : '');
	const toonPath = (toon: string) => `/library/toons/${encodeURIComponent(toon)}`;

	async function fetchCast(toon: string, options: LibraryOptions & { chapter?: number } = {}) {
		const chapter = options.chapter ? `?chapter=${options.chapter}` : '';
		return get<{ registry: CharacterRegistry | null; cast: ToonCast | null }>(`${toonPath(toon)}/cast${chapter}`, options);
	}

	return {
		/**
		 * Published toons (drafts too for creators with `draft`); with a `locale` (the reader's language) without the toons
		 * whose page is closed in it (their language status – announced ones stay)
		 */
		fetchToons: (options?: LibraryOptions, locale?: string) => get<ToonIndex[]>(`/library/toons${q(locale)}`, options),

		/** The toon page's data in one answer: index entry, settings, every season with its chapter list, the map */
		fetchToon: (toon: string, locale?: string, options?: LibraryOptions) => get<ToonBundle>(`${toonPath(toon)}${q(locale)}`, options),

		fetchSeason: (toon: string, number: number, locale?: string, options?: LibraryOptions) =>
			get<SeasonOverview>(`${toonPath(toon)}/seasons/${number}${q(locale)}`, options),

		/**
		 * A chapter with its VN scenes inlined. ApiError 403 `coming_soon` (body.chapter = the summary) / `not_public`,
		 * 404 `not_found`.
		 */
		fetchChapter: (toon: string, number: number, locale?: string, options?: LibraryOptions) =>
			get<Chapter>(`${toonPath(toon)}/chapters/${number}${q(locale)}`, options),

		fetchCast,

		/**
		 * The toon's characters in one locale (resolveCast: registry + cast → the VN's character list); [] without a cast.
		 * `fallbacks`: locales tried when a name/description is missing in `locale`.
		 */
		async fetchCharacters(toon: string, locale: string, options: LibraryOptions & { chapter?: number; fallbacks?: string[] } = {}): Promise<Character[]> {
			const { registry, cast } = await fetchCast(toon, options);
			if (!registry || !cast) return [];
			return resolveCast(registry, cast, toon, locale, { fallbacks: options.fallbacks });
		},

		/** The whole character registry (e.g. every A-pose body for the avatar picker) */
		fetchRegistry: (options?: LibraryOptions) => get<CharacterRegistry>('/library/characters', options),

		/** Credits with the people they name and the role vocabulary; ApiError 404 when the toon has none */
		fetchCredits: (toon: string, options?: LibraryOptions) =>
			get<{ credits: ToonCredits; people: PeopleRegistry | null; roles: RoleVocabulary | null }>(`${toonPath(toon)}/credits`, options),

		fetchEvents: (options?: LibraryOptions) => get<Event[]>('/events/', options),

		/**
		 * The SCUTOONS world map for a reader (agents/MAP.md): places / routes of the readable toons, names in `locale`;
		 * with `toon` the map's start `view` for that toon. ApiError 404 when no map is stored or the toon isn't readable.
		 */
		fetchMap: (locale?: string, toon?: string, options?: LibraryOptions) => {
			const params = new URLSearchParams();
			if (locale) params.set('locale', locale);
			if (toon) params.set('toon', toon);
			const query = params.toString();
			return get<ReaderMap>(`/library/map${query ? `?${query}` : ''}`, options);
		}
	};
}
