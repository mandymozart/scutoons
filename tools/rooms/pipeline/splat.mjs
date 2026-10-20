// A place's room from a Gaussian splat (agents/research/LOCATION-SCENES.md §3, issue #74): the XV3dGS plugin's data
// textures (unreal/export_splat.py: pos / quat / scale / sh_0 as float EXR – Unreal cm, Z up, colour 0..1 + opacity)
// → one `.splat` per media quality (32 bytes a splat: position 3 × f32, scale 3 × f32, rgba u8, rotation u8 w x y z –
// the common antimatter15 layout), sorted by importance and capped per level, floaters and far splats cropped; the
// room's `view` and `spots` go into `<room>.splat.json` next to it.
// Usage: node splat.mjs <room.json> --source <export dir> [--out <dir>]   (room.json `splat` – tools/rooms/README.md)
import { readFile, writeFile, mkdir } from "node:fs/promises";
import { dirname, resolve, join } from "node:path";
import * as THREE from "three";
import { EXRLoader } from "three/addons/loaders/EXRLoader.js";

const roomFile = resolve(process.argv[2] ?? "");
const room = JSON.parse(await readFile(roomFile, "utf8"));
const arg = (k) => {
	const i = process.argv.indexOf(k);
	return i > 0 ? process.argv[i + 1] : undefined;
};
const src = resolve(arg("--source") ?? ".");
const out = resolve(
	arg("--out") ?? resolve(dirname(roomFile), "../../../media/scutoons/rooms"),
);
await mkdir(out, { recursive: true });
const cfg = room.splat;
const scale = room.scale ?? 1;

async function exr(name) {
	const buf = await readFile(join(src, name + ".exr"));
	const t = new EXRLoader()
		.setDataType(THREE.FloatType)
		.parse(buf.buffer.slice(buf.byteOffset, buf.byteOffset + buf.byteLength));
	return t.data;
}
const [P, Q, S, C] = await Promise.all(
	["pos", "quat", "scale", "sh_0"].map(exr),
);
const n = P.length / 4;

// Unreal (cm, Z up, left-handed) → glTF / three (m, Y up): (x, z, y) / 100 – a mirror, so a rotation (w, v) becomes
// (w, −Mv) and the scales swap y / z like the axes
const keep = [];
const pos = new Float32Array(n * 3);
for (let i = 0; i < n; i++) {
	const x = P[i * 4],
		y = P[i * 4 + 1],
		z = P[i * 4 + 2],
		a = C[i * 4 + 3];
	if (
		!Number.isFinite(x) ||
		!Number.isFinite(y) ||
		!Number.isFinite(z) ||
		!(a >= cfg.minOpacity)
	)
		continue;
	const big = Math.max(S[i * 4], S[i * 4 + 1], S[i * 4 + 2]) / 100;
	if (big > cfg.maxScale) continue;
	pos[i * 3] = (x / 100) * scale;
	pos[i * 3 + 1] = (z / 100) * scale;
	pos[i * 3 + 2] = (y / 100) * scale;
	keep.push(i);
}
// `level: "pca"`: a scan without an upright (not placed in any level) – its flattest axis (the smallest spread of the
// centres, the floor's normal in a room) turns to +Y around the mean; positions and rotations turn with it
let turn = null; // the quaternion [x, y, z, w] applied to every splat
if (cfg.level === "pca") {
	const mean = [0, 0, 0];
	for (const i of keep)
		for (let k = 0; k < 3; k++) mean[k] += pos[i * 3 + k] / keep.length;
	const C = [
		[0, 0, 0],
		[0, 0, 0],
		[0, 0, 0],
	];
	for (const i of keep) {
		const d = [0, 1, 2].map((k) => pos[i * 3 + k] - mean[k]);
		for (let a = 0; a < 3; a++)
			for (let c = 0; c < 3; c++) C[a][c] += (d[a] * d[c]) / keep.length;
	}
	// the smallest eigenvector: power iteration on (tr·I − C)
	const tr = C[0][0] + C[1][1] + C[2][2];
	let v = [0, 1, 0];
	for (let it = 0; it < 300; it++) {
		const w = [0, 1, 2].map(
			(a) => tr * v[a] - (C[a][0] * v[0] + C[a][1] * v[1] + C[a][2] * v[2]),
		);
		const l = Math.hypot(...w);
		v = w.map((x) => x / l);
	}
	if (v[1] < 0) v = v.map((x) => -x);
	// the rotation taking v to +Y: axis v × Y, angle acos(v·Y)
	const axis = [-v[2], 0, v[0]],
		al = Math.hypot(...axis),
		ang = Math.acos(Math.min(1, v[1]));
	if (al > 1e-6) {
		const sn = Math.sin(ang / 2);
		turn = [(axis[0] / al) * sn, 0, (axis[2] / al) * sn, Math.cos(ang / 2)];
		const [x, y, z, w] = turn;
		const R = [
			1 - 2 * (y * y + z * z),
			2 * (x * y - z * w),
			2 * (x * z + y * w),
			2 * (x * y + z * w),
			1 - 2 * (x * x + z * z),
			2 * (y * z - x * w),
			2 * (x * z - y * w),
			2 * (y * z + x * w),
			1 - 2 * (x * x + y * y),
		];
		for (const i of keep) {
			const d = [0, 1, 2].map((k) => pos[i * 3 + k] - mean[k]);
			for (let k = 0; k < 3; k++)
				pos[i * 3 + k] =
					mean[k] + R[k * 3] * d[0] + R[k * 3 + 1] * d[1] + R[k * 3 + 2] * d[2];
		}
	}
	console.log(
		`level: floor normal ${v.map((x) => x.toFixed(3))} → +Y (${((ang * 180) / Math.PI).toFixed(1)}°)`,
	);
}

