// What is saved but not live, in every toon this member works on (the switcher's Not published, the bar's Publish dot):
// the one collector's list (publish-tree.ts `pendingFiles`, scenes and shows included). Sending it out is the bar's
// Publish dialog (publish-flow.svelte.ts, RULES H18).
import type { ContentIndexEntity } from '@scutoons/api';
import { pendingFiles } from '$lib/editor/publish-tree.ts';
import { shared } from '$lib/utils/shared.ts';

export type ToonRef = { id: string; name: string };

class NotPublished {
	list = $state.raw<ContentIndexEntity[]>([]);
	#for = '';

	/** the list for these toons (once per set of toons, unless `again`) */
	async load(toons: ToonRef[], again = false): Promise<void> {
		const ids = toons.map((t) => t.id).join(',');
		if (!ids || (ids === this.#for && !again)) return;
		this.#for = ids;
		try {
			const list = await pendingFiles({ kind: 'all', toons });
			this.list = list.sort((a, b) => (b.latest_at ?? '').localeCompare(a.latest_at ?? ''));
		} catch {
			this.list = [];
		}
	}
}

export const notPublished = shared('creator-not-published', () => new NotPublished());
