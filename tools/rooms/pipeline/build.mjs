// A place's room for the web (agents/research/LOCATION-SCENES.md §3, issue #74): an Unreal level export
// (tools/rooms/unreal/export_level.py: level.glb without baked materials + tex/*.png + manifest.json + lights.json)
// → one .glb per media quality (low / medium / high), each self-contained (ASSETS.md "3D models").
//   - the scans' base-colour textures go back onto their materials (the export can't bake without a GPU renderer)
//   - Unreal's lights are baked into vertex colours (no shadows) and every material becomes unlit: zero light cost at runtime
//   - the level's CineCamera becomes the node `view`; the room's spots become empty nodes `spot_<id>`
//   - weld, simplify (per level), quantize, meshopt; textures resized + WebP (KTX2 once toktx is installed)
// Usage: node build.mjs <room.json> [--source <export dir>] [--out <dir>] – prints and writes a report per level.
// Default out: media/scutoons/rooms/ (the local dev CDN copy); uploads go through the asset manager.
import { readFile, writeFile, mkdir, stat } from "node:fs/promises";
import { dirname, resolve, join } from "node:path";
import { NodeIO } from "@gltf-transform/core";
import {
	ALL_EXTENSIONS,
	EXTTextureWebP,
	KHRMaterialsUnlit,
	EXTMeshoptCompression,
} from "@gltf-transform/extensions";
import {
	weld,
	simplify,
	quantize,
	dedup,
	prune,
	join as joinMeshes,
	flatten,
	compactPrimitive,
} from "@gltf-transform/functions";
import {
	MeshoptDecoder,
	MeshoptEncoder,
	MeshoptSimplifier,
} from "meshoptimizer";
import sharp from "sharp";

const roomFile = resolve(process.argv[2] ?? "");
const room = JSON.parse(await readFile(roomFile, "utf8"));
const base = dirname(roomFile);
const arg = (k) => {
	const i = process.argv.indexOf(k);
	return i > 0 ? process.argv[i + 1] : undefined;
};
const src = resolve(arg("--source") ?? resolve(base, room.source ?? "."));
const out = resolve(
	arg("--out") ?? resolve(base, "../../../media/scutoons/rooms"),
);
await mkdir(out, { recursive: true });
await Promise.all([
	MeshoptDecoder.ready,
	MeshoptEncoder.ready,
	MeshoptSimplifier.ready,
]);
const io = new NodeIO()
	.registerExtensions(ALL_EXTENSIONS)
	.registerDependencies({
		"meshopt.decoder": MeshoptDecoder,
		"meshopt.encoder": MeshoptEncoder,
	});
const manifest = JSON.parse(await readFile(join(src, "manifest.json"), "utf8"));
// `allLights`: also the lights the scene keeps off for its renders (a Blender file's hide_render)
const lights = JSON.parse(
	await readFile(join(src, "lights.json"), "utf8"),
).lights.filter((l) => room.allLights || l.visible);

/** Unreal (cm, Z up, X forward, left-handed) → glTF (m, Y up): the exporter's mapping, read off the actors */
const ue = ([x, y, z]) => [x / 100, z / 100, y / 100];
const ueDir = ([x, y, z]) => [x, z, y];

/** a vertex's light from the level's lights: inverse square with Unreal's radius window, the spot cones, n·l */
function lightAt(ps, n) {
	// the bake runs in Unreal's own size: a room scaled with `scale` (room.json) is lit as composed
	const p = ps.map((v) => v / (room.scale ?? 1));
	const c = [0, 0, 0];
	for (const l of lights) {
		if (l.class === "DirectionalLightComponent") {
			// a sun: no falloff, n·l against its direction
			const ld = ueDir(l.dir);
			const ndl = Math.max(0, -(n[0] * ld[0] + n[1] * ld[1] + n[2] * ld[2]));
			for (let i = 0; i < 3; i++)
				c[i] += (l.color[i] / 255) * l.intensity * ndl * (room.sun ?? 0.1);
			continue;
		}
		const lp = ue(l.pos);
		const d = [lp[0] - p[0], lp[1] - p[1], lp[2] - p[2]];
		const dist = Math.hypot(...d) || 1e-3;
		const r = parseFloat(l.attenuation_radius ?? "1000") / 100;
		if (dist > r) continue;
		const win = Math.max(0, 1 - (dist / r) ** 4) ** 2;
		const lv = d.map((v) => v / dist);
		const ndl = Math.max(0, n[0] * lv[0] + n[1] * lv[1] + n[2] * lv[2]);
		let cone = 1;
		if (l.class === "SpotLightComponent") {
			const ld = ueDir(l.dir);
			const cos = -(lv[0] * ld[0] + lv[1] * ld[1] + lv[2] * ld[2]);
			const outer = Math.cos((parseFloat(l.outer_cone_angle) * Math.PI) / 180);
			const inner = Math.cos(
				(parseFloat(l.inner_cone_angle || "0") * Math.PI) / 180,
			);
			cone = Math.min(
				1,
				Math.max(0, (cos - outer) / Math.max(1e-3, inner - outer)),
			);
		} else if (l.class === "RectLightComponent") {
			const ld = ueDir(l.dir);
			cone = Math.max(0, -(lv[0] * ld[0] + lv[1] * ld[1] + lv[2] * ld[2]));
		}
		const e = (l.intensity * win * cone * ndl) / (dist * dist);
		for (let i = 0; i < 3; i++) c[i] += (l.color[i] / 255) * e;
	}
	return c;
}

