// The shared base model of a record in its sleeve, built in code (owner 2026-10-16: "dass wir insgesamt auch ganz oft
// Shared 3D-Modelle anlegen können"): geometry, groove maps and materials are the same for every LP; an item gives
// only its look (`VinylLook`: sleeve and label pictures, a colour – ITEMS.md "3D item models"). The same model as
// tools/vinyl3d/build.py's `.set.glb` (that script stays for reference): metres, +Y up, the sleeve standing on y = 0,
// its front +Z, the spine on the left, open at +X with the record (side A +Z) standing in it, PEEK out. Groups named
// "sleeve" and "record" – the record player takes the record out (viewer record-stage.js). Plain JS like model-view.js.
// A next base model (7" single, cassette …): a builder like this one, its look another field.

const MM = 0.001;
const R_OUT = 151 * MM; // Ø 302 mm
const R_LABEL = 50 * MM; // label Ø 100 mm
const R_HOLE = 3.62 * MM; // hole Ø 7.24 mm
const HALF_T = 1 * MM; // 2 mm thick
const SEG = 96; // around: a flat disc, a chord dips < 0.1 px at the stage's size
const LEAD_IN = 150; // mm: the rounded edge outside of it
const GROOVES = [146, 60]; // mm, the grooved area (outside → in)
const TRACK_GAPS = 3; // 3 tracks a side → 2 gaps
const SLEEVE = { w: 315 * MM, h: 312 * MM, d: 3 * MM }; // a spined sleeve (specs/spinedsleeve_12inch_2019.pdf)
const INNER = 310 * MM; // a plain paper inner sleeve (a test press)
const PEEK = 25 * MM; // how far the record looks out of the opening
const ROWS = 1024; // groove map rows, edge → label (0.1 mm a row)

// linear colours (as the .glb's factors)
const BLACK = [0.012, 0.012, 0.014];
const WHITE_LABEL = [0.93, 0.92, 0.89];
const PAPER = [0.95, 0.94, 0.91];
const CARD = [0.85, 0.83, 0.78];

/** the pictures of a look (CDN-root paths) – what a page loads ahead (viewer model-warmup.ts) */
export function vinylPictures(look) {
	return ['sleeveFront', 'sleeveBack', 'sleeveSpine', 'labelA', 'labelB'].map((k) => look?.[k]).filter((p) => typeof p === 'string' && p);
}

// ---------------------------------------------------------------- groove maps (once per page)

let grooves = null;
/** the vinyl ring's normal + metal/roughness rows along the radius (row 0 = the edge): fine groove noise, louder bands,
 *  lead-in, track gaps, run-out (as build.py groove_maps) */
function grooveRows() {
	if (grooves) return grooves;
	let seed = 12;
	const rand = () => {
		seed = (seed + 0x6d2b79f5) | 0;
		let t = Math.imul(seed ^ (seed >>> 15), 1 | seed);
		t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t;
		return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
	};
	const gauss = () => Math.sqrt(-2 * Math.log(1 - rand())) * Math.cos(2 * Math.PI * rand());
	const h = ROWS, outer = R_OUT / MM, inner = R_LABEL / MM;
	const dr = (outer - inner) / (h - 1);
	const r = Float64Array.from({ length: h }, (_, i) => outer + ((inner - outer) * i) / (h - 1));
	// loudness: a slow random envelope (~10 mm window) – the visible bands of a record
	const k = Math.max(1, Math.round(10 / dr));
	const noise = Float64Array.from({ length: h }, gauss);
	const env = new Float64Array(h);
	for (let i = 0; i < h; i++) {
		let sum = 0;
		for (let j = i - (k >> 1); j < i - (k >> 1) + k; j++) if (j >= 0 && j < h) sum += noise[j];
		env[i] = sum / k;
	}
	let lo = Infinity, hi = -Infinity;
	for (const e of env) (lo = Math.min(lo, e)), (hi = Math.max(hi, e));
	for (let i = 0; i < h; i++) env[i] = 0.55 + (0.45 * (env[i] - lo)) / (hi - lo + 1e-9);
	const height = new Float64Array(h);
	const grooved = new Uint8Array(h);
	for (let i = 0; i < h; i++) {
		let gap = false;
		for (let g = 1; g < TRACK_GAPS; g++) if (Math.abs(r[i] - (GROOVES[0] - ((GROOVES[0] - GROOVES[1]) * g) / TRACK_GAPS)) < 0.6) gap = true;
		grooved[i] = r[i] <= GROOVES[0] && r[i] >= GROOVES[1] && !gap ? 1 : 0;
		const n = Math.max(-2.5, Math.min(2.5, gauss()));
		if (grooved[i]) height[i] = n * env[i] * 0.2 * dr;
		else if (r[i] < GROOVES[1] && r[i] > inner + 3) height[i] = 0.01 * Math.sin((2 * Math.PI * r[i]) / 2.5) ** 8; // run-out
		if (r[i] > LEAD_IN) height[i] = -0.02 * ((r[i] - LEAD_IN) / (outer - LEAD_IN)) ** 2; // the rounded edge
	}
	const normal = new Uint8Array(h * 4), mr = new Uint8Array(h * 4);
	for (let i = 0; i < h; i++) {
		const a = Math.max(0, i - 1), b = Math.min(h - 1, i + 1);
		const slope = (height[b] - height[a]) / (r[b] - r[a]);
		// tangent space: T = around (u), B = towards the edge → n = (0, −dh/dr, 1)
		const len = Math.hypot(slope, 1);
		normal.set([128, Math.round(((-slope / len) * 0.5 + 0.5) * 255), Math.round(((1 / len) * 0.5 + 0.5) * 255), 255], i * 4);
		// roughness (G): the grooves scatter, gaps / lead-in / run-out are glossy; metalness (B) 0
		const rough = r[i] > LEAD_IN ? 0.3 : grooved[i] ? 0.34 + 0.08 * env[i] : 0.18;
		mr.set([255, Math.round(rough * 255), 0, 255], i * 4);
	}
	grooves = { normal, mr };
	return grooves;
}

