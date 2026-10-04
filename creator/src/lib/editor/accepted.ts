// Accepted warnings on an editor page (@scutoons/content accepted.ts): a warning is stored in the document it is about,
// in the warning's locale (the source locale for one about every locale). Used by the Problems panel and the counts.
import { isAcceptedWarning, withAcceptedWarning, type Problem } from '@scutoons/content';
import type { ContentDoc } from './doc.svelte.ts';

/** the document and locale a warning is stored in; null for an error / note, a file the page doesn't hold or a
 *  locale without a document */
export function acceptStore(docs: readonly ContentDoc[], p: Problem): { doc: ContentDoc; locale: string } | null {
	if (p.level !== 'warning') return null;
	const doc = docs.find((d) => d.entity === p.file);
	const locale = p.locale && doc?.locales.includes(p.locale) ? p.locale : doc?.primary;
	return doc && locale && doc.data[locale] ? { doc, locale } : null;
}

export function isAccepted(docs: readonly ContentDoc[], p: Problem): boolean {
	const at = acceptStore(docs, p);
	return !!at && isAcceptedWarning(p, at.doc.data[at.locale]);
}

/** accepts the warning, or takes the acceptance back – an edit of that document (unsaved, undoable) */
export function toggleAccepted(docs: readonly ContentDoc[], p: Problem): void {
	const at = acceptStore(docs, p);
	if (!at) return;
	const data = at.doc.data[at.locale];
	at.doc.setLocal(at.locale, ['accepted_warnings'], withAcceptedWarning(p, data, !isAcceptedWarning(p, data)));
}
