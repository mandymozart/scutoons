// An endless band's picture, cropped so it still wraps (#109, owner 2026-10-21: "crop it").
//
// The band art is a seamless 3:1 panorama (2532 × 844 at 1x, 7596 × 2532 at 3x): its last column continues into its
// first. The band is 6 slides of 390 = 2340 wide, so the picture is cropped to the band's ratio (2340 : 844) – a plain
// crop breaks the join (the centred FILL lost 96 px a side: the mountain line and a tree jump). Here instead:
//   columns 0 … outW of the panorama stay; in the last `blend` columns the picture switches over to the panorama's
//   columns outW … srcW ("read ahead"), which run on into column 0 – so the right edge continues into the left again.
//   The switch is not a fade across the whole zone (on line art that leaves a ghosted half tree) but a cut along the
//   path where both pictures agree most (minimum-error seam, top to bottom, ±1 column a row), feathered a few px.
// The pixels go through PIL (python -I, like the rest of the storyboard pipeline, figma-chapter.mjs bytesOf); the plan,
// the seam and the mask are here and tested.
//
//   node --experimental-transform-types --no-warnings scripts/storyboard/band-wrap.ts <panorama> <out.png> [--blend n]
//   test: node --experimental-transform-types --no-warnings --test scripts/storyboard/band-wrap.test.ts

