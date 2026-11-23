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
	edition: () => m['creator.catalog_edition'](),
	module: () => m['creator.sound_module']()
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
		scene: (id) => `${m['creator.publish_part_scene']()}: ${id}`,
		show: (id) => `${m['creator.publish_part_show']()}: ${id}`,
		catalog: (kind, id) => `${CATALOG_WORD[kind]?.() ?? `${kind[0].toUpperCase()}${kind.slice(1)}`} ${id}`
	});
}

/** the kinds stored one document per entry, named by the entry's own name */
const PART_WORD: Record<string, () => string> = {
	place: () => m['creator.publish_part_place'](),
	route: () => m['creator.publish_part_route'](),
	region: () => m['creator.publish_part_region'](),
	character: () => m['creator.publish_part_character'](),
	scene: () => m['creator.publish_part_scene'](),
	show: () => m['creator.publish_part_show']()
};

/** an entity key's kind as a word, for a line that shows the item's own name ("Scene", "Item" …); null: none */
export function kindWord(key: string): string | null {
	const kind = key.split(':')[0];
	const word = PART_WORD[kind] ?? CATALOG_WORD[kind];
	if (word) return word();
	return key.includes(':') ? `${kind[0].toUpperCase()}${kind.slice(1)}` : null;
}

/** a line's name: the item's own name when known (`known`), an entry by its id, else the file's label */
export function ownName(key: string, known?: string): string {
	if (known) return known;
	const kind = key.split(':')[0];
	return kind in PART_WORD || kind in CATALOG_WORD ? key.slice(key.lastIndexOf(':') + 1) : fileLabel(key);
}

/** Several storage keys as one readable list ("Chapter 2, Season 1"). */
export const fileLabels = (paths: string[]) => paths.map(fileLabel).join(', ');

/** An open document's label (`fileLabel` of its entity) */
export function docLabel(doc: ContentDoc): string {
	return fileLabel(doc.entity);
}
