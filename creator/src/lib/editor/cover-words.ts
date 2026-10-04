// The cover editor's words (COMPONENTS.md step 9: cover-editor in parts): a line's text source, colours, alignments.
import type { CoverLine } from '@scutoons/content';
import { m } from '$lib/paraglide/messages.js';

export const sourceLabel = (text: CoverLine['text']) =>
	text === 'toon' ? m['creator.cover_text_toon']() : text === 'season' ? m['creator.cover_text_season']() : m['creator.cover_text_custom']();
export const colorLabel = (c: string) => (m as unknown as Record<string, () => string>)[`creator.cover_color_${c}`]?.() ?? c;
export const alignLabel = (a: string) => (m as unknown as Record<string, () => string>)[`creator.cover_align_${a}`]?.() ?? a;
