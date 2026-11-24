// The deck from the creator (lap2 A3, A3b; agents/research/redesign/lap2/DECK-LINK.md): the switcher's deck cards – the
// deck's workspace row (server/schema/tables/deck-link.json) with each one's last item from the deck's device row – and
// the hook the installed creator fills when the machine has no deck. Pure: test/deck.test.ts.
import { DECK_CARDS, deckLink, type DeckState } from '@scutoons/api';

export interface DeckCard {
	/** `deck-<workspace>`: its still is `workspace:deck-<workspace>` */
	id: string;
	/** the deck's own name of the workspace */
	workspace: string;
	/** its last item (an address), else null */
	item: string | null;
	/** the link that opens the deck there */
	href: string;
}

/** the deck's cards in the deck's order, each on its last item where the deck's row has one */
export function deckCards(state: DeckState | null): DeckCard[] {
	return DECK_CARDS.flatMap((workspace) => {
		const item = state?.last[workspace] ?? null;
		const href = deckLink(workspace, item);
		return href ? [{ id: `deck-${workspace}`, workspace, item, href }] : [];
	});
}

export interface DeckHooks {
	/** true: this machine has no deck to take the link */
	missing: () => boolean;
	/** what the creator offers instead (the deck's download) */
	offer: () => void;
}

const hooks: DeckHooks = { missing: () => false, offer: () => {} };

/**
 * Whether this machine has no deck. The web creator cannot tell and just tries the link (false); the installed creator
 * fills it from the system's handler for the scheme, with the download it offers instead (`fillDeckHooks`, TODO(#860)).
 */
export const deckMissing = (): boolean => hooks.missing();

export function fillDeckHooks(next: Partial<DeckHooks>): void {
	Object.assign(hooks, next);
}

/** a click on a deck link: it goes to the system, unless there is no deck – then the offer instead */
export function onDeckLink(event: Pick<MouseEvent, 'preventDefault'>): void {
	if (!deckMissing()) return;
	event.preventDefault();
	hooks.offer();
}
