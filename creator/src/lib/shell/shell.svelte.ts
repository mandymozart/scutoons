import { untrack, type Snippet } from 'svelte';
import type { ContentDoc } from '$lib/editor/doc.svelte.ts';
import { shared } from '$lib/utils/shared.ts';
import { close, closeAll, closeOthers, EMPTY, frontTab, readTabs, rename, retarget, stepTab, visit, type TabsState } from './tabs.ts';
import { workspaceOf } from './address.ts';
import { entered, readUse, type WorkspaceUse } from './workspaces.ts';
import { viewsOn, type Floats } from '@scutoons/ui/editor';
import { clampScale, DEFAULTS, readSettings, SETTINGS_KEY, type MachineSettings, type ViewSettings } from './settings.ts';
import { PRESETS, tabIdOf } from './presets.ts';
import { clampSplit, freeRoom } from './stage.ts';
import { readPicks, remember, type LaunchEntry, type LaunchPick } from './launcher.ts';
import { shownWorkspace } from './layouts.ts';
import { layouts } from './layouts.svelte.ts';

const TABS_KEY = 'sc-creator-tabs';
const USE_KEY = 'sc-creator-workspaces';
const PICKS_KEY = 'sc-creator-launcher';

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
 * What the open item's page hands the shell: whether an autosave runs (the status bar's "saving…") and ↶ ↷ for the bar,
 * its Publish for the bar's Publish menu and its documents for the History view. A page without documents of its own
 * (the scenes, the toon board) hands only its Publish.
 */
export interface PageDoc {
	saving?: boolean;
	errors?: number;
	warnings?: number;
	/** something saved is not live yet; false hides the bar's Publish, unknown (not given) keeps it */
	unpublished?: boolean;
	publish?: Snippet;
	docs?: ContentDoc[];
	undo?: () => void;
	redo?: () => void;
}

/**
 * The creator's shell (research/redesign/PLAN.md §2.4–§2.5, panels.html): the open tabs (the front one is the URL), this
 * machine's settings applied to the page, the panel tabs the front page fills (lib/shell/panels.ts) and its document.
 */
class Shell {
	state = $state<TabsState>(typeof localStorage === 'undefined' ? EMPTY : readTabs(readJson(TABS_KEY)));
	/** which workspaces were used how often and what each showed last (the bar's workspace row) */
	use = $state<WorkspaceUse>(typeof localStorage === 'undefined' ? readUse(null) : readUse(readJson(USE_KEY)));
	settings = $state<MachineSettings>(typeof localStorage === 'undefined' ? DEFAULTS : readSettings(readJson(SETTINGS_KEY)));
	/** the front page's document (null: the page has nothing to save or publish) */
	doc = $state.raw<PageDoc | null>(null);
	/** the panel tabs the front page fills, by tab id (lib/shell/panels.ts) */
	tabs = $state.raw<Record<string, Snippet>>({});
	/** the front workspace's panels (set by the workspace host: the Views menu reads and changes them) */
	floats = $state.raw<Floats | null>(null);
	/** the tabs with content in the front workspace (the Views list) */
	available = $state.raw<ReadonlySet<string>>(new Set());
	/** the front workspace's kind */
	kind = $state<string>('home');
	/** a tab (or a panel) asked to open in the front workspace (the workspace takes it and clears it) */
	wanted = $state<string | null>(null);
	/** the dashboard lies over everything (the menu button, Esc back); `views`: with the front workspace's Views open */
	dashboard = $state<false | 'cards' | 'views'>(false);
	/** the launcher lies centred over everything (Alt Space, Cmd Space, Ctrl K; Esc closes) */
	launcher = $state(false);
	/** the launcher's recent picks, newest first */
	picks = $state<LaunchPick[]>(typeof localStorage === 'undefined' ? [] : readPicks(readJson(PICKS_KEY)));

