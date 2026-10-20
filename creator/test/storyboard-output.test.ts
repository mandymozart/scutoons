// The Export step's Output (src/lib/editor/storyboard-output.ts, #126): the chapter's videos from the records in toon
// order and where they are, the shots a render takes, a download's name.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import type { Shot } from '@scutoons/content/storyboard';
import type { Generation } from '../src/lib/editor/storyboard-generations.ts';
import type { ShotEntry } from '../src/lib/editor/storyboard-shots.ts';
import { downloadName, handheldShots, outputCounts, outputRows } from '../src/lib/editor/storyboard-output.ts';

const gen = (over: Partial<Generation>): Generation => ({
	id: 1, toon: 'catpope', chapter: 3, shot: 'sb-1', kind: 'video', take: null, parents: [], generator: 'runway', model: null, status: 'succeeded',
	prompt: null, seed: null, ratio: null, duration: null, estimate: null, cost: null, currency: null, file: 'sb-1--g1.mp4', machine: 'pc',
	work_path: null, work_url: null, mark: null, approved: false, dataset: false, note: null, error: null, author: null, via: null, created_at: null, ...over
});

test('the videos in toon order – per shot the renders first, the newest first; images, failed and rejected stay out', () => {
	const rows = outputRows(
		[
			gen({ id: 1, shot: 'sb-2' }),
			gen({ id: 2, shot: 'sb-1', generator: 'handheld-render', file: 'sb-1--g2.webm' }),
			gen({ id: 3, shot: 'sb-1', file: 'sb-1--g3.mp4', work_url: '/api/storyboard/work/catpope/3/sb-1--g3.mp4' }),
			gen({ id: 4, shot: 'sb-1', generator: 'handheld-render', file: 'sb-1--g4.webm', machine: 'laptop' }),
			gen({ id: 5, kind: 'image' }),
			gen({ id: 6, status: 'failed' }),
			gen({ id: 7, mark: 'rejected' }),
			gen({ id: 8, shot: 'sb-gone' })
		],
		['sb-1', 'sb-2'],
		new Set(['sb-1--g2.webm', 'sb-1--g1.mp4'])
	);
	assert.deepEqual(rows.map((r) => [r.g.id, r.kind, r.place.at]), [
		[4, 'rendered', 'elsewhere'],
		[2, 'rendered', 'local'],
		[3, 'generated', 'work'],
		[1, 'generated', 'local'],
		[8, 'generated', 'local']
	]);
	assert.deepEqual(outputCounts(rows), { total: 5, local: 3, work: 1, elsewhere: 1 });
});

test('the shots a render takes: the camera on and an active take with samples', () => {
	const s = (id: string, takes?: Shot['takes']): ShotEntry => ({ id, shot: { id, takes } as Shot, scene: {} as ShotEntry['scene'], sceneIndex: 0, path: [] });
	const take = { id: 't1', created: '', smooth: 0, scale: 1, in: 0, out: 1, samples: [{ t: 0, position: [0, 0, 0], quaternion: [0, 0, 0, 1] }, { t: 1, position: [0, 0, 0], quaternion: [0, 0, 0, 1] }] } as NonNullable<Shot['takes']>['takes'][number];
	const got = handheldShots([s('a', { on: true, active: 't1', takes: [take] }), s('b', { on: false, active: 't1', takes: [take] }), s('c', { on: true, active: null, takes: [take] }), s('d')]);
	assert.deepEqual(got.map((x) => [x.entry.id, x.take.id]), [['a', 't1']]);
});

test('a download is named after the toon, the chapter and the file', () => {
	assert.equal(downloadName('catpope', 3, 'sub/sb-1--g2.webm'), 'catpope-3-sb-1--g2.webm');
});
