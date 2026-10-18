// The collage inspector's words (agents/research/COLLAGE.md §7): the looks, the 16 blend modes, a piece's kind and name.
import { moodLabel, pieceKind, pieceName, type CharacterRegistry, type CollagePiece, type MoodVocabulary } from '@scutoons/content';
import type { ScoreLabels } from '@scutoons/reader/score';
import { shownCharacterName, textLanguage } from '$lib/editor/text-language.svelte.ts';
import { shapeTitle } from '$lib/editor/collage-pick.ts';
import { shapeLibrary } from '$lib/editor/shapes.svelte.ts';
import { m } from '$lib/paraglide/messages.js';

const word = (key: string, fallback: string) => (m as unknown as Record<string, (() => string) | undefined>)[key]?.() ?? fallback;

/** a look's name (`creator.look_<id>`; an id the code doesn't know – a later owner-made look – as it is) */
export const lookLabel = (id: string) => word(`creator.look_${id}`, id);
/** a blend mode's name (`creator.blend_<mode>`, the hyphen as `_`: color-dodge → blend_color_dodge) */
export const blendLabel = (mode: string) => word(`creator.blend_${mode.replace(/-/g, '_')}`, mode);
/** Shape / Character / Image */
export const pieceKindLabel = (piece: Pick<CollagePiece, 'src' | 'figure'>) => word(`creator.piece_${pieceKind(piece)}`, pieceKind(piece));
/**
 * THE name of a piece – the Layers list, the canvas box, the score stage's list and overlay: a character picture by its
 * name · mood (the header's text language), a shape by its library title (once the library is loaded – a list holding
 * a shape loads it, `hasShapePiece`), else its file's name (`pieceName`). Only reads the library: a load() here would
 * write $state inside a derived / the template, which throws.
 */
export function pieceLabel(piece: CollagePiece | undefined, registry: CharacterRegistry | null | undefined, moods: MoodVocabulary | null | undefined): string {
	if (!piece) return '?';
	const figure = piece.figure;
	if (!figure) return (pieceKind(piece) === 'shape' && shapeTitle(piece.src, shapeLibrary.items)) || pieceName(piece);
	const character = registry?.characters.find((c) => c.id === figure.id);
	if (!character) return pieceName(piece);
	return [shownCharacterName(character, figure.persona), figure.mood && moodLabel(figure.mood, textLanguage.names, moods)].filter(Boolean).join(' · ');
}

/** whether a list of pieces holds a shape (its name then wants the library's titles) */
export const hasShapePiece = (pieces: unknown) => Array.isArray(pieces) && pieces.some((p) => !!p && typeof p === 'object' && pieceKind(p) === 'shape');

/** the score view's texts in the score stage (research/COLLAGE.md §4.4) – the creator's language */
export const scoreViewLabels = (): ScoreLabels => ({
	close: m['creator.close'](),
	zoomIn: m['creator.score_larger'](),
	zoomOut: m['creator.score_smaller'](),
	previous: m['creator.score_previous'](),
	next: m['creator.score_next'](),
	page: m['creator.score_page']({ page: '{page}', pages: '{pages}' }),
	pdf: 'PDF',
	loading: m['creator.loading'](),
	failed: m['creator.score_failed'](),
	play: m['creator.score_play'](),
	stop: m['creator.score_stop'](),
	pad: m['creator.score_pad']()
});
