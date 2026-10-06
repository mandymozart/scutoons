// Accepted warnings on an editor page (@scutoons/content accepted.ts): a warning is stored in the document it is about,
// in the warning's locale (the source locale for one about every locale). A language-neutral document (chapter, scene,
// season) holds every language's: a warning about one language ("missing en text") keeps its locale in the key.
// Used by the Problems panel and the counts.
import { isAcceptedWarning, withAcceptedWarning, type AcceptableWarning, type Problem } from '@scutoons/content';
import type { ContentDoc } from './doc.svelte.ts';

/** the document and locale a warning is stored in; null for an error / note, a file the page doesn't hold or a
 *  locale without a document */
export function acceptStore(docs: readonly ContentDoc[], p: Problem): { doc: ContentDoc; locale: string } | null {
	if (p.level !== 'warning') return null;
	const doc = docs.find((d) => d.entity === p.file);
	const locale = p.locale && doc?.locales.includes(p.locale) ? p.locale : doc?.primary;
	return doc && locale && doc.data[locale] ? { doc, locale } : null;
}

/** the warning as it is keyed: a neutral document's per-locale warning with its locale */
const keyed = (doc: ContentDoc, p: Problem): AcceptableWarning => ({ path: p.path, message: p.message, scope: doc.neutral ? p.locale : null });

export function isAccepted(docs: readonly ContentDoc[], p: Problem): boolean {
	const at = acceptStore(docs, p);
	return !!at && isAcceptedWarning(keyed(at.doc, p), at.doc.data[at.locale]);
}

/** accepts the warning, or takes the acceptance back – an edit of that document (unsaved, undoable) */
export function toggleAccepted(docs: readonly ContentDoc[], p: Problem): void {
	const at = acceptStore(docs, p);
	if (!at) return;
	const data = at.doc.data[at.locale];
	const warning = keyed(at.doc, p);
	at.doc.setLocal(at.locale, ['accepted_warnings'], withAcceptedWarning(warning, data, !isAcceptedWarning(warning, data)));
}
