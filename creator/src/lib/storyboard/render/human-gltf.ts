// Cat Pope from the owner's model (#124, moved from tools/storyboard-render/human-gltf.js; static/storyboard/models/
// catpope.glb, 2026-10-10: "got a better 3d from cat pope regular"): a skinned mesh whose skeleton has the figurines'
// joint names (pelvis · spine · chest · neck · head · robe · shL/R · elL/R · wrL/R · hipL/R · knL/R · anL/R). The file
// stores every bone at the origin – the joints live in the skin's inverse bind matrices – so the skeleton is rebuilt
// from them here (bone world = inverse(bind)), the rest pose kept, and the same pose table as the procedural figurine
// drives it (poses.ts) as rotations ON TOP of each joint's rest – the arms are bound in an A-pose (≈ 22° out).
import * as THREE from 'three';
import type { GLTF } from 'three/addons/loaders/GLTFLoader.js';
import type { Figurine, Joints } from './figures.ts';

/** the joints a model must have to be posed as the figurine (a persona model without them falls back) */
export const HUMAN_JOINTS = ['pelvis', 'spine', 'chest', 'neck', 'head'];

/**
 * The model without the cat ears (owner 2026-10-10: PLF in the chapters is either the human priest – no cat features –
 * or the sphynx cat; "als 3d reicht einfach cat pope ohne ohren"): every point of the upper head that lies outside a
 * round skull (bind pose: centre 1.65 m up, radius 10.5 cm) is pulled onto it – the ears fold into the skull and the
 * surface stays closed (deleting their triangles left holes to look through).
 */
function withoutEars(geo: THREE.BufferGeometry): THREE.BufferGeometry {
	const p = geo.attributes.position;
	const c = new THREE.Vector3(0, 1.65, 0.02),
		r = 0.105,
		v = new THREE.Vector3();
	for (let i = 0; i < p.count; i++) {
		v.fromBufferAttribute(p, i);
		if (v.y < 1.62) continue; // the face and the neck stay
		const d = v.distanceTo(c);
		if (d > r) {
			v.sub(c).multiplyScalar(r / d).add(c);
			p.setXYZ(i, v.x, v.y, v.z);
		}
	}
	p.needsUpdate = true;
	geo.computeVertexNormals();
	return geo;
}

/** the model's first skinned mesh when its skeleton has the figurine's joints, else null */
export function humanRigOf(gltf: GLTF): THREE.SkinnedMesh | null {
	let mesh: THREE.SkinnedMesh | null = null;
	gltf.scene.traverse((o) => {
		if ((o as THREE.SkinnedMesh).isSkinnedMesh && !mesh) mesh = o as THREE.SkinnedMesh;
	});
	const found = mesh as THREE.SkinnedMesh | null;
	const names = new Set(found?.skeleton.bones.map((b) => b.name) ?? []);
	return found && HUMAN_JOINTS.every((n) => names.has(n)) ? found : null;
}

export function gltfHuman(gltf: GLTF, { ears = true } = {}): Figurine {
	const mesh = humanRigOf(gltf);
	if (!mesh) throw new Error('catpope.glb: no skinned mesh with the figurine joints');
	const root = new THREE.Group();
	const skel = mesh.skeleton;
	// the bones' world transforms in the bind pose = the inverses of the bind matrices
	const world = new Map(skel.bones.map((b, i) => [b, skel.boneInverses[i].clone().invert()]));
	// a fresh hierarchy: each bone's local = inverse(parent world) × world
	const fresh = new Map<THREE.Object3D, THREE.Bone>();
	for (const b of skel.bones) {
		const n = new THREE.Bone();
		n.name = b.name;
		fresh.set(b, n);
	}
	for (const b of skel.bones) {
		const n = fresh.get(b)!;
		const parent = b.parent ? fresh.get(b.parent) : undefined;
		const w = world.get(b)!;
		const local = parent ? world.get(b.parent as THREE.Bone)!.clone().invert().multiply(w) : w.clone();
		local.decompose(n.position, n.quaternion, n.scale);
		n.scale.set(1, 1, 1);
		(parent ?? root).add(n);
	}
	const bones = skel.bones.map((b) => fresh.get(b)!);
	const geo = mesh.geometry.clone();
	if (!ears) withoutEars(geo);
	geo.deleteAttribute('color'); // the look is lit white: the hard light, not the paint, makes black and white
	const skinned = new THREE.SkinnedMesh(geo, new THREE.MeshLambertMaterial({ color: 0xffffff }));
	skinned.castShadow = skinned.receiveShadow = true;
	skinned.frustumCulled = false;
	root.add(skinned);
	root.updateMatrixWorld(true);
	skinned.bind(new THREE.Skeleton(bones, skel.boneInverses.map((m) => m.clone())), new THREE.Matrix4());
	const J: Joints = {};
	for (const n of bones) {
		J[n.name] = n;
		n.userData.rest = { position: n.position.clone(), quaternion: n.quaternion.clone() };
	}
	return { kind: 'human', root, joints: { ...J, rigged: true, humanRig: true }, mesh: skinned };
}
