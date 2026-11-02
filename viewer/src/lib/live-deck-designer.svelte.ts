// /live/deck's edit mode (#282, owner 2026-11-02: "performance and edit (rehearsal) – everything for editing must cost
// nothing in performance mode"). Off (every load, unless this device left it on): no tab row, no designer code, no
// CodeMirror, nothing fetched. On: @scutoons/rig-designer comes in with a dynamic import() – the first time only – and
// the tab row appears (Deck · Parameters · Nodes · Rust · Assignment). The deck page imports NOTHING from the package
// statically (types only, erased), and the creator's transport strip is a dynamic import too – a member's page loads neither. Switching the mode or a tab hides, never unmounts, the deck: engine, relay and being run on.
import type { DesignerTabId, RigDesigner } from '@scutoons/rig-designer';
import { getSessionToken } from '@scutoons/api';
import { API_URL } from '$lib/config.ts';
import { getLocale } from '$lib/paraglide/runtime.js';
import { EDIT_KEY, readEditPref } from '$lib/live-deck.ts';

type DesignerModule = typeof import('@scutoons/rig-designer');

export class DeckEdit {
	/** the creator wants edit mode (remembered per device) */
	on = $state(false);
	tab = $state<DesignerTabId>('deck');
	/** the package, once loaded (a module namespace: not proxied) */
	mod = $state.raw<DesignerModule | null>(null);
	designer = $state.raw<RigDesigner | null>(null);
	loading = $state(false);
	failed = $state(false);

	/** the creator's transport strip (a viewer component, loaded for creators only – everybody else's page never fetches it) */
	Strip = $state.raw<typeof import('$lib/components/live-deck/deck-transport.svelte').default | null>(null);

	async loadStrip(): Promise<void> {
		if (this.Strip) return;
		try {
			this.Strip = (await import('$lib/components/live-deck/deck-transport.svelte')).default;
		} catch (e) {
			console.warn('live deck: the transport strip did not load', e);
		}
	}

	/** the saved choice of this device (a throwing or empty localStorage is "off") */
	restore(): void {
		this.on = readEditPref(() => {
			try {
				return localStorage.getItem(EDIT_KEY);
			} catch {
				return null;
			}
		});
	}

	set(on: boolean): void {
		this.on = on;
		try {
			localStorage.setItem(EDIT_KEY, on ? '1' : '0');
		} catch {
			/* private mode */
		}
		// off: back to the Deck tab, the tab row and the editor go (the loaded code stays in memory, nothing of it runs)
		if (!on) this.tab = 'deck';
	}

	/** the package, loaded on the first edit (a failed load can be tried again by switching the mode off and on) */
	async ensure(): Promise<void> {
		if (this.mod || this.loading) return;
		this.loading = true;
		this.failed = false;
		try {
			const mod = await import('@scutoons/rig-designer');
			mod.setupRigDesigner({ locale: getLocale });
			this.mod = mod;
		} catch (e) {
			console.warn('live deck: the designer did not load', e);
			this.failed = true;
		} finally {
			this.loading = false;
		}
	}

	/** the Rust tab's state is made when it is first opened, then kept (its files survive Edit off / on) */
	open(id: DesignerTabId): void {
		this.tab = id;
		if (id === 'rust' && this.mod && !this.designer) {
			this.designer = new this.mod.RigDesigner(this.mod.createDesignerApi({ fetch: (...a) => fetch(...a), baseUrl: API_URL, getToken: getSessionToken }));
		}
	}
}
