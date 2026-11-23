import { entities, parseEntityKey, type ToonIndex } from '@scutoons/content';
import { contentIndex, experiencesApi } from '$lib/api.ts';
import { CATALOG_LABELS, CATALOG_PAGE_KINDS } from '$lib/editor/catalog-docs.ts';
import { experienceKey, experienceRows, titleIn, type ExperienceRow } from '$lib/editor/experiences.ts';
import { fileLabel } from '$lib/labels.ts';
import { m } from '$lib/paraglide/messages.js';
import { getLocale } from '$lib/paraglide/runtime.js';
import { shared } from '$lib/utils/shared.ts';
import { AREAS, hrefOf, iconOf } from './address.ts';
import { wordsIn } from './launcher.ts';

/** one thing that opens as a tab: the launcher and the Library list the same */
export interface Place {
	address: string;
	label: string;
	/** the toon or area it belongs to */
	hint?: string;
	href: string;
	icon: string;
	keywords?: string;
	/** what the Library groups it by */
	group: 'toon' | 'season' | 'chapter' | 'area' | 'catalog';
	toon?: string;
}

/**
 * What the existing APIs give to open: the areas (characters, the map, scenes, the flow, the catalog's
 * kinds, the guide), every toon with its seasons and chapters (its content index, read once per toon on demand) and
 * the XR experiences (read once, when first asked).
 */
class Places {
	parts = $state<Record<string, { seasons: number[]; chapters: number[] }>>({});
	#asked = new Set<string>();
	#xr = $state<ExperienceRow[]>([]);
	#xrAsked = false;

	load(toons: readonly ToonIndex[]) {
		for (const t of toons) {
			if (this.#asked.has(t.id)) continue;
			this.#asked.add(t.id);
			contentIndex(t.id)
				.then((index) => {
					const refs = index.entities.map((e) => parseEntityKey(e.key));
					const numbers = (kind: string) => [...new Set(refs.filter((r) => r?.kind === kind && r.number !== undefined).map((r) => r!.number!))].sort((a, b) => a - b);
					const seasons = numbers('season');
					this.parts[t.id] = { seasons: seasons.length ? seasons : [1], chapters: numbers('chapter') };
				})
				.catch(() => this.#asked.delete(t.id));
		}
	}

	areas(): Place[] {
		const area = (address: string, label: string, keywords: string): Place => ({ address, label, href: hrefOf(address)!, icon: iconOf(address), keywords, group: 'area' });
		return [
			area(AREAS.characters, m['creator.characters'](), 'characters figuren'),
			area(AREAS.map, m['creator.map'](), 'map karte places'),
			area(AREAS.scenes, m['creator.scenes'](), 'scenes szenen deck'),
			area(AREAS.flow, m['creator.flow_universe'](), 'flow ablauf'),
			area(AREAS.generate, m['shell.ws_generate'](), 'generate generator spore canvas ai generieren'),
			...CATALOG_PAGE_KINDS.map((kind) => ({ ...area(`catalog:${kind}`, CATALOG_LABELS[kind](), `catalog katalog ${kind.replace('-', ' ')}`), hint: m['creator.catalog'](), group: 'catalog' as const })),
			area(AREAS.guide, m['creator.style_guide'](), 'style guide design help')
		];
	}

	/** the XR experiences, each its own place (the list is read on the first call) */
	experiences(): Place[] {
		if (!this.#xrAsked) {
			this.#xrAsked = true;
			experiencesApi
				.list()
				.then((r) => (this.#xr = experienceRows(r.entries, getLocale())))
				.catch(() => (this.#xrAsked = false));
		}
		return this.#xr.map((r): Place => {
			const address = experienceKey(r.id);
			return { address, label: titleIn(r.title, getLocale(), r.id), hint: m['creator.experiences'](), href: hrefOf(address)!, icon: iconOf(address), keywords: `xr ${r.type} ${r.id}`, group: 'area' };
		});
	}

	/** the toons' own places, the toon `first` first */
	toons(toons: readonly ToonIndex[], name: (id: string) => string, first: string | null = null): Place[] {
		const sorted = [...toons].sort((a, b) => Number(b.id === first) - Number(a.id === first));
		return sorted.flatMap((t): Place[] => {
			const toon = name(t.id);
			const parts = this.parts[t.id];
			const place = (address: string, label: string, group: Place['group'], keywords: string): Place => ({ address, label, hint: toon, href: hrefOf(address)!, icon: iconOf(address), keywords: `${t.id} ${keywords}`, group, toon: t.id });
			return [
				{ ...place(entities.toon(t.id), toon, 'toon', 'toon'), hint: m['palette.toon']() },
				...(parts?.seasons ?? []).map((n) => place(entities.season(t.id, n), fileLabel(entities.season(t.id, n)), 'season', 'season staffel')),
				...(parts?.chapters ?? []).map((n) => place(entities.chapter(t.id, n), fileLabel(entities.chapter(t.id, n)), 'chapter', 'chapter kapitel'))
			];
		});
	}
}

export const places = shared('creator-places', () => new Places());

/** does a place match what was typed (its name, its toon, its address, its words)? */
export function matches(place: Place, query: string): boolean {
	return wordsIn(`${place.label} ${place.hint ?? ''} ${place.address} ${place.keywords ?? ''}`, query);
}
