// Review before publish (server docs/API.md "Reviews", agents/CUTOVER.md 10.28): an admin decides per toon who needs a
// review (`needs_review`) and who reviews (`reviewer`). For a needs-review account the bar's Publish dialog submits for
// review (publish-flow.svelte.ts, plus a note); a reviewer publishes a submit there too and sends it back on the switcher.
// The API enforces it – this only decides what the creator shows.
import { needsReview, type ContentReview, type ReviewOverview } from '@scutoons/api';
import { contentApi } from '$lib/api.ts';
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

/** one submit: the reviews of the same author, time, toon and state */
export interface ReviewGroup {
	id: string;
	toon: string;
	items: ContentReview[];
}

/** the reviews of one submit together, in the list's order */
export function reviewGroups(list: ContentReview[]): ReviewGroup[] {
	const out = new Map<string, ReviewGroup>();
	for (const r of list) {
		const id = `${r.author?.id ?? ''}|${r.created_at}|${r.toon}|${r.state}`;
		if (!out.has(id)) out.set(id, { id, toon: r.toon, items: [] });
		out.get(id)!.items.push(r);
	}
	return [...out.values()];
}

