// The Publish dialog's choice (lib/editor/publish-choice.ts, #857): ticks, the primary's kind, the confirm.
import assert from 'node:assert/strict';
import { test } from 'node:test';
import { decisions, needsConfirm, primaryKind, startTicks, toggled } from '../src/lib/editor/publish-choice.ts';

const lines = [
	{ id: 'character:sandgirl', review: false },
	{ id: 'module:walk', review: false },
	{ id: 'chapter:feilchen:3:de', review: true },
	{ id: 'review:a', review: false, decide: [7, 8] }
];

test('every line starts ticked but what the open page holds back', () => {
	assert.deepEqual([...startTicks(lines, new Set(['module:walk']))], ['character:sandgirl', 'chapter:feilchen:3:de', 'review:a']);
	assert.equal(startTicks([], new Set()).size, 0);
});

test('a tick on and off leaves the rest', () => {
	const ticked = new Set(['character:sandgirl']);
	assert.deepEqual([...toggled(ticked, 'module:walk', true)], ['character:sandgirl', 'module:walk']);
	assert.deepEqual([...toggled(ticked, 'character:sandgirl', false)], []);
	assert.deepEqual([...ticked], ['character:sandgirl']);
});

test('a line the server refuses starts unticked and cannot be ticked', () => {
	const withRefused = [...lines, { id: 'scene:catpope:1:intro:de', review: false, refused: 'Unknown entity key.' }];
	assert.equal(startTicks(withRefused, new Set()).has('scene:catpope:1:intro:de'), false);
	assert.equal(startTicks(withRefused, new Set()).size, 4);
	assert.deepEqual([...toggled(new Set(), 'scene:catpope:1:intro:de', true, withRefused)], []);
	assert.deepEqual([...toggled(new Set(), 'module:walk', true, withRefused)], ['module:walk']);
});

test('the primary submits as soon as a ticked line goes to review', () => {
	assert.equal(primaryKind(lines, new Set(['character:sandgirl'])), 'publish');
	assert.equal(primaryKind(lines, new Set(['character:sandgirl', 'chapter:feilchen:3:de'])), 'submit');
});

test('a confirm comes for the open page’s errors or someone else’s submit', () => {
	assert.equal(needsConfirm(lines, new Set(['character:sandgirl']), 0), false);
	assert.equal(needsConfirm(lines, new Set(['character:sandgirl']), 2), true);
	assert.equal(needsConfirm(lines, new Set(['review:a']), 0), true);
	assert.deepEqual(decisions(lines, new Set(['review:a', 'module:walk'])).map((l) => l.id), ['review:a']);
});
