// "Continue reading" – the installed app's shortcut (manifest `shortcuts`, agents/research/PWA.md §1; long-press on
// the icon): `/?continue` → the chapter the reader opened last (any toon), where they stopped. Resolved from the save
// game's checkpoints (lib/contexts/world.svelte.ts – localStorage, synced for members), like the toon page's start
// button: an unfinished chapter opens at its spot (`?resume`); a finished one → its toon page, whose start button offers
// the next chapter. Nothing read yet → stays on the start page. Pure – test/continue-reading.test.ts.
import { checkpointPosition, checkpointSpot, type Checkpoint } from '@scutoons/content';

/** the query the shortcut opens the app with */
export const CONTINUE_PARAM = 'continue';

const CHAPTER_URI = /^([A-Za-z0-9_-]{1,64})\/chapter\/(\d{1,6})$/;

const time = (value: unknown) => (typeof value === 'string' ? Date.parse(value) || 0 : 0);

/** the unprefixed path to open (localize it), or null = nothing read yet */
export function continuePath(progress: readonly Checkpoint[] | null | undefined): string | null {
	let last: { toon: string; chapter: number; checkpoint: Checkpoint } | null = null;
	for (const checkpoint of Array.isArray(progress) ? progress : []) {
		const match = typeof checkpoint?.uri === 'string' ? CHAPTER_URI.exec(checkpoint.uri) : null;
		if (!match) continue;
		if (last && time(checkpoint.updated_at) <= time(last.checkpoint.updated_at)) continue;
		last = { toon: match[1], chapter: Number(match[2]), checkpoint };
	}
	if (!last) return null;
	if (checkpointPosition(last.checkpoint) >= 1) return `/${last.toon}`;
	return `/${last.toon}/chapter/${last.chapter}${checkpointSpot(last.checkpoint) ? '?resume' : ''}`;
}
