// Collage pieces as CSS (agents/research/COLLAGE.md §6): the pure builders CollagePieceView draws with, so the reader
// and the creator draw a piece identically (node-tested: test/collage-css.test.ts). Every effect length is in cqw of
// the piece box (`.piece` is the container): % of the piece's width – the same look at every size and on every host.
// Order on `.fx`: outline ×4 → shadow → blur (the outlined shape casts the shadow, the blur softens it all); never a
// filter on the masked `.fill` (a filter runs before the mask – the blur would be cut at the silhouette).
import { anchorTranslate, pieceStyle, type CollagePiece, type PiecePoint, type ResolvedPieceStyle } from '@scutoons/content';

const round = (n: number) => {
	const v = Math.round(n * 1000) / 1000;
	return Object.is(v, -0) ? 0 : v;
};
/** a number for CSS: at most 3 decimals, never "-0" */
export const num = (n: number): string => String(round(n));
const cqw = (n: number) => (round(n) === 0 ? '0' : `${num(n)}cqw`);

const HEX = /^#(?:([0-9a-f])([0-9a-f])([0-9a-f])|([0-9a-f]{2})([0-9a-f]{2})([0-9a-f]{2}))$/i;

/** #rgb / #rrggbb at an opacity (0…1, default 1): the colour as written when opaque, else `rgb(r g b / a)` */
export function withAlpha(color: string, opacity?: number): string {
	const m = opacity !== undefined && opacity < 1 ? HEX.exec(color) : null;
	if (!m) return color;
	const [r, g, b] = m[1] ? [m[1], m[2], m[3]].map((c) => parseInt(c + c, 16)) : [m[4], m[5], m[6]].map((c) => parseInt(c, 16));
	return `rgb(${r} ${g} ${b} / ${num(Math.max(0, opacity!))})`;
}

/** the `.fill` background: a colour or one linear gradient across the whole silhouette; null = the file as drawn */
export function fillCss(fill: ResolvedPieceStyle['fill']): string | null {
	if (typeof fill === 'string') return fill || null;
	const stops = Array.isArray(fill?.stops) ? fill.stops : [];
	if (!stops.length) return null;
	if (stops.length === 1) return withAlpha(stops[0].color, stops[0].opacity);
	return `linear-gradient(${num(fill!.angle ?? 180)}deg, ${stops.map((s) => `${withAlpha(s.color, s.opacity)} ${num(s.at)}%`).join(', ')})`;
}

/** the `.fx` filter: outline (4 zero-blur drop shadows = a dilation) → shadow → blur, in cqw; null = none */
export function filterCss(style: Pick<ResolvedPieceStyle, 'outline' | 'shadow' | 'blur'>): string | null {
	const parts: string[] = [];
	const outline = style.outline;
	if (outline && outline.width > 0) {
		const [w, n, c] = [cqw(outline.width), cqw(-outline.width), outline.color];
		parts.push(`drop-shadow(${w} 0 0 ${c})`, `drop-shadow(${n} 0 0 ${c})`, `drop-shadow(0 ${w} 0 ${c})`, `drop-shadow(0 ${n} 0 ${c})`);
	}
	const shadow = style.shadow;
	if (shadow) parts.push(`drop-shadow(${cqw(shadow.x)} ${cqw(shadow.y)} ${cqw(Math.max(0, shadow.blur))} ${withAlpha(shadow.color, shadow.opacity)})`);
	if (style.blur && style.blur.amount > 0) parts.push(`blur(${cqw(style.blur.amount)})`);
	return parts.length ? parts.join(' ') : null;
}

/**
 * A line from → to (% of a box whose width ÷ height = `ratio`) as a CSS linear-gradient: its angle (CSS degrees,
 * 0 = to the top) and both points' positions along the gradient line in %. CSS centres the line in the box with the
 * length |w·sin θ| + |h·cos θ|; a point's position is its projection onto it. from = to: straight down, one position.
 */
export function gradientLine(from: PiecePoint, to: PiecePoint, ratio: number): { angle: number; start: number; end: number } {
	const w = ratio > 0 ? ratio : 1;
	const h = 1;
	const dx = ((to.x - from.x) / 100) * w;
	const dy = ((to.y - from.y) / 100) * h;
	const theta = dx === 0 && dy === 0 ? Math.PI : Math.atan2(dx, -dy);
	const sin = Math.sin(theta);
	const cos = Math.cos(theta);
	const length = Math.abs(w * sin) + Math.abs(h * cos);
	const along = (p: PiecePoint) => (((p.x / 100 - 0.5) * w * sin - (p.y / 100 - 0.5) * h * cos) / length + 0.5) * 100;
	const angle = (theta * 180) / Math.PI;
	return { angle: round(angle < 0 ? angle + 360 : angle), start: round(along(from)), end: round(along(to)) };
}

/** how far a piece's paint reaches past its box, in cqw: the outline, the shadow (offset + 1.5× its radius), 3× the blur */
export function spill(style: Pick<ResolvedPieceStyle, 'outline' | 'shadow' | 'blur'>): number {
	const outline = style.outline && style.outline.width > 0 ? style.outline.width : 0;
	const s = style.shadow;
	const shadow = s ? Math.max(Math.abs(s.x), Math.abs(s.y)) + 1.5 * Math.max(0, s.blur) : 0;
	const blur = style.blur && style.blur.amount > 0 ? 3 * style.blur.amount : 0;
	return outline + shadow + blur;
}

