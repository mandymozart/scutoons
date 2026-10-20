// One shot → one frame (#124, moved from tools/storyboard-render/core.js renderShot): the scene through the look, a
// panorama (loop360: the figures on a circle around the camera, slices all around) or the little planet (polar), the
// texts and the storyboard marks on top. The 3D view renders live (frames are not stored, STUDIO-IN-CREATOR §2.4); this
// is the still for an export / a generator's reference.
import * as THREE from 'three';
import type { ShotSpec } from '@scutoons/content/storyboard';
import { D } from '../../editor/storyboard-3d-place.ts';
import { pose } from './poses.ts';
import { buildScene, type BuildOptions } from './scene.ts';
import { sinPass } from './sin-pass.ts';
import { drawDialog, drawMarks, figureBoxes, loadFonts, seedOf, type FigureBox } from './texts.ts';

export interface RenderOptions extends BuildOptions {
	renderer?: THREE.WebGLRenderer;
	canvas?: HTMLCanvasElement;
	/** the storyboard marks (id, format, air) – default on */
	marks?: boolean;
	/** the texts – default on */
	bubbles?: boolean;
	/** also a copy without texts and marks */
	clean?: boolean;
}

const canvas2d = (w: number, h: number) => {
	const c = document.createElement('canvas');
	c.width = w;
	c.height = h;
	return c;
};

/** one shot → { canvas, clean, figures } */
export async function renderShot(shot: ShotSpec, opts: RenderOptions = {}): Promise<{ canvas: HTMLCanvasElement; clean: HTMLCanvasElement | null; figures: FigureBox[] }> {
	const W = shot.w,
		H = shot.h;
	const canvas = opts.canvas ?? document.createElement('canvas');
	canvas.width = W;
	canvas.height = H;
	const out2d = canvas.getContext('2d')!;
	const own = !opts.renderer;
	const renderer = opts.renderer ?? new THREE.WebGLRenderer({ antialias: false, preserveDrawingBuffer: true });
	const pass = sinPass(renderer);
	const built = await buildScene(shot, opts);
	const { scene, camera, figs, camY, K, skyWhite } = built;
	const look = { sky: skyWhite ? 1 : 0, accent: shot.accent ?? '#c00000', K, seed: seedOf(shot.id) };

	const panorama = (pw: number, ph: number) => {
		const n = Math.max(4, Math.round(pw / (390 * K)));
		const sliceW = Math.round(pw / n);
		const pano = canvas2d(sliceW * n, ph);
		const g = pano.getContext('2d')!;
		const cam = new THREE.PerspectiveCamera(1, sliceW / ph, 0.05, 600);
		const hfov = 360 / n;
		cam.fov = (2 * Math.atan(Math.tan((hfov * D) / 2) / cam.aspect)) / D;
		cam.updateProjectionMatrix();
		cam.position.set(0, camY, 0);
		cam.rotation.order = 'YXZ';
		for (let i = 0; i < n; i++) {
			cam.rotation.set(0, -i * hfov * D, 0);
			cam.updateMatrixWorld();
			g.drawImage(pass.render(scene, cam, sliceW, ph, look), i * sliceW, 0);
		}
		return pano;
	};

	let image: HTMLCanvasElement | null = null;
	if (shot.format === 'loop360') {
		for (const [i, m] of figs.entries()) {
			const fig = shot.figures[i];
			const bearing = -(fig.x ?? 0.5) * 2 * Math.PI;
			const r = 4 + (1 - (fig.h ?? 0.3)) * 8;
			m.root.position.set(Math.sin(-bearing) * -r, 0, -Math.cos(bearing) * r);
			m.root.rotation.y = Math.atan2(-m.root.position.x, -m.root.position.z) + Math.PI + (fig.turn ?? 0) * D;
			m.root.scale.setScalar((((fig.h ?? 0.3) * 2.2 * r) / 4));
			pose(m, fig.pose ?? 'stand');
		}
		image = panorama(W, H);
	} else if (shot.projection === 'polar') {
		const pano = panorama(1560 * K, 520 * K);
		const src = pano.getContext('2d')!.getImageData(0, 0, pano.width, pano.height).data;
		const out = out2d.createImageData(W, H);
		const cx = W / 2,
			cy = H * 0.62,
			R = Math.min(W, H) * 0.62;
		for (let y = 0; y < H; y++)
			for (let x = 0; x < W; x++) {
				const dx = x - cx,
					dy = y - cy,
					r = Math.hypot(dx, dy) / R;
				const a = (Math.atan2(dx, dy) / (2 * Math.PI) + 1) % 1;
				const sy = Math.min(pano.height - 1, Math.max(0, Math.round((1 - Math.min(1, r)) * (pano.height - 1))));
				const sx = Math.min(pano.width - 1, Math.round(a * (pano.width - 1)));
				const si = (sy * pano.width + sx) * 4,
					oi = (y * W + x) * 4;
				out.data[oi] = src[si];
				out.data[oi + 1] = src[si + 1];
				out.data[oi + 2] = src[si + 2];
				out.data[oi + 3] = 255;
			}
		out2d.putImageData(out, 0, 0);
	} else {
		image = pass.render(scene, camera, W, H, look);
	}
	if (image) out2d.drawImage(image, 0, 0, W, H);
	const figures = figureBoxes(figs, camera, W, H, K);
	let clean: HTMLCanvasElement | null = null;
	if (opts.clean) {
		clean = canvas2d(W, H);
		clean.getContext('2d')!.drawImage(canvas, 0, 0);
	}
	if (opts.bubbles !== false && (shot.dialog ?? []).length) await loadFonts();
	if (opts.bubbles !== false) drawDialog(out2d, shot, figures.map((f) => ({ ...f, box: { x0: f.box.x0 * K, y0: f.box.y0 * K, x1: f.box.x1 * K, y1: f.box.y1 * K } })), W, H, K);
	if (opts.marks !== false) drawMarks(out2d, shot, W, H, K);
	built.dispose();
	pass.dispose();
	if (own) renderer.dispose();
	return { canvas, clean, figures };
}
