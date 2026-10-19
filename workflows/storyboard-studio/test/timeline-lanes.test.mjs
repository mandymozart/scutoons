// The studio's timeline lanes (src/lib/timeline-lanes.js): seconds → ms back to back, the 1 s fallback, the scene
// markers, the take on the chapter axis, the reveal range.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { revealOf, shotsLane, takeLane } from '../src/lib/timeline-lanes.js';

const shots = [
	{ id: 'sb-25-1', seconds: 8.25, scene: 25, review: 'ok' },
	{ id: 'sb-25-2', seconds: 0, scene: 25 },
	{ id: 'sb-26-1', seconds: 4.5, scene: 26, review: 'change' }
];

test('shots lie back to back in ms, a missing reading time counts 1 s', () => {
	const { lane, span } = shotsLane(shots);
	assert.deepEqual(lane.items.map((i) => [i.id, i.start, i.length]), [['sb-25-1', 0, 8250], ['sb-25-2', 8250, 1000], ['sb-26-1', 9250, 4500]]);
	assert.equal(span, 13750);
	assert.equal(lane.tall, true);
	assert.deepEqual(lane.items.map((i) => i.state), ['ok', '', 'change']);
});

test('a marker at each scene start', () => {
	assert.deepEqual(shotsLane(shots).markers, [{ at: 0, label: '25' }, { at: 9250, label: '26' }]);
});

test('no shots: an empty lane, span 0', () => {
	const { lane, markers, span } = shotsLane([]);
	assert.equal(lane.items.length, 0);
	assert.equal(markers.length, 0);
	assert.equal(span, 0);
});

test('the take sits at its shot\'s start, its kept part as the trim', () => {
	const take = { id: 't2', in: 1.2, out: 4.8, samples: [{ t: 0 }, { t: 3 }, { t: 6.01 }] };
	const lane = takeLane(take, 8250);
	assert.equal(lane.id, 'take');
	assert.deepEqual(lane.items[0], { id: 't2', start: 8250, length: 6010, label: 't2', trim: { in: 1200, out: 4800, min: 100 } });
	assert.ok(Math.abs(lane.step - 1000 / 24) < 1e-9);
});

test('no take, or a take without samples: no lane', () => {
	assert.equal(takeLane(null, 0), null);
	assert.equal(takeLane({ id: 't1', in: 0, out: 0, samples: [] }, 0), null);
});

test('reveal: the shot, or longer when its take runs past it', () => {
	const { lane } = shotsLane(shots);
	assert.deepEqual(revealOf(lane.items[0], null), [0, 8250]);
	const take = takeLane({ id: 't1', in: 0, out: 9, samples: [{ t: 0 }, { t: 9 }] }, 0);
	assert.deepEqual(revealOf(lane.items[0], take), [0, 9000]);
	assert.equal(revealOf(null, take), null);
});
