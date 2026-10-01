// On-device image classification for the `scan_object` skill (cucumber camera).
// TF.js + MobileNet are lazy-loaded only when a scan_object skill is used.

export interface Prediction {
	className: string;
	probability: number;
}

export interface Classifier {
	classify(source: HTMLVideoElement | HTMLImageElement | HTMLCanvasElement): Promise<Prediction[]>;
}

let classifierPromise: Promise<Classifier> | null = null;

export function loadClassifier(): Promise<Classifier> {
	// DEV only – the browser checks (scripts/check-testo.mjs) set `globalThis.__scutoonsFakePredictions` to drive the
	// object loop without downloading MobileNet; never in a production build
	if (import.meta.env.DEV && (globalThis as any).__scutoonsFakePredictions) {
		return Promise.resolve({ classify: async () => ((globalThis as any).__scutoonsFakePredictions as Prediction[]) ?? [] });
	}
	classifierPromise ??= (async () => {
		const tf = await import('@tensorflow/tfjs');
		await tf.ready();
		const mobilenet = await import('@tensorflow-models/mobilenet');
		const model = await mobilenet.load({ version: 2, alpha: 1.0 });
		const classifier: Classifier = {
			classify: (source) => model.classify(source, 5)
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