/** the quaternion that turns a glTF camera (looking down −Z, +Y up) at `at` towards `look` */
function lookRotation(at, look) {
	const f = look.map((v, i) => v - at[i]),
		fl = Math.hypot(...f);
	const z = f.map((v) => -v / fl);
	const cx = [1 * z[2] - 0 * z[1], 0 * z[0] - 0 * z[2], 0 * z[1] - 1 * z[0]]; // up (0,1,0) × z
	const xl = Math.hypot(...cx),
		x = cx.map((v) => v / xl);
	const y = [
		z[1] * x[2] - z[2] * x[1],
		z[2] * x[0] - z[0] * x[2],
		z[0] * x[1] - z[1] * x[0],
	];
	const [m00, m01, m02, m10, m11, m12, m20, m21, m22] = [
		x[0],
		y[0],
		z[0],
		x[1],
		y[1],
		z[1],
		x[2],
		y[2],
		z[2],
	];
	const tr = m00 + m11 + m22;
	if (tr > 0) {
		const s = Math.sqrt(tr + 1) * 2;
		return [(m21 - m12) / s, (m02 - m20) / s, (m10 - m01) / s, s / 4];
	}
	if (m00 > m11 && m00 > m22) {
		const s = Math.sqrt(1 + m00 - m11 - m22) * 2;
		return [s / 4, (m01 + m10) / s, (m02 + m20) / s, (m21 - m12) / s];
	}
	if (m11 > m22) {
		const s = Math.sqrt(1 + m11 - m00 - m22) * 2;
		return [(m01 + m10) / s, s / 4, (m12 + m21) / s, (m02 - m20) / s];
	}
	const s = Math.sqrt(1 + m22 - m00 - m11) * 2;
	return [(m02 + m20) / s, (m12 + m21) / s, s / 4, (m10 - m01) / s];
}

const doc = await io.read(join(src, "level.glb"));
const root = doc.getRoot();
const unlit = doc.createExtension(KHRMaterialsUnlit);
doc.createExtension(EXTTextureWebP).setRequired(true);

// 1. drop what the room doesn't show
for (const node of root.listNodes())
	if ((room.drop ?? []).some((p) => node.getName().startsWith(p)))
		node.dispose();

// 1b. `scale`: the level at real size (a scan blown up for a shot) – view, spots and cut are in the scaled metres
if (room.scale && room.scale !== 1) {
	for (const node of root.listScenes()[0].listChildren()) {
		node.setTranslation(node.getTranslation().map((v) => v * room.scale));
		node.setScale(node.getScale().map((v) => v * room.scale));
	}
}

