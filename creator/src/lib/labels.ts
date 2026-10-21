import type { ContentDoc } from '$lib/editor/doc.svelte.ts';
import { entityLabel } from '@scutoons/ui';
import { m } from '$lib/paraglide/messages.js';

/**
 * What people see instead of an entity key (no file names in the UI, owner 2026-09-30): `chapter:catpope:2` →
 * "Chapter 2", `season:catpope:1` → "Season 1", `toon-settings:catpope` → "Toon settings" … One
 * mapping for the whole creator (@scutoons/ui `entityLabel`).
 */
/** a catalog document's kind as a word ("Item camera", agents/CATALOG-IN-CREATOR.md) */
const CATALOG_WORD: Record<string, () => string> = {
	item: () => m['creator.catalog_item'](),
	recipe: () => m['creator.catalog_recipe'](),
	album: () => m['creator.catalog_album'](),
	product: () => m['creator.catalog_product'](),
	edition: () => m['creator.catalog_edition']()
};

export function fileLabel(key: string): string {
	return entityLabel(key, {
		chapter: (number) => m['creator.entity_chapter']({ number }),
		season: (number) => m['creator.season']({ number }),
		toon: m['creator.edit_toon'](),
		entry: m['creator.edit_toon'](),
		characters: m['creator.characters'](),
		moods: m['creator.characters_moods'](),
		colorations: m['creator.entity_colorations'](),
		requests: m['creator.characters_wanted'](),
		cast: m['creator.cast'](),
		credits: m['creator.credits'](),
		people: m['creator.credits_tab_people'](),
		roles: m['creator.credits_tab_roles'](),
		storyboard: (number) => m['storyboard.entity']({ number }),
		seasonScript: (number) => `${m['creator.script']()} · ${m['creator.season']({ number })}`,
		chapterScript: (number) => `${m['creator.script']()} · ${m['creator.entity_chapter']({ number })}`,
		kanon: (subject) => `${m['creator.kanon']()} · ${subject.split('.').slice(1).join(' · ')}`,
		place: (id) => `${m['creator.publish_part_place']()}: ${id}`,
		route: (id) => `${m['creator.publish_part_route']()}: ${id}`,
		region: (id) => `${m['creator.publish_part_region']()}: ${id}`,
		character: (id) => `${m['creator.publish_part_character']()}: ${id}`,
		catalog: (kind, id) => `${CATALOG_WORD[kind]?.() ?? kind} ${id}`
	});
}

/** Several storage keys as one readable list ("Chapter 2, Season 1"). */
export const fileLabels = (paths: string[]) => paths.map(fileLabel).join(', ');

/** An open document's label (`fileLabel` of its entity) */
export function docLabel(doc: ContentDoc): string {
	return fileLabel(doc.entity);
}
