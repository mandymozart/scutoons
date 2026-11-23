// What is saved but not live, in every toon this member works on (the dashboard's Not published, the bar's Publish when
// nothing is open): the one collector's list (publish-tree.ts `pendingFiles`, scenes and shows included) and its Publish.
import type { ContentIndexEntity } from '@scutoons/api';
import { pendingFiles, publishAndLog } from '$lib/editor/publish-tree.ts';
import { m } from '$lib/paraglide/messages.js';
import { shared } from '$lib/utils/shared.ts';

export type ToonRef = { id: string; name: string };

class NotPublished {
	list = $state.raw<ContentIndexEntity[]>([]);
	busy = $state(false);
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

	/** the list to tick, everything not live in these toons */
	async publishAll(toons: ToonRef[]): Promise<void> {
		this.busy = true;
		try {
			if (await publishAndLog({ kind: 'all', toons }, m['creator.publish_tree_title_all']())) await this.load(toons, true);
		} finally {
			this.busy = false;
		}
	}
}

export const notPublished = shared('creator-not-published', () => new NotPublished());
