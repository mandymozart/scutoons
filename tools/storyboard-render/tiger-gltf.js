// The mountain tiger from a real model (models/mountain-tiger.glb – "Tiger" by Poly by Google, CC BY 3.0,
// models/CREDITS.md), rigged here at load time: a skeleton like the procedural tiger's (hips → waist → chest → neck →
// head + jaw, four legs in three parts, a two-part tail) placed by the mesh's proportions; every vertex bound to the
// bone of its body region with a soft blend across the joints. The texture stays: under the hard threshold the fur
// goes white and the stripes black – a white mountain tiger. Posed by figures.js's tiger table (joints.rigged).
import * as THREE from 'three';

export function gltfTiger(gltf, { length = 2.8 } = {}) {
	let src = null;
	gltf.scene.traverse((o) => { if (o.isMesh && !src) src = o; });
	const geo = src.geometry.clone();
	geo.computeBoundingBox();
	const bb = geo.boundingBox;
	const pos = geo.attributes.position;
	const v = new THREE.Vector3();
	// the long axis is z; the head end is the one with more mass up high
	let hiFront = 0, hiBack = 0;
	const midZ = (bb.min.z + bb.max.z) / 2, highY = bb.min.y + (bb.max.y - bb.min.y) * 0.75;
	for (let i = 0; i < pos.count; i++) { v.fromBufferAttribute(pos, i); if (v.y > highY) (v.z > midZ ? hiFront++ : hiBack++); }
	const flip = hiBack > hiFront ? -1 : 1;
	const s = length / (bb.max.z - bb.min.z);
	for (let i = 0; i < pos.count; i++) {
		v.fromBufferAttribute(pos, i);
		pos.setXYZ(i, (v.x - (bb.min.x + bb.max.x) / 2) * s * flip, (v.y - bb.min.y) * s, (v.z - midZ) * s * flip);
	}
	pos.needsUpdate = true;
	geo.computeBoundingBox();
	geo.computeVertexNormals();
	const b = geo.boundingBox;
	const Lz = b.max.z - b.min.z, Hy = b.max.y;
	const zAt = (t) => b.min.z + Lz * t; // 0 = the tail end, 1 = the nose
	const legTop = Hy * 0.5;

	const bones = {};
	const mk = (name, parent, x, y, z) => {
		const bo = new THREE.Bone();
		bo.name = name;
		bo.userData.world = new THREE.Vector3(x, y, z);
		if (parent) { bo.position.copy(bo.userData.world).sub(bones[parent].userData.world); bones[parent].add(bo); }
		else bo.position.copy(bo.userData.world);
		bones[name] = bo;
	};
	mk('hips', null, 0, Hy * 0.62, zAt(0.22));
	mk('waist', 'hips', 0, Hy * 0.64, zAt(0.45));
	mk('chest', 'waist', 0, Hy * 0.66, zAt(0.66));
	mk('neck', 'chest', 0, Hy * 0.72, zAt(0.8));
	mk('head', 'neck', 0, Hy * 0.8, zAt(0.9));
	mk('jaw', 'head', 0, Hy * 0.7, zAt(0.95));
	const legX = (b.max.x - b.min.x) * 0.22;
	for (const [leg, parent, t] of [['FL', 'chest', 0.72], ['FR', 'chest', 0.72], ['BL', 'hips', 0.2], ['BR', 'hips', 0.2]]) {
		const x = leg.endsWith('L') ? legX : -legX;
		mk(leg, parent, x, legTop, zAt(t));
		mk(leg + '2', leg, x, Hy * 0.3, zAt(t));
		mk(leg + '3', leg + '2', x, Hy * 0.1, zAt(t));
	}
	mk('tail', 'hips', 0, Hy * 0.6, zAt(0.07));
	mk('tail2', 'tail', 0, Hy * 0.45, zAt(0.0));
	const order = Object.keys(bones);
	const index = Object.fromEntries(order.map((n, i) => [n, i]));

	const skinIndex = [], skinWeight = [];
	const spine = [['hips', 0.0], ['waist', 0.34], ['chest', 0.56], ['neck', 0.76], ['head', 0.86]];
	for (let i = 0; i < pos.count; i++) {
		v.fromBufferAttribute(pos, i);
		const t = (v.z - b.min.z) / Lz;
		const y = v.y / Hy;
		let a = 'hips', c = 'hips', w = 1;
		const front = t > 0.5;
		if (v.y < legTop && ((t > 0.58 && t < 0.86) || (t > 0.08 && t < 0.36))) {
			const leg = (front ? 'F' : 'B') + (v.x > 0 ? 'L' : 'R');
			if (y > 0.42) { a = leg; c = front ? 'chest' : 'hips'; w = 1 - Math.max(0, (y - 0.44) / 0.06); }
			else if (y > 0.36) { a = leg; c = leg + '2'; w = (y - 0.36) / 0.06; }
			else if (y > 0.18) { a = leg + '2'; c = a; }
			else if (y > 0.14) { a = leg + '2'; c = leg + '3'; w = (y - 0.14) / 0.04; }
			else { a = leg + '3'; c = a; }
		} else if (t < 0.08 && y > 0.3) {
			a = t < 0.03 ? 'tail2' : 'tail'; c = a;
		} else {
			for (let k = spine.length - 1; k >= 0; k--) if (t >= spine[k][1]) {
				a = spine[k][0];
				const prev = spine[k - 1];
				// blend into the previous bone across 5 % behind this border
				if (prev && t < spine[k][1] + 0.05) { c = prev[0]; w = (t - spine[k][1]) / 0.05 * 0.5 + 0.5; } else c = a;
				break;
			}
			if (a === 'head' && y < 0.72 && t > 0.92) { a = 'jaw'; c = 'jaw'; w = 1; }
		}
		w = Math.max(0, Math.min(1, w));
		skinIndex.push(index[a], index[c], 0, 0);
		skinWeight.push(w, 1 - w, 0, 0);
	}
	geo.setAttribute('skinIndex', new THREE.Uint16BufferAttribute(skinIndex, 4));
	geo.setAttribute('skinWeight', new THREE.Float32BufferAttribute(skinWeight, 4));
	const map = src.material.map ?? null;
	if (map) map.colorSpace = THREE.SRGBColorSpace;
	const mesh = new THREE.SkinnedMesh(geo, new THREE.MeshLambertMaterial({ map, color: 0xffffff }));
	mesh.castShadow = mesh.receiveShadow = true;
	mesh.frustumCulled = false;
	const root = new THREE.Group();
	root.add(bones.hips);
	root.add(mesh);
	root.updateMatrixWorld(true);
	mesh.bind(new THREE.Skeleton(order.map((n) => bones[n])));
	for (const n of order) bones[n].userData.restPos = bones[n].position.clone();
	// the rune glyph on the forehead (the owner's mountain tiger, 2026-10-09): a vertical stroke, a crescent on top,
	// two cross bars – black, so it reads under the hard light; it rides on the head bone
	const glyph = new THREE.Group();
	const ink = new THREE.MeshBasicMaterial({ color: 0x000000 });
	const u = Hy * 0.018;
	const part = (geo, x, y, rz = 0) => { const m = new THREE.Mesh(geo, ink); m.position.set(x, y, 0); m.rotation.z = rz; glyph.add(m); };
	part(new THREE.BoxGeometry(u * 0.9, u * 9, u * 0.3), 0, 0);
	part(new THREE.BoxGeometry(u * 4.2, u * 0.9, u * 0.3), 0, u * 1.2);
	part(new THREE.BoxGeometry(u * 4.2, u * 0.9, u * 0.3), 0, -u * 1.2);
	part(new THREE.TorusGeometry(u * 1.8, u * 0.45, 6, 16, Math.PI), 0, u * 4.2, Math.PI);
	part(new THREE.TorusGeometry(u * 1.4, u * 0.4, 6, 16, Math.PI), 0, -u * 4.4);
	// on the brow: the frontmost surface point of the head's middle at brow height (from the vertices), the glyph laid
	// on the surface there (tipped back by the local slope), a hair in front of it
	const half = (b.max.x - b.min.x) * 0.06;
	let top = null, low = null;
	for (let i = 0; i < pos.count; i++) {
		v.fromBufferAttribute(pos, i);
		if (Math.abs(v.x) > half || (v.z - b.min.z) / Lz < 0.8) continue;
		const y = v.y / Hy;
		if (y > 0.8 && y < 0.95 && (!top || v.z > top.z)) top = v.clone();
		if (y > 0.68 && y < 0.8 && (!low || v.z > low.z)) low = v.clone();
	}
	const brow = top ?? new THREE.Vector3(0, Hy * 0.86, zAt(0.955));
	const slope = top && low ? Math.atan2(top.z - low.z, top.y - low.y) : 0.55; // how far the forehead leans back
	glyph.position.copy(brow.clone().add(new THREE.Vector3(0, 0, u * 0.25)).sub(bones.head.userData.world));
	glyph.rotation.x = -slope;
	glyph.traverse((o) => { if (o.isMesh) { o.castShadow = false; o.userData.glyph = true; } });
	bones.head.add(glyph);
	return { kind: 'tiger', root, joints: { ...bones, tailSegs: [bones.tail, bones.tail2], rigged: true }, mesh };
}
