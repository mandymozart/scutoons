// The whole way: image(s) → grid → mesh → (rig) → flat-coloured SkinnedMesh / Mesh → GLB. Browser only (canvas for
// decoding, GLTFExporter for the file).
import * as THREE from 'three';
import { GLTFExporter } from 'three/addons/exporters/GLTFExporter.js';
import { gridMask, sheet } from './mask.js';
import { buildMesh, flatColored } from './mesh.js';
import { autoRig, skinnedFigure } from './rig.js';

/** an image URL / Blob → ImageData (longest side at most maxSide) */
export async function loadImageData(src, maxSide = 1400) {
	const blob = typeof src === 'string' ? await (await fetch(src)).blob() : src;
	const bmp = await createImageBitmap(blob);
	const k = Math.min(1, maxSide / Math.max(bmp.width, bmp.height));
	const w = Math.round(bmp.width * k), h = Math.round(bmp.height * k);
	const c = new OffscreenCanvas(w, h);
	const g = c.getContext('2d', { willReadFrequently: true });
	g.drawImage(bmp, 0, 0, w, h);
	return g.getImageData(0, 0, w, h);
}

export const DEFAULTS = { rows: 112, mode: 'inflate', depth: 0.8, smooth: 4, tris: 1500, colors: 10, height: 1.8, rig: true, input: 'auto', tol: 10, mono: true };

/** an image → its figures (a character sheet: front, ¾, back … side by side) – mask.js sheet() */
export function analyse(img, o = {}) {
	o = { ...DEFAULTS, ...o };
	return sheet(img, { mode: o.input, tol: o.tol });
}

/**
 * front, side: { img, sheet, box } (side may be null), options → the mesh stage: { grid, mesh, ms, input }
 * (slow part; the rig stage below reruns alone when joints are moved by hand)
 */
export function buildBase(front, side, o = {}) {
	o = { ...DEFAULTS, ...o };
	const t0 = performance.now();
	const grid = gridMask(front.img, { rows: o.rows, full: front.sheet.full, box: front.box });
	let sideGrid = side ? gridMask(side.img, { rows: o.rows, full: side.sheet.full, box: side.box }) : null;
	if (sideGrid && facesLeft(sideGrid)) sideGrid = mirrored(sideGrid);
	const t1 = performance.now();
	const mesh = buildMesh(grid, { height: o.height, depth: sideGrid ? Math.max(o.depth, 1.4) : o.depth, mode: o.mode, side: sideGrid, smooth: o.smooth, tris: o.tris });
	return { grid, mesh, input: front.sheet.mode, ms: { mask: Math.round(t1 - t0), mesh: Math.round(performance.now() - t1) } };
}

/**
 * the mesh stage + options + joints moved by hand (rig.js landmarks: { 'arm.L.el': [gx, gy], … }) →
 * { object (Mesh or SkinnedMesh), figure (for posing), grid, rig, mesh, info }
 */
export function finish(base, o = {}, joints = null) {
	o = { ...DEFAULTS, ...o };
	const { grid, mesh: m } = base;
	const t2 = performance.now();
	const geo = m.geometry;
	let rig = null;
	if (o.rig && o.mode === 'inflate') rig = autoRig(geo, grid, m, joints);
	const flat = flatColored(geo, grid, m.toGrid, { colors: o.colors, mono: o.mono ? [0.82, 0.82, 0.8] : null });
	const material = new THREE.MeshStandardMaterial({ vertexColors: true, flatShading: true, roughness: 0.85, metalness: 0 });
	let object, figure = null;
	if (rig) {
		figure = skinnedFigure(flat, rig, material);
		object = figure.mesh;
	} else {
		object = new THREE.Mesh(flat, material);
		object.name = 'body';
	}
	return {
		object, figure, grid, rig, mesh: m,
		info: { ...m.info, input: base.input, ms: { ...base.ms, rig: Math.round(performance.now() - t2) }, warn: rig?.lm.warn ?? [] }
	};
}

/** both stages at once */
export const build = (front, side, o = {}, joints = null) => finish(buildBase(front, side, o), o, joints);

/** the built object (in its rest / bind pose) → a binary glTF */
export async function toGLB(object, name = 'figure') {
	const scene = new THREE.Scene();
	const group = new THREE.Group();
	group.name = name;
	group.add(object);
	scene.add(group);
	const buf = await new GLTFExporter().parseAsync(scene, { binary: true, onlyVisible: true });
	group.remove(object);
	return buf;
}

/** a side view's facing: the feet (bottom 4 %) reach further forward than the body's middle (rows 30–60 %) */
function facesLeft(g) {
	const H = g.sole - g.top + 1;
	const mean = (y0, y1) => { let s = 0, n = 0; for (let y = Math.round(y0); y <= Math.round(y1); y++) for (let x = 0; x < g.w; x++) if (g.mask[y * g.w + x]) { s += x; n++; } return n ? s / n : 0; };
	const feet = [], body = mean(g.top + H * 0.3, g.top + H * 0.6);
	// the toes: the farthest-out figure cell of the bottom rows, on either side of the body's middle
	let lo = Infinity, hi = -Infinity;
	for (let y = Math.round(g.sole - H * 0.04); y <= g.sole; y++) for (let x = 0; x < g.w; x++) if (g.mask[y * g.w + x]) { lo = Math.min(lo, x); hi = Math.max(hi, x); }
	feet.push(lo, hi);
	return body - lo > hi - body;
}

/** the grid mirrored left ↔ right */
function mirrored(g) {
	const mask = new Uint8Array(g.mask.length), color = new Float32Array(g.color.length);
	for (let y = 0; y < g.h; y++) for (let x = 0; x < g.w; x++) {
		const a = y * g.w + x, b = y * g.w + (g.w - 1 - x);
		mask[a] = g.mask[b];
		color[a * 3] = g.color[b * 3]; color[a * 3 + 1] = g.color[b * 3 + 1]; color[a * 3 + 2] = g.color[b * 3 + 2];
	}
	return { ...g, mask, color, mirrored: true };
}
