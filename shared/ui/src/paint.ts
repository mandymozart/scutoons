// A paint like Figma's (issue #94, owner 2026-10-18: "a gradient tool like in figma for radial linear gradients for
// outline and for fills"): a solid colour (a hex string, as every colour field stores it) or a linear / radial gradient –
// the shape of shared/content's DialogPaint (content.ts; its CSS: content paint.ts – ui doesn't depend on content).

export interface GradientStop {
	/** #rrggbb */
	color: string;
	/** position along the gradient, 0–100 % */
	at: number;
}
export type Gradient =
	| { type: 'linear'; /** degrees, CSS: 0 = to the top, 90 = to the right */ angle: number; stops: GradientStop[] }
	| { type: 'radial'; /** % of the box */ center: { x: number; y: number }; /** % of the box's farthest corner */ radius: number; stops: GradientStop[] };
export type Paint = string | Gradient;
export type PaintKind = 'solid' | Gradient['type'];

export const isGradient = (p: Paint | null | undefined): p is Gradient => typeof p === 'object' && p !== null && Array.isArray(p.stops);
export const paintKind = (p: Paint | null | undefined): PaintKind => (isGradient(p) ? p.type : 'solid');

const sorted = (stops: GradientStop[]) => [...stops].sort((a, b) => a.at - b.at);
const stopList = (stops: GradientStop[]) => sorted(stops).map((s) => `${s.color} ${round(s.at)}%`).join(', ');
const round = (n: number) => Math.round(n * 10) / 10;

/** the paint as a CSS background (a swatch, a stop bar, text cut to it) */
export function paintCss(p: Paint | null | undefined): string | null {
	if (!p) return null;
	if (!isGradient(p)) return p;
	if (p.type === 'linear') return `linear-gradient(${round(p.angle)}deg, ${stopList(p.stops)})`;
	// the radius in % of the farthest corner: the stops scale with it (as shared/content paint.ts)
	return `radial-gradient(circle farthest-corner at ${round(p.center.x)}% ${round(p.center.y)}%, ${stopList(p.stops.map((s) => ({ ...s, at: (s.at * p.radius) / 100 })))})`;
}

/** the stops left to right, as a horizontal bar shows them (the stop editor) */
export const stopBarCss = (stops: GradientStop[]) => `linear-gradient(to right, ${stopList(stops)})`;

/** switching the kind keeps what it can: the stops, the angle / centre; a solid becomes a gradient from its colour to white */
export function toKind(p: Paint | null | undefined, kind: PaintKind, fallback = '#000000'): Paint {
	const stops: GradientStop[] = isGradient(p) ? p.stops : [{ color: typeof p === 'string' && p ? p : fallback, at: 0 }, { color: '#ffffff', at: 100 }];
	if (kind === 'solid') return isGradient(p) ? sorted(p.stops)[0].color : (p || fallback);
	if (kind === 'linear') return { type: 'linear', angle: isGradient(p) && p.type === 'linear' ? p.angle : 180, stops };
	return { type: 'radial', center: isGradient(p) && p.type === 'radial' ? p.center : { x: 50, y: 50 }, radius: isGradient(p) && p.type === 'radial' ? p.radius : 100, stops };
}

/** the colour at a position (a new stop clicked onto the bar takes it) */
export function colourAt(stops: GradientStop[], at: number): string {
	const s = sorted(stops);
	if (at <= s[0].at) return s[0].color;
	const last = s[s.length - 1];
	if (at >= last.at) return last.color;
	const i = s.findIndex((x) => x.at >= at);
	const a = s[i - 1], b = s[i], t = (at - a.at) / (b.at - a.at || 1);
	const mix = (k: number) => Math.round(parseInt(a.color.slice(k, k + 2), 16) * (1 - t) + parseInt(b.color.slice(k, k + 2), 16) * t).toString(16).padStart(2, '0');
	return `#${mix(1)}${mix(3)}${mix(5)}`;
}
