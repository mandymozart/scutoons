// The member's saved workspace layouts (layouts.ts): read once from the server, kept in this browser too – the copy used
// when the API is missing or fails (before its migration, offline); a save always lands here and goes to the server when
// it answers. Nothing here throws: a save the server refused says so (`'browser'`).
import { createLayoutsApi } from '@scutoons/api';
import { api } from '$lib/api.ts';
import { shared } from '$lib/utils/shared.ts';
import { NO_LAYOUTS, readLayouts, readChosen, readKept, startKind, startOf, toKept, withSaved, without, type LayoutsState } from './layouts.ts';
import type { WorkspaceKind } from './address.ts';
import { readJson, writeJson, writesHeld } from './kept.ts';

const KEPT_KEY = 'sc-creator-layouts';
export const CHOSEN_KEY = 'sc-creator-chosen';

const layoutsApi = createLayoutsApi(api);

/** where a save landed: with the account, or only in this browser (the server did not take it) */
export type SavedWhere = 'account' | 'browser';

class Layouts {
	state = $state.raw<LayoutsState>(typeof localStorage === 'undefined' ? NO_LAYOUTS : readKept(readJson(KEPT_KEY)));
	/** the own workspace each kind shows, by kind (kept in this browser) */
	chosen = $state<Record<string, string>>(typeof localStorage === 'undefined' ? {} : readChosen(readJson(CHOSEN_KEY)));
	/** the server answered with its layouts */
	fromServer = $state(false);
	private loading: Promise<void> | null = null;

	/** once per page; a missing or failing API keeps this browser's copy */
	load(read: () => Promise<unknown> = layoutsApi.mine): Promise<void> {
		this.loading ??= readLayouts(read).then((state) => {
			if (!state) return;
			this.fromServer = true;
			this.#set(state);
		});
		return this.loading;
	}

	startOf(id: string): unknown {
		return startOf(this.state, id);
	}
	startKind(id: string) {
		return startKind(this.state, id);
	}

	#set(state: LayoutsState) {
		this.state = state;
		writeJson(KEPT_KEY, toKept(state));
	}

	/** the workspace's layout becomes its saved state; `own`: a new own workspace (or its new name) */
	async save(id: string, layout: unknown, own?: { name: string; kind: WorkspaceKind }): Promise<SavedWhere> {
		this.#set(withSaved(this.state, id, layout, own));
		if (writesHeld()) return 'browser';
		try {
			await layoutsApi.save(id, { layout, ...(own ? { name: own.name, kind: own.kind } : {}) });
			return 'account';
		} catch {
			return 'browser';
		}
	}

	/** a save taken back: a creator workspace back to its default, an own workspace gone */
	async remove(id: string): Promise<void> {
		this.#set(without(this.state, id));
		this.choose(Object.fromEntries(Object.entries(this.chosen).filter(([, shown]) => shown !== id)));
		if (writesHeld()) return;
		try {
			await layoutsApi.remove(id);
		} catch {
			// kept out of this browser at least
		}
	}

	/** which workspace a kind shows (its own id, or an own workspace's) */
	show(kind: string, id: string) {
		this.choose({ ...this.chosen, [kind]: id });
	}

	private choose(chosen: Record<string, string>) {
		this.chosen = chosen;
		writeJson(CHOSEN_KEY, chosen);
	}
}

export const layouts = shared('creator-layouts', () => new Layouts());
