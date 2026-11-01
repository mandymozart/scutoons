// #231: the Live vote demo's data and bots (src/lib/live-vote-demo.ts).
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { changesMind, crowdFor, pickOption, SCENARIOS, voteDelay } from '../src/lib/live-vote-demo.ts';

test('every scenario is a valid vote: labels match the options, the fallback is one of them, first has n', () => {
	assert.deepEqual(new Set(SCENARIOS.map((s) => s.open.vote)).size, SCENARIOS.length);
	for (const { open } of SCENARIOS) {
		for (const l of Object.values(open.labels ?? {})) assert.equal(l.length, open.options.length, open.vote);
		assert.ok((open.fallback ?? 0) < open.options.length, open.vote);
		if (open.rule === 'first') assert.ok((open.n ?? 0) >= 1, open.vote);
		assert.ok(open.question?.en && open.question.de, open.vote);
	}
	assert.deepEqual(new Set(SCENARIOS.map((s) => s.open.rule)), new Set(['groups', 'majority', 'first']));
});

test('the sandbox question is the one the chapter asks', () => {
	assert.equal(SCENARIOS[0].open.question?.de, 'Tauscht die Katze das Löschblatt?');
});

test('pickOption follows the weights', () => {
	assert.equal(pickOption([1, 3], 2, 0), 0);
	assert.equal(pickOption([1, 3], 2, 0.5), 1);
	assert.equal(pickOption([0, 1], 2, 0.01), 1);
	assert.equal(pickOption([], 3, 0.9), 2, 'no weights: the last');
});

test('a phone votes early rather than late, never after 80 % of the time', () => {
	assert.equal(voteDelay(10, 0), 0);
	assert.ok(voteDelay(10, 1) <= 8000);
	assert.ok(voteDelay(10, 0.5) < voteDelay(10, 1) / 2);
	assert.equal(changesMind(0.05), true);
	assert.equal(changesMind(0.5), false);
});

test('a crowd per scenario group; the silent scenario has none', () => {
	assert.deepEqual(crowdFor({ a: [1, 2] }, 5), [{ id: 'a', phones: 5, lean: [1, 2] }]);
	assert.deepEqual(crowdFor(SCENARIOS[3].crowd, 5), []);
});
