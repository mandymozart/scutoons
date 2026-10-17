// The endless band's geometry and motion (components/toon/renderer/strip-loop.ts; BACKLOG 10.100, TEXT-ACROSS-SLICES.md §8):
//   npm test -w viewer
// Measured in the prototype (3 engines): a fractional turn shifted the picture 1–3 device px per turn, a wrap decided on
// the unrounded track position left a 0.25 px sliver of ink at the screen's edge, and the glide's first frame stepped
// backwards when the rAF stamp preceded its start. The four ch3–5 groups: 6 slices 390 × 844; the band is
// min(100svh, 100lvh − 4rem) high, so 780 px on a 390 × 844 phone and 836 px at 1440 × 900.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import {
	ease,
	frameLayout,
	glide,
	GLIDE_TAU,
	indexAt,
	keepPlace,
	nearestEdge,
	pageLayout,
	placeFrames,
	releaseVelocity,
	revealTarget,
	settleTarget,
	SHORT_SWIPE_MS,
	snap,
	stepTarget,
	stripFit,
	widest,
	type Overhang,
	type StripLayout
} from '../src/lib/components/toon/renderer/strip-loop.ts';

const FOUR = Array.from({ length: 6 }, () => 390 / 844);
/** band heights: min(100svh, 100lvh − min(4rem, 10lvh)) at a 16 px rem, from 640 px high */
const band = (height: number) => Math.min(height, height - 64);
const onGrid = (px: number, dpr: number) => Math.abs(px * dpr - Math.round(px * dpr)) < 1e-6;
const close = (a: number, b: number, eps = 1e-6) => Math.abs(a - b) < eps;

test('T, every start and k·T sit on the device grid (T is fractional unsnapped: 2317.78 px at 1440 × 900)', () => {
	for (const [height, dpr] of [[780, 3], [836, 1], [836, 1.25], [836, 1.5], [836, 2.625], [561, 1.25], [326, 3]]) {
		const layout = frameLayout(FOUR, height, dpr);
		assert.ok(onGrid(layout.turn, dpr), `T ${layout.turn} at ${dpr}`);
		assert.ok(Math.abs(layout.turn - FOUR.reduce((a, b) => a + b) * height) <= 0.5 / dpr + 1e-9, 'within half a device px');
		for (const start of layout.starts) assert.ok(onGrid(start, dpr), `start ${start} at ${dpr}`);
		assert.ok(close(layout.widths.reduce((a, b) => a + b), layout.turn));
		for (const k of [-40, -7, 3, 50]) assert.ok(onGrid(k * layout.turn, dpr), `${k}·T at ${dpr}`);
	}
	assert.equal(frameLayout(FOUR, 836, 1).turn, 2318);
	assert.ok(close(frameLayout(FOUR, 780, 3).turn, 6488 / 3));
});

test('across the turn boundary the picture moves exactly as far as the offset – no jump', () => {
	for (const [width, height, dpr] of [[1440, 900, 1], [390, 844, 3], [1440, 900, 1.25]]) {
		const layout = frameLayout(FOUR, band(height), dpr);
		const fit = stripFit(layout, width, dpr);
		for (const at of [-3 * layout.turn, 2 * layout.turn, -3 * layout.turn - 0.37 * layout.widths[0]]) {
			const a = placeFrames(at - 1 / dpr, layout, fit, dpr);
			const b = placeFrames(at + 1 / dpr, layout, fit, dpr);
			let shown = 0;
			layout.starts.forEach((_, i) => {
				const visible = (p: typeof a) => p.lefts[i] < fit.window && p.lefts[i] + layout.widths[i] > 0;
				if (!visible(a) || !visible(b)) return;
				shown++;
				assert.ok(close(b.lefts[i] - a.lefts[i], 2 / dpr), `frame ${i} at ${width}×${height}@${dpr}: ${b.lefts[i] - a.lefts[i]}`);
			});
			assert.ok(shown > 0);
		}
	}
});

