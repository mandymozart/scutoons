// The optimisation tool's list (BACKLOG 10.64, owner 2026-10-11: "im Admin oder im Creator ein Optimierungstool"): which
// registry assets are off the media standard (media-standard.ts) and what fixing one means. Pure – the admin page lists
// the findings, converts the chosen ones (a NEW file beside the old – PLATFORM.md: a changed file gets a new name, a CDN
// path is never overwritten) and relinks every use to it.
import type { AssetKind, AssetRendition } from '@scutoons/api';
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
		findings.push({ ...base, fix: asset.renditions.some((r) => r.path === target) ? 'twin' : 'convert', target });
	}
	return findings;
}

/** the findings of many assets: the most used first, then the biggest */
export function auditAssets(assets: AuditAsset[]): StandardFinding[] {
	return assets.flatMap(auditAsset).sort((a, b) => b.usages - a.usages || b.size - a.size || a.path.localeCompare(b.path));
}

/** a free name beside `target` when it is taken by another asset: `a.webp` → `a-2.webp`, `a-3.webp` … */
export const nextFreeName = (target: string, n: number) => target.replace(/(\.[a-z0-9]+)$/i, `-${n}$1`);
