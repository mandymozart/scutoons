// Storyboard tooling – shared constants, the rhythm model, the safe-format envelope, reading entities.
// Plan + rules: agents/STORYBOARD.md (§1 formats, §6 rhythm, §9 build strategy), workflows/scenes/storyboard.md.
// Plain Node (no dependencies), so it runs in any checkout / worktree without an install.

import { execFileSync } from 'node:child_process';
import { existsSync, readFileSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

export const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), '../..');

/** Phone canvas: width W, one screen S (390 × 844), the delivery slice height of chapters 1 + 2 */
export const W = 390;
export const S = 844;
export const SLICE_H = 625;

/**
 * The rhythm model (STORYBOARD.md §6), calibrated on chapter 1 (owner 2026-10-09: careful reading 15–20 min; chapters
 * 1 and 2 about the same length, ch2 with more frames and less text). Seconds.
 */
export const RHYTHM = {
	scrollPerScreen: 1.2, // empty scrolling
	lookPerScreen: { careful: 10, skim: 3 }, // looking at the image; careful fitted to ch1 (see rhythm-report --fit)
	perBox: 0.8, // finding a text box / message
	wordsPerSecond: 4, // ~240 wpm
	perClick: 1, // a VN message waiting for a tap
	wall: { words: 35, boxes: 3 }, // per screen
	choicelessMessages: 6, // VN "talking heads"
	airBeat: 0.25, // × W between two beats
	airScene: [0.75, 1.25] // × W at a scene change
};

/** Length target for a chapter (owner 2026-10-09): about chapters 1 and 2 – ~70 screens, 15–20 min careful */
export const TARGET = { screens: [60, 80], minutes: [15, 20] };

/**
 * Safe formats (owner 2026-10-09: "a restriction for what we prompt and suggest"): what the generators are proven to
 * deliver today. Suggest inside, plan below the maximum; beyond = a warning, never an error. Update when a tool proves more.
 */
export const SAFE = {
	viewport: { aspect: '9:16', note: 'one screen' },
	vertical: { maxTall: 7, plan: 4, proved: 'generators (1:4 rare, 1:7 the limit)', date: '2026-10-09' }, // h/w
	loop360: { maxScreens: 13, plan: 8, proved: 'Luma Labs (widest without massive problems; usually less)', date: '2026-10-09' } // width in screens of 390
};

export const FORMATS = ['viewport', 'vertical', 'loop360', 'strip'];
export const PROJECTIONS = ['perspective', 'top-down', 'isometric', 'polar'];
export const MOVES = ['hold', 'tilt-down', 'tilt-up', 'fall', 'climb', 'travel', 'descend', 'reveal', 'look-around', 'pass'];
export const SIZES = ['ecu', 'cu', 'mcu', 'medium', 'full', 'wide', 'ews'];
export const ANGLES = ['eye', 'high', 'low', 'bird', 'worm', 'dutch'];
export const SOURCES = ['schematic', 'photo', 'unreal', 'direct', 'generated', 'art'];
export const STATUS = ['draft', 'layout-ok', 'refs-locked', 'generating', 'still-ok', 'in-chapter'];

/** "1:4" → { w: 1, h: 4 }; "9:16" → { w: 9, h: 16 }; "6:1" → { w: 6, h: 1 } */
export function parseAspect(a) {
	const m = /^(\d+(?:\.\d+)?):(\d+(?:\.\d+)?)$/.exec(String(a ?? ''));
	return m ? { w: Number(m[1]), h: Number(m[2]) } : null;
}

/** height in px of a shot of this aspect on the 390-wide canvas (loop360: the height of one slide row) */
export function shotHeight(aspect, format) {
	const a = parseAspect(aspect);
	if (!a) return S;
	if (format === 'loop360') return S; // a panorama is read as one screen-high stop, dragged sideways
	return Math.round((W * a.h) / a.w);
}

export function words(text) {
	const t = typeof text === 'string' ? text : (text?.de ?? Object.values(text ?? {})[0] ?? '');
	return String(t).split(/\s+/).filter((w) => /[\p{L}\p{N}]/u.test(w)).length;
}

export function textOf(text, locale = 'de') {
	if (typeof text === 'string') return text;
	return text?.[locale] ?? Object.values(text ?? {})[0] ?? '';
}

/** reading time of one text cluster (a box or a message) in seconds */
export function readSeconds(n) {
	return RHYTHM.perBox + n / RHYTHM.wordsPerSecond;
}

/**
 * Read an entity's latest version from the LOCAL dev database (read-only SELECT through the MariaDB container –
 * no client needed on Windows). status: 'published' | 'draft' | 'any'.
 */
export function readEntity(key, status = 'published') {
	const db = process.env.SCUTOONS_DB_NAME || 'scutoons';
	const where = status === 'any' ? '' : ` AND status='${status}'`;
	const sql = `SELECT json FROM \`${db}\`.entity_versions WHERE entity_key='${key.replace(/'/g, '')}'${where} ORDER BY id DESC LIMIT 1`;
	const out = execFileSync('docker', ['exec', 'scutoons-mariadb-1', 'mariadb', '-uroot', '-pscutoons-root', '-N', '--raw', '-e', sql], {
		encoding: 'utf8',
		maxBuffer: 64 * 1024 * 1024
	}).trim();
	if (!out) throw new Error(`entity ${key} (${status}) not found in ${db}`);
	return JSON.parse(out);
}

/** a chapter from a file (json) or the local DB ("catpope:1") */
export function loadChapter(ref) {
	if (existsSync(ref)) return JSON.parse(readFileSync(ref, 'utf8'));
	return readEntity(`chapter:${ref}`);
}

/** The media dev copy (gitignored, in the main checkout): for slice thumbnails. Production is the truth. */
export function mediaRoot() {
	const candidates = [process.env.SCUTOONS_MEDIA, resolve(ROOT, 'media'), resolve(ROOT, '../scutoons/media')].filter(Boolean);
	return candidates.find((p) => existsSync(p)) ?? null;
}

export function esc(s) {
	return String(s ?? '').replace(/[&<>"']/g, (c) => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' })[c]);
}

export const fmtMin = (sec) => `${(sec / 60).toFixed(1)} min`;
