// The link that opens the deck: its grammar, the deck's cards and the deck's device row are server/schema/tables/
// deck-link.json, the one definition the deck reads too (#873); both run deck-link.vectors.json. Where an item opens in the
// deck is the core's (deck-catalog `library::open`): the caller asks it and passes its workspace. Pure: test/deck-link.test.ts.
import table from '../../../server/schema/tables/deck-link.json' with { type: 'json' };

/** the link's scheme (`scutoons-deck`) */
export const DECK_SCHEME: string = table.scheme;
/** what the deck's device row names as its `app` (GET /api/me/session?app=) */
export const DECK_APP: string = table.app;
/** the deck's workspace row in its order: the switcher's deck cards */
export const DECK_CARDS: readonly string[] = table.cards;

const WORD = /^[a-z][a-z0-9-]*$/;
const ADDRESS = /^[a-z][a-z0-9-]*:.+$/s;

const isWord = (v: unknown): v is string => typeof v === 'string' && WORD.test(v);
const isAddress = (v: unknown): v is string => typeof v === 'string' && ADDRESS.test(v);

/** every byte outside A-Z a-z 0-9 - . _ ~ : / as UTF-8 percent-encoding, upper-case hex */
const encode = (address: string): string =>
	encodeURIComponent(address)
		.replace(/[!'()*]/g, (c) => `%${c.charCodeAt(0).toString(16).toUpperCase()}`)
		.replace(/%3A/g, ':')
		.replace(/%2F/g, '/');

/** the link opening the deck on `workspace` and, given, the item at `address`; null when either is not one */
export function deckLink(workspace: string, address: string | null = null): string | null {
	if (!isWord(workspace)) return null;
	if (address === null) return `${DECK_SCHEME}://${workspace}`;
	return isAddress(address) ? `${DECK_SCHEME}://${workspace}/${encode(address)}` : null;
}

/** what the deck's device row says: the workspace in front and each workspace's last item */
export interface DeckState {
	workspace: string | null;
	last: Record<string, string>;
}

const isRecord = (v: unknown): v is Record<string, unknown> => typeof v === 'object' && v !== null && !Array.isArray(v);

/** the deck's device row state, checked (not the deck's: null); a workspace or an item that is not one is left out */
export function readDeckState(raw: unknown): DeckState | null {
	if (!isRecord(raw) || raw.app !== DECK_APP) return null;
	const last = isRecord(raw.last) ? Object.fromEntries(Object.entries(raw.last).filter(([w, a]) => isWord(w) && isAddress(a))) : {};
	return { workspace: isWord(raw.workspace) ? raw.workspace : null, last: last as Record<string, string> };
}
