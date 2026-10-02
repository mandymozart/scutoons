// Albums, sound carriers and players (owner 2026-10-02, agents/ITEMS.md "Players, albums, related items").
//
// The music belongs to an ALBUM (a language-neutral media entity in the item catalog, `catalog.albums`), not to an item.
// A media item (a record, a tape, a CD) says which album it carries and in which FORMAT (`album`, `format`); a PLAYER
// item (record player, tape player, CD player) says which formats it plays (`plays`). Owning a record is not enough to
// hear it in the app: the reader needs a player for its format – otherwise the item detail says which player is missing
// and offers the album download (if the album has one). Paths are CDN-root paths like item icons.
import type { Catalog, ItemDef, Localized, Skill } from './items.ts';
import type { Problem } from './validate.ts';

/** The sound carriers the catalog knows; a player's `plays` lists some of them */
export const MEDIA_FORMATS = ['vinyl', 'tape', 'cd'] as const;
export type MediaFormat = (typeof MEDIA_FORMATS)[number];

export function isMediaFormat(value: unknown): value is MediaFormat {
	return typeof value === 'string' && (MEDIA_FORMATS as readonly string[]).includes(value);
}

export interface AlbumTrack {
	/** stable, unique within the album */
	id: string;
	/** the song's name (proper names are the same in every language) */
	title: string;
	/** CDN-root path of the streamable file (`<toon>/audio/music/<name>.128.mp3`) */
	src: string;
	/** seconds, optional */
	duration?: number;
}

/** An album: the music a media item carries (language-neutral; only the title is text) */
export interface Album {
	/** stable, snake_case (`awakening`) */
	id: string;
	/** the toon it belongs to (null = cross-toon) */
	toon: string | null;
	title: Localized;
	tracks: AlbumTrack[];
	/** CDN-root path of the album download (a zip), if there is one – reachable without a player */
	download?: string | null;
	/** CDN-root path of a cover image, optional */
	cover?: string | null;
}

/** The album an item carries (`item.album`), or undefined */
export function albumOf(catalog: Pick<Catalog, 'albums'> | null | undefined, item: Pick<ItemDef, 'album'> | null | undefined): Album | undefined {
	if (!item?.album) return undefined;
	return (catalog?.albums ?? []).find((a) => a.id === item.album);
}

/** The catalog's players for a format (in catalog order) */
export function playersFor(catalog: Pick<Catalog, 'items'> | null | undefined, format: MediaFormat): ItemDef[] {
	return (catalog?.items ?? []).filter((i) => Array.isArray(i.plays) && i.plays.includes(format));
}

/**
 * What "use" does for a media item, given the item ids the reader owns:
 * - `play`: the reader owns a player for the item's format (`player`) – play the album's `tracks`;
 * - `needs_player`: no player for the format – `players` are the catalog's players for it (the first one is named:
 *   "Du brauchst einen Plattenspieler"), `download` the album's download path if it has one;
 * - null: not a playable media item (no album / format, or the album has no tracks and no download).
 * The old `play_audio { src }` skill (before 2026-10-02) still reads as a one-track album that plays without a player.
 */
export type Playback =
	| { kind: 'play'; album: Album; tracks: AlbumTrack[]; player: ItemDef | null }
	| { kind: 'needs_player'; album: Album; format: MediaFormat; players: ItemDef[]; download: string | null };

export function mediaPlayback(catalog: Pick<Catalog, 'items' | 'albums'> | null | undefined, item: ItemDef | null | undefined, owned: Iterable<string>): Playback | null {
	if (!item) return null;
	const album = albumOf(catalog, item);
	if (album && isMediaFormat(item.format)) {
		const have = new Set(owned);
		const players = playersFor(catalog, item.format);
		const player = players.find((p) => have.has(p.id));
		if (player && album.tracks.length) return { kind: 'play', album, tracks: album.tracks, player };
		if (player) return null;
		if (!album.tracks.length && !album.download) return null;
		return { kind: 'needs_player', album, format: item.format, players, download: album.download ?? null };
	}
	const legacy = legacyPlayAudio(item.skills);
	if (legacy) {
		const one: Album = { id: item.id, toon: item.toon, title: item.name, tracks: [{ id: item.id, title: '', src: legacy }] };
		return { kind: 'play', album: one, tracks: one.tracks, player: null };
	}
	return null;
}

/** The `src` of an old `play_audio` skill (read gracefully; the catalog no longer uses it) */
export function legacyPlayAudio(skills: Skill[] | null | undefined): string | null {
	for (const s of skills ?? []) {
		const any = s as { type?: string; src?: unknown };
		if (any.type === 'play_audio' && typeof any.src === 'string' && any.src) return any.src;
	}
	return null;
}

const isObj = (v: unknown): v is Record<string, any> => typeof v === 'object' && v !== null && !Array.isArray(v);
const AUDIO = /\.(mp3|m4a|aac|ogg|opus|wav|flac)$/i;

/**
 * Albums, formats and players in the catalog (part of `validateCatalog`): album ids unique, tracks with id + audio src,
 * a media item's `album` known and its `format` one of `MEDIA_FORMATS` (both or neither), `plays` only known formats,
 * a format some media item uses without any player = a warning (nobody could ever play it).
 */
