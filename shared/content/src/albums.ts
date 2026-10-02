// Albums, sound carriers and players (owner 2026-10-02, agents/ITEMS.md "Players, albums, related items").
//
// The music belongs to an ALBUM (a language-neutral media entity in the item catalog, `catalog.albums`), not to an item.
// A media item (a record, a tape, a CD) says which album it carries and in which FORMAT (`album`, `format`); a PLAYER
// item (record player, tape player, CD player) says which formats it plays (`plays`). Owning a record is not enough to
// hear it in the app: the reader needs a player for its format – otherwise the item detail says which player is missing
// and offers the album download (if the album has one and `access.download` lets this reader have it). Paths are
// CDN-root paths like item icons.
import type { Catalog, ItemDef, Localized, Skill } from './items.ts';
import type { Problem } from './validate.ts';

/** The sound carriers the catalog knows; a player's `plays` lists some of them */
export const MEDIA_FORMATS = ['vinyl', 'tape', 'cd'] as const;
export type MediaFormat = (typeof MEDIA_FORMATS)[number];

export function isMediaFormat(value: unknown): value is MediaFormat {
	return typeof value === 'string' && (MEDIA_FORMATS as readonly string[]).includes(value);
}

/**
 * Who may do something with an album (owner 2026-10-02; marketing = an open download, retrieval = owners re-download what
 * they bought): `everyone` = any visitor, `members` = a reader with reading access (a member, or a device unlocked by a
 * code - the same rule as the chapters and today's download page), `owners` = a reader who owns a media item that carries
 * the album (the vinyl or the test press of "Awakening").
 */
export const MEDIA_ACCESS = ['everyone', 'members', 'owners'] as const;
export type MediaAccess = (typeof MEDIA_ACCESS)[number];

export interface AlbumAccess {
	/** who gets the album download (`download`), default `members` (the toon page's music download as before) */
	download?: MediaAccess;
	/** who may listen in the app, default `owners`; a player for the format is always needed */
	listen?: MediaAccess;
}

export const DEFAULT_ALBUM_ACCESS: Required<AlbumAccess> = { download: 'members', listen: 'owners' };

/** The reader asking: the item ids they own and whether they have reading access (member or unlocked device) */
export interface Listener {
	owned: Iterable<string>;
	member: boolean;
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
	/** who may download / listen (per album: one zip and one track list, whichever of its media items someone owns) */
	access?: AlbumAccess;
}

/** The album's access with the defaults filled in */
export function albumAccess(album: Pick<Album, 'access'> | null | undefined): Required<AlbumAccess> {
	return { ...DEFAULT_ALBUM_ACCESS, ...(album?.access ?? {}) };
}

/** The catalog's media items that carry an album (in catalog order) */
export function carriersOf(catalog: Pick<Catalog, 'items'> | null | undefined, album: Pick<Album, 'id'>): ItemDef[] {
	return (catalog?.items ?? []).filter((i) => i.album === album.id && isMediaFormat(i.format));
}

/** Does the reader own a media item that carries this album? */
export function ownsAlbum(catalog: Pick<Catalog, 'items'> | null | undefined, album: Pick<Album, 'id'>, owned: Iterable<string>): boolean {
	const have = new Set(owned);
	return carriersOf(catalog, album).some((i) => have.has(i.id));
}

function allows(level: MediaAccess, member: boolean, owner: boolean): boolean {
	return level === 'everyone' || (level === 'members' && (member || owner)) || owner;
}

/** The album download path if this reader may have it, else null */
export function albumDownload(catalog: Pick<Catalog, 'items'> | null | undefined, album: Album | null | undefined, who: Listener): string | null {
	if (!album?.download) return null;
	return allows(albumAccess(album).download, who.member, ownsAlbum(catalog, album, who.owned)) ? album.download : null;
}

/** May this reader listen to the album in the app (a player is still needed)? */
export function mayListen(catalog: Pick<Catalog, 'items'> | null | undefined, album: Album, who: Listener): boolean {
	return allows(albumAccess(album).listen, who.member, ownsAlbum(catalog, album, who.owned));
}

/** The album whose download is this CDN-root path (the season's music zip -> its album and its access) */
export function albumByDownload(catalog: Pick<Catalog, 'albums'> | null | undefined, path: string | null | undefined): Album | undefined {
	if (!path) return undefined;
	return (catalog?.albums ?? []).find((a) => a.download === path);
}

/** What the toon page and `/<toon>/download` show for a season's music (agents/ASSETS.md "Albums and downloads") */
export interface SeasonMusic {
	/** the album (undefined: an old season whose zip is no album's download) */
	album?: Album;
	/** the related item that carries the album (the record), when the season lists one */
	carrier?: string;
	/** CDN-root path of the zip */
	download: string | null;
	/** CDN-root path of the decorative record image (the album's cover; older seasons: `download.image`) */
	image: string | null;
}