// crop: around the median (or room.splat.center) within `radius` metres – floaters far out go
const med = (k) => {
	const v = keep.map((i) => pos[i * 3 + k]).sort((a, b) => a - b);
	return v[v.length >> 1];
};
const center = cfg.center ?? [med(0), med(1), med(2)];
const r2 = cfg.radius * cfg.radius;
const inside = keep.filter(
	(i) =>
		(pos[i * 3] - center[0]) ** 2 +
			(pos[i * 3 + 1] - center[1]) ** 2 +
			(pos[i * 3 + 2] - center[2]) ** 2 <=
		r2,
);
// importance = opacity × volume (the common splat order): the cap keeps the ones that make the picture
const weight = new Float32Array(n);
for (const i of inside)
	weight[i] = C[i * 4 + 3] * S[i * 4] * S[i * 4 + 1] * S[i * 4 + 2];
inside.sort((a, b) => weight[b] - weight[a]);

const bounds = [Infinity, Infinity, Infinity, -Infinity, -Infinity, -Infinity];
for (const i of inside)
	for (let k = 0; k < 3; k++) {
		bounds[k] = Math.min(bounds[k], pos[i * 3 + k]);
		bounds[k + 3] = Math.max(bounds[k + 3], pos[i * 3 + k]);
	}

const report = {
	source: n,
	valid: keep.length,
	inCrop: inside.length,
	center: center.map((v) => +v.toFixed(2)),
	bounds: bounds.map((v) => +v.toFixed(1)),
};
const u8 = (v) => Math.max(0, Math.min(255, Math.round(v)));
for (const [level, max] of Object.entries(cfg.levels)) {
	const list = inside.slice(0, max);
	const buf = Buffer.alloc(list.length * 32);
	let o = 0;
	for (const i of list) {
		buf.writeFloatLE(pos[i * 3], o);
		buf.writeFloatLE(pos[i * 3 + 1], o + 4);
		buf.writeFloatLE(pos[i * 3 + 2], o + 8);
		buf.writeFloatLE((S[i * 4] / 100) * scale, o + 12);
		buf.writeFloatLE((S[i * 4 + 2] / 100) * scale, o + 16);
		buf.writeFloatLE((S[i * 4 + 1] / 100) * scale, o + 20);
		buf[o + 24] = u8(C[i * 4] * 255);
		buf[o + 25] = u8(C[i * 4 + 1] * 255);
		buf[o + 26] = u8(C[i * 4 + 2] * 255);
		buf[o + 27] = u8(C[i * 4 + 3] * 255);
		// Unreal's quaternion is x y z w; mirrored: (w, −x, −z, −y)
		let qx = Q[i * 4],
			qy = Q[i * 4 + 1],
			qz = Q[i * 4 + 2],
			qw = Q[i * 4 + 3];
		const l = Math.hypot(qx, qy, qz, qw) || 1;
		[qw, qx, qy, qz] = [qw / l, -qx / l, -qz / l, -qy / l];
		if (turn) {
			// turn ∘ q
			const [tx, ty, tz, tw] = turn;
			[qw, qx, qy, qz] = [
				tw * qw - tx * qx - ty * qy - tz * qz,
				tw * qx + tx * qw + ty * qz - tz * qy,
				tw * qy - tx * qz + ty * qw + tz * qx,
				tw * qz + tx * qy - ty * qx + tz * qw,
			];
		}
		buf[o + 28] = u8(qw * 128 + 128);
		buf[o + 29] = u8(qx * 128 + 128);
		buf[o + 30] = u8(qy * 128 + 128);
		buf[o + 31] = u8(qz * 128 + 128);
		o += 32;
	}
	await writeFile(join(out, `${room.name}.${level}.splat`), buf);
	report[level] = {
		splats: list.length,
		mb: +(buf.length / 1e6).toFixed(2),
		gpuMB: +((list.length * 64) / 1e6).toFixed(1),
	};
}
// a splat has no nodes: its view and spots go next to it (`<room>.splat.json`, what a .glb carries as `view` / `spot_<id>`)
await writeFile(
	join(out, `${room.name}.splat.json`),
	JSON.stringify({ view: room.view, spots: room.spots ?? [] }),
);
await writeFile(
	join(out, `${room.name}.report.json`),
	JSON.stringify(report, null, 1),
);
console.log(JSON.stringify(report, null, 1));
