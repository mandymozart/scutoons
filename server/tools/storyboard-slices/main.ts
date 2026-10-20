// Prints the whole-slices changes for chapters 4–5 (#111; the logic and its tests: slice-ops.ts) as JSON – the `ops` for
// the SCUTOONS MCP's save_draft on storyboard:catpope:4 / :5, one draft each; the owner confirms in the creator.
//   node --experimental-transform-types --no-warnings server/tools/storyboard-slices/main.ts [out.json]
// The documents are built from the scene files exactly as #120's import stored them (import-sql.ts chapterDoc), so the
// paths hold only while the import is the entity's newest version: save with that version as `base` (a newer save
// in between → 409, nothing is changed).
import { readdirSync, readFileSync, writeFileSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import type { ChapterStoryboard } from '@scutoons/content/storyboard';
import { chapterDoc } from '../storyboard-import/import-sql.ts';
import { sliceOps, unusedPicks } from './slice-ops.ts';

const repo = resolve(dirname(fileURLToPath(import.meta.url)), '../../..');

/**
 * The 13 shots at 1:4 (2.5 slices), picked by content – assumed overnight 2026-10-21, the owner confirms (#111 comment):
 * generated shots plan 2 slices (SLICE-GRID §1.6); the owner's art / photo / direct ones get 3 where the shot runs in
 * three beats or carries many boxes.
 */
export const PICKS: Record<string, string> = {
	'sb-35-6': '1:3.2', // glitching sky that calms down, PLF looks up; 2 boxes
	'sb-36-5': '1:3.2', // generated: the top-down run, the cat ahead
	'sb-36-9': '1:4.8', // a column of headstones, one per section; 5 boxes
	'sb-36-13': '1:4.8', // the patriarchs' monuments rising, PLF small; 4 boxes
	'sb-36-16': '1:4.8', // the inscriptions glitching into titles; 6 boxes (2 slices = 3 each, the wall)
	'sb-37-5': '1:4.8', // the water sphere → the graveyard shifting → PLF; 2 long boxes
	'sb-37-7': '1:3.2', // generated: the polar vortex, PLF tiny in the centre
	'sb-37-16': '1:4.8', // the bridge of letters over the void to the gate; air top and bottom
	'sb-38-1': '1:4.8', // ch5's opener: evening sky down into the garden
	'sb-38-7': '1:4.8', // the collage of the day's five pictures, PLF below
	'sb-38-18': '1:4.8', // sun / PLF arms wide / hands in the grass; 281 characters
	'sb-38-21': '1:4.8', // nose in the beans / the beans / a hand with compost
	'sb-38-24': '1:3.2' // generated: PLF over the puddle / Cat Pope mirrored below, one slice per half
};

export const CHAPTERS = [
	{ key: 'storyboard:catpope:4', dir: 'storyboards/catpope/s1c4', chapter: 4 },
	{ key: 'storyboard:catpope:5', dir: 'storyboards/catpope/s1c5', chapter: 5 }
];

/** the documents as #120's import stored them (its scenes untyped: the files are content's StoryboardDoc) */
export function chapterDocs(): { key: string; doc: ChapterStoryboard }[] {
	return CHAPTERS.map(({ key, dir, chapter }) => {
		const files = readdirSync(join(repo, dir))
			.filter((f) => /^scene-.+\.json$/.test(f))
			.map((f) => ({ path: `${dir}/${f}`, content: JSON.parse(readFileSync(join(repo, dir, f), 'utf8')) }));
		return { key, doc: chapterDoc('catpope', chapter, files) as unknown as ChapterStoryboard };
	});
}

export const plans = () => chapterDocs().map(({ key, doc }) => ({ key, ...sliceOps(doc, PICKS) }));

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
	const all = plans();
	const unused = unusedPicks(all, PICKS);
	if (unused.length) throw new Error(`picks for no 1:4 shot: ${unused.join(', ')}`);
	for (const p of all) {
		const by = Object.entries(Object.groupBy(p.changes, (c) => `${c.from} → ${c.to}`)).map(([k, v]) => `${v?.length} × ${k}`);
		console.log(`${p.key}: ${p.ops.length} shots – ${by.join(', ')}${p.left.length ? `; left: ${p.left.map((l) => `${l.id} ${l.format} ${l.aspect}`).join(', ')}` : ''}`);
	}
	const out = process.argv[2];
	if (out) {
		writeFileSync(out, JSON.stringify(all.map(({ key, ops }) => ({ key, ops })), null, '\t'));
		console.log(`→ ${out}`);
	}
}
