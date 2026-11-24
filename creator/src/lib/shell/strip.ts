// A row that scrolls sideways (the bar's open items, its workspace row): it only ever stops with a whole item at its
// left edge, so no name is cut at its start. Pure: test/shell.test.ts.

/** an item's extent along the row, in the row's scroll coordinates */
export interface Span {
	left: number;
	right: number;
}

/** where the row stands: its scroll position and the room it adds after its last item so that position can be reached */
export interface StripPlace {
	left: number;
	pad: number;
}

const near = (a: number, b: number) => Math.abs(a - b) <= 1;

/** `left` and the room after the last item it needs (`width`: the row's visible width) */
function place(spans: readonly Span[], left: number, width: number): StripPlace {
	const end = spans.at(-1)?.right ?? 0;
	if (end <= width || !spans.length || left <= spans[0].left + 1) return { left: 0, pad: 0 };
	return { left, pad: Math.max(0, Math.ceil(left + width - end)) };
}

/**
 * The place that shows item `at` whole, an item's start at the left edge; as it is when it already does. `endRoom`: the
 * room the right end's fade takes while more lies beyond (an item under it is not shown whole).
 */
export function showWhole(spans: readonly Span[], at: number, scrollLeft: number, width: number, endRoom = 0): StripPlace {
	const target = spans[at];
	if (!target) return { left: scrollLeft, pad: 0 };
	const room = at === spans.length - 1 ? width : Math.max(target.right - target.left, width - endRoom);
	const start = spans.find((s) => near(s.left, scrollLeft));
	if ((start || scrollLeft <= 1) && target.left >= scrollLeft - 1 && target.right <= scrollLeft + room + 1) return place(spans, start?.left ?? 0, width);
	if (target.left < scrollLeft || target.right - target.left >= room) return place(spans, target.left, width);
	const first = spans.find((s) => s.left >= target.right - room - 1 && s.left <= target.left);
	return place(spans, first?.left ?? target.left, width);
}

/** the place one step along (`by` items; a page: as many as fit), always on an item's start */
export function stepWhole(spans: readonly Span[], scrollLeft: number, width: number, by: number): StripPlace {
	if (!spans.length) return { left: 0, pad: 0 };
	const at = Math.max(0, spans.findIndex((s) => s.left >= scrollLeft - 1));
	const next = Math.min(spans.length - 1, Math.max(0, at + by));
	return place(spans, spans[next].left, width);
}

/** how many items lie whole in the visible width from the left edge (at least one): an arrow's page */
export function wholeIn(spans: readonly Span[], scrollLeft: number, width: number): number {
	return Math.max(1, spans.filter((s) => s.left >= scrollLeft - 1 && s.right <= scrollLeft + width + 1).length);
}

/** the items' spans of a row element, read from the page */
export function spansOf(row: HTMLElement, items: readonly HTMLElement[]): Span[] {
	const box = row.getBoundingClientRect();
	return items.map((el) => {
		const r = el.getBoundingClientRect();
		return { left: r.left - box.left + row.scrollLeft, right: r.right - box.left + row.scrollLeft };
	});
}