import { execFileSync } from 'node:child_process';
import { mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

/** the band at 1x: 6 slides of 390, one screen (844) high */
export const BAND_W = 2340;
export const BAND_H = 844;

export interface WrapPlan {
	srcW: number;
	srcH: number;
	/** the cropped picture: the band's ratio at the source's height */
	outW: number;
	outH: number;
	/** srcW - outW: how far the tail reads ahead (the columns the crop drops) */
	shift: number;
	/** the switch-over zone at the right edge (columns outW - blend … outW - 1) */
	blend: number;
	/** half the feather around the seam, in px */
	feather: number;
}

/**
 * The crop of a srcW × srcH panorama to the band's ratio, or null when there is nothing to crop (the picture is not
 * wider than the band's ratio). `blend` defaults to the dropped width (2532 → 2340: 192 columns), at most outW; the
 * feather is 1/24 of it (8 px at 1x), at least 1 and leaving room for the seam.
 */
export function wrapPlan(srcW: number, srcH: number, opts: { bandW?: number; bandH?: number; blend?: number } = {}): WrapPlan | null {
	const bandW = opts.bandW ?? BAND_W, bandH = opts.bandH ?? BAND_H;
	if (!(srcW > 0 && srcH > 0 && bandW > 0 && bandH > 0)) throw new Error(`bad size ${srcW} × ${srcH} / ${bandW} × ${bandH}`);
	const outW = Math.round((srcH * bandW) / bandH);
	const shift = srcW - outW;
	if (shift <= 0) return null;
	const blend = Math.max(1, Math.min(outW, Math.round(opts.blend ?? shift)));
	const feather = Math.max(0, Math.min(Math.max(1, Math.round(blend / 24)), Math.floor((blend - 1) / 2)));
	return { srcW, srcH, outW, outH: srcH, shift, blend, feather };
}

/** smoothstep: 0 → 0, 1 → 1, flat at both ends */
export const smooth = (t: number): number => {
	const x = Math.min(1, Math.max(0, t));
	return x * x * (3 - 2 * x);
};

/**
 * The minimum-error seam through a w × h difference map (row-major, one byte a pixel): per row the column where the
 * picture switches, moving at most one column from row to row, kept `margin` columns off both sides. → h columns
 */
export function seam(diff: Uint8Array, w: number, h: number, margin = 0): number[] {
	if (diff.length !== w * h) throw new Error(`diff is ${diff.length} bytes, not ${w} × ${h}`);
	const lo = Math.min(margin, Math.floor((w - 1) / 2)), hi = w - 1 - lo;
	const cost = new Float64Array(w * h), from = new Int32Array(w * h);
	for (let y = 0; y < h; y++)
		for (let x = lo; x <= hi; x++) {
			let best = 0, at = x;
			if (y > 0) {
				best = Infinity;
				for (let dx = -1; dx <= 1; dx++) {
					const px = x + dx;
					if (px < lo || px > hi) continue;
					const c = cost[(y - 1) * w + px];
					// ties go straight down: a calm, vertical seam
					if (c < best || (c === best && dx === 0)) { best = c; at = px; }
				}
			}
			cost[y * w + x] = best + diff[y * w + x];
			from[y * w + x] = at;
		}
	let x = lo;
	for (let c = lo; c <= hi; c++) if (cost[(h - 1) * w + c] < cost[(h - 1) * w + x]) x = c;
	const out = new Array<number>(h);
	for (let y = h - 1; y >= 0; y--) { out[y] = x; x = from[y * w + x]; }
	return out;
}

/**
 * The read-ahead weight (0–255) per pixel of the switch zone: 0 left of the seam, 255 right of it, smoothed over
 * ±feather px. Row-major, w × seam.length bytes.
 */
export function seamMask(cut: number[], w: number, feather: number): Uint8Array {
	const m = new Uint8Array(w * cut.length);
	for (let y = 0; y < cut.length; y++)
		for (let x = 0; x < w; x++)
			m[y * w + x] = feather > 0 ? Math.round(255 * smooth((x - cut[y] + feather) / (2 * feather))) : x > cut[y] ? 255 : 0;
	return m;
}

const PY_HEAD = `import sys,json
from PIL import Image, ImageChops
src,p=sys.argv[1],json.loads(sys.argv[2])
im=Image.open(src);im.load()
im=im.convert('RGBA' if 'A' in im.getbands() else 'RGB')
if im.size!=(p['srcW'],p['srcH']): sys.exit('size changed: %r' % (im.size,))
W,H,d,b=p['outW'],p['outH'],p['shift'],p['blend']
own=im.crop((W-b,0,W,H)); ahead=im.crop((W-b+d,0,W+d,H))
`;
/** PIL, step 1: the switch zone's own vs read-ahead difference (the channels' max), raw bytes → argv[3] */
const PY_DIFF = `${PY_HEAD}df=ImageChops.difference(own.convert('RGB'),ahead.convert('RGB')).split()
m=ImageChops.lighter(ImageChops.lighter(df[0],df[1]),df[2])
open(sys.argv[3],'wb').write(m.tobytes())
`;
/** PIL, step 2: the crop with the switch zone composed through the mask (raw bytes argv[3]) → PNG argv[4] */
const PY_COMPOSE = `${PY_HEAD}m=Image.frombytes('L',(b,H),open(sys.argv[3],'rb').read())
res=im.crop((0,0,W,H)); res.paste(Image.composite(ahead,own,m),(W-b,0))
res.save(sys.argv[4],'PNG')
`;

/** the source's size through PIL */
export function sizeOf(file: string, python = 'python'): [number, number] {
	const [w, h] = execFileSync(python, ['-I', '-c', 'import sys\nfrom PIL import Image\nprint(*Image.open(sys.argv[1]).size)', file]).toString().trim().split(/\s+/).map(Number);
	return [w, h];
}

/** the join: the mean absolute difference (0–255, over the channels) between the picture's last and first column */
export function joinOf(file: string, python = 'python'): number {
	const py = `import sys
from PIL import Image, ImageChops, ImageStat
im=Image.open(sys.argv[1]).convert('RGB');w,h=im.size
d=ImageStat.Stat(ImageChops.difference(im.crop((w-1,0,w,h)),im.crop((0,0,1,h)))).mean
print(sum(d)/len(d))`;
	return Number(execFileSync(python, ['-I', '-c', py, file]).toString().trim());
}

/**
 * Writes the band version of `input` to `output` (PNG) and returns its plan – or null, writing nothing, when the
 * picture needs no crop.
 */
export function wrapBand(input: string, output: string, opts: { blend?: number; python?: string } = {}): WrapPlan | null {
	const python = opts.python ?? 'python';
	const [w, h] = sizeOf(input, python);
	const p = wrapPlan(w, h, { blend: opts.blend });
	if (!p) return null;
	const dir = mkdtempSync(join(tmpdir(), 'band-wrap-'));
	try {
		const diffFile = join(dir, 'diff.raw'), maskFile = join(dir, 'mask.raw');
		execFileSync(python, ['-I', '-c', PY_DIFF, input, JSON.stringify(p), diffFile]);
		const cut = seam(new Uint8Array(readFileSync(diffFile)), p.blend, p.outH, p.feather);
		writeFileSync(maskFile, seamMask(cut, p.blend, p.feather));
		execFileSync(python, ['-I', '-c', PY_COMPOSE, input, JSON.stringify(p), maskFile, output]);
	} finally {
		rmSync(dir, { recursive: true, force: true });
	}
	return p;
}

// the CLI – a thin entry
if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
	const args = process.argv.slice(2);
	const i = args.indexOf('--blend');
	const blend = i >= 0 ? Number(args.splice(i, 2)[1]) : undefined;
	const [input, output] = args;
	if (!input || !output) {
		console.error('usage: band-wrap.ts <panorama> <out.png> [--blend n]');
		process.exit(2);
	}
	const p = wrapBand(input, output, { blend });
	if (!p) console.log(`${input}: not wider than the band (${BAND_W} : ${BAND_H}) – nothing to crop`);
	else console.log(`${p.srcW} × ${p.srcH} → ${p.outW} × ${p.outH}, the last ${p.blend} columns switch over to ${p.outW}–${p.srcW} → ${output}\njoin (last vs first column): source ${joinOf(input).toFixed(1)} · band ${joinOf(output).toFixed(1)}`);
}
