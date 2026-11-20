// The storyboard's door to the core's follow (src/lib/handheld/follow.ts): the wasm's arrays and JSON text as plain values. The
// recording itself is the core's (deck-space dop/take.rs; shared/live-deck/test/hand.test.ts runs it on the real wasm).
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { followPort } from '../src/lib/handheld/follow.ts';
import type { FollowCore } from '../src/lib/handheld/hand-port.ts';

const core = (over: Partial<FollowCore> = {}): FollowCore => ({ pose: () => true, recenter() {}, keep() {}, secs: () => 1.5, held: () => new Float64Array(), take: () => '', free() {}, ...over });

test('before the first pose and for a take too short: nothing', () => {
	const f = followPort(core());
	assert.equal(f.held(), null);
	assert.equal(f.take(), null);
	assert.equal(f.secs(), 1.5);
});

test('the camera now and the take come out as plain values; the poses go in as the core takes them', () => {
	const calls: string[] = [];
	const f = followPort(
		core({
			pose: (t, p, q) => (calls.push(`${t}:${[...p]}:${[...q]}`), true),
			keep: (on) => void calls.push(`keep ${on}`),
			held: () => Float64Array.of(1, 2, 3, 0, 0, 0, 1),
			take: () => JSON.stringify([{ t: 0, position: [0, 1.5, 0], quaternion: [0, 0, 0, 1] }, { t: 2, position: [2, 1.5, 0], quaternion: [0, 0, 0, 1] }])
		})
	);
	assert.equal(f.pose(10, [0, 1, 0], [0, 0, 0, 1]), true);
	f.keep(true);
	f.keep(false);
	assert.deepEqual(calls, ['10:0,1,0:0,0,0,1', 'keep true', 'keep false']);
	assert.deepEqual(f.held(), { position: [1, 2, 3], quaternion: [0, 0, 0, 1] });
	assert.deepEqual(f.take()?.map((k) => k.t), [0, 2]);
});
