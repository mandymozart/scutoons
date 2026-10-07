// The Flow's pictures (owner 2026-10-06, agents/DESIGN.md "Flow panel" – images): what @scutoons/content flow.ts names as
// data (`FlowImage`, `FlowFace`) resolved to small CDN URLs – slices' low variants, chapters' thumbnails, toons' avatars
// (the server's flow index), catalog items' images, the registry's avatar view of a character (the same rule the reader
// uses, `characterImage`). A picture that can't be found is '' – the box draws its dashed empty slot.
import { getContext, setContext } from 'svelte';
import { characterImage, characterIndex, isMissingImagePath, type Catalog, type CharacterRegistry, type FlowFace, type FlowImage, type FlowIndex } from '@scutoons/content';
import { api } from '$lib/api.ts';
import { asset } from '$lib/utils/cdn.ts';
import { loadCatalog, itemName } from '$lib/editor/catalog.svelte.ts';
import { shownCharacterName } from '$lib/editor/text-language.svelte.ts';
import { pickText } from '$lib/editor/toon-flow.ts';
import { m } from '$lib/paraglide/messages.js';
import { getLocale } from '$lib/paraglide/runtime.js';
import { contextKey } from '$lib/utils/shared.ts';

/** a resolved picture: its URL ('' = none) and its name (the image's `title`) */
export interface Picture {
	src: string;
	title: string;
}

let indexPromise: Promise<FlowIndex | null> | null = null;
/** the server's flow index, once per page load (it is content-addressed on the server; the Flow pages read it too) */
export function loadFlowIndex(): Promise<FlowIndex | null> {
	indexPromise ??= api.get<FlowIndex>('/creator/flow-index').catch(() => ((indexPromise = null), null));
	return indexPromise;
}

/** a path relative to a toon's folder (`s1/c2/…`, `../catpope/…`) as a CDN URL */
const inToon = (toon: string, path: string | null | undefined) => (path ? asset(`${toon}/${path}`) : '');

export class FlowPictures {
	index = $state.raw<FlowIndex | null>(null);
	catalog = $state.raw<Catalog | null>(null);
	registry = $state.raw<CharacterRegistry | null>(null);
	#characters = $derived(characterIndex(this.registry));

	/** `toon`: the flow's own toon (pictures without one are its); `index` given: no request */
	constructor(
		readonly toon: string,
		options: { index?: FlowIndex | null; registry?: CharacterRegistry | null; items?: boolean } = {}
	) {
		this.registry = options.registry ?? null;
		if (options.index) this.index = options.index;
		else void loadFlowIndex().then((index) => (this.index = index));
		if (options.items) void loadCatalog().then((catalog) => (this.catalog = catalog));
	}

	#toonEntry(id: string) {
		return this.index?.toons.find((t) => t.id === id);
	}
	toonName(id: string): string {
		const entry = this.#toonEntry(id);
		return (entry && pickText(entry.title, getLocale())) || id;
	}

	/** a box's picture (`FlowNode.image` / `badge`) */
	picture(image: FlowImage | undefined): Picture | null {
		if (!image) return null;
		switch (image.kind) {
			case 'slice':
				return { src: inToon(image.toon ?? this.toon, image.path), title: m['creator.flow_slice']({ number: image.slice + 1 }) };
			case 'thumbnail':
				return { src: inToon(image.toon ?? this.toon, image.path), title: this.#chapterTitle(image.chapter, image.toon) };
			case 'chapter': {
				const toon = image.toon ?? this.toon;
				const pics = this.#toonEntry(toon)?.chapters?.[String(image.chapter)];
				// where a label lands: its group's first slice; else (or no slice there) the chapter's thumbnail
				const path = (image.label ? pics?.labels?.[image.label] : undefined) ?? pics?.thumbnail ?? pics?.first;
				return { src: inToon(toon, path), title: this.#chapterTitle(image.chapter, image.toon, image.label) };
			}
			case 'toon': {
				const entry = this.#toonEntry(image.toon);
				// the toon page's rule: the entry's avatar, else the convention `<toon>/avatar.webp`; a toon not in the index has none
				return { src: entry ? inToon(image.toon, entry.avatar || 'avatar.webp') : '', title: this.toonName(image.toon) };
			}
			case 'item': {
				const def = this.catalog?.items.find((x) => x.id === image.item);
				return { src: def?.icon ? asset(def.icon) : '', title: itemName(def, image.item) };
			}
		}
	}

	#chapterTitle(chapter: number, toon?: string, label?: string): string {
		const head = toon && toon !== this.toon ? m['creator.flow_chapter_of_toon']({ toon: this.toonName(toon), value: String(chapter) }) : m['creator.flow_chapter_title']({ number: chapter });
		return label ? `${head} · ${label}` : head;
	}

	/** a character's face: the registry's avatar view (a small downscale of the portrait) for the persona / look / mood */
	face(face: FlowFace): Picture {
		const character = this.#characters.get(face.character);
		const path = character ? characterImage(character, { persona: face.persona, look: face.look, mood: face.mood }, 'avatar', { substitute: '' }) : '';
		return { src: path && !isMissingImagePath(path) ? asset(path) : '', title: shownCharacterName(character, face.persona) || face.character };
	}
}

const KEY = contextKey('flow-pictures');
export const setFlowPictures = (pictures: FlowPictures | undefined) => setContext(KEY, pictures);
export const getFlowPictures = () => getContext<FlowPictures | undefined>(KEY);
