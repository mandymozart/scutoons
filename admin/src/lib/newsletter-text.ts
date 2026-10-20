// Inserting into a newsletter's text (agents/NEWSLETTER.md "Writing a newsletter"; the editor's tool strip, #98): a
// paragraph of its own (a picture, a button, the {signin} / {gift} line) or a placeholder inside the line. Pure – the
// page writes the text back and puts the cursor / selection.

export interface Inserted {
	text: string;
	/** the selection after it (start = end: the cursor) */
	start: number;
	end: number;
}

/**
 * `line` as a paragraph of its own (blank lines around it, none doubled) after the paragraph the cursor is in – never
 * splitting a line; a selection (`at` … `until`) is replaced instead. The cursor after it.
 */
export function insertParagraph(text: string, at: number, line: string, until = at): Inserted {
	if (until === at) {
		const next = text.indexOf('\n\n', at);
		at = until = next < 0 ? text.length : next;
	}
	const before = text.slice(0, at).replace(/\s*$/, '');
	const after = text.slice(until).replace(/^\s*/, '');
	const next = [before, line, after].filter((part) => part !== '').join('\n\n');
	const end = (before ? before.length + 2 : 0) + line.length;
	return { text: next, start: end, end };
}

/** `value` inside the line, replacing the selection; the cursor after it */
export function insertInline(text: string, start: number, end: number, value: string): Inserted {
	const caret = start + value.length;
	return { text: text.slice(0, start) + value + text.slice(end), start: caret, end: caret };
}

/** a button paragraph `[label](https://)` with its label selected – typing replaces it */
export function insertButton(text: string, at: number, label: string): Inserted {
	const done = insertParagraph(text, at, `[${label}](https://)`);
	const start = done.end - `[${label}](https://)`.length + 1;
	return { text: done.text, start, end: start + label.length };
}
