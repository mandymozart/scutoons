// The SCUTOONS media standard (owner 2026-10-11: "wir haben einen Standard definiert, mit dem wir möglichst performante
// Dateien haben – die Leute können andere benutzen, aber wir empfehlen das Umkonvertieren"): ONE place that says whether
// a file meets it and turns it into one that does. Every importer goes through here (owner 2026-10-12: "die Importe über
// den Medienstandard"): the slice import, the character import, the map's import, the catalog's images (admin), the
// asset picker's upload – and the optimisation tool (BACKLOG 10.64).
//   images: PNG / JPEG → WebP (transparency kept); WebP, SVG, GIF stay as they are; a size limit scales down, never up
//   density: 1x / 2x / 3x files of one image (`name.webp`, `name.2x.webp`, `name.3x.webp` – ASSETS.md "Variants are
//           suffixes"), each made from the file delivered for it, else from the smallest bigger one
//   video:  the standard is MP4 (H.264) / WebM in 480p / 720p / 1080p – converting needs ffmpeg, which the server
//           doesn't have: not converted here yet (the place for it, e.g. ffmpeg.wasm in the browser or a worker)
import { IMAGE_QUALITY_DENSITIES, MEDIA_QUALITIES, type MediaQuality } from '@scutoons/content';
import { toWebp } from './media-upload.ts';

export type MediaKind = 'image' | 'video' | 'audio' | 'other';
export type Density = 1 | 2 | 3;

/** a file's type and name (a File, or a registry rendition: `mime` as `type`, the path as `name`) – `size` in bytes */
export type MediaFacts = { type: string; name: string; size?: number };

/** an image above this is "too large" (the optimisation tool lists it) – BACKLOG 10.64, owner to confirm */
export const IMAGE_MAX_BYTES = 1024 * 1024;

export interface MediaVerdict {
	/** the file meets the standard as it is */
	ok: boolean;
	/** what converting it would do (for a hint / the optimisation tool), e.g. "PNG → WebP" */
	advice?: string;
	/** a conversion exists here (else only the advice) */
	convertible: boolean;
}

const EXT_TYPES: Record<string, string> = { png: 'image/png', jpg: 'image/jpeg', jpeg: 'image/jpeg', webp: 'image/webp', svg: 'image/svg+xml', gif: 'image/gif', mp4: 'video/mp4', webm: 'video/webm', mov: 'video/quicktime', mp3: 'audio/mpeg' };
const IMAGE_CONVERT = /^image\/(png|jpeg)$/;

/** the type, else the one the extension says (registry paths and drag-and-drop files may come without) */
export function mediaType(file: Pick<MediaFacts, 'type' | 'name'>): string {
	return file.type || EXT_TYPES[file.name.match(/\.([a-z0-9]+)$/i)?.[1].toLowerCase() ?? ''] || '';
}

export function mediaKind(file: Pick<MediaFacts, 'type' | 'name'>): MediaKind {
	const type = mediaType(file);
	if (type.startsWith('image/')) return 'image';
	if (type.startsWith('video/')) return 'video';
	if (type.startsWith('audio/')) return 'audio';
	return 'other';
}

/** does the file meet the standard – and can it be converted here? */
export function checkMedia(file: MediaFacts): MediaVerdict {
	const kind = mediaKind(file);
	const type = mediaType(file);
	if (kind === 'image') {
		if (IMAGE_CONVERT.test(type)) return { ok: false, advice: `${type === 'image/png' ? 'PNG' : 'JPEG'} → WebP`, convertible: true };
		if (type === 'image/webp' && (file.size ?? 0) > IMAGE_MAX_BYTES) return { ok: false, advice: `> ${IMAGE_MAX_BYTES / 1024 / 1024} MB`, convertible: false };
		return { ok: true, convertible: false };
	}
	if (kind === 'video') {
		// MP4 / WebM are the standard; other containers would need ffmpeg (not on the server – later, BACKLOG 10.64)
		const ok = /^video\/(mp4|webm)$/.test(type);
		return ok ? { ok, convertible: false } : { ok, advice: '→ MP4 (H.264) / WebM', convertible: false };
	}
	return { ok: true, convertible: false };
}

/** the standard file name for a converted file (`a.png` → `a.webp`); unchanged when nothing converts */
export function standardName(name: string, file: Pick<MediaFacts, 'type' | 'name'> = { type: '', name }): string {
	return checkMedia(file).convertible && mediaKind(file) === 'image' ? name.replace(/\.(png|jpe?g)$/i, '.webp') : name;
}

// ── density: 1x / 2x / 3x ────────────────────────────────────────────────────────────────────────────────────────────

/** "….3x.jpg", "…@2x.png", "… 3x.webp", "…-3x.png" → the density a file name says */
const DENSITY_IN_NAME = /(?:^|[._@\- ])([123])x(?=[._\- ]|$)/i;

/** a file name's parts: `catpope/s1c1-slice-26.3x.jpg` → stem `s1c1-slice-26`, density 3, ext `jpg` (folder dropped) */
export function splitMediaName(name: string): { stem: string; density: Density | null; ext: string } {
	const file = name.split('/').pop() ?? '';
	const ext = file.match(/\.([a-z0-9]+)$/i)?.[1] ?? '';
	const withoutExt = ext ? file.slice(0, -ext.length - 1) : file;
	const found = withoutExt.match(DENSITY_IN_NAME);
	return { stem: withoutExt.replace(DENSITY_IN_NAME, ''), density: found ? (Number(found[1]) as Density) : null, ext: ext.toLowerCase() };
}

