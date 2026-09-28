import { QR_HOSTS } from '$lib/config.ts';

/**
 * Extracts a SCUTOONS item code from decoded QR text: `https://qr.scutoons.com/?c=CODE` (or `?code=`).
 * Returns null for anything else ("not a SCUTOONS item").
 */
export function parseScutoonsCode(text: string): string | null {
	let url: URL;
	try {
		url = new URL(text.trim());
	} catch {
		return null;
	}
	if (!QR_HOSTS.includes(url.host.toLowerCase())) return null;
	const code = url.searchParams.get('c') || url.searchParams.get('code');
	return code && /^[A-Za-z0-9_-]{1,32}$/.test(code) ? code : null;
}

type JsQR = typeof import('jsqr').default;
let jsQRPromise: Promise<JsQR> | null = null;

export function loadJsQR(): Promise<JsQR> {
	if (!jsQRPromise) {
		jsQRPromise = import('jsqr').then((mod) => ((mod as any).default ?? mod) as JsQR);
	}
	return jsQRPromise;
}

/** Decode a QR code from the current frame of a video/image/canvas source. */
export function decodeFrame(
	jsQR: JsQR,
	source: HTMLVideoElement | HTMLImageElement,
	canvas: HTMLCanvasElement,
	maxSize = 640
): string | null {
	const w = source instanceof HTMLVideoElement ? source.videoWidth : source.naturalWidth;
	const h = source instanceof HTMLVideoElement ? source.videoHeight : source.naturalHeight;
	if (!w || !h) return null;
	const scale = Math.min(1, maxSize / Math.max(w, h));
	canvas.width = Math.round(w * scale);
	canvas.height = Math.round(h * scale);
	const ctx = canvas.getContext('2d', { willReadFrequently: true });
	if (!ctx) return null;
	// transparent sources (PNG/SVG stills) would otherwise decode as black on black
	ctx.fillStyle = '#ffffff';
	ctx.fillRect(0, 0, canvas.width, canvas.height);
	ctx.drawImage(source, 0, 0, canvas.width, canvas.height);
	const image = ctx.getImageData(0, 0, canvas.width, canvas.height);
	const result = jsQR(image.data, image.width, image.height, { inversionAttempts: 'attemptBoth' });
	return result?.data ?? null;
}
