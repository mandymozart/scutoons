// Links from the admin into the creator for a content entity key (`chapter:catpope:3:de`, `season:catpope:1:de`, …):
// the asset sheet's uses and the optimisation tool's new drafts (BACKLOG 10.64) – the creator page that edits it.
import { CREATOR_URL } from '$lib/config.ts';

/** the creator page of a content entity key; null when there is none to link (the map, registries …) */
export function contentHref(ref: string): string | null {
	const parts = ref.split(':');
	if (parts[0] === 'season') return `${CREATOR_URL}/${parts[1]}/season/${parts[2]}`;
	if (parts[0] === 'chapter' || parts[0] === 'scene') return `${CREATOR_URL}/${parts[1]}/chapter/${parts[2]}`;
	if (parts[0] === 'toon' || parts[0] === 'toon-settings') return `${CREATOR_URL}/${parts[1]}`;
	if (parts[0] === 'map') return `${CREATOR_URL}/map`;
	if (parts[0] === 'characters') return `${CREATOR_URL}/characters`;
	return null;
}
