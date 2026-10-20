// A text box's paint as CSS (issue #94): Dialog.color / stroke is a CSS colour or a linear / radial gradient
// (content.ts DialogPaint). The reader draws a gradient cut to the letters (shared/reader dialog.svelte); the creator's
// paint field shows the same CSS in its swatch (shared/ui paint.ts – the editor's own copy of this small function: ui
// doesn't depend on content).
import type { DialogPaint, Gradient } from './content.ts';

export const isGradient = (p: DialogPaint | null | undefined): p is Gradient => typeof p === 'object' && p !== null && Array.isArray((p as Gradient).stops);

const round = (n: number) => Math.round(n * 100) / 100;

/**
 * the stops as CSS, in their order (the validator keeps them ascending); a stop's opacity as an alpha. A radial
 * gradient's stops are scaled by its radius (% of the farthest corner – CSS `circle farthest-corner` is 100 %).
 */
function stopList(g: Gradient): string {
	const scale = g.type === 'radial' ? g.radius / 100 : 1;
	return g.stops
		.map((s) => {
			const hex = s.color.length === 4 ? `#${[...s.color.slice(1)].map((c) => c + c).join('')}` : s.color;
			const alpha = s.opacity === undefined || s.opacity >= 1 ? '' : Math.round(Math.max(0, s.opacity) * 255).toString(16).padStart(2, '0');
			return `${hex}${alpha} ${round(s.at * scale)}%`;
		})
		.join(', ');
}

/** a gradient as a CSS image; a colour stays a colour (null = none) */
export function paintCss(p: DialogPaint | null | undefined): string | null {
	if (p === undefined || p === null || p === '') return null;
	if (!isGradient(p)) return p;
	if (p.type === 'linear') return `linear-gradient(${round(p.angle)}deg, ${stopList(p)})`;
	return `radial-gradient(circle farthest-corner at ${round(p.center.x)}% ${round(p.center.y)}%, ${stopList(p)})`;
}

/** any paint as a CSS image – a colour as a flat one (a layer cut to the letters needs an image) */
export function paintImage(p: DialogPaint | null | undefined): string | null {
	const css = paintCss(p);
	return css === null ? null : isGradient(p) ? css : `linear-gradient(${css}, ${css})`;
}
