// stage-assets.ts (#465): what a stage asks the page to fetch and decode – a set's .glb and its images, a head's sprites.
// The asking and the use are the core's (deck/space-web); here only the fetches and the browser's image decoder.
import { fetchProof, fetchShowLink, type DeckLinkView, type SignedIn, type SignedView } from './deck-key.ts';
export { fetchSceneAsks, joinScene, type PublishedView } from './published.ts';
export interface SetView {
	give_set(glb: Uint8Array, images: unknown[]): void;
	backend(): string;
	portrait_requests(): unknown[];
	give_portrait(slot: number, width: number, height: number, rgba: Uint8Array): void;
	no_portrait(slot: number, why: string): void;
}

export interface ChapterView {
	give_chapter(i: number, text: string): void;
	give_slice(i: number, width: number, height: number, rgba: Uint8Array): void;
	no_slice(i: number, why: string): void;
}

/** a stage's view and whether it is gone */
export interface StageHost {
	view: SetView;
	stopped: boolean;
}

interface SetFiles {
	api: string;
	cdn: string;
	quality?: 'low' | 'medium' | 'high';
	onStatus?: (s: string) => void;
}


/** an image's bytes → RGBA8, decoded by the browser */
export async function decodeImage(bytes: Uint8Array | Blob, type = 'image/webp'): Promise<{ width: number; height: number; rgba: Uint8Array }> {
	const blob = bytes instanceof Blob ? bytes : new Blob([bytes as BlobPart], { type });
	const bmp = await createImageBitmap(blob, { colorSpaceConversion: 'none', premultiplyAlpha: 'none' });
	const { width, height } = bmp;
	const canvas = typeof OffscreenCanvas !== 'undefined' ? new OffscreenCanvas(width, height) : Object.assign(document.createElement('canvas'), { width, height });
	const g = canvas.getContext('2d') as CanvasRenderingContext2D | OffscreenCanvasRenderingContext2D | null;
	if (!g) throw new Error('no 2d canvas to decode an image');
	g.drawImage(bmp, 0, 0);
	bmp.close();
	const data = g.getImageData(0, 0, width, height).data;
	return { width, height, rgba: new Uint8Array(data.buffer, data.byteOffset, data.byteLength) };
}

/** the set `set` from scene-content and the CDN, its images decoded by the browser, into the view */
export async function fetchSet(host: StageHost, encodedImages: (glb: Uint8Array) => unknown[], sniff: (b: Uint8Array) => string, o: SetFiles, set: string) {
	const quality = o.quality ?? 'low';
	const r = await fetch(`${o.api}/library/scene-content?sets=${encodeURIComponent(set)}`);
	const model = ((await r.json()) as { sets?: Record<string, { model?: Record<string, { path: string }> }> }).sets?.[set]?.model;
	const file = model?.[quality] ?? model?.low;
	if (!file) throw new Error(`no ${set} in scene-content`);
	o.onStatus?.(`3D: loading ${set}`);
	const glb = new Uint8Array(await (await fetch(o.cdn + file.path)).arrayBuffer());
	const encoded = encodedImages(glb);
	const images: unknown[] = [];
	for (let i = 0; i + 1 < encoded.length; i += 2) {
		const bytes = encoded[i + 1] as Uint8Array;
		try {
			const img = await decodeImage(bytes, sniff(bytes));
			images.push(encoded[i], img.width, img.height, img.rgba);
		} catch (e) {
			console.warn(`space: image ${String(encoded[i])} not decoded`, e);
		}
	}
	if (host.stopped) return;
	host.view.give_set(glb, images);
	o.onStatus?.(`3D ${host.view.backend()}`);
}

/** a scene-content entry (`characters.<id>`, `items.<id>`): its fields – a file, or files by mood */
export type ContentEntry = Record<string, unknown>;

/** the pictures the stage asked for – slot, kind, id, field, mood (deck/space-web `portrait_asks`) – fetched and decoded (each once) */
export function askPortraits(host: StageHost, cdn: string, entryOf: (kind: string, id: string) => Promise<ContentEntry>) {
	const a = host.view.portrait_requests();
	for (let i = 0; i + 4 < a.length; i += 5) void givePortrait(host, cdn, entryOf, a[i] as number, a[i + 1] as string, a[i + 2] as string, a[i + 3] as string, a[i + 4] as string);
}

