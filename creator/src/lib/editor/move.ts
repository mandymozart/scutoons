/**
 * Moves the item at `from` to `to` in place (reorder buttons, keyboard, drag and drop). Nothing happens – and it
 * answers false – when either index is outside the list or they are equal.
 */
export function moveAt<T>(list: T[], from: number, to: number): boolean {
	if (from === to || from < 0 || to < 0 || from >= list.length || to >= list.length) return false;
	const [item] = list.splice(from, 1);
	list.splice(to, 0, item);
	return true;
}

/** The same as a new list (for values that are replaced, not edited in place); null when nothing moves. */
export function movedAt<T>(list: readonly T[], from: number, to: number): T[] | null {
	const next = [...list];
	return moveAt(next, from, to) ? next : null;
}
