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
 * front, side: { img, sheet, box } (side may be null), options → { object (Mesh or SkinnedMesh), figure (for posing), grid, rig, info }
 */
export function build(front, side, o = {}) {
	o = { ...DEFAULTS, ...o };
	const t0 = performance.now();
	const grid = gridMask(front.img, { rows: o.rows, full: front.sheet.full, box: front.box });
	const sideGrid = side ? gridMask(side.img, { rows: o.rows, full: side.sheet.full, box: side.box }) : null;
	const t1 = performance.now();
	const m = buildMesh(grid, { height: o.height, depth: sideGrid ? Math.max(o.depth, 1.4) : o.depth, mode: o.mode, side: sideGrid, smooth: o.smooth, tris: o.tris });
	const t2 = performance.now();
	let geo = m.geometry;
	let rig = null;
	if (o.rig && o.mode === 'inflate') rig = autoRig(geo, grid, m);
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
	const t3 = performance.now();
	return {
		object, figure, grid, rig, mesh: m,
		info: { ...m.info, input: front.sheet.mode, ms: { mask: Math.round(t1 - t0), mesh: Math.round(t2 - t1), rig: Math.round(t3 - t2) }, warn: rig?.lm.warn ?? [] }
	};
}

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