/** the window covered by pictures from its left to its right edge: the uncovered width at its left (0 = none) */
function gapAtLeft(layout: StripLayout, lefts: number[], window: number): number {
	const order = lefts.map((_, i) => i).sort((a, b) => lefts[a] - lefts[b]);
	let reach = 0;
	let gap = 0;
	for (const i of order) {
		if (lefts[i] >= window) break;
		if (lefts[i] > reach + 1e-9) gap = Math.max(gap, lefts[i] - reach);
		reach = Math.max(reach, lefts[i] + layout.widths[i]);
	}
	return reach < window - 1e-9 ? Math.max(gap, window - reach) : gap;
}

test('the window stays covered and a frame (with its boxes) wraps only off screen – quarter device px over 3 turns', () => {
	const cases: [string, number, number, number, Overhang[]][] = [
		['390 × 844 phone', 390, 844, 3, []],
		['1440 × 900', 1440, 900, 1, []],
		['844 × 390 landscape (narrowed)', 844, 390, 3, []],
		['1366 × 625 laptop (narrowed)', 1366, 625, 1.25, []],
		['390 × 844, boxes past frames 1, 2, 6', 390, 844, 2, [{ left: 90, right: 0 }, { left: 0, right: 100 }, { left: 0, right: 0 }, { left: 0, right: 0 }, { left: 0, right: 0 }, { left: 0, right: 180 }]]
	];
	for (const [name, width, height, dpr, overhang] of cases) {
		const layout = frameLayout(FOUR, band(height), dpr);
		const fit = stripFit(layout, width, dpr, overhang);
		assert.notEqual(fit.mode, 'still', name);
		const extent = (left: number, i: number) => [left - (overhang[i]?.left ?? 0), left + layout.widths[i] + (overhang[i]?.right ?? 0)];
		const offScreen = (left: number, i: number) => {
			const [l, r] = extent(left, i);
			return r <= 1e-9 || l >= fit.window - 1e-9;
		};
		let before = placeFrames(-1.5 * layout.turn, layout, fit, dpr, overhang);
		let wraps = 0;
		for (let offset = -1.5 * layout.turn; offset <= 1.5 * layout.turn; offset += 1 / (4 * dpr)) {
			const now = placeFrames(offset, layout, fit, dpr, overhang);
			assert.equal(gapAtLeft(layout, now.lefts, fit.window), 0, `${name}: a gap at offset ${offset}`);
			now.lefts.forEach((left, i) => {
				// a wrap moves the frame a turn on screen (at the turn boundary every k changes, the picture doesn't jump)
				if (Math.abs(left - before.lefts[i]) > layout.turn / 2) wraps++;
				// no other place of the frame (a turn away) would show it or its boxes: it is drawn wherever it shows
				for (const m of [-2, -1, 1, 2]) assert.ok(offScreen(left + m * layout.turn, i), `${name}: frame ${i} needed a turn away at ${offset}`);
			});
			for (const left of now.lefts) assert.ok(onGrid(left, dpr), `${name}: ${left} off the grid`);
			before = now;
		}
		assert.ok(wraps >= 6 * 2, `${name}: ${wraps} wraps`);
	}
});

test('the 0.25 px sliver: the wrap is decided on the rounded track position, the one drawn', () => {
	// fractional frames as a browser measures them, DPR 1; the drag stands at −360.5 → the track is drawn at 1801
	const layout: StripLayout = { height: 780, starts: [0, 360.25, 720.5, 1080.75, 1441, 1801.25], widths: [360.25, 360.25, 360.25, 360.25, 360.25, 359.75], turn: 2161 };
	const fit = stripFit(layout, 390, 1);
	const offset = -360.5;
	const placed = placeFrames(offset, layout, fit, 1);
	assert.equal(placed.x, 1801);
	assert.equal(placed.lefts[0], -360, 'frame 1 stays at the left edge (its last 0.25 px show)');
	assert.equal(gapAtLeft(layout, placed.lefts, fit.window), 0);
	// deciding on the unrounded position instead wraps frame 1 away, and frame 2 is drawn at +0.25: ink shows
	const unrounded = ((offset % layout.turn) + layout.turn) % layout.turn;
	const naive = layout.starts.map((start, i) => {
		const reach = layout.widths[i];
		const k = Math.round(((((start + unrounded + reach) % layout.turn) + layout.turn) % layout.turn - reach - start - unrounded) / layout.turn);
		return start + k * layout.turn + placed.x;
	});
	assert.equal(gapAtLeft(layout, naive, fit.window), 0.25);
});

