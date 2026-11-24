// The deck's device row (lib/shell/deck.ts): read each time the switcher opens, so its cards show what the deck has open
// now; no API, offline or no deck yet: its cards with nothing open.
import { createDeviceSessionApi, DECK_APP, readDeckState, type DeckState } from '@scutoons/api';
import { api } from '$lib/api.ts';
import { shared } from '$lib/utils/shared.ts';
import { devices } from './devices.svelte.ts';

const sessionApi = createDeviceSessionApi(api);

class DeckRow {
	state = $state.raw<DeckState | null>(null);
	/** when the deck last wrote its row */
	at = $state<string | null>(null);

	async read(): Promise<void> {
		if (!devices.meta.id) return;
		try {
			const rows = await sessionApi.mine(devices.meta.id, DECK_APP);
			this.state = readDeckState(rows.app?.state);
			this.at = this.state ? (rows.app?.updated_at ?? null) : null;
		} catch {
			this.state = null;
			this.at = null;
		}
	}
}

export const deckRow = shared('creator-deck-row', () => new DeckRow());
