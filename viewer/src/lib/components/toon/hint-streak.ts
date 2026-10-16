const SVG = 'http://www.w3.org/2000/svg';
let gradients = 0;

/**
 * The hint lines' colour ("Show hints", owner 2026-10-08): a radial gradient from the fireflies' core to their glow
 * (--sc-firefly-core → --sc-firefly-glow) in `svg`'s units, `transform` placing it (its middle is the core). Shared by
 * the trails and alien shapes (fireflies.ts) and the endless band's drag hint (renderer/strip-nudge-cue.ts). Returns the
 * `url(#…)` to paint with.
 */
export function hintGradient(svg: SVGSVGElement, transform: string): string {
	const id = `sc-hint-${++gradients}`;
	const gradient = document.createElementNS(SVG, 'radialGradient');
	for (const [k, v] of [['id', id], ['cx', '0'], ['cy', '0'], ['r', '1'], ['gradientUnits', 'userSpaceOnUse'], ['gradientTransform', transform]]) gradient.setAttribute(k, v);
	for (const [offset, color] of [['0', 'var(--sc-firefly-core)'], ['1', 'var(--sc-firefly-glow)']]) {
		const stop = document.createElementNS(SVG, 'stop');
		stop.setAttribute('offset', offset);
		stop.style.stopColor = color;
		gradient.appendChild(stop);
	}
	svg.prepend(gradient);
	return `url(#${id})`;
}

export interface Streak {
	/** the lit share of the path */
	length: number;
	/** the dash offset at the start and the end */
	from: number;
	to: number;
	ms: number;
	easing: string;
	/** fully lit from / until these shares of `ms` (fading in before, out after) */
	fade: [number, number];
}

/** a streak of light running once along `path` (`pathLength` 1) – the trails' move */
export function hintStreak(path: SVGPathElement, { length, from, to, ms, easing, fade }: Streak): Animation {
	path.setAttribute('stroke-dasharray', `${length} ${1 + length}`);
	return path.animate(
		[
			{ strokeDashoffset: `${from}`, opacity: 0 },
			{ opacity: 1, offset: fade[0] },
			{ opacity: 1, offset: fade[1] },
			{ strokeDashoffset: `${to}`, opacity: 0 }
		],
		{ duration: ms, easing, fill: 'forwards' }
	);
}
