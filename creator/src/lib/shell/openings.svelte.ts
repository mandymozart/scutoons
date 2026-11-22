// "Items to open" past the toons and the areas: the characters, the map's places, the catalog's items, the scenes, the
// rigs and the modules, each list read once on demand. Where each kind opens, or why it does not, is the core's
// (deck-catalog `library::open` through space-web); this loads the lists and routes an answer to its page.
import { localeText } from '@scutoons/content';
import type { LibraryOpenKind, LibraryOpening } from '@scutoons/media';
import type { LibraryOpener } from '@scutoons/media/library';
import { api, contentApi, library as reader, scenesApi } from '$lib/api.ts';
import { library } from '$lib/editor/library.svelte.ts';
import { mapPlaces } from '$lib/editor/map.ts';
import { sceneRows, sceneTitle } from '$lib/editor/scenes.ts';
import { getLocale } from '$lib/paraglide/runtime.js';
import { shared } from '$lib/utils/shared.ts';
import { hrefOf, iconOf } from './address.ts';
import { matches, type Place } from './places.svelte.ts';

/** one kind's group: the core's words for it, where it opens, the rows that match */
export interface OpenGroup {
	kind: LibraryOpenKind;
	rows: Place[];
}

type Row = { address: string; label: string; toons?: string[] };

const catalog = (kind: string) => api.get<{ entries: { id: string; removed?: boolean }[] }>(`/catalog/docs/${kind}`).then((r) => r.entries.filter((e) => !e.removed));

/** the lists by kind word: each read with the API the kind's own editor reads */
const LOADERS: Record<string, () => Promise<Row[]>> = {
	character: async () => ((await reader.fetchRegistry({ draft: true }))?.characters ?? []).map((c) => ({ address: `character:${c.id}`, label: localeText(c.name, getLocale()) || c.id })),
	place: async () => (await mapPlaces()).map((p) => ({ address: `place:${p.id}`, label: localeText(p.name, getLocale()) || p.id, toons: p.toons })),
	item: async () => (await contentApi.catalogDocs('items')).entries.filter((e) => !e.removed).map((e) => ({ address: e.key, label: localeText(e.content?.name, getLocale()) || e.id, toons: e.toons })),
	scene: async () => sceneRows((await scenesApi.list()).scenes).map((r) => ({ address: r.key, label: sceneTitle(r.facts), toons: r.facts.toons })),
	rig: async () => (await catalog('rigs')).map((e) => ({ address: `rig:${e.id}`, label: e.id })),
	module: async () => (await catalog('modules')).map((e) => ({ address: `module:${e.id}`, label: e.id }))
};

class Openings {
	kinds = $state.raw<LibraryOpenKind[]>([]);
	rows = $state.raw<Record<string, Place[]>>({});
	#rule: LibraryOpener | null = null;
	#asked = false;

	async load() {
		if (this.#asked) return;
		this.#asked = true;
		try {
			this.#rule = await library.opener();
			this.kinds = this.#rule.kinds();
		} catch {
			this.#asked = false;
			return;
		}
		for (const [kind, load] of Object.entries(LOADERS)) {
			load()
				.then((list) => (this.rows = { ...this.rows, [kind]: list.map((r) => this.#place(r)).filter((p): p is Place => !!p) }))
				.catch(() => undefined);
		}
	}

	/** a row the router reaches (a page for its address), with its icon */
	#place(r: Row): Place | null {
		const href = hrefOf(r.address);
		return href ? { address: r.address, label: r.label, href, icon: iconOf(r.address), group: 'area', keywords: r.toons?.join(' '), toon: r.toons?.length === 1 ? r.toons[0] : undefined } : null;
	}

	/** the kinds that open, each with its rows matching `query` (in `toon` when given: a row made for other toons only is left out) */
	groups(query: string, toon: string | null): OpenGroup[] {
		return this.kinds
			.filter((k) => k.workspace && k.workspace !== 'asset')
			.map((kind) => ({ kind, rows: (this.rows[kind.kind] ?? []).filter((p) => matches(p, query) && (!toon || !p.keywords || p.keywords.split(' ').some((t) => t === toon || t === '*'))) }))
			.filter((g) => g.rows.length);
	}

	/** the kinds the creator opens in their asset details (the Library's own tiles) */
	files(): LibraryOpenKind[] {
		return this.kinds.filter((k) => k.workspace === 'asset');
	}

	/** the kinds without a workspace here, each with the core's sentence */
	closed(): LibraryOpenKind[] {
		return this.kinds.filter((k) => k.why);
	}

	open(address: string): LibraryOpening {
		return this.#rule?.open(address) ?? { ok: false, why: `${address}: the Library is still loading` };
	}
}

export const openings = shared('creator-openings', () => new Openings());
