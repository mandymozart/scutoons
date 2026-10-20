// The 3D view's stage (#124; the studio's Viewport.svelte): one WebGL renderer + the look for the page's life, the
// current shot's scene (rebuilt when the shot changes – the old one freed first, a newer change waits for the build
// under way), a pivot per figure (the gizmo and the gestures move / turn the pivot, the figure hangs in it), the frame
// fitted into the stage, draw (the look, the gizmo's helper, then the painters – the texts). The camera controls, the
// figure gestures, the gizmo and the texts are their own components (components/storyboard/view-3d-*.svelte).
import * as THREE from 'three';
import type { OrbitControls } from 'three/addons/controls/OrbitControls.js';
import type { TransformControls } from 'three/addons/controls/TransformControls.js';
import type { Shot, ShotSpec } from '@scutoons/content/storyboard';
import { cameraKey, fitFrame, handCamera, rem } from '$lib/editor/storyboard-3d.ts';
import { applyPlace, screenPlace, yawOf, type ScreenPlace } from '$lib/editor/storyboard-3d-place.ts';
import { buildScene, seedOf, sinPass, type BuiltScene, type LookParams, type ModelUrls, type SinPass } from './render/index.ts';

/** a build never blocks the next change: after 15 s it is given up (the latest state is built next) */
const BUILD_TIMEOUT = 15000;

export class Stage3d {
	readonly renderer: THREE.WebGLRenderer;
	private readonly pass: SinPass;
	/** the gizmo's helper (drawn over the look, never through it) */
	readonly helperScene = new THREE.Scene();
	/** the current shot's scene (null while the first one builds) */
	built = $state.raw<BuiltScene | null>(null);
	spec: ShotSpec | null = null;
	/** a pivot per figure, the figure's anchor (its feet) in the pivot's space, the yaw buildScene gave it */
	pivots: THREE.Group[] = [];
	anchors: THREE.Vector3[] = [];
	private yaws: number[] = [];
	size = $state({ w: 0, h: 0, scale: 1 });
	busy = $state(false);
	error = $state('');
	/** the 2D layers drawn after the scene (the texts) */
	readonly painters = new Set<() => void>();
	/** the camera controls and the gizmo of the current scene (their components set them) */
	controls: OrbitControls | null = null;
	gizmo: TransformControls | null = null;
	private building = false;
	private next: ShotSpec | null = null;
	private look: LookParams = {};
	private disposed = false;

	constructor(private readonly models: ModelUrls) {
		this.renderer = new THREE.WebGLRenderer({ antialias: false, preserveDrawingBuffer: true });
		this.renderer.domElement.classList.add('gl');
		this.pass = sinPass(this.renderer);
	}

	/** builds the scene of `spec` (a build under way: this one comes next, the ones between are skipped) */
	async build(spec: ShotSpec): Promise<void> {
		if (this.building) {
			this.next = spec;
			return;
		}
		this.building = true;
		this.busy = true;
		try {
			this.spec = spec;
			this.free();
			let timer: ReturnType<typeof setTimeout> | undefined;
			const built = await Promise.race([
				buildScene(spec, { shadowSize: 2048, models: this.models }),
				new Promise<never>((_, no) => (timer = setTimeout(() => no(new Error('the scene took longer than 15 s – try again')), BUILD_TIMEOUT)))
			]).finally(() => clearTimeout(timer));
			if (this.disposed) built.dispose();
			else this.adopt(built);
			this.error = '';
		} catch (e) {
			this.error = e instanceof Error ? e.message : String(e);
		}
		this.building = false;
		this.busy = false;
		const next = this.next;
		this.next = null;
		if (next && !this.disposed) await this.build(next);
	}

	/** a new scene: a pivot per figure at the centre of what the frame shows of it (a close-up's feet are below it) */
	private adopt(built: BuiltScene): void {
		const cam = built.camera;
		cam.updateMatrixWorld();
		this.pivots = [];
		this.anchors = [];
		this.yaws = [];
		for (const m of built.figs) {
			const c = new THREE.Box3().setFromObject(m.root).getCenter(new THREE.Vector3());
			const p = c.clone().project(cam);
			// the centre of its part inside the view, roughly: clamp the box centre into the frustum (the studio clamped
			// only y – a big close-up's centre beside the frame put the gizmo out of reach)
			if (Math.abs(p.y) > 1 || Math.abs(p.x) > 1) {
				p.x = Math.max(-0.8, Math.min(0.8, p.x));
				p.y = Math.max(-0.8, Math.min(0.8, p.y));
				c.copy(p.unproject(cam));
			}
			// the yaw buildScene gave it (read BEFORE attach: attach re-derives the Euler and flips a yaw past 90°)
			this.yaws.push(m.root.rotation.y);
			const pivot = new THREE.Group();
			pivot.position.copy(c);
			built.scene.add(pivot);
			pivot.attach(m.root);
			this.anchors.push(pivot.worldToLocal(m.place!.pos.clone())); // buildScene's anchor (the feet), in the pivot
			this.pivots.push(pivot);
		}
		this.look = { sky: built.skyWhite ? 1 : 0, accent: this.spec?.accent ?? '#c00000', K: 1, seed: seedOf(this.spec?.id ?? '') };
		this.built = built;
	}

