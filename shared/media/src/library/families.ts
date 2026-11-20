// One Library tile per asset family: the files that differ only by a version marker (`.2x`, `.480p`, `.900w`, `.poster`,
// `.128` …) or by their format are one family – the names in use (agents/ASSETS.md "Grouping", "Variants are suffixes").
// Pure: test/library-families.test.ts.
import type { AssetKind, AssetQuality, AssetRendition, AssetSummary } from '@scutoons/api';

/** what the Library sorts and filters by: the asset kinds folded to the ones a person tells apart */
export type LibraryKind = 'image' | 'video' | 'audio' | 'model' | 'other';
export const LIBRARY_KINDS: readonly LibraryKind[] = ['image', 'video', 'audio', 'model', 'other'];

const IMAGE_EXT = new Set(['webp', 'png', 'jpg', 'jpeg', 'gif', 'svg', 'avif']);
const VIDEO_EXT = new Set(['mp4', 'webm', 'mov', 'mkv', 'm4v']);
const AUDIO_EXT = new Set(['mp3', 'ogg', 'wav', 'flac', 'm4a', 'aac', 'opus']);
const MODEL_EXT = new Set(['glb', 'gltf']);
/** the audio levels the content reads (96 low, 128 medium, 320 high) and the rates the scanner also accepts */
const AUDIO_RATES = new Set(['64', '96', '128', '160', '192', '256', '320']);

export function kindOfExt(ext: string): LibraryKind {
	const e = ext.toLowerCase();
	if (IMAGE_EXT.has(e)) return 'image';
	if (VIDEO_EXT.has(e)) return 'video';
	if (AUDIO_EXT.has(e)) return 'audio';
	if (MODEL_EXT.has(e)) return 'model';
	return 'other';
}

export function libraryKind(kind: AssetKind): LibraryKind {
	return kind === 'image' || kind === 'video' || kind === 'audio' || kind === 'model' ? kind : 'other';
}

/** a file's family name and its version: `catpope/s1/c1/x.480p.mp4` → base `catpope/s1/c1/x`, label `480p` */
export interface VersionName {
	base: string;
	/** `source` (no marker), `2x` `3x`, `480p` … `<n>w`, `poster`, `<n>k` (audio) */
	label: string;
	ext: string;
	kind: LibraryKind;
}

export function parseVersionName(path: string): VersionName {
	const dot = path.lastIndexOf('.');
	const slash = path.lastIndexOf('/');
	const hasExt = dot > slash + 1;
	const ext = hasExt ? path.slice(dot + 1).toLowerCase() : '';
	const kind = kindOfExt(ext);
	let base = hasExt ? path.slice(0, dot) : path;
	let label = 'source';
	const inner = base.lastIndexOf('.');
	if (inner > slash + 1) {
		const marker = base.slice(inner + 1).toLowerCase();
		let found: string | null = null;
		if (/^[23]x$/.test(marker)) found = marker;
		else if (/^\d{3,4}p$/.test(marker)) found = marker;
		else if (/^\d{3,4}w$/.test(marker)) found = marker;
		else if (marker === 'poster') found = 'poster';
		else if (kind === 'audio' && AUDIO_RATES.has(marker)) found = `${marker}k`;
		if (found) {
			label = found;
			base = base.slice(0, inner);
		}
	}
	return { base, label, ext, kind };
}

/** one file of a family */
export interface FamilyVersion {
	label: string;
	path: string;
	ext: string;
	kind: LibraryKind;
	size: number;
	width: number | null;
	height: number | null;
	duration: number | null;
	quality: AssetQuality | null;
	origin: AssetRendition['origin'];
	/** the asset (registry id) the file belongs to */
	asset: string;
}

