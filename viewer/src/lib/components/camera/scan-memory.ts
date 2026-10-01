// "I know that one!" – the object camera remembers what it granted (owner 2026-10-02: scanning the same cucumber
// again is cheating, and the reader is told so). Only a picture fingerprint (MobileNet embedding: numbers, not an
// image) per grant, only on this device (localStorage); no photos and no location (owner 2026-10-02: privacy). The hard
// rules are the server's: the camera cooldown, the daily limit (`per_day`) and the collection limit (`max`).
//
// Owner phone test 2026-10-02 (similarity to the first frame): the cucumber lying still 0.89–0.95, turned on the
// same spot 0.86–0.95, moved elsewhere 0.74–0.78 – the fingerprint knows "this cucumber in this place".
import { similarity } from './object-detector.ts';

const KEY = 'scutoons.scan-memory';
const KEEP = 20;
const MAX_AGE_MS = 7 * 86400_000;
/** this similar = the same thing again */
export const SAME_LOOK = 0.88;

interface ScanMemory {
	item: string;
	/** ms */
	at: number;
	/** fingerprint, int8-quantised, base64 */
	fp: string;
}

function encode(fp: Float32Array): string {
	let max = 0;
	for (const v of fp) max = Math.max(max, Math.abs(v));
	const scale = max ? 127 / max : 1;
	const bytes = Int8Array.from(fp, (v) => Math.round(v * scale));
	let binary = '';
	for (const b of new Uint8Array(bytes.buffer)) binary += String.fromCharCode(b);
	return btoa(binary);
}

function decode(text: string): Float32Array {
	const binary = atob(text);
	const bytes = Int8Array.from(binary, (c) => (c.charCodeAt(0) << 24) >> 24);
	let norm = 0;
	for (const b of bytes) norm += b * b;
	norm = Math.sqrt(norm) || 1;
	return Float32Array.from(bytes, (b) => b / norm);
}

function load(): ScanMemory[] {
	try {
		const list = JSON.parse(localStorage.getItem(KEY) ?? '[]');
		return Array.isArray(list) ? list.filter((m) => typeof m?.fp === 'string' && Date.now() - m.at < MAX_AGE_MS) : [];
	} catch {
		return [];
	}
}

/** Does this look like something of `item` this device scanned before? */
export function findKnown(item: string, fp: Float32Array): boolean {
	return load().some((m) => m.item === item && similarity(decode(m.fp), fp) >= SAME_LOOK);
}

export function remember(item: string, fp: Float32Array) {
	const list = [...load(), { item, at: Date.now(), fp: encode(fp) }].slice(-KEEP);
	try {
		localStorage.setItem(KEY, JSON.stringify(list));
	} catch {
		// storage full / blocked: the camera just forgets
	}
}
