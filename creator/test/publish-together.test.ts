import assert from 'node:assert/strict';
import { test } from 'node:test';
import { publishTogether, type Version } from '../src/lib/editor/publish-together.ts';

const v = (key: string): Version => ({ key, version: 1 });
const refusal = (code: string, body: object, message = code) => Object.assign(new Error(message), { code, body });

/** a server that refuses `invalid` entries one at a time and names every key of `review` at once */
function server(invalid: string[], review: string[] = []) {
	const calls: string[][] = [];
	const publish = async (versions: Version[]) => {
		calls.push(versions.map((x) => x.key));
		const needs = versions.filter((x) => review.includes(x.key)).map((x) => x.key);
		if (needs.length) throw refusal('review_required', { keys: needs });
		const bad = versions.find((x) => invalid.includes(x.key));
		if (bad) throw refusal('invalid_catalog', { key: bad.key }, `${bad.key}: unknown kind`);
	};
	return { calls, publish };
}

test('everything goes live in one call', async () => {
	const s = server([]);
	assert.deepEqual(await publishTogether([v('a'), v('b')], s.publish), { done: ['a', 'b'], review: [], refused: [] });
	assert.deepEqual(s.calls, [['a', 'b']]);
});

test('an entry the catalog refuses is left out and named, the rest goes live together', async () => {
	const s = server(['instrument:analog_seq']);
	const result = await publishTogether([v('a'), v('instrument:analog_seq'), v('b')], s.publish);
	assert.deepEqual(result.done, ['a', 'b']);
	assert.deepEqual(result.refused, [{ key: 'instrument:analog_seq', why: 'instrument:analog_seq: unknown kind' }]);
	assert.deepEqual(s.calls, [['a', 'instrument:analog_seq', 'b'], ['a', 'b']]);
});

test('several refused entries, and keys that need a review, are set aside one answer at a time', async () => {
	const s = server(['x', 'y'], ['r']);
	const result = await publishTogether([v('r'), v('x'), v('a'), v('y')], s.publish);
	assert.deepEqual(result.done, ['a']);
	assert.deepEqual(result.review, [v('r')]);
	assert.deepEqual(result.refused.map((r) => r.key), ['x', 'y']);
});

test('nothing left to publish is no call', async () => {
	const s = server(['x']);
	assert.deepEqual(await publishTogether([v('x')], s.publish), { done: [], review: [], refused: [{ key: 'x', why: 'x: unknown kind' }] });
	assert.equal(s.calls.length, 1);
});

test('an error that names nothing of the batch stops everything', async () => {
	const down = async () => {
		throw refusal('no_build_service', {});
	};
	await assert.rejects(publishTogether([v('a')], down), /no_build_service/);
	const stranger = async () => {
		throw refusal('invalid_catalog', { key: 'not-in-the-batch' });
	};
	await assert.rejects(publishTogether([v('a')], stranger), /invalid_catalog/);
});
