/**
 * Multi-select + drag for ordered lists (owner 2026-10-03: the chapter tree's groups and a group's slices): a click
 * selects one, Shift+click the range from the last plain click (the anchor), Ctrl / Cmd+click adds or removes one;
 * dragging a selected row (the shared Sortable's grip) moves the whole selection (kept in its order) to the drop place.
 */

/** the selection after a click on `index` (`anchor` = the last plain / Ctrl click) */
export function clickSelect(
	selected: readonly number[],
	anchor: number | null,
	index: number,
	event: { shiftKey?: boolean; ctrlKey?: boolean; metaKey?: boolean }
): { selected: number[]; anchor: number } {
	if (event.shiftKey && anchor !== null) {
		const [from, to] = anchor < index ? [anchor, index] : [index, anchor];
		return { selected: Array.from({ length: to - from + 1 }, (_, i) => from + i), anchor };
	}
	if (event.ctrlKey || event.metaKey) {
		const next = selected.includes(index) ? selected.filter((i) => i !== index) : [...selected, index];
		return { selected: next.sort((a, b) => a - b), anchor: index };
	}
	return { selected: [index], anchor: index };
}

/**
 * The shared Sortable's move with a selection: the row at `from` was dragged to `to` (its index after the move, among
 * the other rows); every picked row goes with it, kept in its order, where the dragged one lands. A `from` that is not
 * picked moves alone. Returns the new list and where every old index went (old → new).
 */
export function moveMany<T>(list: readonly T[], picked: readonly number[], from: number, to: number): { list: T[]; map: Map<number, number> } {
	const set = new Set(picked.includes(from) ? picked.filter((i) => i >= 0 && i < list.length) : [from]);
	// the boundary in the original list the dragged row was dropped on (the others keep their indices around it)
	const point = to < from ? to : to + 1;
	const rest = list.map((item, i) => ({ item, i })).filter((x) => !set.has(x.i));
	const moving = list.map((item, i) => ({ item, i })).filter((x) => set.has(x.i));
	const at = rest.filter((x) => x.i < point).length;
	const out = [...rest.slice(0, at), ...moving, ...rest.slice(at)];
	const map = new Map<number, number>();
	out.forEach((x, n) => map.set(x.i, n));
	return { list: out.map((x) => x.item), map };
}
