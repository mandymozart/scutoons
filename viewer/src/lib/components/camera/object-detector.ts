// On-device image classification for the `scan_object` skill (cucumber camera).
// TF.js + MobileNet are lazy-loaded only when a scan_object skill is used.

import { asset } from '$lib/utils/cdn.ts';

export interface Prediction {
	className: string;
	probability: number;
}

export interface Classifier {
	classify(source: HTMLVideoElement | HTMLImageElement | HTMLCanvasElement, topk?: number): Promise<Prediction[]>;
	/** the model's picture fingerprint (MobileNet embedding, unit length) – "is this the same cucumber again?" */
	embed(source: HTMLVideoElement | HTMLImageElement | HTMLCanvasElement): Promise<Float32Array>;
}

/** MobileNet v2 (alpha 1.0, 224 px) on our CDN – see loadClassifier */
export const MOBILENET_MODEL = 'models/mobilenet_v2_100_224/model.json';

let classifierPromise: Promise<Classifier> | null = null;

export function loadClassifier(): Promise<Classifier> {
	// DEV only – the browser checks (scripts/check-testo.mjs) set `globalThis.__scutoonsFakePredictions` to drive the
	// object loop without downloading MobileNet; never in a production build
	if (import.meta.env.DEV && (globalThis as any).__scutoonsFakePredictions) {
		return Promise.resolve({
			classify: async () => ((globalThis as any).__scutoonsFakePredictions as Prediction[]) ?? [],
			embed: async () => new Float32Array((globalThis as any).__scutoonsFakeEmbedding ?? [Math.random(), Math.random(), Math.random()])
		});
	}
	classifierPromise ??= (async () => {
		const tf = await import('@tensorflow/tfjs');
		await tf.ready();
		const mobilenet = await import('@tensorflow-models/mobilenet');
		// served from our CDN (owner phone test 2026-10-02: the tfhub.dev → Kaggle bucket chain failed in Brave – "cucumber
		// detector could not be loaded" – and every load told Google the reader's IP). Files: cdn/root/models/ (14 MB,
		// MobileNet v2 1.0 224 from tfhub.dev/google/imagenet/mobilenet_v2_100_224/classification/2, Apache-2.0).
		const model = await mobilenet.load({ version: 2, alpha: 1.0, modelUrl: asset(MOBILENET_MODEL), inputRange: [0, 1] });
		const classifier: Classifier = {
			classify: (source, topk = 5) => model.classify(source, topk),
			embed: async (source) => {
				const tensor = tf.tidy(() => {
					const e = model.infer(source, true) as import('@tensorflow/tfjs').Tensor;
					return tf.div(e, tf.norm(e)).flatten();
				});
				const data = (await tensor.data()) as Float32Array;
				tensor.dispose();
				return data;
			}
		};
		return classifier;
	})().catch((error) => {
		classifierPromise = null;
		throw error;
	});
	return classifierPromise;
}

let cropCanvas: HTMLCanvasElement | null = null;

/**
 * MobileNet squashes its input to 224×224; a 16:9 camera frame gets distorted and the object is small.
 * Classify the central square instead (where the reader points the camera).
 */
export function centerCrop(source: HTMLVideoElement | HTMLImageElement, size = 224): HTMLCanvasElement {
	const w = source instanceof HTMLVideoElement ? source.videoWidth : source.naturalWidth;
	const h = source instanceof HTMLVideoElement ? source.videoHeight : source.naturalHeight;
	const side = Math.min(w, h);
	cropCanvas ??= document.createElement('canvas');
	cropCanvas.width = size;
	cropCanvas.height = size;
	const ctx = cropCanvas.getContext('2d');
	ctx?.drawImage(source, (w - side) / 2, (h - side) / 2, side, side, 0, 0, size, size);
	return cropCanvas;
}

/** A square of a video frame / image, in its own pixels. */
export interface Square {
	x: number;
	y: number;
	side: number;
}

const squareCanvases: HTMLCanvasElement[] = [];

/**
 * Copies one square of the source into a 224×224 canvas (`slot`: one canvas per crop of the same frame, so two crops
 * can be classified one after the other without overwriting each other).
 */
export function cropSquare(source: HTMLVideoElement, square: Square, slot = 0, size = 224): HTMLCanvasElement {
	const canvas = (squareCanvases[slot] ??= document.createElement('canvas'));
	canvas.width = size;
	canvas.height = size;
	canvas.getContext('2d')?.drawImage(source, square.x, square.y, square.side, square.side, 0, 0, size, size);
	return canvas;
}

/**
 * The square of the video frame under an on-screen element (the viewfinder), for a `<video>` shown with
 * `object-fit: cover`; `grow` widens it around its centre (clamped to the frame). Null before the video has a size.
 */
export function squareUnder(video: HTMLVideoElement, element: HTMLElement, grow = 1): Square | null {
	const vw = video.videoWidth;
	const vh = video.videoHeight;
	if (!vw || !vh) return null;
	const box = video.getBoundingClientRect();
	const target = element.getBoundingClientRect();
	const scale = Math.max(box.width / vw, box.height / vh);
	const offsetX = (box.width - vw * scale) / 2;
	const offsetY = (box.height - vh * scale) / 2;
	const side = Math.min(vw, vh, Math.min(target.width, target.height) / scale * grow);
	const cx = (target.left + target.width / 2 - box.left - offsetX) / scale;
	const cy = (target.top + target.height / 2 - box.top - offsetY) / scale;
	return {
		x: Math.min(Math.max(cx - side / 2, 0), vw - side),
		y: Math.min(Math.max(cy - side / 2, 0), vh - side),
		side
	};
}

/**
 * Best matching prediction for any of the wanted labels. ImageNet class names are comma lists
 * ("cucumber, cuke"), so a label matches if it is one of the class name's words/parts.
 */
export function matchLabels(predictions: Prediction[], labels: string[]): Prediction | null {
	const wanted = labels.map((l) => l.toLowerCase());
	let best: Prediction | null = null;
	for (const p of predictions) {
		const parts = p.className.toLowerCase().split(',').map((s) => s.trim());
		const hit = wanted.some((w) => parts.some((part) => part === w || part.split(/\s+/).includes(w)));
		if (hit && (!best || p.probability > best.probability)) best = p;
	}
	return best;
}

/** Cosine similarity of two unit-length fingerprints (1 = identical picture). */
export function similarity(a: ArrayLike<number>, b: ArrayLike<number>): number {
	let dot = 0;
	for (let i = 0; i < Math.min(a.length, b.length); i++) dot += a[i] * b[i];
	return dot;
}
