// A problem in the creator's words for its Log line: its place (groups.0.slices.42.dialog.1.text → scene-12 · Slice 43 ·
// Line 2; a group of the page's own file by its name, the page's own file and a line's "text" field add nothing) and its
// message (an expression's problem in the creator's language – the validator's message is English).
import type { Problem } from '@scutoons/content';
import { fileLabel } from '$lib/labels.ts';
import { exprTextAt } from '$lib/editor/expr-messages.ts';
import { m } from '$lib/paraglide/messages.js';

const NUMBERED: Record<string, (n: number) => string> = {
	groups: (n) => m['creator.group_n']({ number: n }),
	slices: (n) => m['creator.slice_n']({ number: n }),
	dialog: (n) => m['creator.line_n']({ number: n }),
	events: (n) => m['creator.event_n']({ number: n })
};

export const problemMessage = (p: Problem): string => (p.expr && p.code?.startsWith('expr:') ? exprTextAt({ code: p.code, ...p.expr }) : p.message);

export function problemPlace(p: Problem, home?: string, groupLabel?: (index: number) => string): string {
	const parts = p.file === home ? [] : [fileLabel(p.file)];
	const path = p.path.at(-1) === 'text' ? p.path.slice(0, -1) : p.path;
	for (let i = 0; i < path.length; i++) {
		const key = path[i];
		const next = path[i + 1];
		if (key === 'groups' && typeof next === 'number' && i === 0 && p.file === home && groupLabel) {
			parts.push(groupLabel(next));
			i++;
		} else if (typeof key === 'string' && typeof next === 'number' && NUMBERED[key]) {
			parts.push(NUMBERED[key](next + 1));
			i++;
		} else parts.push(typeof key === 'number' ? `#${key + 1}` : String(key));
	}
	if (p.locale) parts.push(p.locale);
	return parts.join(' · ') || fileLabel(p.file);
}
