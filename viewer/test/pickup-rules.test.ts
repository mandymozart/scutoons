// Placed pickups the reader can't take (lib/visual-novel/pickup-rules.ts, agents/ITEMS.md "Placed items" – "Can't take
// it"; owner 2026-10-16: a refused pickup vanished from the slice without anything coming into the bag):
//   npm test -w viewer
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { heldValue, pickState, pickupAnswer, pickupBlock, takenValue } from '../src/lib/visual-novel/pickup-rules.ts';

const sofa = { grantable: ['story' as const] }; // the catalog's furniture: no max = 99
const hours = { hours: 6 };

test('the server had this ref already (granted 0): it stays on the slice, remembered as held – not taken', () => {
	const answer = pickupAnswer({ ok: true, granted: 0 });
	assert.equal(answer.state, 'held');
	assert.equal(answer.error, 'already_taken');
	assert.equal(pickState(heldValue(null, ''), null, ''), 'held', 'once: held for good');
	assert.equal(pickupBlock({ def: sofa, carried: 1, state: 'held' }), 'already_taken', 'a held placement is not offered again');
});

test('a grant that added a piece takes it off the slice; a refusal leaves it there', () => {
	assert.deepEqual(pickupAnswer({ ok: true, granted: 1 }), { state: 'taken' });
	for (const error of ['max_reached', 'rate_limited', 'cooldown', 'failed']) {
		const answer = pickupAnswer({ ok: false, error });
		assert.equal(answer.state, null, `${error}: nothing remembered, it stays`);
		assert.equal(answer.error, error);
		assert.equal(answer.block, undefined, `${error}: may pass – offered again`);
	}
	assert.equal(pickupAnswer({ ok: false }).error, 'failed');
});

test('refusals that last for the page view: the world limit, a content problem', () => {
	assert.equal(pickupAnswer({ ok: false, error: 'taken' }).block, 'taken');
	assert.equal(pickupAnswer({ ok: false, error: 'respawn_out_of_range' }).block, 'not_grantable');
	assert.equal(pickupAnswer(null).block, 'not_grantable', 'not in the catalog');
	assert.equal(pickupBlock({ def: sofa, carried: 0, state: null, refused: 'taken' }), 'taken');
});

test('the collection limit: blocked once the reader carries max (placed furniture does not count – the caller passes carried)', () => {
	assert.equal(pickupBlock({ def: { grantable: ['story'], max: 1 }, carried: 1, state: null }), 'max_reached');
	assert.equal(pickupBlock({ def: { grantable: ['story'], max: 1 }, carried: 0, state: null }), null);
	assert.equal(pickupBlock({ def: sofa, carried: 98, state: null }), null, 'no max = 99');
	assert.equal(pickupBlock({ def: sofa, carried: 99, state: null }), 'max_reached');
});

test('not grantable by a story: never offered', () => {
	assert.equal(pickupBlock({ def: { grantable: ['scan' as const] }, carried: 0, state: null }), 'not_grantable');
	assert.equal(pickupBlock({ def: null, carried: 0, state: null }), 'not_grantable');
});

test('remembered values: taken / held, per period for respawning placements', () => {
	assert.equal(pickState(takenValue(null, ''), null, ''), 'taken');
	assert.equal(pickState(1, null, ''), 'taken', 'the old once value');
	assert.equal(pickState(undefined, null, ''), null);
	const now = 'm360:100';
	assert.equal(pickState(takenValue(hours, now), hours, now), 'taken');
	assert.equal(pickState(heldValue(hours, now), hours, now), 'held');
	assert.equal(pickState(takenValue(hours, 'm360:99'), hours, now), null, 'taken last period: back');
	assert.equal(pickState(heldValue(hours, 'm360:99'), hours, now), null, 'held last period: back');
	assert.equal(pickupBlock({ def: sofa, carried: 0, state: 'taken' }), null, 'taken = hidden by the caller, not blocked');
});
