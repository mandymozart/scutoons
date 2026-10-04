/**
 * Typing into the read-only script asks first (owner 2026-10-03, "Edit script"): a key that would change the text –
 * a character, Enter / Backspace / Delete, paste / cut / undo / redo. Navigation, selection and copy keys pass.
 */
export function isEditKey(event: KeyboardEvent): boolean {
	if (event.ctrlKey || event.metaKey) return ['v', 'x', 'z', 'y'].includes(event.key.toLowerCase());
	return event.key.length === 1 || ['Enter', 'Backspace', 'Delete'].includes(event.key);
}
