// The 3D place's drawing (three.js on a canvas): a model's bytes on a floor grid, an orbit camera (drag = orbit,
// right-drag = pan, wheel = dolly), its skeleton and joints over it, its spots as marks, and a clip playing. What the
// file is (bones, clips, spots) is the core's (core-model.ts); this only draws and plays it back.
import type * as Three from 'three';
import type { GLTF } from 'three/addons/loaders/GLTFLoader.js';
import type { OrbitControls as Orbit } from 'three/addons/controls/OrbitControls.js';

type Lib = { THREE: typeof Three; GLTFLoader: typeof import('three/addons/loaders/GLTFLoader.js').GLTFLoader; OrbitControls: typeof Orbit };
let lib: Promise<Lib> | null = null;
const three = (): Promise<Lib> =>
	(lib ??= Promise.all([import('three'), import('three/addons/loaders/GLTFLoader.js'), import('three/addons/controls/OrbitControls.js')]).then(([THREE, g, o]) => ({ THREE, GLTFLoader: g.GLTFLoader, OrbitControls: o.OrbitControls })));

export interface PlaceColours {
	/** the skeleton and the joints */
	bone: string;
	/** the spots */
	spot: string;
	/** the floor's grid */
	grid: string;
}

export interface ModelPlace {
	/** shows the skeleton and the joints, or hides them */
	skeleton(on: boolean): void;
	/** plays clip `index` from `time` (null: the rest pose); `playing` false holds it at `time` */
	clip(index: number | null, playing: boolean, time: number): void;
	dispose(): void;
}

function disposeTree(obj: Three.Object3D) {
	obj.traverse((o) => {
		const mesh = o as Three.Mesh;
		mesh.geometry?.dispose();
		for (const m of ([] as Three.Material[]).concat(mesh.material ?? [])) {
			for (const v of Object.values(m)) if ((v as Three.Texture | null)?.isTexture) (v as Three.Texture).dispose();
			m.dispose();
		}
	});
}

/**
 * Draws the model in `bytes` on `canvas` until disposed; `ontime(seconds)` follows a playing clip. `spots` are the
 * core's spots at rest (model space).
 */
export async function mountModelPlace(
	canvas: HTMLCanvasElement,
	bytes: ArrayBuffer,
	spots: readonly { at: readonly number[] }[],
	colours: PlaceColours,
	ontime: (seconds: number) => void
): Promise<ModelPlace> {
	const { THREE, GLTFLoader, OrbitControls } = await three();
	const gltf: GLTF = await new GLTFLoader().parseAsync(bytes.slice(0), '');
	const renderer = new THREE.WebGLRenderer({ canvas, antialias: true, alpha: true });
	renderer.setPixelRatio(Math.min(2, window.devicePixelRatio || 1));
	const scene = new THREE.Scene();
	scene.add(new THREE.HemisphereLight(0xffffff, 0x445544, 2.2));
	const key = new THREE.DirectionalLight(0xffffff, 2);
	key.position.set(2, 4, 3);
	scene.add(key);

	const root = gltf.scene;
	scene.add(root);
	root.updateMatrixWorld(true);
	const box = new THREE.Box3().setFromObject(root);
	const size = box.getSize(new THREE.Vector3());
	const centre = box.getCenter(new THREE.Vector3());
	const height = Math.max(size.x, size.y, size.z, 1e-3);

	const grid = new THREE.GridHelper(height * 6, 24, colours.grid, colours.grid);
	grid.position.y = box.min.y;
	(grid.material as Three.Material).transparent = true;
	(grid.material as Three.Material).opacity = 0.35;
	scene.add(grid);

	// the skeleton: the bones as lines, a joint as a dot that rides on its bone
	const overlay: Three.Object3D[] = [];
	const front = (m: Three.Material) => ((m.depthTest = false), (m.transparent = true), m);
	const helper = new THREE.SkeletonHelper(root);
	(helper.material as Three.LineBasicMaterial).color.set(colours.bone);
	front(helper.material as Three.Material);
	helper.renderOrder = 10;
	scene.add(helper);
	overlay.push(helper);
	const dot = new THREE.SphereGeometry(height * 0.008, 8, 6);
	const dotMaterial = front(new THREE.MeshBasicMaterial({ color: colours.bone }));
	root.traverse((o) => {
		if (!(o as Three.Bone).isBone) return;
		const d = new THREE.Mesh(dot, dotMaterial);
		d.renderOrder = 11;
		// a bone's dot keeps its size whatever the bone's scale
		const s = o.getWorldScale(new THREE.Vector3());
		d.scale.set(1 / (s.x || 1), 1 / (s.y || 1), 1 / (s.z || 1));
		o.add(d);
		overlay.push(d);
	});
	const spotMaterial = front(new THREE.MeshBasicMaterial({ color: colours.spot, wireframe: true }));
	const spotShape = new THREE.OctahedronGeometry(height * 0.02);
	for (const s of spots) {
		const mark = new THREE.Mesh(spotShape, spotMaterial);
		mark.position.set(s.at[0] ?? 0, s.at[1] ?? 0, s.at[2] ?? 0);
		mark.renderOrder = 12;
		root.add(mark);
	}

	const camera = new THREE.PerspectiveCamera(30, 1, height * 0.01, height * 50);
	camera.position.set(centre.x + height * 1.1, centre.y + height * 0.35, centre.z + height * 2.6);
	const controls = new OrbitControls(camera, canvas);
	controls.target.copy(centre);
	controls.enableDamping = true;
	controls.update();

	const mixer = new THREE.AnimationMixer(root);
	let action: Three.AnimationAction | null = null;
	let running = false;

	const resize = () => {
		const w = canvas.clientWidth || 1;
		const h = canvas.clientHeight || 1;
		renderer.setSize(w, h, false);
		camera.aspect = w / h;
		camera.updateProjectionMatrix();
	};
	const ro = new ResizeObserver(resize);
	ro.observe(canvas);
	resize();

	const clock = new THREE.Clock();
	let raf = 0;
	let stopped = false;
	const tick = () => {
		if (stopped) return;
		const dt = clock.getDelta();
		if (action && running) {
			mixer.update(dt);
			ontime(action.time);
		}
		controls.update();
		renderer.render(scene, camera);
		raf = requestAnimationFrame(tick);
	};
	tick();

	return {
		skeleton(on) {
			for (const o of overlay) o.visible = on;
		},
		clip(index, playing, time) {
			const want = index === null ? null : gltf.animations[index];
			if (!want) {
				mixer.stopAllAction();
				action = null;
				running = false;
				return;
			}
			if (action?.getClip() !== want) {
				mixer.stopAllAction();
				action = mixer.clipAction(want);
				action.play();
			}
			action.paused = false;
			action.time = time;
			mixer.update(0);
			running = playing;
		},
		dispose() {
			stopped = true;
			cancelAnimationFrame(raf);
			ro.disconnect();
			controls.dispose();
			mixer.stopAllAction();
			disposeTree(scene);
			renderer.dispose();
		}
	};
}
