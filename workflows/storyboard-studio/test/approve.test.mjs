// approve.mjs (scripts/storyboard) on a temp repo: the dataset line names its chapter folder and which generated file was
// chosen (`files.from`, VARIANTS-CANVAS §3.5) – null for an image that is not one of the shot's results; the studio's
// timeline (clips.mjs) then marks the chosen per kind by `from`, not by the copy's bytes.
import { after, test } from 'node:test';
import assert from 'node:assert/strict';
import { mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join } from 'node:path';
import { approve } from '../../../scripts/storyboard/approve.mjs';
import { chapterClips } from '../clips.mjs';

const root = mkdtempSync(join(tmpdir(), 'studio-approve-'));
after(() => rmSync(root, { recursive: true, force: true }));
const put = (rel, bytes) => { const f = join(root, rel); mkdirSync(dirname(f), { recursive: true }); writeFileSync(f, bytes); return f; };
const manifest = () => readFileSync(join(root, 'storyboards/dataset/manifest.jsonl'), 'utf8').trim().split('\n').map(JSON.parse);
const src = join(root, 'storyboards/cat/c1');

put('storyboards/cat/c1/scene-1.json', JSON.stringify({ toon: 'cat', chapter: 1, source: { scene: 1 }, groups: [{ shots: [{ id: 's-1' }, { id: 's-2' }] }] }));
const g2 = put('storyboards/_out/c1/generated/s-1--g2.mp4', 'video two');
put('storyboards/_out/c1/generated/s-1--g3.mp4', 'video two'); // the same bytes – only `from` tells them apart
const g1 = put('storyboards/_out/c1/generated/s-1--g1.png', 'image one');
const photo = put('elsewhere/photo.png', 'a photo');

test('approve writes ch + files.from (the generated file chosen), null for an image from elsewhere; a change writes no dataset line', () => {
	assert.equal(approve({ root, src, id: 's-1', image: g2, note: 'the clip', today: '2026-10-20' }), 's-1: ok → dataset');
	assert.equal(approve({ root, src, id: 's-1', image: g1, today: '2026-10-20' }), 's-1: ok → dataset');
	approve({ root, src, id: 's-2', image: photo, today: '2026-10-20' });
	assert.equal(approve({ root, src, id: 's-2', change: 'lower camera', today: '2026-10-20' }), 's-2: change – lower camera');
	const lines = manifest();
	assert.deepEqual(lines.map((l) => [l.ch, l.shot, l.files]), [
		['c1', 's-1', { render: false, chosen: 'chosen.mp4', from: 's-1--g2.mp4' }],
		['c1', 's-1', { render: false, chosen: 'chosen.png', from: 's-1--g1.png' }],
		['c1', 's-2', { render: false, chosen: 'chosen.png', from: null }]
	]);
	assert.deepEqual([lines[0].toon, lines[0].chapter, lines[0].note], ['cat', 1, 'the clip']);
	assert.equal(readFileSync(join(root, 'storyboards/dataset/files/c1/s-1/chosen.mp4'), 'utf8'), 'video two');
	const doc = JSON.parse(readFileSync(join(src, 'scene-1.json'), 'utf8'));
	assert.deepEqual(doc.groups[0].shots.map((s) => s.review), [{ status: 'ok', date: '2026-10-20' }, { status: 'change', note: 'lower camera', date: '2026-10-20' }]);
	assert.throws(() => approve({ root, src, id: 's-9' }), /no shot s-9/);
});

test('clips: the chosen per kind comes from `from` – g2 although g3 has the same bytes (and is newer)', () => {
	const shots = chapterClips({ boards: join(root, 'storyboards'), toon: 'cat', ch: 'c1' });
	assert.deepEqual(shots['s-1'].generated.map((g) => [g.file, g.approved]), [['s-1--g3.mp4', false], ['s-1--g2.mp4', true], ['s-1--g1.png', true]]);
	// a newer approval of the image names another file: the mark moves, the video's stays
	approve({ root, src, id: 's-1', image: put('storyboards/_out/c1/generated/s-1--g4.png', 'image four'), today: '2026-10-21' });
	const again = chapterClips({ boards: join(root, 'storyboards'), toon: 'cat', ch: 'c1' })['s-1'];
	assert.deepEqual(again.generated.filter((g) => g.approved).map((g) => g.file), ['s-1--g4.png', 's-1--g2.mp4']);
});