test('loop, narrowed or still: the window (width ÷ band height) against Σratio − max(ratio + overhang) = 2.31', () => {
	const fitAt = (width: number, height: number, dpr = 1, ratios = FOUR, overhang: Overhang[] = []) => {
		const layout = frameLayout(ratios, band(height), dpr);
		return { layout, fit: stripFit(layout, width, dpr, overhang), height: band(height) };
	};
	for (const [width, height] of [[390, 844], [375, 667], [412, 915], [1440, 900]]) assert.equal(fitAt(width, height).fit.mode, 'loop', `${width} × ${height}`);
	// frames on screen: 1.08 on a 390 × 844 phone, 1.35 at 375 × 667, 3.73 at 1440 × 900
	for (const [width, height, frames] of [[390, 844, 1.08], [375, 667, 1.35], [1440, 900, 3.73]]) {
		const { layout } = fitAt(width, height);
		assert.ok(Math.abs(width / layout.widths[0] - frames) < 0.01, `${width} × ${height}: ${width / layout.widths[0]}`);
	}
	// a landscape phone ≈ 2.6, a 1366 × 625 laptop window 2.44, 21:9 – narrowed to T − max(W + overhang), centred on ink
	for (const [width, height, dpr] of [[844, 390, 3], [1366, 625, 1.25], [2560, 1080, 1], [3440, 1440, 1]]) {
		const { layout, fit, height: h } = fitAt(width, height, dpr);
		assert.equal(fit.mode, 'narrowed', `${width} × ${height}`);
		assert.ok(fit.window + widest(layout) <= layout.turn + 1e-9, 'any frame fits off screen');
		assert.ok(layout.turn - widest(layout) - fit.window < 1 / dpr, 'narrowed by less than a device px more');
		assert.ok(Math.abs(fit.window / h - 2.31) < 0.01, `${width} × ${height}: ${fit.window / h}`);
		assert.ok(onGrid(fit.window, dpr) && onGrid(fit.inset, dpr));
		assert.ok(Math.abs(fit.inset - (width - fit.window) / 2) <= 0.5 / dpr + 1e-9);
	}
	// a box reaching 600 px past frame 3 makes 1440 × 900 narrow, too
	const inside = { left: 0, right: 0 };
	assert.equal(fitAt(1440, 900, 1, FOUR, [inside, inside, { left: 0, right: 600 }]).fit.mode, 'narrowed');
	// T within the window: whole, still and centred; a single frame wider than the screen can't loop: still
	const three = fitAt(1440, 900, 1, FOUR.slice(0, 3));
	assert.equal(three.fit.mode, 'still');
	const still = placeFrames(-777, three.layout, three.fit, 1);
	assert.equal(still.x, Math.round((1440 - three.layout.turn) / 2));
	assert.deepEqual(still.turns, [0, 0, 0]);
	assert.deepEqual(still.planes, [1, 2, 3]);
	assert.equal(fitAt(390, 844, 3, [3]).fit.mode, 'still');
});

test('planes: the later slice on screen paints on top – DOM order inside, frame 6 under frame 1 at the wrap seam', () => {
	const layout = frameLayout(FOUR, 780, 3);
	const fit = stripFit(layout, 390, 3);
	assert.deepEqual(placeFrames(-layout.widths[0] / 2, layout, fit, 3).planes, [1, 2, 3, 4, 5, 6]);
	const seam = placeFrames(-(layout.starts[5] + layout.widths[5] / 2), layout, fit, 3);
	assert.ok(seam.lefts[5] < 0 && close(seam.lefts[0], seam.lefts[5] + layout.widths[5], 1));
	assert.equal(seam.turns[5], -1, 'frame 6 put one turn back, left of frame 1');
	assert.ok(seam.planes[0] > seam.planes[5]);
	const ranks = seam.lefts.map((left) => seam.lefts.filter((other) => other <= left).length);
	assert.deepEqual(seam.planes, ranks);
});

