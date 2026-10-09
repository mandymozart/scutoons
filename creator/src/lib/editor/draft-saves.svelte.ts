// Every draft this creator tab stored (BACKLOG 10.49, UI-CONCEPTS E.6): the dock's Reader reloads the viewer's draft
// preview after each one – a `ContentDoc.save` from any page (DocSession's Save, the preview link's save) reports here.
// Only a counter and the toon: the Reader decides whether the
// save concerns what it shows.
import { shared } from '$lib/utils/shared.ts';

class DraftSaves {
	/** increases with every stored draft */
	seq = $state(0);
	/** the toon of the last stored draft ('' for platform-wide entities) */
	toon = $state('');

	report(toon: string) {
		this.toon = toon;
		this.seq++;
	}
}

export const draftSaves = shared('draft-saves', () => new DraftSaves());
