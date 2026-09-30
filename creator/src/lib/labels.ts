import { entityLabel } from '@scutoons/ui';
import { m } from '$lib/paraglide/messages.js';

/**
 * What people see instead of a storage key (no file names in the UI, owner 2026-09-30): `chapter2.json` → "Chapter 2",
 * `season1.json` → "Season 1", scene files → "Scene 14", `toon.json` → "Toon settings" … One mapping for the whole
 * creator (@scutoons/ui `entityLabel`); the data refactor switches the keys there.
 */
export function fileLabel(path: string): string {
	return entityLabel(path, {
		chapter: (number) => m['creator.entity_chapter']({ number }),
		season: (number) => m['creator.season']({ number }),
		scene: (label) => m['creator.entity_scene']({ label }),
		toon: m['creator.edit_toon'](),
		languages: m['creator.languages'](),
		characters: m['creator.characters'](),
		moods: m['creator.characters_moods'](),
		requests: m['creator.characters_wanted'](),
		cast: m['creator.cast'](),
		credits: m['creator.credits'](),
		people: m['creator.credits_tab_people'](),
		roles: m['creator.credits_tab_roles']()
	});
}

/** Several storage keys as one readable list ("Chapter 2, Season 1"). */
export const fileLabels = (paths: string[]) => paths.map(fileLabel).join(', ');
