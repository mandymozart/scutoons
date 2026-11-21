// A collage piece's pick (agents/research/COLLAGE.md §5): what a Library pick hands back (`PickedPiece`), the vector
// library's titles and pages, and the new piece a host makes of a pick. Pure (no Svelte; `imageRatio` alone needs a DOM).
import {
	AUTHORED_VIEW,
	CHARACTER_IMAGE_ROOT,
	CHARACTER_IMAGE_VARIANTS,
	DEFAULT_LOOK,
	REGULAR_PERSONA,
	characterImage,
	isMissingImagePath,
	matchCharacterImageFile,
	toonAssetRef,
	type Anchor,
	type CharacterImageVariant,
	type CollagePiece,
	type PieceFigure,
	type RegistryCharacter
} from '@scutoons/content';
import type { AssetSummary } from '@scutoons/api';
import type { AssetFamily } from '@scutoons/media';
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

/**
 * The views of a character picture with a file of their own for its persona / look / mood – the inspector's Picture row.
 * The fallback rule draws an avatar or full view without its own file as the same mood's portrait: such a view would
 * claim a figure readers never see. A view counts when it resolves (`figureImage`) and – beside the portrait – not to
 * the file the portrait resolves to.
 */
export function figureViews(character: RegistryCharacter | null | undefined, figure: Omit<PieceFigure, 'id' | 'view'>, isMissing?: (path: string) => boolean): CharacterImageVariant[] {
	const portrait = figureImage(character, { ...figure, view: AUTHORED_VIEW }, isMissing);
	return CHARACTER_IMAGE_VARIANTS.filter((view) => {
		if (view === AUTHORED_VIEW) return !!portrait;
		const src = figureImage(character, { ...figure, view }, isMissing);
		return !!src && src !== portrait;
	});
}

/**
 * A Library file as a piece: a PNG / JPEG's WebP twin in its family (COLLAGE.md §5), else the file; the family's ratio;
 * a character picture (`characters/<name>/…`) keeps its figure – persona, look, mood, view – so the piece follows the
 * character's pictures
 */
export function libraryPiece(
	path: string,
	family: Pick<AssetFamily, 'title' | 'name' | 'width' | 'height' | 'versions'> | null,
	characters: readonly RegistryCharacter[] = []
): PickedPiece {
	const webp = /\.(png|jpe?g)$/i.test(path) ? family?.versions.find((v) => v.ext.toLowerCase() === 'webp') : undefined;
	const src = webp?.path ?? path;
	const piece: PickedPiece = { src, ratio: assetRatio(webp ?? family) ?? undefined, name: family?.title || family?.name || src.split('/').pop() || src };
	const folder = src.startsWith(`${CHARACTER_IMAGE_ROOT}/`) ? src.split('/')[1] : '';
	const character = folder ? characters.find((c) => folder === c.id || folder.startsWith(`${c.id}-`)) : undefined;
	const slot = character ? matchCharacterImageFile(src, character) : null;
	if (character && slot?.kind === 'expression') {
		const figure: PieceFigure = { id: character.id, view: slot.view, mood: slot.mood };
		if (slot.persona !== REGULAR_PERSONA) figure.persona = slot.persona;
		if (slot.look !== DEFAULT_LOOK) figure.look = slot.look;
		piece.figure = figure;
	}
	return piece;
}

/**
 * A shape piece's library title, by the file's base name – a slice stores `vectors/x.svg` (catpope) or
 * `../catpope/vectors/x.svg`, a score `catpope/vectors/x.svg`; the library's names are unique. undefined = not in it
 * (or the library not loaded yet).
 */
export function shapeTitle(src: unknown, shapes: readonly Pick<AssetSummary, 'path' | 'title'>[] | null | undefined): string | undefined {
	const base = (path: string) => path.split('/').pop() ?? '';
	const file = typeof src === 'string' ? base(src) : '';
	if (!file || !shapes) return undefined;
	return shapes.find((shape) => base(shape.path) === file)?.title || undefined;
}

/** every row of a paged list: page 1, 2 … until `pagination.total` (an empty page ends it too) – the server clamps page_size to 500 */
export async function allPages<T>(page: (n: number) => Promise<{ items: T[]; pagination: { total: number } }>): Promise<T[]> {
	const all: T[] = [];
	for (let n = 1; ; n++) {
		const { items, pagination } = await page(n);
		all.push(...items);
		if (!items.length || all.length >= pagination.total) return all;
	}
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
