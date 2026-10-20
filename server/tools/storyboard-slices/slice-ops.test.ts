// node --experimental-transform-types --no-warnings --test server/tools/storyboard-slices/slice-ops.test.ts
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { aspectSlices, validateChapter, type ChapterStoryboard } from '@scutoons/content/storyboard';
import { sliceOps, unusedPicks, type Op } from './slice-ops.ts';
import { PICKS, chapterDocs, plans } from './main.ts';

type Shots = Record<string, unknown>[];
const doc = (shots: Shots) => ({ scenes: [{ groups: [{ shots: [] as Shots }, { shots }] }] });
/** applies `set` ops the way save_draft does (path = scenes, i, groups, j, shots, k, field) */
function apply<D>(d: D, ops: Op[]): D {
	for (const { path, value } of ops) {
		let at: Record<string | number, unknown> = d as never;
		for (const key of path.slice(0, -1)) at = at[key] as never;
		at[path.at(-1) as string] = value;
	}
	return d;
}

test('the fixed moves, the picks, and what stays', () => {
	const d = doc([
		{ id: 'a', format: 'viewport', aspect: '9:16' },
		{ id: 'b', format: 'vertical', aspect: '1:3' },
		{ id: 'c', format: 'vertical', aspect: '1:4' },
		{ id: 'd', format: 'vertical', aspect: '1:4.8' },
		{ id: 'e', format: 'strip', aspect: '3:1' },
		{ id: 'f', format: 'loop360', aspect: '8:1' },
		{ id: 'g', format: 'vertical', aspect: '1:5' }
	]);
	const p = sliceOps(d, { c: '1:4.8' });
	assert.deepEqual(p.ops, [
		{ op: 'set', path: ['scenes', 0, 'groups', 1, 'shots', 0, 'aspect'], value: '1:1.6' },
		{ op: 'set', path: ['scenes', 0, 'groups', 1, 'shots', 1, 'aspect'], value: '1:3.2' },
		{ op: 'set', path: ['scenes', 0, 'groups', 1, 'shots', 2, 'aspect'], value: '1:4.8' }
	]);
	assert.deepEqual(p.changes.map((c) => [c.id, c.picked]), [['a', false], ['b', false], ['c', true]]);
	assert.deepEqual(p.left, [{ id: 'g', format: 'vertical', aspect: '1:5' }]); // no rule for 1:5 – the owner's
	assert.deepEqual(sliceOps(apply(d, p.ops), { c: '1:4.8' }).ops, []); // a second run plans nothing
});

test('a viewport becomes one slice whatever it was', () => {
	assert.equal(sliceOps(doc([{ id: 'v', format: 'viewport', aspect: '1:3' }]), {}).ops[0].value, '1:1.6');
});

test('a shot at 1:4 needs a valid pick – nothing guessed', () => {
	const d = doc([{ id: 'c', format: 'vertical', aspect: '1:4' }]);
	assert.throws(() => sliceOps(d, {}), /c: vertical 1:4 needs a pick/);
	assert.throws(() => sliceOps(d, { c: '1:6.4' }), /got 1:6.4/);
});

test('unused picks are named', () => {
	const p = sliceOps(doc([{ id: 'c', format: 'vertical', aspect: '1:4' }]), { c: '1:3.2', x: '1:4.8' });
	assert.deepEqual(unusedPicks([p], { c: '1:3.2', x: '1:4.8' }), ['x']);
});

test('chapters 4–5: 70 shots onto whole slices, every pick used, nothing left, only aspects', () => {
	const all = plans();
	assert.deepEqual(all.map((p) => [p.key, p.ops.length]), [['storyboard:catpope:4', 42], ['storyboard:catpope:5', 28]]);
	assert.deepEqual(all.map((p) => p.left), [[], []]);
	assert.deepEqual(unusedPicks(all, PICKS), []);
	const changes = all.flatMap((p) => p.changes);
	assert.equal(changes.filter((c) => c.picked).length, 13);
	assert.ok(changes.every((c) => aspectSlices(c.to)));
	assert.ok(all.flatMap((p) => p.ops).every((o) => o.path.at(-1) === 'aspect'));
});

test('chapters 4–5 after the changes: the content validator has no slice warning left, no error', () => {
	for (const { key, doc } of chapterDocs()) {
		const slices = (d: ChapterStoryboard) => validateChapter(d).warnings.filter((w) => /slices/.test(w));
		const errors = validateChapter(doc).errors.length;
		assert.ok(slices(doc).length > 0, `${key} had slice warnings`);
		apply(doc, sliceOps(doc, PICKS).ops);
		assert.deepEqual(slices(doc), [], key);
		assert.equal(validateChapter(doc).errors.length, errors, `${key}: no new errors`);
	}
});
