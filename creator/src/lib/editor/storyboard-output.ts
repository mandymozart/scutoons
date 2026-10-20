// The Export step's Output (#126; workflows/storyboard-studio README "Render all handheld shots", "Downloads"): the
// chapter's videos – the handheld renders (the takes rendered in this browser into the workspace folder) and the
// generated ones – listed from the ledger's records and found where they are: this member's workspace folder, the
// shared work storage, else "on <machine>". Which shots can be rendered: the camera on and an active take. Pure.
import type { Take } from '@scutoons/content/storyboard';
import { placeOf, type FilePlace, type Generation } from './storyboard-generations.ts';
import { activeTake, RENDER_GENERATOR } from './storyboard-takes.ts';
import type { ShotEntry } from './storyboard-shots.ts';

export interface OutputRow {
	g: Generation;
	/** a handheld render or a generated video */
	kind: 'rendered' | 'generated';
	place: FilePlace;
	/** the shot's place in the toon order (rows sort by it) */
	order: number;
}

/** the chapter's videos in toon order (per shot: the renders first, then the generated; the newest first) – failed and
 *  rejected ones stay out */
export function outputRows(generations: readonly Generation[], order: readonly string[], local: ReadonlySet<string>): OutputRow[] {
	const at = new Map(order.map((id, i) => [id, i]));
	return generations
		.filter((g) => g.kind === 'video' && g.status === 'succeeded' && g.mark !== 'rejected')
		.map((g): OutputRow => ({ g, kind: g.generator === RENDER_GENERATOR ? 'rendered' : 'generated', place: placeOf(g, local), order: at.get(g.shot) ?? Number.MAX_SAFE_INTEGER }))
		.sort((a, b) => a.order - b.order || (a.kind === b.kind ? 0 : a.kind === 'rendered' ? -1 : 1) || b.g.id - a.g.id);
}

/** the shots a render takes: the handheld camera on and an active take (toon order) */
export function handheldShots(entries: readonly ShotEntry[]): { entry: ShotEntry; take: Take }[] {
	return entries.flatMap((entry) => {
		const take = entry.shot.takes?.on ? activeTake(entry.shot.takes) : null;
		return take && take.samples.length > 1 ? [{ entry, take }] : [];
	});
}

/** the file name a download gets: <toon>-<chapter>-<the file's own name> */
export const downloadName = (toon: string, chapter: number, file: string) => `${toon}-${chapter}-${file.split('/').pop()}`;

/** the totals line: how many videos, how many here / shared / elsewhere */
export function outputCounts(rows: readonly OutputRow[]): { total: number; local: number; work: number; elsewhere: number } {
	const n = (at: FilePlace['at']) => rows.filter((r) => r.place.at === at).length;
	return { total: rows.length, local: n('local'), work: n('work'), elsewhere: rows.length - n('local') - n('work') };
}
