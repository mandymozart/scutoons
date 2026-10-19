import assert from 'node:assert/strict';
import { test } from 'node:test';
import { clampTrim, followView, formatTime, frac, inView, itemAt, msOf, ordered, placeOf, revealView, sequence, tickStep, ticks, timeAt, zoomAround, ZOOM_MAX, ZOOM_MIN, type TimelineItem } from '../src/timeline.ts';

const near = (actual: number | null, expected: number, what = '') => assert.ok(actual !== null && Math.abs(actual - expected) < 1e-6, `${what}: ${actual} ≠ ${expected}`);

// a chapter of shots: 4 s, an empty shot (0 s), 2.5 s, 1.5 s → 8 s; the empty one shares its start with the next
const shots = sequence([
	{ id: 'a', length: 4000 },
	{ id: 'zero', length: 0 },
	{ id: 'b', length: 2500 },
	{ id: 'c', length: 1500 }
]);

test('sequence lays items back to back, numbers them and counts a negative length as 0', () => {
	assert.deepEqual(
		shots.map((s) => [s.id, s.start, s.length, s.seq]),
		[['a', 0, 4000, 0], ['zero', 4000, 0, 1], ['b', 4000, 2500, 2], ['c', 6500, 1500, 3]]
	);
	assert.deepEqual(sequence([{ id: 'x', length: -5 }, { id: 'y', length: 10 }], 100).map((s) => [s.start, s.length]), [[100, 0], [100, 10]]);
	assert.deepEqual(ordered([...shots].reverse()).map((s) => s.id), ['a', 'zero', 'b', 'c'], 'ordered sorts by start, then seq');
});

test('itemAt: half-open items, boundaries, ties by seq, gaps and past the end', () => {
	assert.equal(itemAt(shots, 0)?.id, 'a');
	assert.equal(itemAt(shots, 3999.9)?.id, 'a');
	assert.equal(itemAt(shots, 4000)?.id, 'b', 'at an equal start the higher seq that covers t wins over the empty shot');
	assert.equal(itemAt(shots, 6500)?.id, 'c', 'a boundary belongs to the item that starts there');
	assert.equal(itemAt(shots, 7999)?.id, 'c');
	assert.equal(itemAt(shots, 8000), null, 'the end of the last item is outside');
	assert.equal(itemAt(shots, 9000), null, 'past the end');
	assert.equal(itemAt(shots, -1), null, 'before the first');
	assert.equal(itemAt([], 0), null);
	// cues (instants) and a gap: an instant is only at its start; equal starts → the higher seq
	const cues: TimelineItem[] = ordered([
		{ id: 'late', start: 1000, seq: 2 },
		{ id: 'early', start: 1000, seq: 1 },
		{ id: 'clip', start: 2000, length: 500 },
		{ id: 'end', start: 3000 }
	]);
	assert.equal(itemAt(cues, 1000)?.id, 'late');
	assert.equal(itemAt(cues, 1001), null, 'an instant covers only its start');
	assert.equal(itemAt(cues, 2600), null, 'a gap');
	assert.equal(itemAt(cues, 3000)?.id, 'end', 'an instant at the very end');
});

test('placeOf / msOf round trip, zero-length items and clamping', () => {
	for (const t of [0, 1234, 3999, 4000, 5250, 6500, 7999.5]) near(msOf(shots, placeOf(shots, t)!), t, `ms ${t}`);
	assert.deepEqual(placeOf(shots, 5250), { id: 'b', at: 0.5 });
	for (const place of [{ id: 'a', at: 0.25 }, { id: 'b', at: 0 }, { id: 'c', at: 0.999 }]) {
		const back = placeOf(shots, msOf(shots, place)!)!;
		assert.equal(back.id, place.id);
		near(back.at, place.at, place.id);
	}
	// an empty item is its start, whatever `at` says; its place in time belongs to the next item
	assert.equal(msOf(shots, { id: 'zero', at: 0.7 }), 4000);
	assert.deepEqual(placeOf(shots, 4000), { id: 'b', at: 0 });
	// clamped: before the first, past the end, `at` outside 0..1, unknown ids, no items
	assert.deepEqual(placeOf(shots, -500), { id: 'a', at: 0 });
	assert.deepEqual(placeOf(shots, 8000), { id: 'c', at: 1 });
	assert.deepEqual(placeOf(shots, 1e9), { id: 'c', at: 1 });
	assert.equal(msOf(shots, { id: 'c', at: 2 }), 8000);
	assert.equal(msOf(shots, { id: 'a', at: -1 }), 0);
	assert.equal(msOf(shots, { id: 'nope', at: 0 }), null);
	assert.equal(placeOf([], 10), null);
	// a gap: the item before, at its end
	assert.deepEqual(placeOf([{ id: 'p', start: 0, length: 100 }, { id: 'q', start: 500, length: 100 }], 300), { id: 'p', at: 1 });
});

