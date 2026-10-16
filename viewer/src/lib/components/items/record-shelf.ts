// The reader's records for the record player's carousel (owner 2026-10-16: "the records the reader owns stand one after
// another … the one in front is the selected one, and when you press Play it is put on"): every owned record with a 3D
// model (a `vinyl` look or a .glb) whose album the reader may play – and the request that plays one (also item-detail's
// "Abspielen"). ITEMS.md "3D item models".
import { mediaPlayback, type Album, type AlbumTrack, type Catalog, type Entry, type ItemDef, type ModelSource } from '@scutoons/content';
import { localized } from '$lib/contexts/inventory.svelte.ts';
import type { AudioRequest } from '$lib/contexts/items-ui.svelte.ts';
import { mediaQuality } from '$lib/media-quality.svelte.ts';
import { albumTrackUrl } from '$lib/protected-files.ts';

export interface ShelfRecord {
	/** the entry's id */
	key: string;
	name: string;
	model: ModelSource;
	/** plays it (AudioRequest for ui.playAlbum) */
	request: AudioRequest;
}

/** the request that plays an album from an entry (its 3D model rides along for the card's stage) */
export function albumRequest(album: Album, tracks: AlbumTrack[], entry: Entry | null, model: ModelSource | null): AudioRequest {
	return {
		tracks: tracks.map((t) => ({ id: t.id, src: t.src, title: t.title, duration: t.duration, side: t.side })),
		title: localized(album.title),
		toon: album.toon,
		entry,
		cover: album.cover ?? null,
		model,
		// the reader's media quality when the song starts (96k / 128k / 320k – owner 2026-10-16)
		sign: (i) => albumTrackUrl(album.id, tracks[i], mediaQuality.level)
	};
}

interface Shelf {
	catalog: Pick<Catalog, 'items' | 'albums'> | null | undefined;
	entries: Entry[];
	item: (id: string) => ItemDef | null | undefined;
	modelOf: (entry: Entry) => ModelSource | '';
	nameOf: (entry: Entry) => string;
}

/** the owned records the reader may play that have a 3D model, one per entry */
export function shelfOf(inventory: Shelf, member: boolean): ShelfRecord[] {
	const listener = { owned: inventory.entries.map((e) => e.item_id), member };
	const out: ShelfRecord[] = [];
	for (const entry of inventory.entries) {
		const def = inventory.item(entry.item_id);
		if (!def?.album) continue;
		const model = inventory.modelOf(entry);
		const playback = model ? mediaPlayback(inventory.catalog, def, listener) : null;
		if (!model || playback?.kind !== 'play') continue;
		out.push({ key: String(entry.id), name: inventory.nameOf(entry), model, request: albumRequest(playback.album, playback.tracks, entry, model) });
	}
	return out;
}
