// Review before publish (server docs/API.md "Reviews", agents/CUTOVER.md 10.28): an admin decides per toon who needs a
// review (`needs_review`) and who reviews (`reviewer`). For a needs-review account the one "Publish" button of every
// page becomes "Submit for review" (same files, same dialog structure, plus a note); reviewers decide on the dashboard.
// The API enforces it – this only decides what the creator shows.
import { askDialog, confirmDialog } from '@scutoons/ui';
import { NEUTRAL_LOCALE } from '@scutoons/content';
import { needsReview, type ReviewOverview } from '@scutoons/api';
import { contentApi } from '$lib/api.ts';
import { fileLabels } from '$lib/labels.ts';
import { m } from '$lib/paraglide/messages.js';
import type { ContentDoc } from '$lib/editor/doc.svelte.ts';

class Reviews {
	overview = $state<ReviewOverview | null>(null);

	async load(): Promise<void> {
		try {
			this.overview = await contentApi.reviews();
		} catch {
			this.overview = null; // before the migration / offline: the creator publishes as before, the API still decides
		}
	}

	/** Does this account submit instead of publishing in this toon? */
	needsReview(toon: string | null | undefined): boolean {
		return needsReview(this.overview, toon);
	}
}

/** The signed-in account's review state (loaded by the layout once the creator is known) */
export const reviews = new Reviews();

/** "Publish" or "Submit for review" – the label of the page's publish button */
export const publishLabel = (toon: string | null | undefined) => (reviews.needsReview(toon) ? m['creator.submit_review']() : m['creator.publish']());

/**
 * The submit dialog (the publish dialog's structure + a note). Returns the note ('' = none) or null when cancelled;
 * with errors in the problem list it asks once more, like publishing.
 */
export async function confirmSubmit(files: ContentDoc[], errorCount = 0): Promise<string | null> {
	if (errorCount && !(await confirmDialog({ title: m['creator.submit_title'](), message: m['creator.publish_errors']({ count: errorCount }), confirmLabel: m['creator.submit_send']() }))) return null;
	const locales = [...new Set(files.flatMap((d) => [...d.dirtyLocales, ...d.unpublished]))].filter((l) => l !== NEUTRAL_LOCALE).join(', ');
	const file = fileLabels(files.map((d) => d.entity));
	return askDialog({
		title: m['creator.submit_title'](),
		message: locales ? m['creator.submit_confirm']({ file, locales }) : m['creator.submit_confirm_neutral']({ file }),
		label: m['creator.submit_note_label'](),
		placeholder: m['creator.submit_note_placeholder'](),
		confirmLabel: m['creator.submit_send']()
	});
}

/** Submits the saved, unpublished versions of these docs (after the save); returns the notice for the page */
export async function submitForReview(docs: ContentDoc[], note: string): Promise<string> {
	const items = docs.flatMap((doc) =>
		doc.unpublished.flatMap((l) => (doc.base[l] ? [{ key: doc.key(l), version: doc.base[l] as number }] : []))
	);
	if (!items.length) return m['creator.nothing_to_publish']();
	const result = await contentApi.submitReview(items, note.trim());
	void reviews.load();
	return m['creator.submitted']({ reviewers: result.reviewers.join(', ') });
}
