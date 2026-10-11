// Review before publish (server docs/API.md "Reviews", agents/CUTOVER.md 10.28): an admin decides per toon who needs a
// review (`needs_review`) and who reviews (`reviewer`). For a needs-review account the one "Publish" button of every
// page becomes "Submit for review" (the same checklist – publish-tree.ts – plus a note); reviewers decide on the dashboard.
// The API enforces it – this only decides what the creator shows.
import { needsReview, type ReviewOverview } from '@scutoons/api';
import { contentApi } from '$lib/api.ts';
import { m } from '$lib/paraglide/messages.js';
import { shared } from '$lib/utils/shared.ts';

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
export const reviews = shared('reviews', () => new Reviews());

/** "Publish" or "Submit for review" – the label of the page's publish button */
export const publishLabel = (toon: string | null | undefined) => (reviews.needsReview(toon) ? m['creator.submit_review']() : m['creator.publish']());

