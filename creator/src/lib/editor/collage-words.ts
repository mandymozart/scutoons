// The collage inspector's words (agents/research/COLLAGE.md §7): the looks, the 16 blend modes, a piece's kind.
import { pieceKind, type CollagePiece } from '@scutoons/content';
import { m } from '$lib/paraglide/messages.js';

const word = (key: string, fallback: string) => (m as unknown as Record<string, (() => string) | undefined>)[key]?.() ?? fallback;

/** a look's name (`creator.look_<id>`; an id the code doesn't know – a later owner-made look – as it is) */
export const lookLabel = (id: string) => word(`creator.look_${id}`, id);
/** a blend mode's name (`creator.blend_<mode>`, the hyphen as `_`: color-dodge → blend_color_dodge) */
export const blendLabel = (mode: string) => word(`creator.blend_${mode.replace(/-/g, '_')}`, mode);
/** Shape / Character / Image */
export const pieceKindLabel = (piece: Pick<CollagePiece, 'src' | 'figure'>) => word(`creator.piece_${pieceKind(piece)}`, pieceKind(piece));