	readonly front = $derived(frontTab(this.state));
	/** the front workspace's id: its kind, or the own workspace made from it that is chosen for it */
	readonly workspace = $derived(shownWorkspace(layouts.chosen, layouts.state.own, this.kind));
	/** the views on the stage in the front workspace (the floats keep them); empty: the workspace's own view */
	readonly stage: readonly string[] = $derived(this.floats ? viewsOn(this.floats.stage) : []);
	/** the view over everything – the bar, the panels and the status bar hidden – or none */
	readonly full: string | null = $derived(this.floats?.full ?? null);
	/** where a split of the workspace's own view divides it */
	readonly split = $derived(clampSplit(this.view(this.kind).split ?? 0.5));
	/** the front workspace's content keeps clear of the bar's row */
	readonly clear = $derived(this.view(this.kind).clear ?? PRESETS[this.kind as keyof typeof PRESETS]?.clear ?? true);

	#set(next: TabsState) {
		if (next === this.state) return;
		const was = this.state.front;
		this.state = next;
		writeJson(TABS_KEY, next);
		const tab = frontTab(next);
		const use = entered(this.use, was ? workspaceOf(was) : null, tab?.address ?? null, tab?.href ?? '');
		if (use !== this.use) {
			this.use = use;
			writeJson(USE_KEY, use);
		}
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
	/** the page of the tab `by` steps along the strip from the front one (Ctrl Tab, Ctrl Shift Tab), else null */
	step(by: number): string | null {
		const tab = stepTab(this.state, by);
		return tab && tab.address !== this.state.front ? tab.href : null;
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

	view(kind: string): ViewSettings {
		return this.settings.views[kind] ?? {};
	}
	changeView(patch: ViewSettings) {
		this.change({ views: { ...this.settings.views, [this.kind]: { ...this.view(this.kind), ...patch } } });
	}
	setSplit(split: number) {
		this.changeView({ split: clampSplit(split) });
	}
	/** the front workspace back to its start – the last save, else its craft's default, else its preset: its panels, tabs and the stage */
	resetView() {
		this.floats?.reset();
		const { [this.kind]: _gone, ...views } = this.settings.views;
		this.change({ views });
	}

	/** the launcher over everything, or away again – the keys call it, and the pad's Y once the shell reads a pad */
	toggleLauncher(open = !this.launcher) {
		this.launcher = open;
	}
	remember(entry: LaunchEntry) {
		this.picks = remember(this.picks, entry);
		writeJson(PICKS_KEY, this.picks);
	}

	open(tab: string) {
		this.dashboard = false;
		this.wanted = tabIdOf(tab);
	}
}

export const shell = shared('creator-shell', () => new Shell());

/**
 * A page or a part of it fills these panel tabs while it is mounted (call during component init) – the one way content
 * reaches a panel; a tab without content is not offered. Several parts may fill: each takes back only its own.
 */
export function useTabs(tabs: () => Record<string, Snippet | null | undefined>) {
	$effect(() => {
		const mine = Object.entries(tabs()).filter((e): e is [string, Snippet] => !!e[1]);
		untrack(() => (shell.tabs = { ...shell.tabs, ...Object.fromEntries(mine) }));
		return () =>
			untrack(() => {
				const next = { ...shell.tabs };
				for (const [id, snippet] of mine) if (next[id] === snippet) delete next[id];
				shell.tabs = next;
			});
	});
}

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

/** the room a workspace's shown panels leave free across the window (client px) */
export function freeOf(floats: Floats | null, W: number): { left: number; right: number } {
	if (!floats) return { left: 0, right: W };
	return freeRoom(
		Object.entries(floats.views)
			.filter(([id]) => !floats.closed(id) && floats.shown.has(id))
			.map(([, v]) => v.rect),
		W
	);
}

/** a view on the stage or over everything goes back into its panel (the Flow after a box opened its place: the page shows it) */
export function stepBack(tab: string) {
	const floats = shell.floats;
	if (floats && floats.stateOf(tab) !== 'panel' && floats.stateOf(tab) !== 'hidden') floats.setState(tab, 'panel');
}
