// Fills `width` / `height` of every slice from its image on the local CDN copy (owner 2026-09-30: every slice carries its
// size; agents/PERFORMANCE.md P2) and, with --webp, points slices to their 1x WebP.
//
//   node --experimental-transform-types --no-warnings shared/content/scripts/slice-sizes.ts <chapter / scene files …>
//        [--toon=catpope] [--cdn=media] [--webp] [--default-missing] [--dry-run]
//
// Files are chapter or group (VN scene) JSON files, e.g. viewer/static/api/catpope/de/chapter1.json; the toon is taken
// from the path (…/api/<toon>/<locale>/…) unless --toon is given. Image paths are relative to <cdn>/<toon>/.
//   --webp             a slice whose image isn't a WebP gets `s<n>/c<n>/<slug>.webp` when that file exists
//                      (slug = the file name in lower case, runs of other characters → "-": "S1C1 - Slice 7.jpg" →
//                      "s1c1-slice-7"). Only for files whose slices map 1:1.
//   --default-missing  a slice whose image isn't on the local CDN copy gets DEFAULT_SLICE (390 × 625) instead of
//                      staying without a size (reported either way)
// Then publish the files to the store: php server/bin/console import-content viewer/static/api --toon=<toon> --only=… --update
import { existsSync, openSync, readFileSync, readSync, closeSync, writeFileSync } from 'node:fs';
import { basename, dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { DEFAULT_SLICE, forEachSlice } from '../src/index.ts';

const repo = resolve(dirname(fileURLToPath(import.meta.url)), '../../..');
const args = process.argv.slice(2);
const opt = (name: string) => args.find((a) => a.startsWith(`--${name}=`))?.slice(name.length + 3);
const flag = (name: string) => args.includes(`--${name}`);
const CDN = resolve(repo, opt('cdn') ?? 'media');
const files = args.filter((a) => !a.startsWith('--'));
if (!files.length) {
	console.error('usage: slice-sizes.ts <chapter / scene json files…> [--toon=] [--cdn=] [--webp] [--default-missing] [--dry-run]');
	process.exit(2);
}

/** Pixel size of a PNG, JPEG, WebP or GIF file (header only). */
export function imageSize(path: string): { width: number; height: number } | null {
	const fd = openSync(path, 'r');
	const buf = Buffer.alloc(64 * 1024);
	const n = readSync(fd, buf, 0, buf.length, 0);
	closeSync(fd);
	const b = buf.subarray(0, n);
	if (b.readUInt32BE(0) === 0x89504e47) return { width: b.readUInt32BE(16), height: b.readUInt32BE(20) };
	if (b.toString('ascii', 0, 3) === 'GIF') return { width: b.readUInt16LE(6), height: b.readUInt16LE(8) };
	if (b.toString('ascii', 0, 4) === 'RIFF' && b.toString('ascii', 8, 12) === 'WEBP') {
		const chunk = b.toString('ascii', 12, 16);
		if (chunk === 'VP8X') return { width: 1 + b.readUIntLE(24, 3), height: 1 + b.readUIntLE(27, 3) };
		if (chunk === 'VP8 ') return { width: b.readUInt16LE(26) & 0x3fff, height: b.readUInt16LE(28) & 0x3fff };
		if (chunk === 'VP8L') {
			const bits = b.readUInt32LE(21);
			return { width: (bits & 0x3fff) + 1, height: ((bits >> 14) & 0x3fff) + 1 };
		}
		return null;
	}
	if (b[0] === 0xff && b[1] === 0xd8) {
		let i = 2;
		while (i + 9 < b.length) {
			if (b[i] !== 0xff) return null;
			const marker = b[i + 1];
			const length = b.readUInt16BE(i + 2);
			// SOF0…SOF15 except DHT (C4), JPG (C8), DAC (CC)
			if (marker >= 0xc0 && marker <= 0xcf && ![0xc4, 0xc8, 0xcc].includes(marker)) return { width: b.readUInt16BE(i + 7), height: b.readUInt16BE(i + 5) };
			i += 2 + length;
		}
	}
	return null;
}

const slug = (name: string) => name.toLowerCase().replace(/[^a-z0-9]+/g, '-').replace(/^-|-$/g, '');

/** `s<n>/c<n>/<slug>.webp` of an image path (the chapter's folder, agents/ASSETS.md), when its name starts with a chapter code. */
export function webpSlicePath(image: string): string | null {
	const s = slug(basename(image).replace(/\.[^.]+$/, ''));
	const code = s.match(/^s(\d+)c(\d+)-/);
	return code ? `s${code[1]}/c${code[2]}/${s}.webp` : null;
}

let changed = 0;
const report: string[] = [];
for (const file of files) {
	const path = resolve(process.cwd(), file);
	const toon = opt('toon') ?? path.replace(/\\/g, '/').match(/\/api\/([^/]+)\/[^/]+\//)?.[1];
	if (!toon) {
		console.error(`${file}: toon unknown (use --toon=)`);
		process.exit(2);
	}
	const raw = readFileSync(path, 'utf8');
	const data = JSON.parse(raw);
	const kind = Array.isArray(data.groups) ? 'chapter' : 'group';
	let fileChanges = 0;
	const missing: string[] = [];
	forEachSlice(kind, data, (slice) => {
		if (!slice || typeof slice.image !== 'string') return;
		if (flag('webp') && !/\.webp$/i.test(slice.image)) {
			const webp = webpSlicePath(slice.image);
			if (webp && existsSync(resolve(CDN, toon, webp))) {
				slice.image = webp;
				fileChanges++;
			}
		}
		const local = resolve(CDN, toon, slice.image);
		const size = existsSync(local) ? imageSize(local) : null;
		const target = size ?? (flag('default-missing') ? { ...DEFAULT_SLICE } : null);
		if (!size) missing.push(slice.image);
		if (target && (slice.width !== target.width || slice.height !== target.height)) {
			// keep the key order readable: image, width, height, then the rest
			const { image, width: _w, height: _h, ...rest } = slice;
			for (const key of Object.keys(slice)) delete slice[key];
			Object.assign(slice, { image, width: target.width, height: target.height, ...rest });
			fileChanges++;
		}
	});
	if (missing.length) report.push(`${file}: ${missing.length} image(s) not on the local CDN copy${flag('default-missing') ? ` (set to ${DEFAULT_SLICE.width} × ${DEFAULT_SLICE.height})` : ' (no size set)'}: ${[...new Set(missing)].join(', ')}`);
	if (fileChanges && !flag('dry-run')) {
		const indent = raw.match(/\n(\s+)"/)?.[1] ?? '\t';
		const eol = raw.includes('\r\n') ? '\r\n' : '\n';
		writeFileSync(path, JSON.stringify(data, null, indent).replace(/\n/g, eol) + (raw.endsWith('\n') ? eol : ''));
	}
	changed += fileChanges;
	console.log(`${file}: ${fileChanges} change(s)${flag('dry-run') ? ' (dry run)' : ''}`);
}
for (const line of report) console.log(line);
console.log(`${changed} change(s) in ${files.length} file(s)`);
