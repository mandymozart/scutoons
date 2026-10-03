import { MEDIA_QUALITIES, type MediaQuality, type QualityVariants } from '@scutoons/content';
import { qualityVariants, toonRelativePath, type AssetRendition } from '@scutoons/api';

/**
 * A medium with quality variants as the content stores it (PLAN.md "RULE: one file per quality level"): `src` = the
 * default file (the trailer calls it `loop`), `variants` = one file per level (low 480p / medium 720p / high 1080p).
 * Paths are toon-relative. Pure helpers for `media-variants-slot.svelte` and every field with this shape (season
 * trailer, slice video, later slice images 1x / 2x / 3x and music).
 */
export interface VariantMedia {
	src?: string;
	variants?: QualityVariants;
}

/**
 * The default file of a set of variants: medium, else the LOWER one (less data), else high (owner 2026-10-03:
 * "when 720p is cleared the new default is the lower one first").
 */
export function preferredVariant(variants: QualityVariants | undefined): string | undefined {
	return variants?.medium || variants?.low || variants?.high || undefined;
}

/** the variants in level order, empty levels left out; nothing left → undefined (the key is removed) */
export function cleanVariants(variants: QualityVariants | undefined): QualityVariants | undefined {
	const out = Object.fromEntries(MEDIA_QUALITIES.filter((l) => variants?.[l]).map((l) => [l, variants![l]!])) as QualityVariants;
	return Object.keys(out).length ? out : undefined;
}

/**
 * A whole asset from the picker: its quality renditions of the right kind (480p / 720p / 1080p …) become the variants,
 * the medium one the default. An asset without levels (one plain file) → that file as `src`, no variants.
 */
export function mediaFromAsset(renditions: AssetRendition[], pickedPath: string, toon: string, mime = 'video/'): VariantMedia {
	const levels = qualityVariants(renditions.filter((r) => r.mime?.startsWith(mime)));
	const variants = cleanVariants(Object.fromEntries(MEDIA_QUALITIES.filter((l) => levels[l]).map((l) => [l, toonRelativePath(levels[l]!, toon)])));
	return { src: preferredVariant(variants) ?? toonRelativePath(pickedPath, toon), variants };
}

/**
 * One level replaced (path) or cleared (undefined). The default follows the levels: while it is one of them (or there
 * is none) it is the preferred level afterwards (medium, else low, else high); a default that is no level (an older
 * single file) stays.
 */
export function withVariant(media: VariantMedia, level: MediaQuality, path: string | undefined): VariantMedia {
	const variants = cleanVariants({ ...(media.variants ?? {}), [level]: path });
	let src = media.src;
	// the default is one of the levels (or none yet) → it stays the preferred level (720p first)
	if (!src || Object.values(media.variants ?? {}).includes(src)) src = preferredVariant(variants);
	return src ? { src, variants } : {};
}

/** the level whose file is the default (null: the default is a single file without a level) */
export function defaultLevel(media: VariantMedia): MediaQuality | null {
	return MEDIA_QUALITIES.find((l) => media.src && media.variants?.[l] === media.src) ?? null;
}