/**
 * The season's music, derived from its related items' album (owner 2026-10-02: the season's own `download.music` /
 * `download.image` are redundant now that albums exist): the first related item that carries an album wins; a season
 * without one falls back to its old `download` fields (paths in the toon's folder), so older content keeps working.
 * null = the season has no music.
 */
export function seasonMusic(
	catalog: Pick<Catalog, 'items' | 'albums'> | null | undefined,
	season: { related_items?: string[]; download?: { music?: string; image?: string } | null } | null | undefined,
	toon: string
): SeasonMusic | null {
	if (!season) return null;
	const items = catalog?.items ?? [];
	for (const id of season.related_items ?? []) {
		const album = albumOf(catalog, items.find((i) => i.id === id));
		if (album) {
			return {
				album,
				carrier: id,
				download: album.download ?? (season.download?.music ? `${toon}/${season.download.music}` : null),
				image: album.cover ?? (season.download?.image ? `${toon}/${season.download.image}` : null)
			};
		}
	}
	const music = season.download?.music;
	if (!music) return null;
	const album = albumByDownload(catalog, `${toon}/${music}`);
	return { album, download: `${toon}/${music}`, image: album?.cover ?? (season.download?.image ? `${toon}/${season.download.image}` : null) };
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
 * What a media item offers this reader (the item detail):
 * - `play`: the reader owns a player for the item's format (`player`) and may listen - play the album's `tracks`;
 * - `needs_player`: no player for the format - `players` are the catalog's players for it (the first one is named:
 *   "Du brauchst einen Plattenspieler"), `download` the album download if this reader may have it (`access.download`);
 * - null: not a playable media item (no album / format), or nothing to offer.
 * The old `play_audio { src }` skill (before 2026-10-02) still reads as a one-track album that plays without a player.
 */
export type Playback =
	| { kind: 'play'; album: Album; tracks: AlbumTrack[]; player: ItemDef | null; download: string | null }
	| { kind: 'needs_player'; album: Album; format: MediaFormat; players: ItemDef[]; download: string | null };

export function mediaPlayback(catalog: Pick<Catalog, 'items' | 'albums'> | null | undefined, item: ItemDef | null | undefined, who: Listener): Playback | null {
	if (!item) return null;
	const album = albumOf(catalog, item);
	if (album && isMediaFormat(item.format)) {
		const have = new Set(who.owned);
		const listener = { owned: have, member: who.member };
		const players = playersFor(catalog, item.format);
		const player = players.find((p) => have.has(p.id));
		const download = albumDownload(catalog, album, listener);
		if (player && album.tracks.length && mayListen(catalog, album, listener)) return { kind: 'play', album, tracks: album.tracks, player, download };
		if (player) return download ? { kind: 'needs_player', album, format: item.format, players: [], download } : null;
		if (!album.tracks.length && !download) return null;
		return { kind: 'needs_player', album, format: item.format, players, download };
	}
	const legacy = legacyPlayAudio(item.skills);
	if (legacy) {
		const one: Album = { id: item.id, toon: item.toon, title: item.name, tracks: [{ id: item.id, title: '', src: legacy }] };
		return { kind: 'play', album: one, tracks: one.tracks, player: null, download: null };
	}
	return null;
}

/**
 * A player's albums (the player's item detail): every album carried by a media item of a format the player plays, with
 * the carrier (the owned one first, else the first in the catalog - the one to buy), whether the reader owns it, may
 * listen (`access.listen`) and may download.
 */
export interface PlayerAlbum {
	album: Album;
	carrier: ItemDef;
	owned: boolean;
	listen: boolean;
	download: string | null;
}

export function playerAlbums(catalog: Pick<Catalog, 'items' | 'albums'> | null | undefined, player: ItemDef | null | undefined, who: Listener): PlayerAlbum[] {
	const plays = player?.plays;
	if (!Array.isArray(plays) || !plays.length) return [];
	const have = new Set(who.owned);
	const listener = { owned: have, member: who.member };
	const out: PlayerAlbum[] = [];
	for (const album of catalog?.albums ?? []) {
		const carriers = carriersOf(catalog, album).filter((i) => plays.includes(i.format as MediaFormat));
		if (!carriers.length) continue;
		const mine = carriers.find((i) => have.has(i.id));
		out.push({
			album,
			carrier: mine ?? carriers[0],
			owned: !!mine,
			listen: album.tracks.length > 0 && mayListen(catalog, album, listener),
			download: albumDownload(catalog, album, listener)
		});
	}
	return out;
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
		if (album.access !== undefined) {
			if (!isObj(album.access)) add('error', [...path, 'access'], `${album.id}: access must be { download?, listen? }`);
			else for (const key of Object.keys(album.access)) {
				if (key !== 'download' && key !== 'listen') add('error', [...path, 'access', key], `${album.id}: unknown access "${key}" (download, listen)`);
				else if (!(MEDIA_ACCESS as readonly string[]).includes(album.access[key])) add('error', [...path, 'access', key], `${album.id}: access.${key} must be one of ${MEDIA_ACCESS.join(', ')}`);
			}
		}
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
