// The stored stills (server docs/API.md "Thumbnails", #842): asked for in rounds as cards and tiles appear, uploaded when
// this browser made one. With the API missing or failing nothing shows and nothing is sent – the icons stay.
import { createThumbsApi, THUMB_HEIGHT, THUMB_WIDTH, type Thumb } from '@scutoons/api';
import { api } from '$lib/api.ts';
import { shared } from '$lib/utils/shared.ts';
import { centreCut } from './stills.ts';

const thumbsApi = createThumbsApi(api);
/** lookups wait this long to be asked together */
const GATHER_MS = 60;

/** a picture cut to the still's shape and encoded as WebP; null where the browser cannot (no WebP encoder) */
export async function toStill(source: CanvasImageSource, w: number, h: number): Promise<Blob | null> {
	const { sx, sy, sw, sh } = centreCut(w, h);
	if (!sw || !sh) return null;
	const canvas = document.createElement('canvas');
	canvas.width = THUMB_WIDTH;
	canvas.height = THUMB_HEIGHT;
	const ctx = canvas.getContext('2d');
	if (!ctx) return null;
	ctx.imageSmoothingQuality = 'high';
	ctx.drawImage(source, sx, sy, sw, sh, 0, 0, THUMB_WIDTH, THUMB_HEIGHT);
	const blob = await new Promise<Blob | null>((done) => canvas.toBlob(done, 'image/webp', 0.82));
	return blob?.type === 'image/webp' ? blob : null;
}

/** a picture at a URL (a data URL too) as a still */
export async function urlToStill(url: string): Promise<Blob | null> {
	const img = new Image();
	img.crossOrigin = 'anonymous';
	img.src = url;
	try {
		await img.decode();
	} catch {
		return null;
	}
	return toStill(img, img.naturalWidth, img.naturalHeight);
}

class Thumbs {
	/** the stills the server has, by key */
	map = $state.raw<Record<string, Thumb>>({});
	/** keys the server answered for (with or without a still) */
	looked = $state.raw<ReadonlySet<string>>(new Set());
	/** null: not asked yet; false: no thumbnails on this server (before its migration) or it failed */
	available = $state<boolean | null>(null);
	#pending = new Set<string>();
	#asked = new Set<string>();
	#timer: ReturnType<typeof setTimeout> | null = null;

	/** these keys are on screen: asked for in the next round (each once per visit) */
	want(keys: readonly string[]) {
		if (this.available === false) return;
		for (const key of keys) if (!this.#asked.has(key)) this.#pending.add(key);
		if (this.#pending.size && !this.#timer) this.#timer = setTimeout(() => void this.#ask(), GATHER_MS);
	}

	/** a key asked again (a still just went up elsewhere, a card shows anew) */
	forget(key: string) {
		this.#asked.delete(key);
	}

	async #ask() {
		this.#timer = null;
		const keys = [...this.#pending];
		this.#pending.clear();
		for (const key of keys) this.#asked.add(key);
		try {
			const answer = await thumbsApi.lookup(keys);
			this.available = answer.available;
			if (!answer.available) return;
			this.map = { ...this.map, ...answer.thumbs };
		} catch {
			this.available = false;
			return;
		}
		this.looked = new Set([...this.looked, ...keys]);
	}

	get(key: string): Thumb | undefined {
		return this.map[key];
	}

	/** a still this browser made goes up; null when the server did not take it */
	async put(key: string, still: Blob, stamp?: string): Promise<Thumb | null> {
		if (this.available === false) return null;
		try {
			const { thumb } = await thumbsApi.put(key, still, stamp);
			this.available = true;
			this.map = { ...this.map, [key]: thumb };
			this.looked = new Set([...this.looked, key]);
			return thumb;
		} catch {
			return null;
		}
	}
}

export const thumbs = shared('creator-thumbs', () => new Thumbs());