function rowTexture(THREE, data) {
	const t = new THREE.DataTexture(data, 1, ROWS, THREE.RGBAFormat);
	t.wrapS = THREE.RepeatWrapping;
	t.wrapT = THREE.ClampToEdgeWrapping;
	t.magFilter = THREE.LinearFilter;
	t.minFilter = THREE.LinearMipmapLinearFilter;
	t.generateMipmaps = true;
	t.needsUpdate = true;
	return t;
}

// ---------------------------------------------------------------- geometry

function geometry(THREE, pos, nor, uvs, idx, tan) {
	const g = new THREE.BufferGeometry();
	g.setAttribute('position', new THREE.Float32BufferAttribute(pos, 3));
	g.setAttribute('normal', new THREE.Float32BufferAttribute(nor, 3));
	if (uvs) g.setAttribute('uv', new THREE.Float32BufferAttribute(uvs, 2));
	if (tan) g.setAttribute('tangent', new THREE.Float32BufferAttribute(tan, 4));
	g.setIndex(idx);
	return g;
}

/** a flat ring r0 < r1 at height z facing +Z (f = 1) or −Z (f = −1); uv(x, y, r, a) → [u, v]; polar: tangents around */
function ring(THREE, r0, r1, z, f, uv, polar = false) {
	const pos = [], nor = [], uvs = [], tan = [], idx = [];
	for (const r of [r0, r1]) {
		for (let i = 0; i <= SEG; i++) {
			const a = (2 * Math.PI * i) / SEG, x = r * Math.cos(a), y = r * Math.sin(a);
			pos.push(x, y, z);
			nor.push(0, 0, f);
			if (uv) uvs.push(...uv(x, y, r, a));
			// T = around (counter-clockwise), w so that B = cross(N, T) · w points outwards
			if (polar) tan.push(-Math.sin(a), Math.cos(a), 0, -f);
		}
	}
	for (let i = 0; i < SEG; i++) {
		const a = i, b = i + 1, c = SEG + 1 + i, d = SEG + 2 + i;
		idx.push(...(f > 0 ? [a, c, b, b, c, d] : [a, b, c, b, d, c]));
	}
	return geometry(THREE, pos, nor, uv ? uvs : null, idx, polar ? tan : null);
}

/** the side wall of a cylinder (the edge, the hole), normals outward (+1) or inward (−1) */
function tube(THREE, r, out) {
	const pos = [], nor = [], idx = [];
	for (const z of [-HALF_T, HALF_T]) {
		for (let i = 0; i <= SEG; i++) {
			const a = (2 * Math.PI * i) / SEG;
			pos.push(r * Math.cos(a), r * Math.sin(a), z);
			nor.push(out * Math.cos(a), out * Math.sin(a), 0);
		}
	}
	for (let i = 0; i < SEG; i++) {
		const a = i, b = i + 1, c = SEG + 1 + i, d = SEG + 2 + i;
		idx.push(...(out > 0 ? [a, b, c, b, d, c] : [a, c, b, b, c, d]));
	}
	return geometry(THREE, pos, nor, null, idx);
}

/** quads [corners a b c d, normal] as one geometry, uv (0,1) (1,1) (1,0) (0,0) – a picture's bottom-left at a */
function quads(THREE, list) {
	const pos = [], nor = [], uvs = [], idx = [];
	for (const [corners, n] of list) {
		const base = pos.length / 3;
		for (const c of corners) pos.push(...c), nor.push(...n);
		uvs.push(0, 1, 1, 1, 1, 0, 0, 0);
		idx.push(base, base + 1, base + 2, base, base + 2, base + 3);
	}
	return geometry(THREE, pos, nor, uvs, idx);
}

// ---------------------------------------------------------------- the model

/**
 * Builds the record in its sleeve from a look. `url` turns a CDN-root path into a URL. Resolves when the pictures are
 * there (a failing picture rejects – the caller shows its 2D fallback).
 */
