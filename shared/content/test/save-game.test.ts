// The one save game (DATA.md "The save game"): normalize, merge (two devices), the old per-toon states taken in.
//   npm test -w @scutoons/content
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { absorbToonStates, emptySaveGame, mergeCheckpoints, mergeHistory, mergeSaveGame, normalizeSaveGame } from '../src/index.ts';

const cp = (uri: string, updated: string, isComplete = false, position?: number) => ({ uri, isComplete, position, created_at: updated, updated_at: updated });

test('normalize: missing parts filled, junk dropped', () => {
	const s = normalizeSaveGame({ variables: { a: 1, b: 'x', c: null, d: {} }, progress: [cp('catpope/chapter/1', '2026-01-01T00:00:00Z'), { nope: 1 }], history: { k: { n: 2, first: 1, last: 3 }, bad: 5 }, map: null }, '2026-10-06T00:00:00Z');
	assert.deepEqual(s.variables, { a: 1, b: 'x' });
	assert.equal(s.progress.length, 1);
	assert.deepEqual(Object.keys(s.history), ['k']);
	assert.deepEqual(s.map, { places: {}, routes: {}, positions: {} });
	assert.equal(s.created_at, '2026-10-06T00:00:00Z');
});

test('merge: variables are the newer write, checkpoints / history / map merge, created_at the earlier', () => {
	const older = {
		variables: { tips: 1, old: 'yes' },
		progress: [cp('catpope/chapter/1', '2026-01-02T00:00:00Z', true, 1), cp('toptoy/chapter/1', '2026-01-01T00:00:00Z')],
		history: { 'event:a': { n: 3, first: 10, last: 30 }, 'event:only-old': { n: 1, first: 5, last: 5 } },
		map: { places: { harbour: { unlocked: 5 } }, routes: {}, positions: {} },
		created_at: '2026-01-01T00:00:00Z',
		updated_at: '2026-01-03T00:00:00Z'
	};
	const newer = {
		variables: { tips: 2 },
		progress: [cp('catpope/chapter/1', '2026-01-04T00:00:00Z', false, 0.2)],
		history: { 'event:a': { n: 2, first: 20, last: 40 } },
		map: { places: { market: { unlocked: 7 } }, routes: {}, positions: {} },
		created_at: '2026-01-02T00:00:00Z',
		updated_at: '2026-01-05T00:00:00Z'
	};
	const m = mergeSaveGame(newer, older);
	assert.deepEqual(m.variables, { tips: 2 });
	const c1 = m.progress.find((c) => c.uri === 'catpope/chapter/1')!;
	assert.equal(c1.position, 0.2, 'the later checkpoint');
	assert.equal(c1.isComplete, true, 'finished once stays finished');
	assert.ok(m.progress.find((c) => c.uri === 'toptoy/chapter/1'));
	assert.deepEqual(m.history['event:a'], { n: 3, first: 10, last: 40 });
	assert.ok(m.history['event:only-old']);
	assert.deepEqual(Object.keys(m.map.places).sort(), ['harbour', 'market']);
	assert.equal(m.created_at, '2026-01-01T00:00:00Z');
	assert.equal(m.updated_at, '2026-01-05T00:00:00Z');
	// a newer write without variables (an old world state: map only) keeps the older's
	assert.deepEqual(mergeSaveGame({ map: newer.map }, older).variables, older.variables);
});

test('merge is idempotent for the merged parts', () => {
	const a = { progress: [cp('x/chapter/1', '2026-01-01T00:00:00Z')], history: { k: { n: 1, first: 1, last: 1 } } };
	assert.deepEqual(mergeCheckpoints(a.progress, a.progress), a.progress);
	assert.deepEqual(mergeHistory(a.history, a.history), a.history);
});

test('the old per-toon states: the newest state wins a clash, everything else merges', () => {
	const save = { ...emptySaveGame('2026-03-01T00:00:00Z'), updated_at: '2026-03-01T00:00:00Z' };
	const { save: out, clashes } = absorbToonStates(save, [
		{ toon: 'catpope', state: { variables: { tips: 2, visits_harbour: 3 }, progress: [cp('catpope/chapter/2', '2026-02-01T00:00:00Z')], history: { 'event:c': { n: 1, first: 1, last: 1 } }, created_at: '2026-01-01T00:00:00Z', updated_at: '2026-02-01T00:00:00Z' } },
		{ toon: 'toptoy', state: { variables: { tips: 5, visits_hypercity: 1 }, progress: [cp('toptoy/chapter/1', '2026-01-15T00:00:00Z')], history: {}, created_at: '2026-01-10T00:00:00Z', updated_at: '2026-01-15T00:00:00Z' } }
	]);
	// each toon's variables keep their domain (variables.ts): catpope's tips and toptoy's tips are two variables
	assert.deepEqual(out.variables, { 'catpope/tips': 2, 'catpope/visits_harbour': 3, 'toptoy/tips': 5, 'toptoy/visits_hypercity': 1 });
	assert.deepEqual(clashes, []);
	assert.deepEqual(out.progress.map((c) => c.uri).sort(), ['catpope/chapter/2', 'toptoy/chapter/1']);
	assert.ok(out.history['event:c']);
	assert.equal(out.created_at, '2026-01-01T00:00:00Z', 'the earliest start');
	// the save game is newer than both: its own value wins
	const later = { ...emptySaveGame(), variables: { 'catpope/tips': 9 }, updated_at: '2026-05-01T00:00:00Z' };
	const again = absorbToonStates(later, [{ toon: 'catpope', state: { variables: { tips: 2 }, updated_at: '2026-02-01T00:00:00Z' } }]);
	assert.equal(again.save.variables['catpope/tips'], 9);
	assert.deepEqual(again.clashes, ['catpope/tips']);
});

test('a placed pickup is remembered by its placement id; the old `picked:<slice image>#<id>` keys are renamed', () => {
	const { variables } = normalizeSaveGame({
		variables: {
			'catpope/picked:s1/c1/s1c1-slice-1.webp#cucumber-abc1234': 'm60:497632',
			'picked:s1/c2/a.webp#record-x': 1,
			'catpope/picked:s1/c2/b.webp#twin': 'old',
			'catpope/picked:twin': 'new',
			'catpope/coins': 3
		}
	});
	assert.deepEqual(variables, { 'catpope/picked:cucumber-abc1234': 'm60:497632', 'picked:record-x': 1, 'catpope/picked:twin': 'new', 'catpope/coins': 3 });
});