test('frac and timeAt are inverses on the span and clamp outside it', () => {
	for (const t of [0, 1, 2500, 7999, 8000]) near(timeAt(frac(t, 8000), 8000), t, `t ${t}`);
	for (const f of [0, 0.125, 0.5, 1]) near(frac(timeAt(f, 8000), 8000), f, `f ${f}`);
	assert.equal(frac(-10, 8000), 0);
	assert.equal(frac(9000, 8000), 1);
	assert.equal(frac(10, 0), 0, 'an empty span');
	assert.equal(timeAt(1.5, 8000), 8000);
});

test('zoomAround keeps the point under the pointer and stays inside the zoom range and the strip', () => {
	for (const [zoom, factor, anchor, scroll] of [[1, 2, 0.5, 0], [4, 1.25, 0.2, 0.3], [8, 0.8, 0.9, 0.5], [2, 3, 0.75, 0.25]]) {
		const v = zoomAround(zoom, factor, anchor, scroll);
		near(v.zoom, zoom * factor, 'zoom');
		near(v.scrollFrac + anchor / v.zoom, scroll + anchor / zoom, `anchor ${zoom}×${factor}`);
	}
	assert.deepEqual(zoomAround(1, 0.5, 0.5, 0), { zoom: ZOOM_MIN, scrollFrac: 0 }, 'never below the whole span');
	assert.equal(zoomAround(60, 2, 0.5, 0.5).zoom, ZOOM_MAX);
	const right = zoomAround(2, 1.1, 1, 0.5);
	assert.ok(right.scrollFrac <= 1 - 1 / right.zoom + 1e-9, 'the view never runs past the strip');
});

test('revealView scrolls / zooms a range into view, and leaves a visible one alone', () => {
	const v = { zoom: 4, scrollFrac: 0 };
	assert.equal(revealView(v, [1000, 1500], 8000), v, 'already visible: unchanged');
	const far = revealView(v, [6000, 6500], 8000);
	assert.ok(inView(far, 6000 / 8000) && inView(far, 6500 / 8000));
	const wide = revealView(v, [0, 8000], 8000);
	assert.equal(wide.zoom, ZOOM_MIN, 'a range wider than the view zooms out');
	const close = revealView({ zoom: 1, scrollFrac: 0 }, [4000, 4100], 8000, 0.5);
	near(close.zoom * (100 / 8000), 0.5, 'fill: the range takes half the view');
	assert.ok(inView(close, 4000 / 8000) && inView(close, 4100 / 8000));
});

test('followView: a playhead in view stays put; one that left comes back at a third of the view, inside the strip', () => {
	const v = { zoom: 4, scrollFrac: 0.25 }; // shows 0.25 … 0.5 of the strip
	assert.ok(inView(v, 0.25) && inView(v, 0.5) && !inView(v, 0.51) && !inView(v, 0.2));
	assert.equal(followView(v, 0.4), v, 'in view: unchanged');
	near(followView(v, 0.6).scrollFrac, 0.6 - 1 / 12, 'past the view: at a third');
	assert.ok(inView(followView(v, 0.6), 0.6));
	assert.equal(followView(v, 0.99).scrollFrac, 0.75, 'never past the strip');
	assert.equal(followView(v, 0.01).scrollFrac, 0, 'never before it');
	const whole = { zoom: 1, scrollFrac: 0 };
	assert.equal(followView(whole, 1), whole, 'zoom 1 shows everything');
});

