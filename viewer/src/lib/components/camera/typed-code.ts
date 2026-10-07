/**
 * A code typed by hand (owner 2026-10-07: buyers get it in plain characters too, for when the camera won't read it):
 * the code itself (spaces dropped, case kept – codes are case-sensitive) or a pasted SCUTOONS link (`linkCode`: the
 * scanner's own parser, qr.ts `parseScutoonsCode`). Null = not a code.
 */
export function parseTypedCode(text: string, linkCode: (url: string) => string | null): string | null {
	const typed = text.trim();
	if (!typed) return null;
	if (/^https?:\/\//i.test(typed)) return linkCode(typed);
	const code = typed.replace(/\s+/g, '');
	return /^[A-Za-z0-9_-]{1,32}$/.test(code) ? code : null;
}
