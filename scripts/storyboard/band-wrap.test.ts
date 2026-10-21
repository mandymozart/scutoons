// node --experimental-transform-types --no-warnings --test scripts/storyboard/band-wrap.test.ts
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { mkdtempSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { BAND_W, joinOf, seam, seamMask, sizeOf, smooth, wrapBand, wrapPlan } from './band-wrap.ts';

test('the plan: a 3:1 panorama at 1x and 3x crops to the band ratio, the dropped width is the switch zone', () => {
	assert.deepEqual(wrapPlan(2532, 844), { srcW: 2532, srcH: 844, outW: 2340, outH: 844, shift: 192, blend: 192, feather: 8 });
	assert.deepEqual(wrapPlan(7596, 2532), { srcW: 7596, srcH: 2532, outW: 7020, outH: 2532, shift: 576, blend: 576, feather: 24 });
	assert.equal(wrapPlan(2532, 844, { blend: 96 })?.blend, 96);
	assert.equal(wrapPlan(2532, 844, { blend: 99999 })?.blend, BAND_W, 'never more than the picture');
	assert.equal(wrapPlan(2341, 844)?.feather, 0, 'a one-column zone: a hard switch');
	assert.equal(wrapPlan(2343, 844)?.feather, 1);
});

test('nothing to crop: art at the band ratio or narrower', () => {
	assert.equal(wrapPlan(2340, 844), null);
	assert.equal(wrapPlan(7020, 2532), null);
	assert.equal(wrapPlan(1000, 844), null);
	assert.throws(() => wrapPlan(0, 844));
});

test('smooth: smoothstep, flat ends, symmetric', () => {
	assert.equal(smooth(0), 0);
	assert.equal(smooth(1), 1);
	assert.equal(smooth(0.5), 0.5);
	assert.equal(smooth(-1), 0);
	assert.equal(smooth(2), 1);
	assert.ok(Math.abs(smooth(0.25) + smooth(0.75) - 1) < 1e-12);
});

/** a w × h map, 200 everywhere except where free(x, y) */
const map = (w: number, h: number, free: (x: number, y: number) => boolean) => {
	const d = new Uint8Array(w * h).fill(200);
	for (let y = 0; y < h; y++) for (let x = 0; x < w; x++) if (free(x, y)) d[y * w + x] = 0;
	return d;
};

test('the seam follows where both pictures agree', () => {
	assert.deepEqual(seam(map(9, 5, (x) => x === 6), 9, 5), [6, 6, 6, 6, 6]);
	// a diagonal gap, one column a row
	assert.deepEqual(seam(map(9, 5, (x, y) => x === 2 + y), 9, 5), [2, 3, 4, 5, 6]);
	// all equal: straight down
	const flat = seam(new Uint8Array(40), 8, 5);
	assert.ok(flat.every((x) => x === flat[0]));
	assert.throws(() => seam(new Uint8Array(3), 2, 2));
});

test('the seam stays margin columns off both sides, and moves one column a row at most', () => {
	const towards = (to: number) => Uint8Array.from({ length: 27 }, (_, i) => 20 * Math.abs((i % 9) - to));
	assert.deepEqual(seam(towards(0), 9, 3, 2), [2, 2, 2]);
	assert.deepEqual(seam(towards(8), 9, 3, 2), [6, 6, 6]);
	const cut = seam(map(20, 6, (x, y) => (y < 3 ? x === 2 : x === 17)), 20, 6);
	for (let y = 1; y < cut.length; y++) assert.ok(Math.abs(cut[y] - cut[y - 1]) <= 1);
});

test('the mask: own left of the seam, read-ahead right of it, the ends always pure', () => {
	assert.deepEqual([...seamMask([2], 5, 0)], [0, 0, 0, 255, 255]);
	const p = wrapPlan(2532, 844)!;
	for (const at of [p.feather, 96, p.blend - 1 - p.feather]) {
		const m = seamMask([at], p.blend, p.feather);
		assert.equal(m[0], 0, 'the zone starts as the own picture (continues column outW - blend - 1)');
		assert.equal(m[p.blend - 1], 255, 'the zone ends read ahead (the source last column, which joins column 0)');
		for (let x = 1; x < p.blend; x++) assert.ok(m[x] >= m[x - 1]);
		assert.equal(m[at], 128);
	}
});

// the PIL step on a synthetic seamless panorama (skipped without python + PIL)
const hasPil = (() => { try { execFileSync('python', ['-I', '-c', 'import PIL']); return true; } catch { return false; } })();
test('a seamless panorama stays seamless at the band width', { skip: !hasPil && 'python with PIL not found' }, () => {
	const dir = mkdtempSync(join(tmpdir(), 'band-wrap-'));
	try {
		const src = join(dir, 'pano.png'), out = join(dir, 'band.png'), fill = join(dir, 'fill.png');
		// periodic over 2532: a hill line (sine of the column) over a sky gradient, colours cycling with the column
		execFileSync('python', ['-I', '-c', `import sys,math
from PIL import Image
W,H=2532,844
im=Image.new('RGB',(W,H));px=im.load()
for x in range(W):
  a=2*math.pi*x/W
  hill=int(H*0.55+90*math.sin(3*a)+40*math.sin(7*a+1))
  for y in range(H):
    px[x,y]=(int(128+100*math.sin(a)),int(128+100*math.cos(2*a)),200-y//8) if y<hill else (40+int(60*(1+math.sin(5*a))),90,40)
im.save(sys.argv[1])
im.crop(((W-2340)//2,0,(W-2340)//2+2340,H)).save(sys.argv[2])`, src, fill]);
		const p = wrapBand(src, out);
		assert.deepEqual(sizeOf(out), [2340, 844]);
		assert.equal(p?.outW, 2340);
		const before = joinOf(fill), source = joinOf(src), after = joinOf(out);
		assert.ok(source < 3, `the synthetic source is seamless (${source})`);
		assert.ok(before > 10, `the centred FILL crop breaks the join (${before})`);
		assert.ok(after <= source + 1, `the band joins like the source (${after} vs ${source})`);
		// columns before the blend are the source's own
		const same = execFileSync('python', ['-I', '-c', `import sys
from PIL import Image, ImageChops
a=Image.open(sys.argv[1]).convert('RGB').crop((0,0,2148,844));b=Image.open(sys.argv[2]).convert('RGB').crop((0,0,2148,844))
print(ImageChops.difference(a,b).getbbox() is None)`, src, out]).toString().trim();
		assert.equal(same, 'True');
	} finally {
		rmSync(dir, { recursive: true, force: true });
	}
});