// 1c. `ground: { y, center: [x, z], size: [w, d], color }` – a plain floor where the scan has none (a forecourt), a grid
// fine enough for the light bake
if (room.ground) {
	const { y, center, size, color } = room.ground;
	const nx = 48,
		nz = 32,
		pos = [],
		idx = [];
	for (let j = 0; j <= nz; j++)
		for (let i = 0; i <= nx; i++)
			pos.push(
				center[0] + (i / nx - 0.5) * size[0],
				y,
				center[1] + (j / nz - 0.5) * size[1],
			);
	for (let j = 0; j < nz; j++)
		for (let i = 0; i < nx; i++) {
			const a = j * (nx + 1) + i;
			idx.push(a, a + nx + 1, a + 1, a + 1, a + nx + 1, a + nx + 2);
		}
	const buffer = root.listBuffers()[0];
	const prim = doc
		.createPrimitive()
		.setAttribute(
			"POSITION",
			doc
				.createAccessor()
				.setType("VEC3")
				.setArray(new Float32Array(pos))
				.setBuffer(buffer),
		)
		.setAttribute(
			"NORMAL",
			doc
				.createAccessor()
				.setType("VEC3")
				.setArray(
					new Float32Array(pos.length).map((_, k) => (k % 3 === 1 ? 1 : 0)),
				)
				.setBuffer(buffer),
		)
		.setIndices(
			doc
				.createAccessor()
				.setType("SCALAR")
				.setArray(new Uint32Array(idx))
				.setBuffer(buffer),
		)
		.setMaterial(
			doc.createMaterial("ground").setBaseColorFactor([...color, 1]),
		);
	root
		.listScenes()[0]
		.addChild(
			doc
				.createNode("ground")
				.setMesh(doc.createMesh("ground").addPrimitive(prim)),
		);
}

// 2. textures back onto the scans: actor label = node name → its material → BaseColorTexture → tex/<name>.png
const actors = Object.fromEntries(
	manifest.actors.filter((a) => a.meshes).map((a) => [a.name, a]),
);
const done = new Map();
for (const node of root.listNodes()) {
	const mesh = node.getMesh();
	const actor = actors[node.getName()];
	if (!mesh || !actor) continue;
	const params = manifest.materials[actor.meshes[0].materials[0]] ?? [];
	const baseTex = params.find((p) => p.startsWith("BaseColorTexture="));
	if (!baseTex) continue;
	const file = join(src, "tex", baseTex.split(".").pop() + ".png");
	for (const prim of mesh.listPrimitives()) {
		const mat = prim.getMaterial();
		if (!mat || done.has(mat)) continue;
		const tex = doc
			.createTexture(node.getName())
			.setImage(new Uint8Array(await readFile(file)))
			.setMimeType("image/png");
		mat.setBaseColorTexture(tex).setBaseColorFactor([1, 1, 1, 1]);
		done.set(mat, tex);
	}
}

