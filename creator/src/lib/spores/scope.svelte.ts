// The Spores tab's scope (#763, NETWORK.md §11): the front item offers its weave – a page that has a projector for what
// it shows (the chapter editor: the story's) offers it while it is open; the tab, in any workspace, draws what is offered.
import type { Spores } from '@scutoons/api';
import { shared } from '$lib/utils/shared.ts';

export interface Offer {
	/** the item's address, as the shell names it */
	address: string;
	/** its weave, computed when the tab draws (null: nothing to show yet) */
	weave: () => Spores.Weave | null;
}

class Scope {
	offer = $state<Offer | null>(null);

	/** `o` is what the front item shows from now on */
	set(o: Offer) {
		this.offer = o;
	}

	/** the item at `address` closed: its offer goes (another's stays) */
	drop(address: string) {
		if (this.offer?.address === address) this.offer = null;
	}
}

export const sporeScope = shared('creator-spore-scope', () => new Scope());
