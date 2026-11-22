import { untrack, type Snippet } from 'svelte';
import type { ContentDoc } from '$lib/editor/doc.svelte.ts';
import { shared } from '$lib/utils/shared.ts';
import { close, closeAll, closeOthers, EMPTY, frontTab, readTabs, rename, retarget, visit, type TabsState } from './tabs.ts';
import type { Floats } from '@scutoons/ui/editor';
import { clampScale, DEFAULTS, readSettings, SETTINGS_KEY, type MachineSettings, type ViewSettings } from './settings.ts';
import { PRESETS, stageOf, tabIdOf } from './presets.ts';
import { back, clampSplit, send, splitIn, type Side, type Stage } from './stage.ts';

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
 * What the open item's page hands the shell: its save state and ↶ ↷ for the bar and the status bar, its Publish for the
 * bar's Publish menu and its documents for the bar's History menu (#687). A page without documents of its own (the
 * scenes, the toon board) hands only its Publish.
 */
export interface PageDoc {
	dirty?: boolean;
	saving?: boolean;
	errors?: number;
	warnings?: number;
	/** something saved is not live yet */
	unpublished?: boolean;
	save?: () => void;
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
	settings = $state<MachineSettings>(typeof localStorage === 'undefined' ? DEFAULTS : readSettings(readJson(SETTINGS_KEY)));
	/** the front page's document (null: the page has nothing to save or publish) */
	doc = $state.raw<PageDoc | null>(null);
	/** the panel tabs the front page fills, by tab id (lib/shell/panels.ts) */
	tabs = $state.raw<Record<string, Snippet>>({});
	/** the front workspace's panels (set by the workspace host: the Views menu reads and changes them) */
	floats = $state.raw<Floats | null>(null);
	/** the front workspace's kind */
	kind = $state<string>('home');
	/** a tab (or a panel) asked to open in the front workspace (the workspace takes it and clears it) */
	wanted = $state<string | null>(null);
	/** the tab over everything (⤢): a dialog over the whole creator */
	over = $state<string | null>(null);

	readonly front = $derived(frontTab(this.state));
	/** the tabs on the stage in the front workspace (◱ main), left to right; empty: the workspace's own view */
	readonly stage = $derived(stageOf(this.kind, this.view(this.kind)));
	/** where a split divides the stage */
	readonly split = $derived(clampSplit(this.view(this.kind).split ?? 0.5));
	/** set by the stage while it is drawn: it measures, applies the change and flies what moved (stage.svelte) */
	mover: ((apply: () => void, was: Stage, now: Stage) => void) | null = null;
	/** the front workspace's content keeps clear of the bar's row */
	readonly clear = $derived(this.view(this.kind).clear ?? PRESETS[this.kind as keyof typeof PRESETS]?.clear ?? true);

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

	view(kind: string): ViewSettings {
		return this.settings.views[kind] ?? {};
	}
	changeView(patch: ViewSettings) {
		this.change({ views: { ...this.settings.views, [this.kind]: { ...this.view(this.kind), ...patch } } });
	}
	#stage(next: Stage) {
		const was = this.stage;
		if (next.length === was.length && next.every((t, i) => t === was[i])) return;
		const apply = () => this.changeView({ stage: [...next], ...(next.length === 2 && was.length < 2 ? { split: 0.5 } : {}) });
		if (this.mover) this.mover(apply, was, next);
		else apply();
	}
	/** ◱ main: the tab alone on the stage (whatever was there goes back); pressed again it goes back itself */
	send(tab: string) {
		this.#stage(send(this.stage, tab));
	}
	/** a tab dropped on a half of the stage: a split, or that half replaced */
	splitIn(tab: string, side: Side) {
		this.#stage(splitIn(this.stage, tab, side));
	}
	/** a tab back into its panel (none named: the last one, as Esc) */
	back(tab?: string) {
		this.#stage(back(this.stage, tab));
	}
	/** every tab back: the workspace's own view takes the stage */
	home() {
		this.#stage([]);
	}
	setSplit(split: number) {
		this.changeView({ split: clampSplit(split) });
	}
	/** the front workspace back to its preset: its panels, tabs and the stage */
	resetView() {
		this.floats?.reset();
		const { [this.kind]: _gone, ...views } = this.settings.views;
		this.change({ views });
	}

	open(tab: string) {
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

/** a tab on the stage goes back into its panel (the Flow after a box opened its place: the page shows it) */
export function stepBack(tab: string) {
	if (shell.stage.includes(tab)) shell.back(tab);
}