// 2b. a diorama has no backside: keep only the triangles the view can see while the reader looks around (the view's
// cone widened by the look-around and a margin, up to `far` metres) – room.json `cut: { margin, far, aspect }`
flatten(doc);
// a mesh several nodes share (linked duplicates – the library's 13 tiles) gets one copy each: the cut and the bake work
// in world space, per node
{
	const seen = new Set();
	for (const node of root.listNodes()) {
		const mesh = node.getMesh();
		if (!mesh) continue;
		if (seen.has(mesh)) {
			const copy = mesh.clone();
			copy.listPrimitives().forEach((p, i) => copy.removePrimitive(p));
			for (const p of mesh.listPrimitives()) {
				const q = p.clone();
				for (const sem of p.listSemantics())
					q.setAttribute(sem, p.getAttribute(sem).clone());
				if (p.getIndices()) q.setIndices(p.getIndices().clone());
				copy.addPrimitive(q);
			}
			node.setMesh(copy);
		} else seen.add(mesh);
	}
	// …and so does data several primitives share (Blender's exporter shares the tiles' accessors): the cut rewrites indices
	const used = new Map();
	for (const mesh of root.listMeshes())
		for (const prim of mesh.listPrimitives())
			for (const a of [prim.getIndices(), ...prim.listAttributes()])
				if (a) used.set(a, (used.get(a) ?? 0) + 1);
	const once = new Set();
	for (const mesh of root.listMeshes())
		for (const prim of mesh.listPrimitives()) {
			const idx = prim.getIndices();
			if (idx && used.get(idx) > 1) {
				if (once.has(idx)) prim.setIndices(idx.clone());
				else once.add(idx);
			}
			for (const sem of prim.listSemantics()) {
				const a = prim.getAttribute(sem);
				if (used.get(a) > 1) {
					if (once.has(a)) prim.setAttribute(sem, a.clone());
					else once.add(a);
				}
			}
		}
}
if (room.view && room.cut) {
	const q = lookRotation(room.view.at, room.view.look);
	// the camera's axes from the quaternion (x right, y up, z back)
	const [qx, qy, qz, qw] = q;
	const ax = [
		1 - 2 * (qy * qy + qz * qz),
		2 * (qx * qy + qz * qw),
		2 * (qx * qz - qy * qw),
	];
	const ay = [
		2 * (qx * qy - qz * qw),
		1 - 2 * (qx * qx + qz * qz),
		2 * (qy * qz + qx * qw),
	];
	const az = [
		2 * (qx * qz + qy * qw),
		2 * (qy * qz - qx * qw),
		1 - 2 * (qx * qx + qy * qy),
	];
	const vfov = ((room.view.fov ?? 55) * Math.PI) / 180;
	const deg = Math.PI / 180;
	// the cone's half angles, kept under 90° (a wide view + the look-around + the margin can pass it: tan turns negative)
	const cap = (a) => Math.tan(Math.min(a, 89 * deg));
	const tanH = cap(
		Math.atan(Math.tan(vfov / 2) * (room.cut.aspect ?? 2)) +
			(18 + room.cut.margin) * deg,
	);
	const tanV = cap(vfov / 2 + (6 + room.cut.margin) * deg);
	const far = room.cut.far ?? 30;
	let kept = 0,
		all = 0;
	for (const node of root.listNodes()) {
		const mesh = node.getMesh();
		if (!mesh) continue;
		const m = node.getWorldMatrix();
		for (const prim of mesh.listPrimitives()) {
			const pos = prim.getAttribute("POSITION"),
				idx = prim.getIndices();
			if (!idx) continue;
			const src = idx.getArray(),
				keep = [];
			const p = [0, 0, 0],
				c = [0, 0, 0];
			for (let t = 0; t < src.length; t += 3) {
				let seen = false;
				// a triangle stays when one of its corners is in the cone (big scan triangles reach in from outside)
				for (let k = 0; k < 3 && !seen; k++) {
					pos.getElement(src[t + k], p);
					for (let i = 0; i < 3; i++)
						c[i] =
							m[i] * p[0] +
							m[4 + i] * p[1] +
							m[8 + i] * p[2] +
							m[12 + i] -
							room.view.at[i];
					const vz = -(c[0] * az[0] + c[1] * az[1] + c[2] * az[2]);
					if (vz <= 0.05 || vz > far) continue;
					const vx = c[0] * ax[0] + c[1] * ax[1] + c[2] * ax[2],
						vy = c[0] * ay[0] + c[1] * ay[1] + c[2] * ay[2];
					seen = Math.abs(vx / vz) < tanH && Math.abs(vy / vz) < tanV;
				}
				all++;
				if (seen) {
					kept++;
					keep.push(src[t], src[t + 1], src[t + 2]);
				}
			}
			idx.setArray(
				src.length > 65535 * 3 || pos.getCount() > 65535
					? new Uint32Array(keep)
					: new Uint16Array(keep),
			);
			compactPrimitive(prim);
		}
	}
	console.log(`cut: ${kept} of ${all} triangles in view`);
}

// 3. bake the lights into COLOR_0 (world space), tone-mapped so the room keeps Unreal's mood; every material unlit
for (const node of root.listNodes()) {
	const mesh = node.getMesh();
	if (!mesh) continue;
	const m = node.getWorldMatrix();
	const glows = (room.glow ?? []).some((g) => node.getName().startsWith(g));
	for (const prim of mesh.listPrimitives()) {
		const pos = prim.getAttribute("POSITION");
		const nor = prim.getAttribute("NORMAL");
		const color = new Float32Array(pos.getCount() * 4);
		const p = [0, 0, 0],
			n = [0, 1, 0];
		for (let i = 0; i < pos.getCount(); i++) {
			pos.getElement(i, p);
			if (nor) nor.getElement(i, n);
			const w = [
				m[0] * p[0] + m[4] * p[1] + m[8] * p[2] + m[12],
				m[1] * p[0] + m[5] * p[1] + m[9] * p[2] + m[13],
				m[2] * p[0] + m[6] * p[1] + m[10] * p[2] + m[14],
			];
			let wn = [
				m[0] * n[0] + m[4] * n[1] + m[8] * n[2],
				m[1] * n[0] + m[5] * n[1] + m[9] * n[2],
				m[2] * n[0] + m[6] * n[1] + m[10] * n[2],
			];
			const len = Math.hypot(...wn);
			wn = len > 1e-6 ? wn.map((v) => v / len) : [0, 1, 0];
			// scans have no reliable normals everywhere: half-lambert keeps backfacing scan geometry from going black
			const L = lightAt(w, wn);
			const L2 = lightAt(
				w,
				wn.map((v) => -v),
			);
			for (let k = 0; k < 3; k++) {
				const l = room.ambient[k] + room.exposure * (L[k] + 0.35 * L2[k]);
				// `glow`: nodes that shine themselves (a window, a lamp's face) – full colour, no bake
				color[i * 4 + k] = glows ? 1 : 1 - Math.exp(-l);
			}
			color[i * 4 + 3] = 1;
		}
		prim.setAttribute(
			"COLOR_0",
			doc
				.createAccessor()
				.setType("VEC4")
				.setArray(color)
				.setBuffer(root.listBuffers()[0]),
		);
		// unlit: normals and tangents only cost bytes and keep vertices apart (weld / simplify can't merge them)
		prim.setAttribute("NORMAL", null).setAttribute("TANGENT", null);
		const mat = prim.getMaterial();
		if (mat)
			mat
				.setExtension("KHR_materials_unlit", unlit.createUnlit())
				.setDoubleSided(true) // a room is seen from inside and from below: no face may vanish
				.setMetallicRoughnessTexture(null)
				.setNormalTexture(null)
				.setOcclusionTexture(null);
	}
}