export interface AssetFamily {
	/** the family name: the CDN-root path without marker and extension */
	key: string;
	/** the last part of the key */
	name: string;
	/** the folder the files are in (`catpope/s1/c1`) */
	folder: string;
	title: string;
	/** the leading kind: a video's poster and its stills are part of the video */
	kind: LibraryKind;
	/** the registry assets folded in (usually one) */
	assets: string[];
	versions: FamilyVersion[];
	/** labels a family of this kind and place is expected to have and doesn't */
	missing: string[];
	/** references of its files (content + catalog) */
	usages: number;
	size: number;
	duration: number | null;
	width: number | null;
	height: number | null;
	/** where the source came from */
	origin: AssetRendition['origin'];
	tags: string[];
	updated: string | null;
	/** the file a tile shows (an image or a poster), '' = none */
	preview: string;
	/** the file Use hands on: the version a slot of this kind takes by default */
	main: string;
}

const LEAD: readonly LibraryKind[] = ['video', 'model', 'audio', 'image', 'other'];
const VIDEO_LEVELS: readonly [string, number][] = [
	['480p', 480],
	['720p', 720],
	['1080p', 1080]
];
/** a chapter's folder (`<toon>/s<n>/c<n>/`): its images are slices and key visuals, delivered as 1x / 2x / 3x */
const CHAPTER_FOLDER = /(^|\/)s\d+\/c\d+$/;

/** the order the versions are listed in (the tray, the details' table) */
export function versionRank(label: string): number {
	if (label === 'source') return 0;
	const p = /^(\d+)p$/.exec(label);
	if (p) return 100 - Number(p[1]) / 100;
	const w = /^(\d+)w$/.exec(label);
	if (w) return 200 - Number(w[1]) / 100;
	if (label === 'poster') return 300;
	const x = /^(\d)x$/.exec(label);
	if (x) return 400 + Number(x[1]);
	const k = /^(\d+)k$/.exec(label);
	if (k) return 500 - Number(k[1]) / 100;
	return 900;
}

/** a tile's duration: `0:12`, `2:10`, `1:02:05` */
export function formatDuration(seconds: number | null): string {
	if (!seconds || seconds <= 0) return '';
	const s = Math.round(seconds);
	const two = (n: number) => String(n).padStart(2, '0');
	return s >= 3600 ? `${Math.floor(s / 3600)}:${two(Math.floor((s % 3600) / 60))}:${two(s % 60)}` : `${Math.floor(s / 60)}:${two(s % 60)}`;
}

/** which versions a family should have (agents/ASSETS.md; RULES I3): video levels up to the source's short side + a poster, the three audio levels, a chapter image's 2x / 3x */
export function expectedVersions(family: Pick<AssetFamily, 'kind' | 'folder' | 'versions'>): string[] {
	if (family.kind === 'video') {
		const sides = family.versions.filter((v) => v.kind === 'video' && v.width && v.height).map((v) => Math.min(v.width!, v.height!));
		const top = sides.length ? Math.max(...sides) : Infinity;
		return [...VIDEO_LEVELS.filter(([, side]) => side <= top || side === 480).map(([label]) => label), 'poster'];
	}
	if (family.kind === 'audio') return ['96k', '128k', '320k'];
	if (family.kind === 'image' && (CHAPTER_FOLDER.test(family.folder) || family.versions.some((v) => /^[23]x$/.test(v.label)))) return ['2x', '3x'];
	return [];
}

function titleOf(name: string): string {
	return name.replace(/[-_]+/g, ' ').replace(/\s+/g, ' ').trim();
}

/** the version a slot of the family's kind takes: a video's 720p (the default level), an audio's 128k, an image's 1x, else the source */
function mainOf(kind: LibraryKind, versions: readonly FamilyVersion[]): string {
	const ofKind = versions.filter((v) => v.kind === kind);
	const by = (label: string) => ofKind.find((v) => v.label === label)?.path;
	if (kind === 'video') return by('720p') ?? by('source') ?? ofKind[0]?.path ?? '';
	if (kind === 'audio') return by('128k') ?? by('source') ?? ofKind[0]?.path ?? '';
	if (kind === 'image') return by('1x') ?? ofKind.find((v) => v.label === 'source' && v.ext === 'webp')?.path ?? by('source') ?? ofKind[0]?.path ?? '';
	return ofKind[0]?.path ?? versions[0]?.path ?? '';
}