	/** the anchor (feet) of figure i in the world now */
	anchorOf(i: number): THREE.Vector3 {
		this.pivots[i].updateMatrixWorld();
		return this.pivots[i].localToWorld(this.anchors[i].clone());
	}

	/** figure i's place as the storyboard stores it, measured through the current camera */
	placeOf(i: number): ScreenPlace | null {
		const b = this.built,
			s = this.spec,
			m = b?.figs[i];
		if (!b || !s || !m?.place) return null;
		b.scene.updateMatrixWorld(true);
		return screenPlace(b.camera, { W: s.w, H: s.h, K: b.K }, this.anchorOf(i), m.place, this.yaws[i] + yawOf(this.pivots[i].quaternion));
	}

	/** the write of figure i's place (one gesture), null when it has none */
	figureEdit(i: number): ((shot: Shot) => void) | null {
		const p = this.placeOf(i);
		return p ? (shot) => void (shot.figures?.[i] && applyPlace(shot.figures[i], p)) : null;
	}

	/**
	 * The write of the camera as the shot's hand camera (position, target – the orbit's centre –, lens): the camera is
	 * first put where the rounded numbers rebuild it, then every figure is measured through it – they stay where they
	 * are in the world, their screen places follow the camera. null: the camera hasn't moved since `since` (a click ends
	 * an orbit too) or it isn't a perspective one (an isometric shot keeps its preset camera).
	 */
	cameraEdit(target: THREE.Vector3, since: string): { key: string; change: (shot: Shot) => void } | null {
		const cam = this.built?.camera;
		if (!cam || !(cam as THREE.PerspectiveCamera).isPerspectiveCamera) return null;
		const key = cameraKey(cam.position.toArray(), target.toArray());
		if (key === since) return null;
		const camera = handCamera(cam.position.toArray(), target.toArray(), (cam as THREE.PerspectiveCamera).fov);
		cam.position.fromArray(camera.pos!);
		target.fromArray(camera.target!);
		cam.lookAt(target);
		cam.updateMatrixWorld();
		const places = this.built!.figs.map((_, i) => this.placeOf(i));
		return {
			key,
			change: (shot) => {
				shot.camera = camera;
				places.forEach((p, i) => p && shot.figures?.[i] && applyPlace(shot.figures[i], p));
			}
		};
	}

	/** the frame fitted into the stage (px of the stage), its canvas sized in rem */
	fit(stageW: number, stageH: number): void {
		if (!this.spec) return;
		this.size = fitFrame(stageW, stageH, this.spec.outW, this.spec.outH);
		const root = parseFloat(getComputedStyle(document.documentElement).fontSize);
		this.renderer.domElement.style.width = rem(this.size.w, root);
		this.renderer.domElement.style.height = rem(this.size.h, root);
	}

	draw(): void {
		const b = this.built;
		if (!b || !this.size.w || this.disposed) return;
		this.pass.render(b.scene, b.camera, this.size.w, this.size.h, this.look);
		this.renderer.autoClear = false;
		this.renderer.render(this.helperScene, b.camera);
		this.renderer.autoClear = true;
		for (const paint of this.painters) paint();
	}

	/** the figure under a pointer (index) or -1 */
	figureAt(e: { clientX: number; clientY: number }): number {
		const b = this.built;
		if (!b) return -1;
		// a skinned figure keeps the bounds it was first measured with (posed elsewhere, before it was placed / moved): a
		// ray then misses it everywhere – measured fresh for each pick
		for (const m of b.figs)
			m.root.traverse((o) => {
				const s = o as THREE.SkinnedMesh;
				if (s.isSkinnedMesh) {
					s.boundingBox = null as unknown as THREE.Box3;
					s.boundingSphere = null as unknown as THREE.Sphere;
				}
			});
		const ray = new THREE.Raycaster();
		ray.setFromCamera(this.ndcOf(e), b.camera);
		const hit = ray.intersectObjects(b.figs.map((m) => m.root), true).find((h) => h.object.userData.fig);
		return hit ? hit.object.userData.fig - 1 : -1;
	}

	/** where a pointer meets a plane (null: beyond 300 m) */
	onPlane(e: { clientX: number; clientY: number }, plane: THREE.Plane): THREE.Vector3 | null {
		const b = this.built;
		if (!b) return null;
		const ray = new THREE.Raycaster();
		ray.setFromCamera(this.ndcOf(e), b.camera);
		const p = ray.ray.intersectPlane(plane, new THREE.Vector3());
		return p && p.distanceTo(b.camera.position) < 300 ? p : null;
	}

	private ndcOf(e: { clientX: number; clientY: number }): THREE.Vector2 {
		const r = this.renderer.domElement.getBoundingClientRect();
		return new THREE.Vector2(((e.clientX - r.left) / r.width) * 2 - 1, -((e.clientY - r.top) / r.height) * 2 + 1);
	}

	/** the current scene off the GPU (geometries, materials, textures, shadow maps) */
	private free(): void {
		this.helperScene.clear();
		this.built?.dispose();
		this.built = null;
		this.pivots = [];
		this.anchors = [];
	}

	/** the page leaves: everything freed, the WebGL context released */
	dispose(): void {
		this.disposed = true;
		this.free();
		this.pass.dispose();
		this.renderer.dispose();
		this.renderer.forceContextLoss();
		this.renderer.domElement.remove();
	}
}
