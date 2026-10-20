// The figurines' models (#124): the owner's Cat Pope and the mountain tiger as .glb – a character's own persona model
// (#56, `personaModel` – a published asset on the CDN) when it has one, else the creator's static copy
// (static/storyboard/models – the CDN is only for reader material, owner 2026-10-20). Each URL is loaded once per page
// session; a model that fails to load or can't be posed as the figurine falls back to the next URL, then to the
// procedural figurine.
import { GLTFLoader, type GLTF } from 'three/addons/loaders/GLTFLoader.js';

const cache = new Map<string, Promise<GLTF>>();

function load(url: string): Promise<GLTF> {
	let p = cache.get(url);
	if (!p) {
		p = new GLTFLoader().loadAsync(url);
		cache.set(url, p);
		p.catch(() => cache.delete(url)); // a failed load is tried again next time
	}
	return p;
}

/** the first of `urls` that loads and is `usable`, else null (the procedural figurine) */
export async function firstModel(urls: readonly string[], usable: (gltf: GLTF) => boolean): Promise<GLTF | null> {
	for (const url of urls) {
		try {
			const gltf = await load(url);
			if (usable(gltf)) return gltf;
			console.warn(`storyboard: ${url} can't be posed as the figurine – the next model`);
		} catch (e) {
			console.warn(`storyboard: ${url} didn't load – the next model`, e);
		}
	}
	return null;
}

/** where a build finds a figurine's model: the URLs to try, the persona's own first ([] = the procedural figurine) */
export type ModelUrls = (figurine: 'catpope' | 'tiger', persona: string | undefined) => string[] | Promise<string[]>;
