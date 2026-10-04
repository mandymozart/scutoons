// The editors' keyboard (COMPONENTS.md step 11, owner 2026-10-03 – 4.4.1): Ctrl+S saves everywhere; Ctrl+Z /
// Ctrl+Shift+Z / Ctrl+Y undo and redo the edited file only OUTSIDE a text field – inside one (input, textarea, select,
// contentEditable) they belong to the browser and undo the typing in that field.

/** an element the user types into (its own undo belongs to the browser) */
export function isField(target: EventTarget | null): boolean {
	const el = target as HTMLElement | null;
	return !!el && (el.tagName === 'INPUT' || el.tagName === 'TEXTAREA' || el.tagName === 'SELECT' || el.isContentEditable);
}

/**
 * Handles Ctrl/Cmd + S / Z / Shift+Z / Y; returns true when the key was taken (default prevented). Undo / redo leave a
 * field's own keys alone.
 */
export function editorKeys(event: KeyboardEvent, actions: { save: () => void; undo: () => void; redo: () => void }): boolean {
	if (!(event.ctrlKey || event.metaKey)) return false;
	const key = event.key.toLowerCase();
	if (key === 's') {
		event.preventDefault();
		actions.save();
		return true;
	}
	if ((key === 'z' || key === 'y') && !isField(event.target)) {
		event.preventDefault();
		if (key === 'y' || event.shiftKey) actions.redo();
		else actions.undo();
		return true;
	}
	return false;
}
