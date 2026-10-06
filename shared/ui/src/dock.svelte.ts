import { untrack, type Snippet } from 'svelte';
import type { PanelWidth } from './panel-width.svelte.ts';

/**
 * The AppShell's right dock (agents/DOCKING.md): ONE page panel at a time – the creator's Problems / History / Flow, the
 * Info (help) panel of every ActionBar, the admin's list detail (SplitView). A panel opens with `useDock`; opening one
 * closes the one shown before (its `close`, so its page's toggle follows – they take turns). The shell shows
 * `shellDock.current`, sizes it (`size`, a foldable `panelWidth`) and folds it to a rail labelled `title` / `icon`.
 * One shell per app: a module-level registry (the tools are client-only SPAs).
 */
export interface DockPanel {
	panel: Snippet;
	size: PanelWidth;
	/** shown on the rail when folded (Problems, History, Flow, Info, the detail's title) */
	title: string;
	/** Feather icon name on the rail */
	icon?: string;
	/** the page closes it (another panel took the dock) */
	close: () => void;
}

/** what fills the shell while maximized: the right dock, or a region of the main area by its id (the slice inspector) */
export type MaximizeTarget = 'dock' | (string & {});

class ShellDock {
	#current = $state.raw<{ owner: object; panel: DockPanel } | null>(null);
	#maximized = $state<MaximizeTarget | null>(null);

	/**
	 * The maximized region (owner 2026-10-06, agents/DOCKING.md "Maximize"): `'dock'` – the docked panel fills everything
	 * between the sidebar's rail and Ask; another id – that region of the main area does (the page hides the rest of its
	 * main area). The other regions are only hidden (their widths, folds, scroll positions and edits stay). One at a
	 * time; not remembered.
	 */
	get maximized(): MaximizeTarget | null {
		return this.#maximized;
	}

	/** maximizes `target` (null: restores); the dock opens unfolded for it */
	maximize(target: MaximizeTarget | null) {
		if (target === 'dock') {
			if (!this.#current) return;
			this.#current.panel.size.fold(false);
		}
		this.#maximized = target;
	}

	/** back to the normal layout (the button, Esc, Ctrl + Space, a click in the Flow that navigates the main area) */
	restore() {
		this.#maximized = null;
	}

	/** the docked panel (null = the dock is closed) */
	get current(): DockPanel | null {
		return this.#current?.panel ?? null;
	}

	/** the owner's panel opens (null: closes); a panel of another owner gives way */
	set(owner: object, panel: DockPanel | null) {
		const shown = this.#current;
		if (!panel) {
			if (shown?.owner === owner) {
				this.#current = null;
				if (this.#maximized === 'dock') this.#maximized = null;
			}
			return;
		}
		// another panel taking the dock while it is maximized stays maximized (owner 2026-10-06: the dock is what fills)
		if (shown && shown.owner !== owner) shown.panel.close();
		// a panel (re)opens unfolded; the same one updating (its snippet, its title) keeps its fold
		if (!shown || shown.owner !== owner || shown.panel.title !== panel.title) panel.size.fold(false);
		this.#current = { owner, panel };
	}
}

// once per page even when the dev server serves this module twice (stale `?t=` timestamps after branch switches): the
// shell would show one registry while the pages dock into the other (creator utils/shared.ts)
const SHELL_DOCK = Symbol.for('scutoons:shell-dock');
export const shellDock: ShellDock = ((globalThis as unknown as Record<symbol, ShellDock | undefined>)[SHELL_DOCK] ??= new ShellDock());

/**
 * Docks the component's open panel on the right while it is mounted (call during component init): `current()` says
 * which one is open or null. The shell makes room for it – left of Ask, never under it; with Ask open too the sidebar
 * folds to its rail. Pages never position panels themselves.
 */
export function useDock(current: () => DockPanel | null) {
	const owner = {};
	$effect(() => {
		const panel = current();
		// the registry itself is no dependency: another panel taking the dock closes this one through `close`
		untrack(() => shellDock.set(owner, panel));
	});
	// only on unmount (a re-run of the effect above must not drop and re-open the panel)
	$effect(() => () => shellDock.set(owner, null));
}
