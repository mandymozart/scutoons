// The storyboard tools' pure parts (#122): the outline and shot paths, the checks, the generation bodies, the creator
// link, the references found in a storyboards folder.
import { strict as assert } from 'node:assert';
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { test } from 'node:test';
import type { ChapterStoryboard, StoryboardDoc } from '@scutoons/content/storyboard';
import { creatorHref } from '../src/services.ts';
import { approveBody, checkStoryboard, compactGeneration, generationTotals, locateShot, markBody, outline, recordBody, shotIds, storyboardHref, type Generation } from '../src/storyboard-model.ts';
import { shotReferences, studioFolders } from '../src/storyboard-refs.ts';

const scene = (n: number, shots: string[], extra: Partial<StoryboardDoc> = {}): StoryboardDoc => ({
	storyboard: 2,
	toon: 'catpope',
	chapter: 3,
	source: { scene: n, heading: `SCENE ${n}` },
	cast: [{ ref: 'plf', character: 'catpope', persona: 'human' }],
	groups: [
		{
			label: `scene-${n}`,
			shots: shots.map((id) => ({
				id,
				format: 'vertical',
				aspect: '9:16',
				status: 'draft',
				action: { de: 'PLF steht im Schnee.' },
				figures: [{ cast: 'plf', framing: 'full', mood: 'cold' }],
				dialog: [{ rect: { x: 5, y: 5, width: 60 }, text: { de: 'Es war bitterkalt.' } }]
			}))
		}
	],
	...extra
});
const doc = (): ChapterStoryboard => ({ storyboard: 2, toon: 'catpope', chapter: 3, scenes: [scene(25, ['sb-25-1', 'sb-25-2']), scene(26, ['sb-26-1'])] });

test('outline: scenes → groups → shots with their paths and counts', () => {
	const o = outline(doc());
	assert.equal(o.scenes, 2);
	assert.equal(o.shots, 3);
	const shot = o.outline[0].groups[0].shots![1];
	assert.deepEqual(shot.path, ['scenes', 0, 'groups', 0, 'shots', 1]);
	assert.equal(shot.id, 'sb-25-2');
	assert.equal(shot.format, 'vertical');
	assert.equal(shot.aspect, '9:16');
	assert.equal(shot.status, 'draft');
	assert.deepEqual(shot.figures, ['plf']);
	assert.equal(shot.dialog, 1);
	assert.ok(typeof shot.seconds === 'number' && shot.seconds > 0);
	// empty fields stay out
	assert.equal('review' in shot, false);
	assert.equal('move' in shot, false);
	assert.deepEqual(o.outline[1].path, ['scenes', 1]);
});

test('outline: a review is listed, an empty document has no shots', () => {
	const d = doc();
	d.scenes[0].groups[0].shots![0].review = { status: 'change', note: 'more air' };
	assert.deepEqual(outline(d).outline[0].groups[0].shots![0].review, { status: 'change', note: 'more air' });
	assert.equal(outline({ storyboard: 2, toon: 'x', chapter: 1, scenes: [] }).shots, 0);
});

test('locateShot: the shot, its scene and the path save_draft ops use', () => {
	const d = doc();
	const found = locateShot(d, 'sb-26-1');
	assert.ok(found);
	assert.equal(found.scene.source.scene, 26);
	assert.deepEqual(found.path, ['scenes', 1, 'groups', 0, 'shots', 0]);
	assert.equal(locateShot(d, 'sb-99-1'), null);
	assert.deepEqual(shotIds(d), ['sb-25-1', 'sb-25-2', 'sb-26-1']);
});

test('checkStoryboard: validateChapter + the rhythm', () => {
	const ok = checkStoryboard(doc());
	assert.deepEqual(ok.errors, []);
	assert.equal(ok.rhythm!.scenes.length, 2);
	assert.equal(ok.rhythm!.total.shots, 3);
	assert.ok(ok.rhythm!.total.seconds > 0);
	const bad = doc();
	bad.scenes[1].groups[0].shots![0].id = 'sb-25-1';
	bad.scenes[0].groups[0].shots![0].format = 'square' as never;
	const r = checkStoryboard(bad);
	assert.ok(r.errors.some((e) => e.includes('duplicate id')), r.errors.join('\n'));
	assert.ok(r.errors.some((e) => e.includes('format "square"')), r.errors.join('\n'));
	assert.deepEqual(checkStoryboard(null), { errors: ['chapter: no storyboard document'], warnings: [] });
	assert.equal(checkStoryboard({ storyboard: 2, toon: 'x', chapter: 1 }).rhythm, undefined);
});

test('recordBody: only given fields, kind from the extension, the hash in lower case', () => {
	const base = { toon: 'catpope', chapter: 3, shot: 'sb-25-1', file: 'sb-25-1--g1.MP4', machine: 'studio-pc', generator: 'comfy-local' };
	assert.deepEqual(recordBody(base), { ...base, kind: 'video' });
	const b = recordBody({ ...base, file: 'sb-25-1--g2.png', file_hash: 'A'.repeat(64), seed: 7, prompt: undefined });
	assert.equal(b.kind, 'image');
	assert.equal(b.file_hash, 'a'.repeat(64));
	assert.equal('prompt' in b, false);
	assert.equal(recordBody({ ...base, kind: 'image' }).kind, 'image');
});