/** the path of one density: `base` (no extension, no density) + `.2x` / `.3x` (1x = no suffix) + `.webp` */
export const densityPath = (base: string, density: Density, ext = 'webp') => `${base}${density > 1 ? `.${density}x` : ''}.${ext}`;

/**
 * The widths of 1x / 2x / 3x: a number = the 1x width, the others its multiples (slices: 390 → 780 / 1170); or each its
 * own (place images: 256 / 512 / 1024 – owner 2026-10-11)
 */
export type DensityWidths = number | Record<Density, number>;
export const densityWidth = (widths: DensityWidths, density: Density): number => (typeof widths === 'number' ? widths * density : widths[density]);

/** the density a file of this width delivers: the highest one it is wide enough for (a 1170 px slice → 3x; at least 1x) */
export function densityFromWidth(width: number, widths: DensityWidths): Density {
	for (const density of [3, 2] as const) if (width + 1 >= densityWidth(widths, density)) return density;
	return 1;
}

/** the quality level of a density (`imageVariants` / `variants`: low 1x, medium 2x, high 3x) */
export const densityLevel = (density: Density): MediaQuality => MEDIA_QUALITIES.find((q) => IMAGE_QUALITY_DENSITIES[q] === density)!;

/**
 * Which delivered file each density 1x … the highest delivered is made from: the file delivered for that density, else
 * the smallest bigger one (owner 2026-10-03: "always generate the smaller ones"; never scaled up).
 */
export function densityPlan<T extends { density: Density }>(sources: T[]): { density: Density; source: T }[] {
	if (!sources.length) return [];
	const top = Math.max(...sources.map((s) => s.density)) as Density;
	const plan: { density: Density; source: T }[] = [];
	for (let d = 1; d <= top; d++) {
		const density = d as Density;
		const source = sources.find((s) => s.density === density) ?? [...sources].filter((s) => s.density > density).sort((a, b) => a.density - b.density)[0];
		plan.push({ density, source });
	}
	return plan;
}

// ── encoding (browser) ───────────────────────────────────────────────────────────────────────────────────────────────

/** an image in the standard: WebP bytes, its size, and whether it was re-encoded (false = the delivered WebP as it is) */
export interface StandardImage {
	blob: Blob;
	width: number;
	height: number;
	converted: boolean;
}

/** the width a bitmap keeps under the limits (never wider than it is) */
function limitWidth(width: number, height: number, size: { maxWidth?: number; maxSide?: number }): number {
	let w = width;
	if (size.maxWidth) w = Math.min(w, size.maxWidth);
	if (size.maxSide) w = Math.min(w, Math.round(width * Math.min(1, size.maxSide / Math.max(width, height))));
	return Math.max(1, w);
}

async function encodeBitmap(file: File, bitmap: ImageBitmap, width: number): Promise<StandardImage> {
	// a delivered WebP that needs no scaling keeps its bytes (no second lossy pass)
	if (mediaType(file) === 'image/webp' && width >= bitmap.width) return { blob: file, width: bitmap.width, height: bitmap.height, converted: false };
	return { ...(await toWebp(bitmap, width)), converted: true };
}

/**
 * One image in the standard for an importer whose target is a `.webp` path: WebP, at most `maxWidth` wide / `maxSide`
 * on the longest side (scaled down, never up), transparency kept. A WebP within the limits stays as it is; any other
 * raster image (PNG, JPEG, GIF's first frame …) is encoded.
 */
export async function standardImage(file: File, size: { maxWidth?: number; maxSide?: number } = {}): Promise<StandardImage> {
	const bitmap = await createImageBitmap(file);
	try {
		return await encodeBitmap(file, bitmap, limitWidth(bitmap.width, bitmap.height, size));
	} finally {
		bitmap.close();
	}
}

/**
 * The 1x / 2x / 3x files of ONE image from the file(s) delivered for it (`slice-26.jpg` + `slice-26.3x.jpg`, or one
 * master): each file's density from its name, else from its width; every density up to the highest delivered one, 1x
 * first, each at most its width (`widths`: the 1x width, or one per density).
 */
export async function standardDensities(files: File[], widths: DensityWidths): Promise<(StandardImage & { density: Density })[]> {
	const sources: { file: File; bitmap: ImageBitmap; density: Density }[] = [];
	try {
		for (const file of files) {
			const bitmap = await createImageBitmap(file);
			sources.push({ file, bitmap, density: splitMediaName(file.name).density ?? densityFromWidth(bitmap.width, widths) });
		}
		const out: (StandardImage & { density: Density })[] = [];
		for (const { density, source } of densityPlan(sources)) out.push({ density, ...(await encodeBitmap(source.file, source.bitmap, Math.min(source.bitmap.width, densityWidth(widths, density)))) });
		return out;
	} finally {
		for (const { bitmap } of sources) bitmap.close();
	}
}

/**
 * The file in the standard (the asset picker's upload): a PNG / JPEG as WebP at full size (`maxWidth` scales down),
 * anything else as it is (video: not yet – see above). Returns the original when nothing converts.
 */
export async function toStandard(file: File, options: { maxWidth?: number } = {}): Promise<File> {
	if (!checkMedia(file).convertible) return file;
	const { blob } = await standardImage(file, options);
	return new File([blob], standardName(file.name, file), { type: 'image/webp' });
}
