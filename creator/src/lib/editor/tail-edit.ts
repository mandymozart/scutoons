// A drawn bubble's tail in the inspector (owner 2026-10-18, DESIGN.md Part 2 "Tails"): its kind (none / speech /
// thought) and a reset; the canvas handles (canvas-tail-handles.svelte) move its tip and base.
import { anchoredRect, defaultTail, dialogFit, dialogTail, freeTailFrom, type Dialog, type Path, type TailBox, type TailKind } from '@scutoons/content';
import { drawnHeights, drawnKey, drawnWidths } from '$lib/editor/drawn-heights.svelte.ts';

/** the box as the canvas drew it (% of the slice): an auto width at its drawn width, an auto height at its drawn height */
export function drawnBox(path: Path, d: Dialog): TailBox {
	const key = drawnKey(path);
	const width = dialogFit(d) === 'width' ? (drawnWidths[key] ?? d.rect.width ?? 40) : (d.rect.width ?? 40);
	const b = anchoredRect({ ...d.rect, width }, d.anchor, d.rect.height ?? drawnHeights[key] ?? 10);
	return { left: b.left, top: b.top, width: b.width, height: b.height };
}

/**
 * A tail kind for a drawn bubble (in place): none stores 'none'; a free tail keeps its tip and base; an old fixed tail
 * becomes a free one where it was; else a default tail for `box`.
 */
export function setTailKind(d: Dialog, kind: TailKind | 'none', box: TailBox, ratio: number) {
	if (kind === 'none') {
		d.tail = 'none';
		return;
	}
	const now = dialogTail(d);
	d.tail = now && typeof now === 'object' ? { ...now, kind } : typeof now === 'string' ? freeTailFrom(now, box, d.size ?? 4, ratio, kind) : defaultTail(kind, box, ratio);
}

/** the tail back to a new one of its kind (base on the bottom, tip below-left) */
export function resetTail(d: Dialog, box: TailBox, ratio: number) {
	const now = dialogTail(d);
	if (!now) return;
	d.tail = defaultTail(typeof now === 'object' ? now.kind : 'speech', box, ratio);
}
