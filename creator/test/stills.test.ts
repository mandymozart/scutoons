// Stills (lib/shell/stills.ts, #842): the centre cut to the 264 × 168 shape, what this browser makes again, when a
// workspace snapshot is due.
import assert from 'node:assert/strict';
import { test } from 'node:test';
import { centreCut, snapshotDue, wantsStill, SNAPSHOT_EVERY_MS, SNAPSHOT_SETTLE_MS } from '../src/lib/shell/stills.ts';

const thumb = (stamp: string, stale = false) => ({ url: 'u', stamp, stale, origin: 'client' as const, taken_at: null });

test('a picture is cut to the still from its middle: the longer side gives way', () => {
	assert.deepEqual(centreCut(264, 168), { sx: 0, sy: 0, sw: 264, sh: 168 });
	// wide: the sides go
	const wide = centreCut(1920, 1080);
	assert.equal(wide.sh, 1080);
	assert.ok(Math.abs(wide.sw / wide.sh - 264 / 168) < 1e-9);
	assert.ok(Math.abs(wide.sx - (1920 - wide.sw) / 2) < 1e-9);
	// tall (a slice): top and bottom go
	const tall = centreCut(390, 624);
	assert.equal(tall.sw, 390);
	assert.ok(Math.abs(tall.sy - (624 - tall.sh) / 2) < 1e-9);
	assert.deepEqual(centreCut(0, 10), { sx: 0, sy: 0, sw: 0, sh: 0 });
});

test('a still is made again when the server has none, a stale one or one of another stamp – never without the server', () => {
	assert.equal(wantsStill(false, undefined), false);
	assert.equal(wantsStill(true, undefined), true);
	assert.equal(wantsStill(true, thumb('3', true)), true);
	assert.equal(wantsStill(true, thumb('3')), false);
	assert.equal(wantsStill(true, thumb('3'), '4'), true);
	assert.equal(wantsStill(true, thumb('3'), '3'), false);
});

test('a snapshot waits until the workspace held still, and not more often than its interval', () => {
	assert.equal(snapshotDue(1000, null), 1000 + SNAPSHOT_SETTLE_MS);
	assert.equal(snapshotDue(1000, 900), 900 + SNAPSHOT_EVERY_MS);
	assert.equal(snapshotDue(60_000, 1000), 60_000 + SNAPSHOT_SETTLE_MS);
});
