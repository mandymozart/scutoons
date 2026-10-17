// How a text box is sized (owner 2026-10-17, Figma's three modes; agents/DESIGN.md Part 2 "Box sizing"):
//   width  – auto width: the box hugs its text on one line, lines break only at a line break; `rect.width` (optional) is a
//            max width – the text wraps there, so a longer translation can't run off the slice
//   height – auto height: the width is fixed, the text wraps, the box grows (by the vertical anchor)
//   fixed  – width and height fixed; the text shrinks to fit (down to 80 %), then runs past the box (never clipped)
// No `fit` = what boxes always did: a `rect.height` = fixed size, none = auto height (no migration).

export const DIALOG_FITS = ['width', 'height', 'fixed'] as const;
export type DialogFit = (typeof DIALOG_FITS)[number];

export function isDialogFit(value: unknown): value is DialogFit {
	return typeof value === 'string' && (DIALOG_FITS as readonly string[]).includes(value);
}

/** the mode a text box is in: its `fit`, else today's rule (a height = fixed size, none = auto height) */
export function dialogFit(d: { fit?: unknown; rect?: { height?: unknown } | null }): DialogFit {
	if (isDialogFit(d.fit)) return d.fit;
	return typeof d.rect?.height === 'number' ? 'fixed' : 'height';
}

/** average advance of a character in em (IBM Plex Mono 0.6; the lettering font is narrower) */
const ADVANCE = { mono: 0.6, lettering: 0.5 } as const;

/**
 * An estimate of an auto-width box's drawn width in % of the slice – no measuring (the validator's side check; the
 * reader lays it out with CSS): the longest line × the font's average advance (+ letter spacing) × the size, plus a
 * drawn frame's side padding (1em each), capped by the max width.
 */
export function estimatedWidth(d: {
	text?: unknown;
	size?: number;
	font?: string;
	variant?: string;
	frame?: boolean;
	letterSpacing?: number;
	rect?: { width?: number } | null;
}): number {
	const boxed = d.variant === 'caption' || d.variant === 'narration' || d.variant === 'lyrics';
	const font = d.font === 'mono' || d.font === 'lettering' ? d.font : boxed ? 'mono' : 'lettering';
	const spacing = d.letterSpacing ?? (font === 'lettering' ? 0.03 : 0);
	const lines = (Array.isArray(d.text) ? d.text : [d.text]).flatMap((t) => String(t ?? '').split('\n'));
	const longest = Math.max(0, ...lines.map((l) => [...l].length));
	const framed = boxed ? d.frame !== false : d.frame === true;
	const size = d.size ?? 4;
	const width = (longest * (ADVANCE[font] + spacing) + (framed ? 2 : 0)) * size;
	const max = d.rect?.width;
	return Math.round((typeof max === 'number' && max > 0 ? Math.min(width, max) : width) * 100) / 100;
}