export async function buildVinyl(THREE, look, url = (p) => p) {
	const linear = (c) => new THREE.Color().setRGB(c[0], c[1], c[2]);
	const loader = new THREE.TextureLoader();
	const picture = async (path) => {
		if (!path) return null;
		const t = await loader.loadAsync(url(path));
		t.colorSpace = THREE.SRGBColorSpace;
		t.flipY = false; // the uvs are the .glb's (glTF: v down)
		t.anisotropy = 4;
		return t;
	};
	const [front, back, spine, labelA, labelB] = await Promise.all(
		[look.sleeveFront, look.sleeveBack, look.sleeveSpine, look.labelA, look.labelB].map(picture)
	);
	const std = (o) => new THREE.MeshStandardMaterial({ metalness: 0, ...o });

	// the record (its centre at the origin, side A +Z)
	const record = new THREE.Group();
	record.name = 'record';
	const labelUv = (sign) => (x, y) => [0.5 + (sign * x) / (2 * R_LABEL), 0.5 - y / (2 * R_LABEL)]; // side B mirrored so it reads
	for (const [tex, z, f] of [[labelA, HALF_T, 1], [labelB, -HALF_T, -1]]) {
		const mat = tex ? std({ map: tex, roughness: 0.75 }) : std({ color: linear(WHITE_LABEL), roughness: 0.8 });
		record.add(new THREE.Mesh(ring(THREE, R_HOLE, R_LABEL, z, f, tex ? labelUv(f) : null), mat));
	}
	const color = look.color ? new THREE.Color(look.color) : linear(BLACK);
	const { normal, mr } = grooveRows();
	const mrTex = rowTexture(THREE, mr);
	const vinyl = new THREE.MeshPhysicalMaterial({
		color,
		metalness: 1,
		roughness: 1,
		metalnessMap: mrTex,
		roughnessMap: mrTex,
		normalMap: rowTexture(THREE, normal),
		// the grooves' radial sheen: anisotropy along the tangent (= around)
		anisotropy: 0.75
	});
	const polar = (x, y, r, a) => [a / (2 * Math.PI), (R_OUT - r) / (R_OUT - R_LABEL)];
	for (const [z, f] of [[HALF_T, 1], [-HALF_T, -1]]) record.add(new THREE.Mesh(ring(THREE, R_LABEL, R_OUT, z, f, polar, true), vinyl));
	const edge = std({ color, roughness: 0.35 });
	record.add(new THREE.Mesh(tube(THREE, R_OUT, 1), edge), new THREE.Mesh(tube(THREE, R_HOLE, -1), edge));

	// the sleeve: printed (spined, the pictures) or a plain paper inner sleeve (no front picture – a test press)
	const printed = !!front;
	const w = printed ? SLEEVE.w : INNER, h = printed ? SLEEVE.h : INNER, d = SLEEVE.d;
	const [x0, x1, y0, y1, z0, z1] = [-w / 2, w / 2, -h / 2, h / 2, -d / 2, d / 2];
	const plain = std({ color: linear(printed ? CARD : PAPER), roughness: printed ? 0.9 : 0.85 });
	const face = (tex) => (tex ? std({ map: tex, roughness: 0.7 }) : plain);
	const e = d * 0.02; // the opening's insides, a hair inside: a look into it shows paper, not the void
	const sleeve = new THREE.Group();
	sleeve.name = 'sleeve';
	sleeve.add(
		new THREE.Mesh(quads(THREE, [[[[x0, y0, z1], [x1, y0, z1], [x1, y1, z1], [x0, y1, z1]], [0, 0, 1]]]), face(front)),
		new THREE.Mesh(quads(THREE, [[[[x1, y0, z0], [x0, y0, z0], [x0, y1, z0], [x1, y1, z0]], [0, 0, -1]]]), face(back)),
		new THREE.Mesh(quads(THREE, [[[[x0, y0, z0], [x0, y0, z1], [x0, y1, z1], [x0, y1, z0]], [-1, 0, 0]]]), face(spine)),
		new THREE.Mesh(
			quads(THREE, [
				[[[x0, y1, z1], [x1, y1, z1], [x1, y1, z0], [x0, y1, z0]], [0, 1, 0]],
				[[[x0, y0, z0], [x1, y0, z0], [x1, y0, z1], [x0, y0, z1]], [0, -1, 0]],
				[[[x1, y0, z1 - e], [x0, y0, z1 - e], [x0, y1, z1 - e], [x1, y1, z1 - e]], [0, 0, -1]],
				[[[x0, y0, z0 + e], [x1, y0, z0 + e], [x1, y1, z0 + e], [x0, y1, z0 + e]], [0, 0, 1]]
			]),
			plain
		)
	);
	sleeve.position.set(0, h / 2, 0);
	record.position.set(w / 2 + PEEK - R_OUT, h / 2, 0);
	const set = new THREE.Group();
	set.name = 'vinyl';
	set.add(sleeve, record);
	return set;
}
