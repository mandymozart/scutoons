// stage-assets.ts (#465): what a stage asks the page to fetch and decode – a set's .glb and its images, a head's sprites.
// The asking and the use are the core's (deck/space-web); here only the fetches and the browser's image decoder.
import { fetchShowLink, type DeckLinkView, type SignedView } from './deck-key.ts';
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

type Portraits = Record<string, { path: string }>;

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

/** the sprites the heads asked for, fetched and decoded (each once) */
export function askPortraits(host: StageHost, cdn: string, portraitsOf: (character: string) => Promise<Portraits>) {
	const asked = host.view.portrait_requests();
	for (let i = 0; i + 2 < asked.length; i += 3) void givePortrait(host, cdn, portraitsOf, asked[i] as number, asked[i + 1] as string, asked[i + 2] as string);
}

async function givePortrait(host: StageHost, cdn: string, portraitsOf: (character: string) => Promise<Portraits>, slot: number, character: string, mood: string) {
	try {
		const p = (await portraitsOf(character))[mood];
		if (!p) throw new Error(`no ${character}_portrait-${mood}`);
		const img = await decodeImage(await (await fetch(cdn + p.path)).blob());
		if (!host.stopped) host.view.give_portrait(slot, img.width, img.height, img.rgba);
	} catch (e) {
		if (!host.stopped) host.view.no_portrait(slot, (e as Error).message);
	}
}

/** a head's sprites by mood, from scene-content (each character once) */
export function portraitsAt(api: string, known: Map<string, Promise<Portraits>>, character: string): Promise<Portraits> {
	let p = known.get(character);
	if (!p) {
		p = fetch(`${api}/library/scene-content?characters=${encodeURIComponent(character)}`)
			.then((r) => r.json())
			.then((j: { characters?: Record<string, { portraits?: Portraits }> }) => j.characters?.[character]?.portraits ?? {});
		known.set(character, p);
	}
	return p;
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

/** what the outlet asked for: a chapter (c:, API), a slice (p:, CDN), a medium (m:, URL), a signed link (s:, deck-key.ts) */
export function fetchOutletAsks(host: { view: OutletView; stopped: boolean }, o: { api: string; cdn: string; link?: DeckLinkView }) {
	const asked = host.view.outlet_asks();
	for (let i = 0; i + 1 < asked.length; i += 2) void fetchOutletAsk(host, o, asked[i] as string, asked[i + 1] as string);
}

async function fetchOutletAsk(host: { view: OutletView; stopped: boolean }, o: { api: string; cdn: string; link?: DeckLinkView }, key: string, path: string) {
	try {
		if (key[0] === 's') return void (await fetchShowLink(host, o, key, path));
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
