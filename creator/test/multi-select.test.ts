// moveMany (src/lib/editor/multi-select.ts): the shared Sortable reports (from, to) – `to` the dragged row's index after
// the move; a selection moves with it (chapter tree groups, a group's slices, VN events).
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { moveMany } from '../src/lib/editor/multi-select.ts';

const L = ['a', 'b', 'c', 'd', 'e', 'f'];

test('one row moves like moveAt (down and up)', () => {
	assert.deepEqual(moveMany(L, [], 1, 4).list, ['a', 'c', 'd', 'e', 'b', 'f']);
	assert.deepEqual(moveMany(L, [], 4, 1).list, ['a', 'e', 'b', 'c', 'd', 'f']);
	assert.deepEqual(moveMany(L, [], 0, 5).list, ['b', 'c', 'd', 'e', 'f', 'a']);
	assert.deepEqual(moveMany(L, [2], 2, 2).list, L);
});

test('a row that is not picked moves alone', () => {
	assert.deepEqual(moveMany(L, [3, 4], 0, 2).list, ['b', 'c', 'a', 'd', 'e', 'f']);
});

test('the picked rows go with the dragged one, in their order, where it lands', () => {
	// b + d picked, b dragged to after e (b's index among the others: 3 = after e)
	const down = moveMany(L, [1, 3], 1, 4);
	assert.deepEqual(down.list, ['a', 'c', 'e', 'b', 'd', 'f']);
	assert.equal(down.map.get(1), 3);
	assert.equal(down.map.get(3), 4);
	// d + e picked, e dragged to the top
	assert.deepEqual(moveMany(L, [3, 4], 4, 0).list, ['d', 'e', 'a', 'b', 'c', 'f']);
});
