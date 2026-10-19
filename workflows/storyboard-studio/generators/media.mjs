// Small media helpers for the generators: MIME types by extension, data URIs, an MP4's duration (read from its `mvhd`
// box – no ffprobe needed), the nearest ratio of a list.

import { openSync, readSync, closeSync, statSync, readFileSync } from 'node:fs';
import { extname } from 'node:path';

export const MIME = { '.png': 'image/png', '.jpg': 'image/jpeg', '.jpeg': 'image/jpeg', '.webp': 'image/webp', '.mp4': 'video/mp4', '.webm': 'video/webm', '.mov': 'video/quicktime' };
export const EXT = { 'image/png': 'png', 'image/jpeg': 'jpg', 'image/webp': 'webp', 'video/mp4': 'mp4', 'video/webm': 'webm', 'video/quicktime': 'mov' };
export const mimeOf = (file) => MIME[extname(file).toLowerCase()] ?? 'application/octet-stream';

/** the length of `data:<mime>;base64,<…>` for a file of n bytes */
export const dataUriLength = (file) => `data:${mimeOf(file)};base64,`.length + Math.ceil(statSync(file).size / 3) * 4;
export const dataUri = (file) => `data:${mimeOf(file)};base64,${readFileSync(file).toString('base64')}`;

/** seconds of an MP4 / MOV (moov → mvhd: timescale + duration), or null */
export function mp4Duration(file) {
	const fd = openSync(file, 'r');
	try {
		const size = statSync(file).size;
		const read = (pos, n) => { const b = Buffer.alloc(n); readSync(fd, b, 0, n, pos); return b; };
		// walk the boxes from `start` to `end`, descending into moov
		const walk = (start, end) => {
			let pos = start;
			while (pos + 8 <= end) {
				const h = read(pos, 16);
				let len = h.readUInt32BE(0);
				const type = h.toString('latin1', 4, 8);
				let head = 8;
				if (len === 1) { len = Number(h.readBigUInt64BE(8)); head = 16; } else if (len === 0) len = end - pos;
				if (len < 8) return null;
				if (type === 'moov') return walk(pos + head, pos + len);
				if (type === 'mvhd') {
					const b = read(pos + head, 32);
					const v = b.readUInt8(0);
					const scale = v === 1 ? b.readUInt32BE(20) : b.readUInt32BE(12);
					const dur = v === 1 ? Number(b.readBigUInt64BE(24)) : b.readUInt32BE(16);
					return scale ? dur / scale : null;
				}
				pos += len;
			}
			return null;
		};
		return walk(0, size);
	} finally {
		closeSync(fd);
	}
}

/** "9:16" → the list's entry ("720:1280") nearest in aspect (log distance) */
export function nearestRatio(aspect, list) {
	const r = (s) => { const [a, b] = String(s).split(':').map(Number); return a / b; };
	const want = r(aspect || '9:16');
	if (!Number.isFinite(want) || !list?.length) return list?.[0];
	return list.reduce((a, b) => (Math.abs(Math.log(r(b) / want)) < Math.abs(Math.log(r(a) / want)) ? b : a));
}
