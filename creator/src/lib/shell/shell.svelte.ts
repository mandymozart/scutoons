import { untrack, type Snippet } from 'svelte';
import type { ContentDoc } from '$lib/editor/doc.svelte.ts';
import type { PublishRefusal } from '$lib/editor/publish-report.ts';
import { shared } from '$lib/utils/shared.ts';
import { close, closeAll, closeOthers, EMPTY, frontTab, keepIn, readTabs, rename, retarget, stepTab, visit, type Tab, type TabsState } from './tabs.ts';
import { isGone, workspaceOf } from './address.ts';
import { entered, readUse, workspaceFor, type WorkspaceRef, type WorkspaceUse } from './workspaces.ts';
import { viewsOn, type Floats } from '@scutoons/ui/editor';
import { clampScale, DEFAULTS, readSettings, SETTINGS_KEY, type MachineSettings, type ViewSettings } from './settings.ts';
import { PRESETS, tabIdOf } from './presets.ts';
import { clampSplit, freeRoom } from './stage.ts';
import { readPicks, remember, type LaunchEntry, type LaunchPick } from './launcher.ts';
import { shownWorkspace } from './layouts.ts';
import { layouts } from './layouts.svelte.ts';

export const TABS_KEY = 'sc-creator-tabs';
const USE_KEY = 'sc-creator-workspaces';
const PICKS_KEY = 'sc-creator-launcher';

export function readJson(key: string): unknown {
	try {
		return JSON.parse(localStorage.getItem(key) ?? 'null');
	} catch {
		return null;
	}
}
export function writeJson(key: string, value: unknown): void {
	try {
		localStorage.setItem(key, JSON.stringify(value));
	} catch {
		// storage blocked or full: it holds for this visit
	}
}

/**
 * What the open item's page hands the shell: whether an autosave runs (the status bar's "saving…") and ↶ ↷ for the bar,
 * what the bar's one Publish dialog asks of it (RULES H18) and its documents for the History view.
 */
export interface PageDoc {
	saving?: boolean;
	errors?: number;
	warnings?: number;
	/** something saved is not live yet (the bar's Publish dot) */
	unpublished?: boolean;
	/** before the Publish dialog sends: what waits is saved (false: it stops – a conflict, a failed save) */
	prepare?: () => Promise<boolean>;
	/** after a publish: the page reads again what went live and shows its own outcome */
	published?: (result: { published: string[]; refused: PublishRefusal[] }) => void | Promise<void>;
	/** errors that make Publish ask once more before it sends (the problem list's) */
	blocking?: number;
	/** files the page holds back: unticked in the dialog, the reason as their hover text */
	held?: { key: string; reason: string }[];
	docs?: ContentDoc[];
	undo?: () => void;
	redo?: () => void;
}

/** a line of the ☰ menu that opens its own panel beside it */
export type MenuSub = 'views' | 'help' | 'go';

/**
 * The creator's shell (research/redesign/PLAN.md §2.4–§2.5, panels.html): the open tabs (the front one is the URL), this
 * machine's settings applied to the page, the panel tabs the front page fills (lib/shell/panels.ts) and its document.
 */
class Shell {
	state = $state<TabsState>(typeof localStorage === 'undefined' ? EMPTY : readTabs(readJson(TABS_KEY), isGone));
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
	/** the switcher lies on the overlay level (Ctrl K, ⌘K, Alt Tab where the page gets it, the workspace row's ⌄; Esc back) */
	dashboard = $state<false | 'cards'>(false);
	/** the ☰ menu is open, and which of its lines has its panel open beside it */
	menu = $state(false);
	menuSub = $state<MenuSub | null>(null);
	/** Search lies on the overlay level (Alt Space, Cmd Space; Esc closes) */
	launcher = $state(false);
	/** what Search starts with (a key typed in the switcher) */
	searchSeed = $state('');
	/** the user menu under the account icon */
	userMenu = $state(false);
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
		if (tab && tab.address !== was) this.#arrive(tab, was);
	}

	/** an item came to the front: it shows in its workspace (the one it remembers, else the front one if it fits, else its kind's) and remembers it */
	#arrive(tab: Tab, was: string | null) {
		const kind = workspaceOf(tab.address);
		if (!kind) return;
		const own = layouts.state.own;
		const wasKind = was ? workspaceOf(was) : null;
		const front = wasKind ? { kind: wasKind, id: shownWorkspace(layouts.chosen, own, wasKind) } : null;
		const picked = this.#picked?.kind === kind ? this.#picked.id : undefined;
		this.#picked = null;
		const id = workspaceFor(tab.address, picked ?? tab.workspace, front, layouts.chosen, own);
		if (!id) return;
		if (shownWorkspace(layouts.chosen, own, kind) !== id) layouts.show(kind, id);
		this.keepFrontIn(id);
	}

	/** a workspace tab picked whose item is not the front one: the item it opens comes in it */
	#picked: WorkspaceRef | null = null;
	/** a workspace tab picked: shown for its kind; the front item keeps it when it fits (`keeps`), else the item it opens comes in it */
	pickWorkspace(ref: WorkspaceRef, keeps: boolean) {
		layouts.show(ref.kind, ref.id);
		if (keeps) this.keepFrontIn(ref.id);
		else this.#picked = ref;
	}

	/** the front item remembers workspace `id` */
	keepFrontIn(id: string) {
		const front = this.state.front;
		if (!front) return;
		const next = keepIn(this.state, front, id);
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
	/** the page of the tab `by` steps along the strip from the front one (Ctrl Tab, Ctrl Shift Tab), else null */
	step(by: number): string | null {
		const tab = stepTab(this.state, by);
		return tab && tab.address !== this.state.front ? tab.href : null;
	}
	closeAll() {
		this.#set(closeAll());
	}
	/** the tabs as resumed (the start, another member's, items gone left out): no workspace counts a use */
	restore(next: TabsState) {
		if (next === this.state) return;
		this.state = next;
		writeJson(TABS_KEY, next);
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

	/** Search on the overlay level, or away again – the keys call it, and the pad's Y once the shell reads a pad; it takes the switcher's place */
	toggleLauncher(open = !this.launcher, seed = '') {
		this.searchSeed = open ? seed : '';
		if (open) this.dashboard = false;
		this.launcher = open;
	}
	/** the switcher on the overlay level, or away again; it takes Search's place */
	toggleSwitcher(open = !this.dashboard) {
		if (open) this.launcher = false;
		this.dashboard = open ? 'cards' : false;
	}
	remember(entry: LaunchEntry) {
		this.picks = remember(this.picks, entry);
		writeJson(PICKS_KEY, this.picks);
	}

	/** the ☰ menu with Views open beside it (Alt V, the launcher's Views) */
	openViews() {
		this.dashboard = false;
		this.menu = true;
		this.menuSub = 'views';
	}

	open(tab: string) {
		this.dashboard = false;
		this.menu = false;
		this.menuSub = null;
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
