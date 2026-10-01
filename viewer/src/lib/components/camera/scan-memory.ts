// "I know that one!" – the object camera remembers what it granted (owner 2026-10-02: scanning the same cucumber
// again is cheating, and the reader is told so). Only on this device (localStorage): a picture fingerprint (MobileNet
// embedding), a small photo and, when the reader allows it, the rough place. Nothing of it goes to the server; the
// hard rule is the server's daily limit (`per_day`, 429 daily_limit).
//
// Owner phone test 2026-10-02 (similarity to the first frame): the cucumber lying still 0.89–0.95, turned on the
// same spot 0.86–0.95, moved elsewhere 0.74–0.78 – the fingerprint knows "this cucumber in this place".
import { similarity } from './object-detector.ts';

const KEY = 'scutoons.scan-memory';
const KEEP = 20;
const MAX_AGE_MS = 7 * 86400_000;
/** this similar = the same thing (anywhere) */
export const SAME_LOOK = 0.88;
/** this similar AND at the same place = the same thing */
export const SAME_LOOK_HERE = 0.8;
/** metres (plus the readings' accuracy) that still count as the same place */
export const SAME_PLACE_M = 30;

export interface Place {
	lat: number;
	lon: number;
	accuracy: number;
}

export interface ScanMemory {
	item: string;
	/** ms */
	at: number;
	/** fingerprint, int8-quantised, base64 */
	fp: string;
	/** small JPEG data URL of what was scanned */
	photo: string;
	place?: Place;
}

function encode(fp: Float32Array): string {
	const bytes = new Int8Array(fp.length);
	// unit-length vectors: components are small; scale so the largest fits in int8
	let max = 0;
	for (const v of fp) max = Math.max(max, Math.abs(v));
	const scale = max ? 127 / max : 1;
	for (let i = 0; i < fp.length; i++) bytes[i] = Math.round(fp[i] * scale);
	let binary = '';
	for (const b of new Uint8Array(bytes.buffer)) binary += String.fromCharCode(b);
	return btoa(binary);
}

function decode(text: string): Float32Array {
	const binary = atob(text);
	const bytes = new Int8Array(binary.length);
	for (let i = 0; i < binary.length; i++) bytes[i] = (binary.charCodeAt(i) << 24) >> 24;
	let norm = 0;
	for (const b of bytes) norm += b * b;
	norm = Math.sqrt(norm) || 1;
	return Float32Array.from(bytes, (b) => b / norm);
}

function load(): ScanMemory[] {
	try {
		const list = JSON.parse(localStorage.getItem(KEY) ?? '[]');
		return Array.isArray(list) ? list.filter((m) => Date.now() - m.at < MAX_AGE_MS) : [];
	} catch {
		return [];
	}
}

/** metres between two places (haversine) */
export function distance(a: Place, b: Place): number {
	const rad = Math.PI / 180;
	const dLat = (b.lat - a.lat) * rad;
	const dLon = (b.lon - a.lon) * rad;
	const h = Math.sin(dLat / 2) ** 2 + Math.cos(a.lat * rad) * Math.cos(b.lat * rad) * Math.sin(dLon / 2) ** 2;
	return 2 * 6371000 * Math.asin(Math.sqrt(h));
}

/** The earlier scan of `item` this one looks like (the most similar), or null. */
export function findKnown(item: string, fp: Float32Array, place?: Place | null): (ScanMemory & { similarity: number }) | null {
	let best: (ScanMemory & { similarity: number }) | null = null;
	for (const m of load()) {
		if (m.item !== item) continue;
		const sim = similarity(decode(m.fp), fp);
		const here = !!place && !!m.place && distance(place, m.place) <= SAME_PLACE_M + place.accuracy + m.place.accuracy;
		if ((sim >= SAME_LOOK || (here && sim >= SAME_LOOK_HERE)) && (!best || sim > best.similarity)) best = { ...m, similarity: sim };
	}
	return best;
}

export function remember(item: string, fp: Float32Array, photo: string, place?: Place | null) {
	const list = [...load(), { item, at: Date.now(), fp: encode(fp), photo, ...(place ? { place } : {}) }].slice(-KEEP);
	try {
		localStorage.setItem(KEY, JSON.stringify(list));
	} catch {
		// storage full / blocked: the camera just forgets
	}
}

/** A small JPEG of a crop canvas (for "earlier / now"). */
export function snapshot(source: HTMLCanvasElement, size = 160): string {
	const canvas = document.createElement('canvas');
	canvas.width = size;
	canvas.height = size;
	canvas.getContext('2d')?.drawImage(source, 0, 0, size, size);
	return canvas.toDataURL('image/jpeg', 0.7);
}

/** The reader's rough place, if they allow it (asked once per camera session); null otherwise. */
export function locate(): Promise<Place | null> {
	if (typeof navigator === 'undefined' || !navigator.geolocation) return Promise.resolve(null);
	return new Promise((done) =>
		navigator.geolocation.getCurrentPosition(
			(p) => done({ lat: p.coords.latitude, lon: p.coords.longitude, accuracy: Math.min(p.coords.accuracy, 200) }),
			() => done(null),
			{ enableHighAccuracy: false, timeout: 8000, maximumAge: 60000 }
		)
	);
}
