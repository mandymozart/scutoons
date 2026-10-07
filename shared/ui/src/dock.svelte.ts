import { untrack, type Component, type Snippet } from 'svelte';
import type { PanelWidth } from './panel-width.svelte.ts';

/**
 * The AppShell's right dock (agents/DOCKING.md): ONE page panel at a time – the creator's Problems / History / Flow, the
 * Info (help) panel of every ActionBar, the admin's list detail (SplitView). A panel opens with `useDock`; opening one
 * closes the one shown before (its `close`, so its page's toggle follows – they take turns). The shell shows
 * `shellDock.current`, sizes it (`size`, a foldable `panelWidth`) and folds it to a rail labelled `title` / `icon`.
 * One shell per app: a module-level registry (the tools are client-only SPAs).
 *
 * Panels that survive navigation (UI-CONCEPTS E.3 b): a panel with a `kind` and a `content` (a component + its props)
 * is mounted by the shell, not by the page. When the page leaves, the panel is HELD (still shown, its last props) until
 * the next page docks the same kind – it takes the mounted instance over and hands it its own props (no remount, no
 * flash: the Flow keeps its zoom, Info its frame) – or, when nothing takes it, until the end of the navigation (the
 * task after the old page left; a `DockHold` on a page that is still loading keeps it until that page has decided). A
 * page that opens asks `shellDock.has(kind)` whether such a panel is docked and opens its own then (the Flow stays open
 * toon → season → chapter). A panel with only a `panel` snippet belongs to its page and goes with it.
 */
export interface DockContent {
	// any component: the props are the component's own
	// eslint-disable-next-line @typescript-eslint/no-explicit-any
	component: Component<any>;
	/** read by the shell (reactive): the page's current input for the panel */
	props: () => Record<string, unknown>;
}

export interface DockPanel {
	/** the page's own panel (unmounted with the page) */
	panel?: Snippet;
	/** what the panel is (the Flow, Problems, Info): a panel of the same kind docked by the next page takes it over */
	kind?: string;
	/** with `kind`: the panel as a component the shell mounts (instead of `panel`) – survives navigation */
	content?: DockContent;
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
	/** `held`: its page left – shown until the next page takes it over or the navigation ends */
	#current = $state.raw<{ owner: object; panel: DockPanel; held?: boolean } | null>(null);
	#maximized = $state<MaximizeTarget | null>(null);
	/** pages still loading (`DockHold`): a held panel waits for them */
	#holds = 0;

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

	/** a panel of this kind is docked – shown by a page or held across a navigation (the next page opens its own then) */
	has(kind: string): boolean {
		return !!kind && this.#current?.panel.kind === kind;
	}

	/** the owner's panel opens (null: closes); a panel of another owner gives way, one of the same kind is taken over */
	set(owner: object, panel: DockPanel | null) {
		const shown = this.#current;
		if (!panel) {
			if (shown?.owner === owner) {
				this.#current = null;
				if (this.#maximized === 'dock') this.#maximized = null;
			}
			return;
		}
		const other = !!shown && shown.owner !== owner;
		// the same kind from another owner (the next page's Flow): the shell keeps the mounted panel, it gets the new props
		const takeOver = other && !!panel.kind && !!panel.content && shown.panel.kind === panel.kind;
		// another panel taking the dock while it is maximized stays maximized (owner 2026-10-06: the dock is what fills)
		if (other && !shown.held) shown.panel.close();
		// a panel (re)opens unfolded; the same one updating (its snippet, its title) or taken over keeps its fold
		if (takeOver) panel.size.fold(shown.panel.size.folded);
		else if (!shown || other || shown.panel.title !== panel.title) panel.size.fold(false);
		this.#current = { owner, panel };
	}

	/** the owner unmounts: a panel with a `kind` is held for the next page (E.3 b), any other closes */
	leave(owner: object) {
		const shown = this.#current;
		if (shown?.owner !== owner) return;
		if (!shown.panel.kind || !shown.panel.content) return this.set(owner, null);
		this.#current = { ...shown, held: true };
		// leaving the page ends a maximize (agents/DOCKING.md)
		if (this.#maximized === 'dock') this.#maximized = null;
		this.#release();
	}

	/** a page still loading keeps a held panel until it has decided (`DockHold`); returns the release */
	hold(): () => void {
		this.#holds++;
		let done = false;
		return () => {
			if (done) return;
			done = true;
			this.#holds--;
			this.#release();
		};
	}

	/** after this task (the next page's effects have run): a held panel nobody took over closes */
	#release() {
		setTimeout(() => {
			if (this.#holds > 0) return;
			if (this.#current?.held) {
				this.#current = null;
				if (this.#maximized === 'dock') this.#maximized = null;
			}
		}, 0);
	}
}

// once per page even when the dev server serves this module twice (stale `?t=` timestamps after branch switches): the
// shell would show one registry while the pages dock into the other (creator utils/shared.ts); `:2` since the held
// panels (E.3 b) – an open page keeps its old registry across a hot update
const SHELL_DOCK = Symbol.for('scutoons:shell-dock:2');
export const shellDock: ShellDock = ((globalThis as unknown as Record<symbol, ShellDock | undefined>)[SHELL_DOCK] ??= new ShellDock());

/**
 * Docks the component's open panel on the right while it is mounted (call during component init): `current()` says
 * which one is open or null. The shell makes room for it – left of Ask, never under it; with Ask open too the sidebar
 * folds to its rail. Pages never position panels themselves. A panel with `kind` + `content` outlives the component
 * until the next page takes it over (E.3 b).
 */
export function useDock(current: () => DockPanel | null) {
	const owner = {};
	$effect(() => {
		const panel = current();
		// the registry itself is no dependency: another panel taking the dock closes this one through `close`
		untrack(() => shellDock.set(owner, panel));
	});
	// only on unmount (a re-run of the effect above must not drop and re-open the panel)
	$effect(() => () => shellDock.leave(owner));
}
