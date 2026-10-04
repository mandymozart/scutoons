/**
 * Arrow keys in a grid of cards (the picker sheets): put it on the grid's container as `onkeydown={gridArrows}`; the
 * cells carry `data-cell`. Left / right move one cell, up / down one row (the columns are read from the cell's grid
 * container), clamped at the ends. Keys outside a cell are left alone.
 */
export function gridArrows(event: KeyboardEvent) {
	const keys: Record<string, 'left' | 'right' | 'up' | 'down'> = { ArrowLeft: 'left', ArrowRight: 'right', ArrowUp: 'up', ArrowDown: 'down' };
	const dir = keys[event.key];
	if (!dir) return;
	const cells = [...(event.currentTarget as HTMLElement).querySelectorAll<HTMLElement>('[data-cell]')];
	const index = cells.indexOf(document.activeElement as HTMLElement);
	if (index < 0) return;
	event.preventDefault();
	const columns = Math.max(1, getComputedStyle(cells[index].parentElement!).gridTemplateColumns.split(' ').length);
	const next = gridStep(index, dir, columns, cells.length);
	cells[next]?.focus();
}

/** the cell an arrow goes to: one step left / right, one row up / down, clamped to 0 … count − 1 */
export function gridStep(index: number, dir: 'left' | 'right' | 'up' | 'down', columns: number, count: number): number {
	const step = dir === 'left' ? -1 : dir === 'right' ? 1 : dir === 'up' ? -columns : columns;
	return Math.min(count - 1, Math.max(0, index + step));
}