test('markBody / approveBody: none clears, a note only when given', () => {
	assert.deepEqual(markBody('none'), { mark: null });
	assert.deepEqual(markBody('review', 'look at the light'), { mark: 'review', note: 'look at the light' });
	assert.deepEqual(approveBody(true), { approved: true });
	assert.deepEqual(approveBody(false, ''), { approved: false, note: '' });
});

test('compactGeneration / generationTotals: short records, counts and cost', () => {
	const g = (id: number, extra: Partial<Generation> = {}): Generation => ({
		id, toon: 'catpope', chapter: 3, shot: 'sb-25-1', kind: 'image', status: 'succeeded', prompt: 'x'.repeat(200), inputs: [{ role: 'still' }], params: null,
		parents: [], mark: null, approved: false, cost: null, currency: null, file: `sb-25-1--g${id}.png`, machine: 'pc', ...extra
	});
	const c = compactGeneration(g(1));
	assert.equal(c.id, 1);
	assert.equal(c.file, 'sb-25-1--g1.png');
	assert.equal((c.prompt as string).length, 81);
	for (const k of ['inputs', 'params', 'parents', 'mark', 'approved', 'toon', 'chapter']) assert.equal(k in c, false, k);
	const t = generationTotals([g(1, { mark: 'review', cost: 0.5, currency: 'USD' }), g(2, { approved: true, cost: 0.25, currency: 'USD' }), g(3, { status: 'failed' })]);
	assert.deepEqual(t, { count: 3, status: { succeeded: 2, failed: 1 }, mark: { review: 1, none: 2 }, approved: 1, cost: { USD: 0.75 } });
});

test('the creator link: the storyboard page, also for save_draft on storyboard:…', () => {
	const links = { creator: 'https://creator.scutoons.com', admin: '', embassy: '', viewer: '' };
	assert.equal(storyboardHref(links.creator, 'catpope', 3), 'https://creator.scutoons.com/catpope/chapter/3/storyboard');
	assert.equal(creatorHref(links, 'storyboard:catpope:3'), 'https://creator.scutoons.com/catpope/chapter/3/storyboard');
	assert.equal(creatorHref(links, 'chapter:catpope:3'), 'https://creator.scutoons.com/catpope/chapter/3');
});

test('studioFolders: the env setting, a checkout, nothing in a member\'s copy', () => {
	const dir = mkdtempSync(join(tmpdir(), 'sb-folders-'));
	try {
		mkdirSync(join(dir, 'storyboards'));
		assert.deepEqual(studioFolders({}, dir, false), { storyboards: join(dir, 'storyboards'), media: null });
		assert.deepEqual(studioFolders({}, dir, true), { storyboards: null, media: null });
		assert.equal(studioFolders({ SCUTOONS_STORYBOARDS: join(dir, 'storyboards') }, '/nowhere', true).storyboards, join(dir, 'storyboards'));
		assert.equal(studioFolders({ SCUTOONS_STORYBOARDS: join(dir, 'missing') }, dir, false).storyboards, null);
	} finally {
		rmSync(dir, { recursive: true, force: true });
	}
});

test('shotReferences: the render, the style pictures, the character image – and what is missing', () => {
	const dir = mkdtempSync(join(tmpdir(), 'sb-refs-'));
	try {
		const sb = join(dir, 'storyboards');
		mkdirSync(join(sb, 'catpope', 's1c3'), { recursive: true });
		mkdirSync(join(sb, '_out', 's1c3', 'clean'), { recursive: true });
		mkdirSync(join(sb, '_out', 's1c3', 'frames'), { recursive: true });
		mkdirSync(join(sb, 'style', 'refs', 'catpope', 'human', 'default'), { recursive: true });
		writeFileSync(join(sb, '_out', 's1c3', 'clean', 'sb-25-1.png'), '');
		writeFileSync(join(sb, 'style', 'look.png'), '');
		writeFileSync(join(sb, 'style', 'notes.md'), '');
		writeFileSync(join(sb, 'style', 'refs', 'catpope', 'human', 'default', 'full-1.jpg'), '');
		const folders = { storyboards: sb, media: null };
		const d = doc();
		const r = shotReferences(folders, 'catpope', 3, d.scenes[0], d.scenes[0].groups[0].shots![0]);
		assert.equal(r.structure, join(sb, '_out', 's1c3', 'clean', 'sb-25-1.png'));
		assert.deepEqual(r.style, [join(sb, 'style', 'look.png')]);
		assert.deepEqual(r.characters, [join(sb, 'style', 'refs', 'catpope', 'human', 'default', 'full-1.jpg')]);
		assert.deepEqual(r.missing, []);
		const none = shotReferences({ storyboards: null, media: null }, 'catpope', 3, d.scenes[0], d.scenes[0].groups[0].shots![1]);
		assert.equal(none.structure, null);
		assert.equal(none.missing.length, 3);
	} finally {
		rmSync(dir, { recursive: true, force: true });
	}
});
