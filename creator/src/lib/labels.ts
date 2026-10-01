import { sceneEntityOfRef } from '@scutoons/content';
import { entityLabel } from '@scutoons/ui';
import { m } from '$lib/paraglide/messages.js';

/**
 * What people see instead of an entity key (no file names in the UI, owner 2026-09-30): `chapter:catpope:2` →
 * "Chapter 2", `season:catpope:1` → "Season 1", scenes → "Scene 14", `toon-settings:catpope` → "Toon settings" … One
 * mapping for the whole creator (@scutoons/ui `entityLabel`).
 */
export function fileLabel(key: string): string {
	return entityLabel(key, {
		chapter: (number) => m['creator.entity_chapter']({ number }),
		season: (number) => m['creator.season']({ number }),
		scene: (label) => m['creator.entity_scene']({ label }),
		toon: m['creator.edit_toon'](),
		entry: m['creator.toon_entry'](),
		characters: m['creator.characters'](),
		moods: m['creator.characters_moods'](),
		requests: m['creator.characters_wanted'](),
		cast: m['creator.cast'](),
		credits: m['creator.credits'](),
		people: m['creator.credits_tab_people'](),
		roles: m['creator.credits_tab_roles'](),
		script: m['creator.script']()
	});
}

/** Several storage keys as one readable list ("Chapter 2, Season 1"). */
export const fileLabels = (paths: string[]) => paths.map(fileLabel).join(', ');

/** A chapter group's scene reference (`ref.src`, content format) → its label ("Scene 14") */
export const sceneLabel = (toon: string, src: string) => fileLabel(sceneEntityOfRef(toon, src) ?? 'scene');