test('the glide: x₀ + vτ(1 − e^(−t/τ)) with t clamped ≥ 0 – never a step backwards', () => {
	const turn = frameLayout(FOUR, 836, 1).turn;
	const fling = glide(100, -2, 1000, turn)!;
	// a rAF stamp 6.8 ms before the start (Chromium): the unclamped formula gives 113.7 – against the fling
	assert.equal(fling.at(1000 - 6.8).x, 100);
	assert.ok(fling.at(1016).x < 100);
	assert.equal(fling.end, 100 - 2 * GLIDE_TAU);
	assert.equal(fling.ease, false);
});

/** a glide's frames at `hz` (the first rAF stamp 3 ms before the start), optionally with one main-thread block */
function run(hz: number, block = 0) {
	const turn = frameLayout(FOUR, 836, 1).turn;
	const fling = glide(0, -turn / GLIDE_TAU, 0, turn)!;
	const xs: number[] = [];
	for (let now = -3, frame = 0; ; frame++, now += 1000 / hz + (frame === 10 ? block : 0)) {
		const { x, done } = fling.at(now);
		xs.push(x);
		if (done) return { xs, ms: now };
	}
}

test('the same glide at 60 and 120 Hz and after a 250 ms block: the same way, the same end, monotone', () => {
	const turn = frameLayout(FOUR, 836, 1).turn;
	const runs = [run(60), run(120), run(60, 250)];
	for (const { xs, ms } of runs) {
		assert.ok(close(xs.at(-1)!, -turn), 'one full turn at the cap');
		for (let i = 1; i < xs.length; i++) assert.ok(xs[i] <= xs[i - 1], `monotone at frame ${i}`);
		assert.ok(Math.abs(xs.at(-1)! - xs.at(-2)!) < 0.5 * Math.exp(1000 / 60 / GLIDE_TAU), 'the last step ≈ 0.5 px');
		assert.ok(ms > 2000 && ms < 3000, `comes to rest in ${ms} ms`);
	}
	assert.ok(runs[1].xs.length > 1.9 * runs[0].xs.length);
	// at the same moments both rates stand at the same place: every second 120 Hz frame is a 60 Hz frame
	runs[0].xs.forEach((x, i) => assert.ok(close(x, runs[1].xs[2 * i], 1e-6), `60 Hz frame ${i}`));
});

test('the velocity cap |v| ≤ T/τ (one turn per fling), slow releases don\'t glide', () => {
	const turn = frameLayout(FOUR, 780, 3).turn;
	assert.ok(close(glide(0, -50, 0, turn)!.end, -turn));
	assert.ok(close(glide(0, 8, 0, turn)!.end, turn));
	assert.equal(glide(0, 1, 0, turn)!.end, GLIDE_TAU);
	assert.equal(glide(0, 0.01, 0, turn), null);
	assert.equal(glide(0, Number.NaN, 0, turn), null);
	assert.equal(glide(0, 5, 0, 0), null);
});

test('the release velocity comes from the 100 ms up to the last move (the move before it when sparse); a rest is no fling', () => {
	// 10 px/ms for 100 ms, then 1 px/ms for 100 ms
	const samples = Array.from({ length: 21 }, (_, i) => ({ t: i * 10, x: i <= 10 ? i * 100 : 1000 + (i - 10) * 10 }));
	assert.ok(close(releaseVelocity(samples, 200), 1));
	assert.ok(close(releaseVelocity(samples.slice(0, 11), 100), 10));
	assert.equal(releaseVelocity(samples, 260), 0, 'rested 60 ms');
	assert.equal(releaseVelocity([{ t: 0, x: 5 }], 10), 0);
	assert.equal(releaseVelocity([], 10), 0);
	// sparse moves (a janky phone): released 46 ms after the last move, the one before 55 ms earlier still flings
	assert.ok(close(releaseVelocity([{ t: 6500, x: 0 }, { t: 6578.7, x: -110 }, { t: 6633.7, x: -220 }], 6680), -2));
	assert.equal(releaseVelocity([{ t: 0, x: 0 }, { t: 200, x: -220 }], 220), 0, 'moves 200 ms apart: no pair');
});