async function givePortrait(host: StageHost, cdn: string, entryOf: (kind: string, id: string) => Promise<ContentEntry>, slot: number, kind: string, id: string, field: string, mood: string) {
	try {
		const f = (await entryOf(kind, id))[field] as (Record<string, { path?: string }> & { path?: string }) | undefined;
		const file = mood ? f?.[mood] : f;
		if (!file?.path) throw new Error(`no ${kind}.${id}.${field}${mood ? `.${mood}` : ''}`);
		const img = await decodeImage(await (await fetch(cdn + file.path)).blob());
		if (!host.stopped) host.view.give_portrait(slot, img.width, img.height, img.rgba);
	} catch (e) {
		if (!host.stopped) host.view.no_portrait(slot, (e as Error).message);
	}
}

/** a scene-content entry by kind and id, from the API (each once) */
export function contentAt(api: string, known: Map<string, Promise<ContentEntry>>, kind: string, id: string): Promise<ContentEntry> {
	const key = `${kind}:${id}`;
	if (!known.has(key)) known.set(key, fetch(`${api}/library/scene-content?${kind}=${encodeURIComponent(id)}`).then(async (r) => ((await r.json()) as Record<string, Record<string, ContentEntry> | undefined>)[kind]?.[id] ?? {}));
	return known.get(key)!;
}
/** chapter node `i`'s chapter from the API, into the view */
export async function fetchChapter(host: { view: ChapterView; stopped: boolean }, url: string, i: number) {
	try {
		const text = await (await fetch(url)).text();
		if (!host.stopped) host.view.give_chapter(i, text);
	} catch (e) {
		console.warn(`scene: the chapter ${url} failed`, e);
	}
}

/** chapter node `i`'s slice from the CDN, decoded by the browser, into the view */
export async function fetchSlice(host: { view: ChapterView; stopped: boolean }, url: string, i: number) {
	try {
		const img = await decodeImage(await (await fetch(url)).blob());
		if (!host.stopped) host.view.give_slice(i, img.width, img.height, img.rgba);
	} catch (e) {
		if (!host.stopped) host.view.no_slice(i, (e as Error).message);
	}
}

/** #465: an outlet's view – what its shelf asks for and the answers */
export interface OutletView extends SignedView {
	outlet_asks(): unknown[];
	outlet_text(key: string, text: string): void;
	outlet_image(key: string, width: number, height: number, rgba: Uint8Array): void;
	outlet_fetched(key: string): void;
	outlet_failed(key: string, why: string): void;
}

/** what the outlet asked for: a chapter or a fetched piece (c:, cr:, API), a slice (p:, CDN), a medium (m:, URL), a signed link, a flashed show or its proof (s:, sf:, r:, deck-key.ts) */
export function fetchOutletAsks(host: { view: OutletView; stopped: boolean }, o: { api: string; cdn: string; link?: DeckLinkView; user?: SignedIn }) {
	const asked = host.view.outlet_asks();
	for (let i = 0; i + 1 < asked.length; i += 2) void fetchOutletAsk(host, o, asked[i] as string, asked[i + 1] as string);
}

async function fetchOutletAsk(host: { view: OutletView; stopped: boolean }, o: { api: string; cdn: string; link?: DeckLinkView; user?: SignedIn }, key: string, path: string) {
	try {
		if (key[0] === 's' || key[0] === 'r') return void (await (key[0] === 's' ? fetchShowLink(host, o, key, path) : fetchProof(host, o, path)));
		const r = await fetch(key[0] === 'c' ? o.api + path : key[0] === 'p' ? o.cdn + path : path, { credentials: 'omit' });
		if (!r.ok) throw new Error(`HTTP ${r.status}`);
		if (key[0] === 'c') return void (host.stopped || host.view.outlet_text(key, await r.text()));
		const blob = await r.blob();
		if (key[0] === 'm' && !blob.type.startsWith('image/')) return void (host.stopped || host.view.outlet_fetched(key));
		const img = await decodeImage(blob);
		if (!host.stopped) host.view.outlet_image(key, img.width, img.height, img.rgba);
	} catch (e) {
		if (!host.stopped) host.view.outlet_failed(key, (e as Error).message);
	}
}
