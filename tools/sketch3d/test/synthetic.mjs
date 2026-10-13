// A synthetic A-pose silhouette (capsules) → mask → mesh → rig, in Node, with timings. No image decoder needed.
//   node tools/sketch3d/test/synthetic.mjs [rows] [tris]
import { pathToFileURL } from 'node:url';
import { gridMask } from '../core/mask.js';
import { buildMesh } from '../core/mesh.js';
import { autoRig } from '../core/rig.js';

export function aPose(W = 600, H = 900) {
	const data = new Uint8ClampedArray(W * H * 4);
	const caps = [];
	const cap = (x0, y0, x1, y1, r) => caps.push([x0, y0, x1, y1, r]);
	const cx = W / 2;
	cap(cx, 110, cx, 120, 70); // head
	cap(cx, 180, cx, 220, 28); // neck
	cap(cx, 260, cx, 430, 85); // torso
	for (const s of [-1, 1]) {
		cap(cx + s * 95, 250, cx + s * 175, 470, 30); // arm
		cap(cx + s * 45, 470, cx + s * 70, 840, 38); // leg
		cap(cx + s * 70, 850, cx + s * 85, 855, 30); // foot
	}
	for (let y = 0; y < H; y++) for (let x = 0; x < W; x++) {
		let inside = false;
		for (const [x0, y0, x1, y1, r] of caps) {
			const dx = x1 - x0, dy = y1 - y0, L = dx * dx + dy * dy;
			const t = L ? Math.max(0, Math.min(1, ((x - x0) * dx + (y - y0) * dy) / L)) : 0;
			if ((x - x0 - t * dx) ** 2 + (y - y0 - t * dy) ** 2 <= r * r) { inside = true; break; }
		}
		const i = (y * W + x) * 4;
		data[i] = 200; data[i + 1] = 170; data[i + 2] = 150; data[i + 3] = inside ? 255 : 0;
	}
	return { width: W, height: H, data };
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
	const rows = Number(process.argv[2] ?? 112), tris = Number(process.argv[3] ?? 1500);
	const t0 = performance.now();
	const grid = gridMask(aPose(), { rows });
	const t1 = performance.now();
	const m = buildMesh(grid, { tris });
	const t2 = performance.now();
	const rig = autoRig(m.geometry, grid, m);
	const t3 = performance.now();
	console.log({ grid: `${grid.w}x${grid.h}`, ...m.info, ms: [t1 - t0, t2 - t1, t3 - t2].map(Math.round), warn: rig.lm.warn });
	for (const [k, v] of Object.entries(rig.P)) console.log(k.padEnd(6), v.toArray().map((n) => n.toFixed(2)).join(' '));
}