test('keys step from frame edge to frame edge, presses add up, across the wrap', () => {
	const layout = frameLayout(FOUR, 780, 3);
	assert.ok(close(stepTarget(0, 1, layout), -layout.starts[1]));
	assert.ok(close(stepTarget(-layout.starts[1], 1, layout), -layout.starts[2]));
	assert.ok(close(stepTarget(0, -1, layout), layout.widths[5]), 'back from frame 1 to frame 6');
	assert.ok(close(stepTarget(-layout.starts[1] + 0.3, 1, layout), -layout.starts[2]), 'on an edge already');
	let at = 0;
	for (let i = 0; i < 6; i++) at = stepTarget(at, 1, layout);
	assert.ok(close(at, -layout.turn), 'six presses: one turn');
	for (let i = 0; i < 6; i++) at = stepTarget(at, -1, layout);
	assert.ok(close(at, 0));
	const ride = ease(0, -layout.starts[1], 1000);
	assert.equal(ride.ease, true);
	assert.equal(ride.at(990).x, 0);
	assert.deepEqual(ride.at(1300), { x: -layout.starts[1], done: true });
	assert.ok(ride.at(1150).x < -layout.starts[1] / 2, 'ease-out');
});

test('keyboard focus brings a frame in by the shortest way; a shown frame stays', () => {
	const layout = frameLayout(FOUR, 780, 3);
	const fit = stripFit(layout, 390, 3);
	const at = (offset: number) => placeFrames(offset, layout, fit, 3);
	assert.equal(revealTarget(0, 0, at(0), layout, fit.window), null);
	const right = revealTarget(0, 1, at(0), layout, fit.window)!;
	assert.ok(close(at(right).lefts[1] + layout.widths[1], fit.window, 0.5), 'its right edge to the window\'s');
	assert.equal(revealTarget(-100, 0, at(-100), layout, fit.window), 0, 'its left edge to the window\'s');
	assert.equal(revealTarget(right, 1, at(right), layout, fit.window), null);
});

test('a resize keeps the place: the same share of a turn', () => {
	const wide = frameLayout(FOUR, 836, 1).turn;
	const phone = frameLayout(FOUR, 780, 3).turn;
	assert.ok(close(keepPlace(-1000, wide, phone), (-1000 * phone) / wide));
	assert.equal(keepPlace(-1000, 0, phone), 0);
	assert.equal(snap(0.4, 1), 0);
	assert.ok(close(snap(0.2, 3), 1 / 3));
});

// the home slider (components/home/slides-pager.ts): equal pages, 1 per view below 768 px, 3 from 768 px, a 16 px gap
test('pages: the pitch on the device grid rounded down (the last page in view never clipped), loop with one page more than fit', () => {
	for (const [width, per, gap, dpr] of [[358, 1, 16, 3], [358, 1, 16, 2.625], [694, 3, 16, 1.25], [903.33, 3, 16, 1.5], [1000, 3, 16, 1]]) {
		const { pitch, layout, fit } = pageLayout(7, per, width, gap, dpr);
		assert.ok(onGrid(pitch, dpr), `pitch ${pitch} at ${dpr}`);
		assert.ok(per * pitch - gap <= width + 1e-9, 'the last page in view inside the window');
		assert.ok(per * pitch - gap > width - per / dpr - 1e-9, 'less than a device px per page short');
		assert.ok(per * pitch >= width, 'the next page starts outside the window');
		assert.deepEqual(layout.starts.map((s) => Math.round((s / pitch) * 1e6) / 1e6), [0, 1, 2, 3, 4, 5, 6]);
		assert.equal(fit.mode, 'loop');
		assert.equal(fit.window, width);
	}
	assert.equal(pageLayout(4, 3, 1000, 16, 1).fit.mode, 'loop', 'one page more than fit loops');
	assert.equal(pageLayout(3, 3, 1000, 16, 1).fit.mode, 'still', 'as many as fit: whole and still');
	assert.equal(pageLayout(2, 3, 1000, 16, 1).fit.mode, 'still');
	assert.equal(pageLayout(1, 1, 358, 16, 3).fit.mode, 'still');
	assert.equal(pageLayout(0, 1, 358, 16, 3).layout.turn, 0);
});

