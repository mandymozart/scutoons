// The collage picker's logic (agents/research/COLLAGE.md §5): what a pick hands back (`PickedPiece`), the vector
// library's filters, and the new piece a host makes of a pick. Pure (no Svelte; `imageRatio` alone needs a DOM).
import { characterImage, isMissingImagePath, toonAssetRef, type Anchor, type CollagePiece, type PieceFigure, type RegistryCharacter } from '@scutoons/content';
import type { AssetRendition, AssetSummary } from '@scutoons/api';
import { newPlacementId } from './clip.ts';

/** what the picker hands back: the file from the CDN root, its ratio (w ÷ h) when known, how a figure was picked */
export interface PickedPiece {
	/** CDN-root path (catpope/vectors/wreath.svg, characters/catpope/…webp) */
	src: string;
	ratio?: number;
	figure?: PieceFigure;
	/** for lists and toasts */
	name: string;
}

/** width ÷ height of a file with both sizes, else null */
export function assetRatio(file: { width?: number | null; height?: number | null } | null | undefined): number | null {
	const w = file?.width;
	const h = file?.height;
	return typeof w === 'number' && typeof h === 'number' && w > 0 && h > 0 ? Math.round((w / h) * 10000) / 10000 : null;
}

/** a file's ratio by decoding it (a character picture, an image without sizes); null when it can't be read */
export function imageRatio(url: string): Promise<number | null> {
	return new Promise((done) => {
		if (!url || typeof Image === 'undefined') return done(null);
		const img = new Image();
		img.decoding = 'async';
		img.onload = () => done(assetRatio({ width: img.naturalWidth, height: img.naturalHeight }));
		img.onerror = () => done(null);
		img.src = url;
	});
}

/**
 * A character picture's file (from the CDN root) – the fallback rule (`characterImage`), but '' when no file resolves or
 * the one it resolves to is known to be missing (`isMissing`): the "?" substitute is a placeholder, never a piece's art
 */
export function figureImage(character: RegistryCharacter | null | undefined, figure: Omit<PieceFigure, 'id'>, isMissing?: (path: string) => boolean): string {
	const src = characterImage(character, { persona: figure.persona, look: figure.look, mood: figure.mood }, figure.view, { isMissing, substitute: '' });
	return isMissingImagePath(src) ? '' : src;
}

/** a shape of the vector library as a pick */
export function shapePick(shape: AssetSummary): PickedPiece {
	return { src: shape.path, ratio: assetRatio(shape) ?? undefined, name: shape.title || shape.slug };
}

/**
 * An image from the asset picker as a pick: the WebP rendition of a PNG / JPEG (a PNG + WebP twin – COLLAGE.md §5), else
 * the chosen file as it is
 */
export function imagePick(asset: AssetSummary, rendition: AssetRendition | null, path: string): PickedPiece {
	const webp = /\.(png|jpe?g)$/i.test(path) ? asset.renditions.find((r) => r.mime === 'image/webp' || /\.webp$/i.test(r.path)) : undefined;
	const file = webp ?? rendition ?? asset;
	return { src: webp?.path ?? path, ratio: assetRatio(file) ?? assetRatio(asset) ?? undefined, name: asset.title || asset.slug };
}

/** the library's tags by how often they are used (then by name) – the tag select */
export function shapeTags(shapes: readonly Pick<AssetSummary, 'tags'>[]): { tag: string; count: number }[] {
	const counts = new Map<string, number>();
	for (const shape of shapes) for (const tag of new Set(shape.tags ?? [])) counts.set(tag, (counts.get(tag) ?? 0) + 1);
	return [...counts].map(([tag, count]) => ({ tag, count })).sort((a, b) => b.count - a.count || a.tag.localeCompare(b.tag));
}

/** the shapes whose title, slug or tags hold every word of `query`, with `tag` when one is chosen */
export function filterShapes<T extends Pick<AssetSummary, 'title' | 'slug' | 'tags'>>(shapes: readonly T[], query: string, tag = ''): T[] {
	const words = query.toLowerCase().split(/\s+/).filter(Boolean);
	return shapes.filter((shape) => {
		if (tag && !(shape.tags ?? []).includes(tag)) return false;
		if (!words.length) return true;
		const text = `${shape.title} ${shape.slug} ${(shape.tags ?? []).join(' ')}`.toLowerCase();
		return words.every((word) => text.includes(word));
	});
}

/**
 * The new piece a host makes of a pick: `src` in the host's path form (a toon document: `toonAssetRef` – catpope's
 * `vectors/x.svg`, another toon's `../catpope/vectors/x.svg`; a score / catalog row: `toon` '' = the CDN-root path as
 * it is), a fresh id, the ratio (1 when unknown), the place; anchor 'tl' is stored as no key.
 */
export function pickedToPiece(
	picked: PickedPiece,
	toon: string,
	place: { rect: CollagePiece['rect']; anchor?: Anchor },
	taken: ReadonlySet<string> = new Set()
): CollagePiece {
	const piece: CollagePiece = {
		id: newPlacementId('piece', taken),
		src: toonAssetRef(toon, picked.src),
		ratio: picked.ratio && picked.ratio > 0 ? picked.ratio : 1,
		rect: { ...place.rect }
	};
	if (picked.figure) piece.figure = { ...picked.figure };
	if (place.anchor && place.anchor !== 'tl') piece.anchor = place.anchor;
	return piece;
}