// 4. the camera = `view` (the spots = empty nodes `spot_<id>` come per level, step 5 – LOCATION-SCENES §5)
const scene = root.listScenes()[0];
const cam = root.listNodes().find((n) => n.getCamera());
if (room.view) {
	// the room's own view (room.json `view: { at, look, fov? }`) replaces the level's (cinematic) camera
	for (const n of root.listNodes()) if (n.getCamera()) n.dispose();
	const camera = doc
		.createCamera("view")
		.setType("perspective")
		.setYFov(((room.view.fov ?? 55) * Math.PI) / 180)
		.setZNear(0.05)
		.setZFar(300)
		.setAspectRatio(1);
	scene.addChild(
		doc
			.createNode("view")
			.setCamera(camera)
			.setTranslation(room.view.at)
			.setRotation(lookRotation(room.view.at, room.view.look)),
	);
} else if (cam) cam.setName("view");
await doc.transform(dedup(), weld());

// 5. per quality level: simplify, textures, quantize + meshopt
const report = {};
const plain = await io.writeBinary(doc);
for (const [level, cfg] of Object.entries(room.levels)) {
	const d = await io.readBinary(plain);
	if (cfg.ratio < 1)
		await d.transform(
			simplify({
				simplifier: MeshoptSimplifier,
				ratio: cfg.ratio,
				error: cfg.error ?? 0.002,
			}),
		);
	const gpuOf = new Map(); // a texture's GPU bytes (RGBA + mips) – summed after prune, over what stays
	for (const t of d.getRoot().listTextures()) {
		const buf = await sharp(Buffer.from(t.getImage()), {
			limitInputPixels: false,
		})
			.resize(cfg.texture, cfg.texture, {
				fit: "inside",
				withoutEnlargement: true,
			})
			.webp({ quality: cfg.quality ?? 80 })
			.toBuffer();
		const meta = await sharp(buf).metadata();
		gpuOf.set(t, (meta.width * meta.height * 4 * 4) / 3);
		t.setImage(new Uint8Array(buf)).setMimeType("image/webp");
	}
	d.createExtension(EXTMeshoptCompression)
		.setRequired(true)
		.setEncoderOptions({ method: EXTMeshoptCompression.EncoderMethod.FILTER });
	await d.transform(
		joinMeshes(),
		quantize({ quantizeColor: 8 }),
		prune({ keepLeaves: true }),
	);
	// the spots last: the transforms above prune empty nodes
	for (const spot of room.spots ?? [])
		d.getRoot()
			.listScenes()[0]
			.addChild(d.createNode("spot_" + spot.id).setTranslation(spot.at));
	const file = join(out, `${room.name}.${level}.glb`);
	await io.write(file, d);
	let tris = 0,
		draws = 0;
	for (const mesh of d.getRoot().listMeshes())
		for (const prim of mesh.listPrimitives()) {
			draws++;
			tris += (prim.getIndices()?.getCount() ?? 0) / 3;
		}
	report[level] = {
		mb: +((await stat(file)).size / 1e6).toFixed(2),
		tris: Math.round(tris),
		drawCalls: draws,
		textures: d.getRoot().listTextures().length,
		gpuTexturesMB: +(
			d
				.getRoot()
				.listTextures()
				.reduce((n, t) => n + (gpuOf.get(t) ?? 0), 0) / 1e6
		).toFixed(1),
	};
}
await writeFile(
	join(out, `${room.name}.report.json`),
	JSON.stringify(report, null, 1),
);
console.table(report);
