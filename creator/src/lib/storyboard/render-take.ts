// A take rendered as a video (#126; owner 2026-10-19: "a function that renders all shots made this way as videos, which
// I then send to an AI as control frames"; STUDIO-IN-CREATOR §2.6: "the take is recorded in the browser (MediaRecorder,
// replacing ffmpeg)"): the shot's scene along the take's baked track (smoothed, trimmed, at fps) through the look (the Sin
// City pass – the lineart a video model reads as structure), without texts, into a WebM. MediaRecorder records in real
// time: each frame is drawn at its moment (the canvas' stream gets one frame per draw), so a render takes as long as the
// take is kept – its own renderer, never the 3D view's. Loaded only when a take is rendered (a dynamic import).
import * as THREE from 'three';
import type { ShotSpec } from '@scutoons/content/storyboard';
import { bake, type TakeTrack } from '$lib/editor/storyboard-handheld.ts';
import { buildScene, seedOf, sinPass, type ModelUrls } from './render/index.ts';

/** the video types tried, the first one the browser records */
export const WEBM_TYPES = ['video/webm;codecs=vp9', 'video/webm;codecs=vp8', 'video/webm'];

export interface RenderedTake {
	blob: Blob;
	/** the video's length (s): every frame 1 / fps */
	seconds: number;
	frames: number;
	width: number;
	height: number;
}

const even = (n: number) => Math.max(2, 2 * Math.round(n / 2));

export async function renderTake(spec: ShotSpec, take: TakeTrack, o: { fps?: number; models?: ModelUrls; progress?: (done: number, total: number) => void; signal?: AbortSignal } = {}): Promise<RenderedTake> {
	const fps = o.fps ?? 24;
	const frames = bake(take, fps);
	if (frames.length < 2) throw new Error('The take is too short to render.');
	const type = WEBM_TYPES.find((t) => MediaRecorder.isTypeSupported(t));
	if (!type) throw new Error('This browser cannot record a WebM video.');
	const width = even(spec.w),
		height = even(spec.h);
	const renderer = new THREE.WebGLRenderer({ antialias: false, preserveDrawingBuffer: true });
	const pass = sinPass(renderer);
	const built = await buildScene(spec, { shadowSize: 2048, models: o.models });
	const out = document.createElement('canvas');
	out.width = width;
	out.height = height;
	const g = out.getContext('2d')!;
	const stream = out.captureStream(0);
	const track = stream.getVideoTracks()[0] as MediaStreamTrack & { requestFrame?: () => void };
	const recorder = new MediaRecorder(stream, { mimeType: type, videoBitsPerSecond: 8_000_000 });
	const chunks: Blob[] = [];
	recorder.ondataavailable = (e) => e.data.size && chunks.push(e.data);
	const stopped = new Promise<void>((done) => (recorder.onstop = () => done()));
	const cam = built.camera as THREE.PerspectiveCamera;
	const look = { sky: built.skyWhite ? 1 : 0, accent: spec.accent ?? '#c00000', K: built.K ?? 1, seed: seedOf(spec.id) };
	const draw = (f: (typeof frames)[number]) => {
		cam.position.fromArray(f.position);
		cam.quaternion.fromArray(f.quaternion);
		if (cam.isPerspectiveCamera && f.fov) {
			cam.fov = f.fov;
			cam.updateProjectionMatrix();
		}
		cam.updateMatrixWorld();
		g.drawImage(pass.render(built.scene, cam, width, height, look), 0, 0, width, height);
	};
	const sleep = (ms: number) => new Promise((go) => setTimeout(go, Math.max(0, ms)));
	try {
		// the first frame once before recording: the look's shaders compile (a stall that would squeeze the clock)
		draw(frames[0]);
		recorder.start();
		await sleep(50);
		for (const [i, f] of frames.entries()) {
			if (o.signal?.aborted) throw new DOMException('Cancelled', 'AbortError');
			const t0 = performance.now();
			draw(f);
			track.requestFrame?.();
			o.progress?.(i + 1, frames.length);
			// every frame stays 1 / fps on the screen (the recorder stamps frames by the clock – never a catch-up burst)
			await sleep(1000 / fps - (performance.now() - t0));
		}
	} finally {
		if (recorder.state !== 'inactive') recorder.stop();
		await stopped;
		track.stop();
		built.dispose();
		pass.dispose();
		renderer.dispose();
		renderer.forceContextLoss();
	}
	return { blob: new Blob(chunks, { type: 'video/webm' }), seconds: frames.length / fps, frames: frames.length, width, height };
}