function previewOf(versions: readonly FamilyVersion[]): string {
	const images = versions.filter((v) => v.kind === 'image');
	const pick = images.find((v) => v.label === 'poster') ?? images.find((v) => v.label === '1x') ?? images.find((v) => v.label === 'source' && v.ext === 'webp') ?? images.find((v) => v.label === 'source') ?? images[0];
	return pick?.path ?? '';
}

/**
 * The registry's assets as families: every file goes to the family of its name (an asset's renditions usually are one
 * family; a still or poster filed as its own asset joins its video). Sorted by key.
 */
export function groupFamilies(assets: readonly AssetSummary[]): AssetFamily[] {
	const byKey = new Map<string, { versions: FamilyVersion[]; assets: AssetSummary[] }>();
	for (const asset of assets) {
		const files = asset.renditions.length ? asset.renditions : [{ id: 0, label: 'original', quality: null, path: asset.path, size: asset.size, mime: null, width: asset.width, height: asset.height, duration: asset.duration, origin: 'imported' as const }];
		for (const r of files) {
			const name = parseVersionName(r.path);
			let entry = byKey.get(name.base);
			if (!entry) byKey.set(name.base, (entry = { versions: [], assets: [] }));
			if (!entry.assets.includes(asset)) entry.assets.push(asset);
			if (entry.versions.some((v) => v.path === r.path)) continue;
			entry.versions.push({
				label: name.label,
				path: r.path,
				ext: name.ext,
				kind: name.kind === 'other' ? libraryKind(asset.kind) : name.kind,
				size: r.size,
				width: r.width,
				height: r.height,
				duration: r.duration,
				quality: r.quality,
				origin: r.origin,
				asset: asset.id
			});
		}
	}
	const out: AssetFamily[] = [];
	for (const [key, { versions, assets: members }] of byKey) {
		// two files with the same label (a JPEG master and its WebP twin): the twin is named by its format
		const seen = new Map<string, number>();
		for (const v of versions) seen.set(v.label, (seen.get(v.label) ?? 0) + 1);
		for (const v of versions) if ((seen.get(v.label) ?? 0) > 1 && v.ext === 'webp' && v.label === 'source') v.label = '1x';
		versions.sort((a, b) => versionRank(a.label) - versionRank(b.label) || a.path.localeCompare(b.path));
		const kinds = new Set(versions.map((v) => v.kind));
		const kind = LEAD.find((k) => kinds.has(k)) ?? 'other';
		const slash = key.lastIndexOf('/');
		const name = key.slice(slash + 1);
		const folder = slash < 0 ? '' : key.slice(0, slash);
		const lead = versions.filter((v) => v.kind === kind);
		const sized = lead.find((v) => v.width && v.height) ?? versions.find((v) => v.width && v.height);
		const durations = lead.map((v) => v.duration ?? 0).filter((d) => d > 0);
		const source = versions.find((v) => v.label === 'source') ?? versions[0];
		const titled = members.find((a) => a.title && a.title !== a.slug);
		const family: AssetFamily = {
			key,
			name,
			folder,
			title: titled?.title ?? titleOf(name),
			kind,
			assets: members.map((a) => a.id),
			versions,
			missing: [],
			usages: members.reduce((n, a) => n + (a.usages ?? 0), 0),
			size: versions.reduce((n, v) => n + v.size, 0),
			duration: durations.length ? Math.max(...durations) : null,
			width: sized?.width ?? null,
			height: sized?.height ?? null,
			origin: source.origin,
			tags: [...new Set(members.flatMap((a) => a.tags))],
			updated: members.map((a) => a.updated_at).filter((d): d is string => !!d).sort().at(-1) ?? null,
			preview: previewOf(versions),
			main: mainOf(kind, versions)
		};
		const have = new Set(versions.map((v) => v.label));
		family.missing = expectedVersions(family).filter((label) => !have.has(label));
		out.push(family);
	}
	return out.sort((a, b) => a.key.localeCompare(b.key));
}
