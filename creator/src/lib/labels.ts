import { parseEntityKey, sceneEntityOfRef } from '@scutoons/content';
import type { ContentDoc } from '$lib/editor/doc.svelte.ts';
import { entityLabel } from '@scutoons/ui';
import { m } from '$lib/paraglide/messages.js';

/**
 * What people see instead of an entity key (no file names in the UI, owner 2026-09-30): `chapter:catpope:2` →
 * "Chapter 2", `season:catpope:1` → "Season 1", scenes → "Scene 14", `toon-settings:catpope` → "Toon settings" … One
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
		scene: (label) => m['creator.entity_scene']({ label }),
		toon: m['creator.edit_toon'](),
		entry: m['creator.edit_toon'](),
		characters: m['creator.characters'](),
		moods: m['creator.characters_moods'](),
		requests: m['creator.characters_wanted'](),
		cast: m['creator.cast'](),
		credits: m['creator.credits'](),
		people: m['creator.credits_tab_people'](),
		roles: m['creator.credits_tab_roles'](),
		script: m['creator.script'](),
		catalog: (kind, id) => `${CATALOG_WORD[kind]?.() ?? kind} ${id}`
	});
}

/** Several storage keys as one readable list ("Chapter 2, Season 1"). */
export const fileLabels = (paths: string[]) => paths.map(fileLabel).join(', ');

/** A chapter group's scene reference (`ref.src`, content format) → its label ("Scene 14") */
export const sceneLabel = (toon: string, src: string) => fileLabel(sceneEntityOfRef(toon, src) ?? 'scene');

/**
 * An open document's label: a scene by its group's label and chapter ("Chapter 2 · group "Intro"") – the scene's
 * storage ID (`scene-14`) is internal and not what the chapter tree shows (owner 2026-10-06); else `fileLabel`.
 */
export function docLabel(doc: ContentDoc): string {
	const ref = parseEntityKey(doc.entity);
	const label = doc.get(doc.primary, ['label']);
	if (ref?.kind === 'scene' && typeof label === 'string' && label.trim()) return m['creator.entity_group']({ number: ref.number ?? '', label });
	return fileLabel(doc.entity);
}