export function validateAlbums(catalog: Record<string, any>, add: (level: Problem['level'], path: (string | number)[], message: string) => void) {
	const albums = new Map<string, any>();
	if (catalog.albums !== undefined && !Array.isArray(catalog.albums)) add('error', ['albums'], 'albums must be a list of { id, toon, title, tracks, download?, cover? }');
	(Array.isArray(catalog.albums) ? catalog.albums : []).forEach((album: any, i: number) => {
		const path = ['albums', i];
		if (!isObj(album) || typeof album.id !== 'string' || !/^[a-z0-9][a-z0-9_]*$/.test(album.id)) return add('error', path, `album ${i + 1} needs a snake_case id`);
		if (albums.has(album.id)) add('error', [...path, 'id'], `album id "${album.id}" is used twice`);
		albums.set(album.id, album);
		if (!isObj(album.title) || !Object.values(album.title).some((t) => typeof t === 'string' && t.trim())) add('error', [...path, 'title'], `${album.id}: title must be { <locale>: text }`);
		if (!Array.isArray(album.tracks)) add('error', [...path, 'tracks'], `${album.id}: tracks must be a list of { id, title, src }`);
		else {
			const ids = new Set<string>();
			album.tracks.forEach((t: any, j: number) => {
				const tp = [...path, 'tracks', j];
				if (!isObj(t) || typeof t.id !== 'string' || !t.id) return add('error', tp, `${album.id}: track ${j + 1} needs an id`);
				if (ids.has(t.id)) add('error', [...tp, 'id'], `${album.id}: track id "${t.id}" is used twice`);
				ids.add(t.id);
				if (typeof t.title !== 'string') add('error', [...tp, 'title'], `${album.id}/${t.id}: title must be a string`);
				if (typeof t.src !== 'string' || !AUDIO.test(t.src)) add('error', [...tp, 'src'], `${album.id}/${t.id}: src must be an audio path (.mp3 …)`);
				if (t.duration !== undefined && !(typeof t.duration === 'number' && t.duration > 0)) add('error', [...tp, 'duration'], `${album.id}/${t.id}: duration must be seconds > 0`);
			});
			if (!album.tracks.length && !album.download) add('warning', [...path, 'tracks'], `${album.id}: no tracks and no download`);
		}
		if (album.download != null && (typeof album.download !== 'string' || !/\.zip$/i.test(album.download))) add('error', [...path, 'download'], `${album.id}: download must be a .zip path`);
		if (album.cover != null && (typeof album.cover !== 'string' || !/\.(jpe?g|png|webp)$/i.test(album.cover))) add('error', [...path, 'cover'], `${album.id}: cover must be an image path`);
	});
	const used = new Set<string>();
	const played = new Set<string>();
	(Array.isArray(catalog.items) ? catalog.items : []).forEach((item: any, i: number) => {
		if (!isObj(item)) return;
		const path = ['items', i];
		if (item.plays != null) {
			if (!Array.isArray(item.plays) || !item.plays.length || item.plays.some((f: unknown) => !isMediaFormat(f))) add('error', [...path, 'plays'], `${item.id}: plays must be a list of ${MEDIA_FORMATS.join(', ')}`);
			else for (const f of item.plays) played.add(f);
		}
		const hasAlbum = item.album != null;
		const hasFormat = item.format != null;
		if (hasFormat && !isMediaFormat(item.format)) add('error', [...path, 'format'], `${item.id}: format must be one of ${MEDIA_FORMATS.join(', ')}`);
		if (hasAlbum && (typeof item.album !== 'string' || !albums.has(item.album))) add('error', [...path, 'album'], `${item.id}: unknown album "${item.album}" (catalog.albums)`);
		if (hasAlbum !== hasFormat) add('error', [...path, hasAlbum ? 'format' : 'album'], `${item.id}: a media item needs both album and format`);
		if (hasAlbum && item.category !== 'media') add('warning', [...path, 'category'], `${item.id}: an item with an album should be category "media"`);
		if (hasFormat && isMediaFormat(item.format)) used.add(item.format);
		if (legacyPlayAudio(item.skills) && hasAlbum) add('warning', [...path, 'skills'], `${item.id}: play_audio is replaced by album + format – remove the skill`);
	});
	for (const f of used) if (!played.has(f)) add('warning', ['items'], `no player plays "${f}" (an item with plays: ["${f}"])`);
}

/**
 * `related_items` of a season or chapter (language-neutral: the same list in every locale): catalog item ids, ordered.
 * `known` = the catalog's item ids (no check without it).
 */
export function checkRelatedItems(value: unknown, known: Iterable<string> | undefined, add: (level: Problem['level'], path: (string | number)[], message: string) => void) {
	if (value === undefined) return;
	if (!Array.isArray(value) || value.some((id) => typeof id !== 'string' || !id)) return add('error', ['related_items'], 'related_items must be a list of item ids');
	const ids = known ? new Set(known) : null;
	value.forEach((id: string, i: number) => {
		if (value.indexOf(id) !== i) add('warning', ['related_items', i], `"${id}" is listed twice`);
		else if (ids && !ids.has(id)) add('warning', ['related_items', i], `"${id}" is not in the item catalog`);
	});
}
