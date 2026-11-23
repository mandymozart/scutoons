// Thumbnails (server docs/API.md "Thumbnails", #842): one 264 × 168 WebP still per key – an asset (`asset:<id>`), an
// address (`item:camera`, `scene:<id>` …) or the member's own workspace snapshot (`workspace:<id>`, private).
import type { ApiClient } from './client.ts';
import { toQuery } from './admin.ts';

/** the still's shape: the deck editor's STILL (deck/editor/src/library.rs) */
export const THUMB_WIDTH = 264;
export const THUMB_HEIGHT = 168;
/** keys per lookup the server takes */
export const THUMB_LOOKUP_MAX = 100;

export interface Thumb {
	url: string;
	/** what it was taken from: an asset's version number, an address's updated_at … */
	stamp: string;
	/** the thing changed since: make it again */
	stale: boolean;
	origin: 'client' | 'server';
	taken_at: string | null;
}

export interface ThumbLookup {
	/** false: the server has no thumbnails yet (before its migration) */
	available: boolean;
	thumbs: Record<string, Thumb>;
}

export const assetThumbKey = (id: string) => `asset:${id}`;
export const workspaceThumbKey = (id: string) => `workspace:${id}`;

export function createThumbsApi(api: ApiClient) {
	return {
		/** the stored stills of these keys (missing ones left out); more than THUMB_LOOKUP_MAX are asked in rounds */
		async lookup(keys: readonly string[]): Promise<ThumbLookup> {
			const unique = [...new Set(keys)];
			const out: ThumbLookup = { available: true, thumbs: {} };
			for (let i = 0; i < unique.length; i += THUMB_LOOKUP_MAX) {
				const part = unique.slice(i, i + THUMB_LOOKUP_MAX);
				const answer = await api.get<{ status: string } & ThumbLookup>(`/thumbs${toQuery({ keys: part.join(',') })}`);
				if (!answer.available) return { available: false, thumbs: {} };
				Object.assign(out.thumbs, answer.thumbs);
			}
			return out;
		},
		/** a WebP of exactly THUMB_WIDTH × THUMB_HEIGHT; `stamp`: what it shows (an asset's version number is the server's default) */
		put: (key: string, webp: Blob, stamp?: string) => api.put<{ status: string; thumb: Thumb & { key: string } }>(`/thumbs${toQuery({ key, stamp })}`, webp)
	};
}

export type ThumbsApi = ReturnType<typeof createThumbsApi>;
