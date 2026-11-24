// Sample data of the /kit page (dev only): nothing here is sent or kept.
import { m } from '$lib/paraglide/messages.js';
import type { PublishSource } from '$lib/editor/publish-flow.svelte.ts';
import type { PublishLine } from '$lib/editor/publish-tree.ts';

const line = (id: string, label: string, chip: string, type: string): PublishLine => ({ id, label, chip, type, hint: 'v12 · agent@scutoons.test', review: false });

/** the Publish dialog's lines: the bar's icon lists them, its primary answers with a sample report */
export function publishLines(): PublishLine[] {
	const changed = m['creator.publish_part_changed']();
	const added = m['creator.publish_part_added']();
	return [
		line('character:sandgirl', 'Sandgirl', changed, 'Character'),
		line('character:giantpopstar', 'Giant Popstar', changed, 'Character'),
		line('character:mellowmantis', 'Mellowmantis', changed, 'Character'),
		line('chapter:catpope:3:de', 'Chapter 3 (de)', changed, 'Chapter · Catpope'),
		line('season:catpope:1', 'Season 1', changed, 'Season · Catpope'),
		line('place:foyer', 'Foyer', added, 'Place'),
		line('module:walk', 'walk', added, 'Module'),
		line('show:top-toy', 'top-toy', changed, 'Show')
	];
}

export const publishSamples: PublishSource = {
	load: async () => ({ files: [], lines: publishLines() }),
	send: async (list, chosen) => {
		const out = list.lines.filter((l) => chosen.has(l.id) && l.id !== 'show:top-toy').map((l) => l.label);
		const notices = [
			{ type: 'success' as const, text: m['creator.publish_tree_done_named']({ count: out.length, names: out.join(', ') }) },
			...(chosen.has('show:top-toy') ? [{ type: 'error' as const, text: m['creator.publish_tree_refused']({ name: 'top-toy', reason: 'needs module:walk-2, which is not published' }) }] : [])
		];
		return { message: notices[0].text, published: [], refused: [], notices };
	}
};

/** a drawn capture of a stage (a workspace card's capture, a thumb's still): ground, a horizon, a figure – no file needed */
export function sampleStill(day: boolean): string {
	const [sky, ground, figure, sun] = day ? ['#dfe9f5', '#b9c7a5', '#2b3a4a', '#f2b84b'] : ['#0b1020', '#1c2a22', '#d9e2ec', '#5fb3ff'];
	const svg = `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 264 168"><rect width="264" height="168" fill="${sky}"/><circle cx="206" cy="46" r="18" fill="${sun}"/><rect y="112" width="264" height="56" fill="${ground}"/><ellipse cx="96" cy="118" rx="16" ry="5" fill="#000" opacity=".25"/><rect x="86" y="72" width="20" height="44" rx="10" fill="${figure}"/><circle cx="96" cy="62" r="12" fill="${figure}"/></svg>`;
	return `data:image/svg+xml,${encodeURIComponent(svg)}`;
}