test('pages: the nearest edge and the page at the window\'s left, any number of turns away', () => {
	const { layout, pitch } = pageLayout(5, 1, 358, 16, 2);
	const T = layout.turn;
	assert.equal(nearestEdge(-0.4 * pitch, layout), 0);
	assert.ok(close(nearestEdge(-0.6 * pitch, layout), -pitch));
	assert.ok(close(nearestEdge(-3 * T - 2.3 * pitch, layout), -3 * T - 2 * pitch));
	assert.ok(close(nearestEdge(0.7 * pitch, layout), pitch), 'right of page 0: the last page comes in');
	assert.equal(indexAt(0, layout), 0);
	assert.equal(indexAt(-2.4 * pitch, layout), 2);
	assert.equal(indexAt(-7 * T - 4.6 * pitch, layout), 0, 'past the last page: the first');
	assert.equal(indexAt(0.6 * pitch, layout), 4, 'the picture moved right: the last page');
	assert.equal(indexAt(5 * T + pitch, layout), 4);
	assert.equal(indexAt(0, pageLayout(0, 1, 358, 16, 2).layout), 0);
});

test('pages settle: a fling or a short swipe goes on one page the way it points, a slow drag to the nearest edge', () => {
	const { layout, pitch } = pageLayout(5, 1, 358, 16, 2);
	const T = layout.turn;
	const at = (x: number) => Math.round(x * 100) / 100;
	// a slow drag (no velocity, a long swipe): the nearest edge
	assert.equal(at(settleTarget(-0.3 * pitch, layout, 0, { dx: -0.3 * pitch, ms: 800 })), 0);
	assert.equal(at(settleTarget(-0.7 * pitch, layout, 0, { dx: -0.7 * pitch, ms: 800 })), at(-pitch));
	// the short-swipe rule: a quick 30 px flick moves on, even when it rested before letting go (velocity 0)
	assert.equal(at(settleTarget(-30, layout, 0, { dx: -30, ms: SHORT_SWIPE_MS - 50 })), at(-pitch), 'next page');
	assert.equal(at(settleTarget(30, layout, 0, { dx: 30, ms: 120 })), at(pitch), 'previous page (the last one, round the loop)');
	// a fling decides by its velocity, also after a long drag – and the way it points, not the way the drag went
	assert.equal(at(settleTarget(-0.2 * pitch, layout, -0.5, { dx: -0.2 * pitch, ms: 900 })), at(-pitch));
	assert.equal(at(settleTarget(-0.6 * pitch, layout, 0.5, { dx: -0.6 * pitch, ms: 900 })), 0, 'flicked back');
	assert.equal(at(settleTarget(-0.2 * pitch, layout, -0.29, { dx: -0.2 * pitch, ms: 900 })), 0, 'too slow: nearest');
	// many turns away, and the wheel (no swipe): nearest
	assert.equal(at(settleTarget(-4 * T - 1.2 * pitch, layout, -1, { dx: -40, ms: 100 })), at(-4 * T - 2 * pitch));
	assert.equal(at(settleTarget(-4 * T - 1.2 * pitch, layout)), at(-4 * T - pitch));
	// a cancel (velocity 0, no swipe) right after the dead zone: back
	assert.equal(at(settleTarget(-8, layout, 0)), 0);
});