export interface BlurBands {
	/** both bands reach this far past the piece box (cqw), their `.fx` sits back on it – a mask cuts at its box */
	pad: number;
	/** the line in the padded box */
	angle: number;
	start: number;
	end: number;
	/** the sharp band's mask: shown at `from`, gone from `to` on */
	sharp: string;
	/** the blurred band's mask: the inverse (a crossfade) */
	soft: string;
}

/**
 * A progressive blur (`blur` with from + to): two bands, sharp and blurred, crossfaded by inverse masks (the view adds
 * the soft band with `plus-lighter` in an isolated piece – drawn over the sharp one, the sum would dip). Each band is
 * padded by the paint's spill (3× the blur, + outline and shadow) so its mask doesn't cut the halo; the line is mapped
 * from the piece box into the padded box. null = no ramp (an even blur or none: one `.fx`).
 */
export function blurBands(style: ResolvedPieceStyle, ratio: number): BlurBands | null {
	const blur = style.blur;
	if (!blur || !(blur.amount > 0) || !blur.from || !blur.to) return null;
	const pad = round(spill(style));
	const p = pad / 100;
	const h = 1 / (ratio > 0 ? ratio : 1);
	const [W, H] = [1 + 2 * p, h + 2 * p];
	const into = (q: PiecePoint): PiecePoint => ({ x: ((q.x / 100 + p) / W) * 100, y: (((q.y / 100) * h + p) / H) * 100 });
	const { angle, start, end } = gradientLine(into(blur.from), into(blur.to), W / H);
	const [a, s, e] = [`${num(angle)}deg`, `${num(start)}%`, `${num(end)}%`];
	return { pad, angle, start, end, sharp: `linear-gradient(${a}, #000 ${s}, transparent ${e})`, soft: `linear-gradient(${a}, transparent ${s}, #000 ${e})` };
}

/** the `.piece` box in its host: left / top at the anchor point (% of the host), the width, the height from `ratio` */
export function pieceBox(piece: Pick<CollagePiece, 'ratio' | 'anchor' | 'rotation'>, left: number, top: number, width: number): Record<string, string> {
	const box: Record<string, string> = { left: `${num(left)}%`, top: `${num(top)}%`, width: `${num(width)}%`, 'aspect-ratio': num(piece.ratio > 0 ? piece.ratio : 1) };
	const shift = anchorTranslate(piece.anchor);
	if (shift) box.translate = shift;
	if (piece.rotation) box.rotate = `${num(piece.rotation)}deg`;
	return box;
}

/** declarations as a style attribute (null / undefined / '' left out) – none of them may carry a URL */
export function styleText(css: Record<string, string | null | undefined>): string {
	return Object.entries(css)
		.filter(([, value]) => value != null && value !== '')
		.map(([key, value]) => `${key}: ${value}`)
		.join('; ');
}

/** a file as a CSS image, quoted (a dev URL's Windows drive colon, spaces, brackets) */
export function cssUrl(src: string): string {
	return `url("${src.replace(/[\n\r\f]/g, '').replace(/["\\]/g, '\\$&')}")`;
}

const finite = (v: unknown): v is number => typeof v === 'number' && Number.isFinite(v);

/** a piece the renderer can lay out (an id, a file, a ratio, a rect); anything else is skipped, never thrown on */
export function drawablePiece(piece: unknown): piece is CollagePiece {
	const p = piece as CollagePiece | null;
	return !!p && typeof p === 'object' && typeof p.id === 'string' && typeof p.src === 'string' && !!p.src && finite(p.ratio) && p.ratio > 0 &&
		!!p.rect && finite(p.rect.x) && finite(p.rect.y) && finite(p.rect.width) && p.rect.width > 0;
}

export interface PieceCss {
	/** the `.piece` style: box, blend, opacity, z */
	box: string;
	/** the `.fill` background; null = the file as drawn (an <img>) */
	fill: string | null;
	/** the `.fx` filter (the sharp band's with a ramp) */
	filter: string | null;
	/** a ramp's two bands and the blurred band's filter; null = one `.fx` */
	bands: (BlurBands & { filter: string | null }) | null;
}

/** everything CollagePieceView draws: the piece's look (`pieceStyle`) at `left` / `top` / `width` % of its host */
export function pieceCss(piece: CollagePiece, left: number, top: number, width: number, z?: number): PieceCss {
	const look = pieceStyle(piece);
	const bands = blurBands(look, piece.ratio);
	const opacity = finite(look.opacity) && look.opacity < 1 ? num(Math.max(0, look.opacity)) : null;
	return {
		box: styleText({
			...pieceBox(piece, left, top, width),
			'mix-blend-mode': look.blend && look.blend !== 'normal' ? look.blend : null,
			opacity,
			'z-index': finite(z) ? String(z) : null
		}),
		fill: fillCss(look.fill),
		filter: filterCss(bands ? { outline: look.outline, shadow: look.shadow } : look),
		bands: bands ? { ...bands, filter: filterCss(look) } : null
	};
}