test('ticks: 1-2-5 steps per unit (ms, s, min, h), at most max per view, labels from formatTime', () => {
	const mantissa = (step: number) => {
		const unit = step >= 36e5 ? 36e5 : step >= 6e4 ? 6e4 : step >= 1e3 ? 1e3 : 1;
		return (step / unit) / 10 ** Math.floor(Math.log10(step / unit));
	};
	for (const span of [500, 8000, 95_000, 30 * 60_000, 2 * 3_600_000]) {
		for (const zoom of [1, 3, 16, 64]) {
			for (const max of [4, 10, 25]) {
				const step = tickStep(span / zoom, max);
				assert.ok([1, 2, 5].includes(mantissa(step)), `step ${step} for ${span} @${zoom}`);
				const all = ticks(span, zoom, max);
				const view = all.filter((t) => t.at <= span / zoom);
				assert.ok(view.length <= max, `${view.length} ticks > ${max} in a view of ${span / zoom} ms`);
				assert.ok(all.every((t, i) => i === 0 || Math.abs(t.at - all[i - 1].at - step) < 1e-6));
			}
		}
	}
	assert.deepEqual(ticks(8000, 1, 5).map((t) => t.label), ['0:00', '0:02', '0:04', '0:06', '0:08']);
	assert.deepEqual(ticks(1000, 1, 6).map((t) => t.label), ['0:00.0', '0:00.2', '0:00.4', '0:00.6', '0:00.8', '0:01.0']);
	assert.deepEqual(ticks(60_000, 1, 5, [20_000, 45_000]).map((t) => t.at), [20_000, 40_000], 'only the visible range');
	assert.deepEqual(ticks(0, 1, 5), []);
});

test('clampTrim keeps the bounds and the minimum; the dragged edge stops at the other', () => {
	const len = 3000;
	assert.deepEqual(clampTrim({ in: -200, out: 4000, min: 500 }, len), { in: 0, out: 3000, min: 500 });
	assert.deepEqual(clampTrim({ in: 2800, out: 2900, min: 500 }, len), { in: 2500, out: 3000, min: 500 }, 'too short near the end');
	assert.deepEqual(clampTrim({ in: 1000, out: 1100, min: 500 }, len), { in: 1000, out: 1500, min: 500 });
	assert.deepEqual(clampTrim({ in: 1900, out: 2000, min: 500 }, len, 'in'), { in: 1500, out: 2000, min: 500 }, 'in stops before out');
	assert.deepEqual(clampTrim({ in: 1000, out: 1200, min: 500 }, len, 'out'), { in: 1000, out: 1500, min: 500 }, 'out stops after in');
	assert.deepEqual(clampTrim({ in: 0, out: 3000, min: 9000 }, len), { in: 0, out: 3000, min: 3000 }, 'min never exceeds the length');
	for (const t of [{ in: 500, out: 400, min: 0 }, { in: -1, out: -1, min: 100 }, { in: 5000, out: 6000, min: 250 }]) {
		for (const edge of [undefined, 'in', 'out'] as const) {
			const c = clampTrim(t, len, edge);
			assert.ok(c.in >= 0 && c.out <= len && c.out - c.in >= c.min - 1e-9, JSON.stringify([t, edge, c]));
		}
	}
});

test('formatTime: clock, seconds and frames at 24, 25 and 30 fps', () => {
	assert.equal(formatTime(0), '0:00');
	assert.equal(formatTime(65_400), '1:05');
	assert.equal(formatTime(59_999), '0:59', 'the clock floors – never 0:60');
	assert.equal(formatTime(3_725_000), '1:02:05');
	assert.equal(formatTime(1250, 'clock', 2), '0:01.25');
	assert.equal(formatTime(-1500), '-0:01');
	assert.equal(formatTime(1250, 's'), '1.25 s');
	assert.equal(formatTime(1500, 's'), '1.5 s');
	assert.equal(formatTime(10_000, 's'), '10 s');
	assert.equal(formatTime(41.6667, 's'), '0.04 s');
	assert.equal(formatTime(1500, { fps: 24 }), '0:01:12');
	assert.equal(formatTime((1000 / 24) * 24, { fps: 24 }), '0:01:00', 'a whole second of frames is a whole second');
	assert.equal(formatTime((1000 / 24) * 23, { fps: 24 }), '0:00:23');
	assert.equal(formatTime(1500, { fps: 25 }), '0:01:12');
	assert.equal(formatTime(1960, { fps: 25 }), '0:01:24');
	assert.equal(formatTime(1500, { fps: 30 }), '0:01:15');
	assert.equal(formatTime(61_000 + (1000 / 30) * 29, { fps: 30 }), '1:01:29');
});
