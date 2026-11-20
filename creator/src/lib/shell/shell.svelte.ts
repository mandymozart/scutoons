import type { Snippet } from 'svelte';
import type { ContentDoc } from '$lib/editor/doc.svelte.ts';
import { shared } from '$lib/utils/shared.ts';
import { close, closeAll, closeOthers, EMPTY, frontTab, readTabs, rename, retarget, visit, type TabsState } from './tabs.ts';
import { clampScale, DEFAULTS, readSettings, SETTINGS_KEY, type MachineSettings } from './settings.ts';

const TABS_KEY = 'sc-creator-tabs';

function readJson(key: string): unknown {
	try {
		return JSON.parse(localStorage.getItem(key) ?? 'null');
	} catch {
		return null;
	}
}
function writeJson(key: string, value: unknown): void {
	try {
		localStorage.setItem(key, JSON.stringify(value));
	} catch {
		// storage blocked or full: it holds for this visit
	}
}

/**
 * What the open item's page hands the shell (meta-page): its save state and ↶ ↷ for the bar, its Publish · Problems as
 * panel tabs and its documents for the one History panel (#687).
 */
export interface PageDoc {
	dirty: boolean;
	saving: boolean;
	errors: number;
	warnings: number;
	save: () => void;
	publish: Snippet;
	problems: Snippet;
	docs: ContentDoc[];
	undo: () => void;
	redo: () => void;
}

/**
 * The creator's shell (research/redesign/PLAN.md §2.4–§2.5): the open tabs (the front one is the URL), this machine's
 * settings applied to the page, and what the front page lends to the panels – its own list (the Tree) and its document.
 */
class Shell {
	state = $state<TabsState>(typeof localStorage === 'undefined' ? EMPTY : readTabs(readJson(TABS_KEY)));
	settings = $state<MachineSettings>(typeof localStorage === 'undefined' ? DEFAULTS : readSettings(readJson(SETTINGS_KEY)));
	/** the front page's document (null: the page saves on its own, or has nothing to save) */
	doc = $state.raw<PageDoc | null>(null);
	/** asked by a menu line or ＋: a panel to open in the front workspace (the workspace takes it and clears it) */
	wanted = $state<string | null>(null);
	/** a panel of the front workspace to open or close (the main menu's list) */
	toggled = $state<string | null>(null);
	/** the front workspace's panels that can close, ticked = open (the main menu's list) */
	panels = $state<{ id: string; name: string; open: boolean }[]>([]);
	/** the front workspace's layout back to its defaults */
	resetAsked = $state(false);
	/** what the open panels at the walls and the page's bottom bar take (rem): the page host gives way */
	insets = $state({ left: 0, right: 0, bottom: 0 });

	readonly front = $derived(frontTab(this.state));

	#set(next: TabsState) {
		if (next === this.state) return;
		this.state = next;
		writeJson(TABS_KEY, next);
	}

	visit(address: string | null, href: string, label?: string) {
		this.#set(visit(this.state, address, href, label));
	}
	retarget(address: string | null, href: string) {
		this.#set(retarget(this.state, address, href));
	}
	rename(address: string, label: string) {
		this.#set(rename(this.state, address, label));
	}
	/** closes a tab; the href to go to when it was the front one ('/' = Home), else null */
	close(address: string): string | null {
		const wasFront = this.state.front === address;
		this.#set(close(this.state, address));
		return wasFront ? (this.front?.href ?? '/') : null;
	}
	closeOthers() {
		this.#set(closeOthers(this.state));
	}
	closeAll() {
		this.#set(closeAll());
	}

	change(patch: Partial<MachineSettings>) {
		const next = { ...this.settings, ...patch };
		next.scale = clampScale(next.scale);
		this.settings = next;
		writeJson(SETTINGS_KEY, next);
	}

	open(panel: string) {
		this.wanted = panel;
	}
}

export const shell = shared('creator-shell', () => new Shell());

/** The page hands its document to the shell while it is mounted (call during component init) */
export function usePageDoc(doc: () => PageDoc | null) {
	$effect(() => {
		const value = doc();
		shell.doc = value;
		return () => {
			if (shell.doc === value) shell.doc = null;
		};
	});
}
