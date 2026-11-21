import { dropTarget, markerX, type DropPanel, type DropTarget } from './panel-tabs.ts';
import type { Rect } from './floats.ts';
import type { Floats } from './floats.svelte.ts';

const box = (el: Element): Rect => {
	const r = el.getBoundingClientRect();
	return { x: r.left, y: r.top, w: r.width, h: r.height };
};

/** The tab-holding panels under the pointer, front to back, measured from the page, and where a tab would land */
export function measureDrop(floats: Floats, x: number, y: number): { target: DropTarget; marker: Rect | null } {
	const panels: DropPanel[] = [];
	for (const el of document.querySelectorAll<HTMLElement>('[data-float][data-tabbed]')) {
		const id = el.dataset.float!;
		const strip = el.querySelector('[data-tab-strip]');
		if (!strip) continue;
		panels.push({ id, rect: box(el), strip: box(strip), tabs: [...strip.querySelectorAll('[data-tab]')].map(box) });
	}
	panels.sort((a, b) => floats.z(b.id) - floats.z(a.id));
	const target = dropTarget(x, y, panels);
	const p = target && panels.find((o) => o.id === target.panel);
	if (!target || !p) return { target, marker: null };
	return { target, marker: { x: markerX(p.tabs, p.strip, target.index) - 1, y: p.strip.y, w: 2, h: p.strip.h } };
}
