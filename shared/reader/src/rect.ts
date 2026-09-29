import type { PercentRect } from './types.ts';

/**
 * Whether a pointer event lies inside a rect given in % of `element`'s box. No rect = the whole element.
 */
export function isWithinRect(event: MouseEvent, element: HTMLElement, rect?: PercentRect): boolean {
	if (!rect) return true;

	const bounds = element.getBoundingClientRect();
	const x = ((event.clientX - bounds.left) / bounds.width) * 100;
	const y = ((event.clientY - bounds.top) / bounds.height) * 100;

	return x >= rect.x && x <= rect.x + rect.width && y >= rect.y && y <= rect.y + rect.height;
}
