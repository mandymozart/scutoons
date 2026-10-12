// The optimisation tool's list (BACKLOG 10.64, owner 2026-10-11: "im Admin oder im Creator ein Optimierungstool"): which
// registry assets are off the media standard (media-standard.ts) and what fixing one means. Pure – the admin page lists
// the findings, converts the chosen ones (a NEW file beside the old – PLATFORM.md: a changed file gets a new name, a CDN
// path is never overwritten) and relinks every use to it.
import type { AssetKind, AssetRelinkResult, AssetRendition } from '@scutoons/api';
import { checkMedia, standardName } from './media-standard.ts';

/** what the audit needs of an asset (an AssetSummary; `usages` per file from the list) */
export interface AuditAsset {
	id: string;
	slug: string;
	kind: AssetKind;
	/** the current version's source file */
	path: string;
	usages: number;
	renditions: Pick<AssetRendition, 'path' | 'size' | 'mime' | 'label' | 'usages'>[];
}

export interface StandardFinding {
	asset: string;
	slug: string;
	kind: AssetKind;
	/** the file off the standard that content / the catalog references */
	path: string;
	size: number;
	/** its references */
	usages: number;
	/** what is off, language-neutral: "PNG → WebP", "→ MP4 (H.264) / WebM" */
	advice: string;
	/**
	 * the fix: `convert` = encode to `target` (a new file), then relink; `twin` = the asset already has the WebP at
	 * `target` – relink only; null = nothing here (video needs ffmpeg – not on the host)
	 */
	fix: 'convert' | 'twin' | null;
	target?: string;
	/** the twin's bytes (`twin` only) */
	targetSize?: number;
}

/**
 * The asset's REFERENCED files against the standard (a PNG master nothing uses beside its used WebP twin is fine – that
 * is "Unused"'s business). Without per-file counts (an older API) the source stands for the asset.
 */
export function auditAsset(asset: AuditAsset): StandardFinding[] {
	const perFile = asset.renditions.some((r) => typeof r.usages === 'number');
	const used = perFile ? asset.renditions.filter((r) => (r.usages ?? 0) > 0) : asset.renditions.filter((r) => r.path === asset.path).map((r) => ({ ...r, usages: asset.usages }));
	const findings: StandardFinding[] = [];
	for (const file of used) {
		const facts = { type: file.mime ?? '', name: file.path, size: file.size };
		const verdict = checkMedia(facts);
		if (verdict.ok) continue;
		const base = { asset: asset.id, slug: asset.slug, kind: asset.kind, path: file.path, size: file.size, usages: file.usages ?? 0, advice: verdict.advice ?? '' };
		if (!verdict.convertible) {
			findings.push({ ...base, fix: null });
			continue;
		}
		const target = standardName(file.path, facts);
		const twin = asset.renditions.find((r) => r.path === target);
		findings.push(twin ? { ...base, fix: 'twin', target, targetSize: twin.size } : { ...base, fix: 'convert', target });
	}
	return findings;
}

/** the findings of many assets: the most used first, then the biggest */
export function auditAssets(assets: AuditAsset[]): StandardFinding[] {
	return assets.flatMap(auditAsset).sort((a, b) => b.usages - a.usages || b.size - a.size || a.path.localeCompare(b.path));
}

/** a free name beside `target` when it is taken by another asset: `a.webp` → `a-2.webp`, `a-3.webp` … */
export const nextFreeName = (target: string, n: number) => target.replace(/(\.[a-z0-9]+)$/i, `-${n}$1`);

// ── fixing (the "Convert …" run) ─────────────────────────────────────────────────────────────────────────────────────

/** what a run needs from the app (the admin: CDN fetch, standardImage, the assets API) – injected, so it is testable */
export interface FixDeps {
	/** the file's bytes */
	read: (path: string) => Promise<File>;
	/** the file in the standard (WebP) */
	encode: (file: File) => Promise<Blob>;
	/** a NEW file; must throw an error with `code: 'exists'` when the path is taken */
	upload: (blob: Blob, path: string) => Promise<void>;
	/** every use of each old path → its new path, in one pass (content: one new draft per entity) */
	relink: (map: Record<string, string>) => Promise<AssetRelinkResult>;
	progress?: (done: number, total: number) => void;
}

export interface FixRow {
	path: string;
	/** the new file */
	target?: string;
	result: 'converted' | 'relinked' | 'failed';
	before: number;
	after?: number;
	error?: string;
}

export interface FixReport {
	rows: FixRow[];
	/** the relink of every new file (null when nothing got that far, or it failed – then `error`) */
	relink: AssetRelinkResult | null;
	error?: string;
}

const message = (e: unknown) => (e instanceof Error ? e.message : String(e));
const taken = (e: unknown) => !!e && typeof e === 'object' && (e as { code?: unknown }).code === 'exists';

/**
 * Fixes the chosen findings: `convert` = read → encode → upload under the standard name (`-2`, `-3` … when another file
 * has it – a CDN path is never overwritten, PLATFORM.md); `twin` = nothing to upload. Then ONE relink of every fixed
 * path, so a document touched by several files gets one new draft. Failed files are reported and left alone; the old
 * files stay (the "Unused" tab cleans them up once nothing references them).
 */
export async function fixFindings(findings: StandardFinding[], deps: FixDeps): Promise<FixReport> {
	const rows: FixRow[] = [];
	const map: Record<string, string> = {};
	let done = 0;
	for (const f of findings) {
		try {
			if (f.fix === 'twin' && f.target) {
				map[f.path] = f.target;
				rows.push({ path: f.path, target: f.target, result: 'relinked', before: f.size, after: f.targetSize });
			} else if (f.fix === 'convert' && f.target) {
				const blob = await deps.encode(await deps.read(f.path));
				let target = f.target;
				for (let n = 2; ; n++) {
					try {
						await deps.upload(blob, target);
						break;
					} catch (e) {
						if (!taken(e) || n > 20) throw e;
						target = nextFreeName(f.target, n);
					}
				}
				map[f.path] = target;
				rows.push({ path: f.path, target, result: 'converted', before: f.size, after: blob.size });
			}
		} catch (e) {
			rows.push({ path: f.path, target: f.target, result: 'failed', before: f.size, error: message(e) });
		}
		deps.progress?.(++done, findings.length);
	}
	if (!Object.keys(map).length) return { rows, relink: null };
	try {
		return { rows, relink: await deps.relink(map) };
	} catch (e) {
		return { rows, relink: null, error: message(e) };
	}
}
