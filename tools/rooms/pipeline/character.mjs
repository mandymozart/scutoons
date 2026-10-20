// A character's 3D model from an Unreal export (agents/research/LOCATION-SCENES.md §7d, ASSETS.md "3D models"): the
// skeletal mesh's .glb (no texture – the export can't bake) + its texture PNG → one self-contained .glb to the model
// spec: metres, +Y up, feet on y = 0, its texture back on as WebP, simplified (skin kept), quantized
// (KHR_mesh_quantization) – no meshopt: the tools' and the viewer's ModelPreview loads without a meshopt decoder.
// Usage: node character.mjs <in.glb> <texture.png> <out.glb> [triangles = 40000] [texture px = 2048]
import { readFile, stat } from "node:fs/promises";
import { NodeIO } from "@gltf-transform/core";
import { ALL_EXTENSIONS, EXTTextureWebP } from "@gltf-transform/extensions";
import {
	weld,
	simplify,
	quantize,
	prune,
	dedup,
} from "@gltf-transform/functions";
import { MeshoptSimplifier } from "meshoptimizer";
import sharp from "sharp";

const [, , inp, texFile, out, trisArg = "40000", pxArg = "2048"] = process.argv;
await MeshoptSimplifier.ready;
const io = new NodeIO().registerExtensions(ALL_EXTENSIONS);
const doc = await io.read(inp);
const root = doc.getRoot();
doc.createExtension(EXTTextureWebP).setRequired(true);

const png = await readFile(texFile);
const webp = await sharp(png, { limitInputPixels: false })
	.resize(+pxArg, +pxArg, { fit: "inside", withoutEnlargement: true })
	.webp({ quality: 82 })
	.toBuffer();
const tex = doc
	.createTexture("basecolor")
	.setImage(new Uint8Array(webp))
	.setMimeType("image/webp");
for (const mat of root.listMaterials())
	mat
		.setBaseColorTexture(tex)
		.setBaseColorFactor([1, 1, 1, 1])
		.setMetallicFactor(0)
		.setRoughnessFactor(0.85);

let tris = 0;
for (const m of root.listMeshes())
	for (const p of m.listPrimitives())
		tris += (p.getIndices()?.getCount() ?? 0) / 3;
const ratio = Math.min(1, +trisArg / tris);
await doc.transform(
	dedup(),
	weld(),
	...(ratio < 1
		? [simplify({ simplifier: MeshoptSimplifier, ratio, error: 0.001 })]
		: []),
	quantize(),
	prune(),
);

let after = 0;
for (const m of root.listMeshes())
	for (const p of m.listPrimitives())
		after += (p.getIndices()?.getCount() ?? 0) / 3;
await io.write(out, doc);
console.log(
	JSON.stringify({
		trianglesBefore: tris,
		triangles: after,
		joints: root
			.listSkins()[0]
			?.listJoints()
			.map((j) => j.getName()),
		texturePx: (await sharp(webp).metadata()).width,
		mb: +((await stat(out)).size / 1e6).toFixed(2),
	}),
);
