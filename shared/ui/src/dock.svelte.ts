import { getContext, setContext, untrack, type Component, type Snippet } from 'svelte';
import type { PanelWidth } from './panel-width.svelte.ts';

/**
 * The AppShell's right dock (agents/DOCKING.md): the page panels – the creator's Problems / History / Flow / Reader, the
 * Info (help) panel of every ActionBar, the admin's list detail (SplitView). Several can be open at once (BACKLOG 10.57,
 * owner 2026-10-07): they tile side by side right of the main area, ONE per kind, each in its fixed place
 * (`DOCK_ORDER`), each with its own header, fold, maximize and ✕. A panel opens with `useDock`; a panel of the same kind
 * from another owner replaces the shown one (its `close`, so its page's toggle follows). The shell shows
 * `shellDock.panels`, sizes them (`size`, a foldable `panelWidth`) and folds them to rails labelled `title` / `icon` –
 * by hand, or the least recently used first when the room runs out (`used`).
 * One shell per app: a module-level registry (the tools are client-only SPAs).
 *
 * Panels that survive navigation (UI-CONCEPTS E.3 b): a panel with a `kind` and a `content` (a component + its props)
 * is mounted by the shell, not by the page. When the page leaves, the panel is HELD (still shown, its last props) until
 * the next page docks the same kind – it takes the mounted instance over and hands it its own props (no remount, no
 * flash: the Flow keeps its zoom, Info its frame) – or, when nothing takes it, until the end of the navigation (the
 * task after the old page left; a `DockHold` on a page that is still loading keeps it until that page has decided). A
 * page that opens asks `shellDock.has(kind)` whether such a panel is docked and opens its own then (the Flow stays open
 * toon → season → chapter). Every kind on its own: the Flow, the Reader and Problems all go on together. A panel with
 * only a `panel` snippet belongs to its page and goes with it.
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
	/** what the panel is (the Flow, Problems, Info): one panel per kind – the same kind docked by another owner replaces
	 * it (with `content`: takes the mounted instance over); its place in the dock (`DOCK_ORDER`) */
	kind?: string;
	/** with `kind`: the panel as a component the shell mounts (instead of `panel`) – survives navigation */
	content?: DockContent;
	size: PanelWidth;
	/** shown on the rail when folded (Problems, History, Flow, Info, the detail's title) */
	title: string;
	/** Feather icon name on the rail */
	icon?: string;
	/** a kind outside `DOCK_ORDER` (an app's own panels – the storyboard studio's): its fixed place among the other
	 * unknown kinds, from the main area outwards – 0 … 499 (clamped: an app's kinds always sit between the Reader and
	 * Info); equal places (or none, = 0): opening order */
	order?: number;
	/** the page closes it (another panel of its kind replaced it) */
	close: () => void;
}

/**
 * The panels' places from the main area outwards (owner 2026-10-07, agents/DOCKING.md "Several panels"): what belongs
 * to the page's document next to it (a list's detail, History, Problems), then the Flow (it drives the main area), the
 * Reader (the result), Info (help) outermost on the right. Unknown kinds between the Reader and Info, by their `order`
 * (0 … 499); equal places: opening order.
 */
export const DOCK_ORDER = ['detail', 'history', 'problems', 'flow', 'reader', 'info'];
/** the highest `order`: 4.5 + 499 / 1000 stays below Info's place (5) */
const MAX_ORDER = 499;
const place = (panel: DockPanel) => {
	const at = panel.kind ? DOCK_ORDER.indexOf(panel.kind) : -1;
	if (at >= 0) return at;
	// NaN / none = 0; anything past the range (Infinity too) sits at its end
	const order = Math.min(Math.max(panel.order || 0, 0), MAX_ORDER);
	return DOCK_ORDER.length - 1.5 + order / 1000;
};

/** a docked panel: `id` stays while the panel is taken over by the next page (its place, fold and width go on) */
export interface DockEntry {
	id: number;
	owner: object;
	panel: DockPanel;
	/** its page left – shown until the next page takes it over or the navigation ends */
	held?: boolean;
	/** when it was last opened / unfolded / maximized: the least recently used folds first when the room runs out */
	used: number;
}

/** what fills the shell while maximized: a dock panel (`dock:<id>`; `'dock'` = the most recently used open one), or a
 * region of the main area by its id (the slice inspector) */
export type MaximizeTarget = 'dock' | `dock:${number}` | (string & {});

/** the docked panel a component sits in (set by the shell around each panel – `DockFold` reads it) */
const SLOT = Symbol('scutoons:dock-slot');
export const setDockSlot = (id: () => number) => setContext(SLOT, id);
export const getDockSlot = () => getContext<(() => number) | undefined>(SLOT);

class ShellDock {
	#entries = $state.raw<DockEntry[]>([]);
	#maximized = $state<MaximizeTarget | null>(null);
	#mainFolded = $state(false);
	/** pages still loading (`DockHold`): a held panel waits for them */
	#holds = 0;
	#ids = 0;
	#clock = 0;

	/** the docked panels in their places, main area → right edge */
	get panels(): DockEntry[] {
		return this.#entries;
	}

	/** the panel with this id */
	entry(id: number | undefined): DockEntry | undefined {
		return id === undefined ? undefined : this.#entries.find((e) => e.id === id);
	}

	/** the most recently used panel (folded ones only when no other is open) */
	get latest(): DockEntry | undefined {
		const open = this.#entries.filter((e) => !e.panel.size.folded);
		return (open.length ? open : this.#entries).reduce<DockEntry | undefined>((a, e) => (!a || e.used > a.used ? e : a), undefined);
	}

	/** the most recently used panel (compatibility: "the" docked panel) */
	get current(): DockPanel | null {
		return this.latest?.panel ?? null;
	}

	/**
	 * The maximized region (owner 2026-10-06, agents/DOCKING.md "Maximize"): `dock:<id>` – that dock panel fills
	 * everything right of the sidebar's rail; another id – that region of the main area does (the page hides the
	 * rest of its main area). The other regions are only hidden (their widths, folds, scroll positions and edits stay).
	 * One at a time; not remembered.
	 */
	get maximized(): MaximizeTarget | null {
		return this.#maximized;
	}

	/** the maximized dock panel, if one is */
	get maximizedPanel(): DockEntry | undefined {
		const m = this.#maximized;
		return m?.startsWith('dock:') ? this.entry(Number(m.slice(5))) : undefined;
	}

	/** maximizes `target` (null: restores); a dock panel opens unfolded for it */
	maximize(target: MaximizeTarget | null) {
		if (target === 'dock') {
			const latest = this.latest;
			if (!latest) return;
			target = `dock:${latest.id}`;
		}
		if (target?.startsWith('dock:')) {
			const entry = this.entry(Number(target.slice(5)));
			if (!entry) return;
			entry.panel.size.fold(false);
			this.touch(entry.id);
		}
		this.#maximized = target;
	}

	/** back to the normal layout (the button, Esc, Ctrl + Space, a click in the Flow that navigates the main area) */
	restore() {
		this.#maximized = null;
	}

	/**
	 * The main area folded to its rail (owner 2026-10-07, 10.57: "with the phone preview and Flow or the agent available,
	 * the main could also fold"): only by the author (the leftmost panel's "Fold the page" button, Ctrl + Shift + \),
	 * never by itself; the panels take its room. A Flow box still navigates it (the Reader follows); its rail, the key
	 * again or the last panel closing bring it back. Not remembered.
	 */
	get mainFolded(): boolean {
		return this.#mainFolded && this.#entries.length > 0;
	}

	foldMain(value: boolean) {
		this.#mainFolded = value && this.#entries.length > 0;
	}

	/** the panels that show unfolded, set by the shell (a squeezed panel is a rail): the leftmost carries "Fold the page" */
	shown = $state.raw<number[]>([]);
	/** the shell's labels for the page's fold (its app's language) */
	pageFoldLabel = $state('Fold the page');

	/** the panel was used (opened, unfolded, maximized): it is the last to fold when the room runs out */
	touch(id: number) {
		const at = this.#entries.findIndex((e) => e.id === id);
		if (at < 0) return;
		const next = [...this.#entries];
		next[at] = { ...next[at], used: ++this.#clock };
		this.#entries = next;
	}

	/** a panel of this kind is docked – shown by a page or held across a navigation (the next page opens its own then) */
	has(kind: string): boolean {
		return !!kind && this.#entries.some((e) => e.panel.kind === kind);
	}

	#sorted(list: DockEntry[]) {
		return [...list].sort((a, b) => place(a.panel) - place(b.panel) || a.id - b.id);
	}

	#drop(entry: DockEntry) {
		this.#entries = this.#entries.filter((e) => e !== entry);
		if (!this.#entries.length) this.#mainFolded = false;
		if (this.#maximized === `dock:${entry.id}`) this.#maximized = null;
	}

	/** the owner's panel opens (null: closes); a panel of the same kind from another owner gives way or is taken over */
	set(owner: object, panel: DockPanel | null) {
		const mine = this.#entries.find((e) => e.owner === owner);
		if (!panel) {
			if (mine) this.#drop(mine);
			return;
		}
		if (mine) {
			// the same panel updating (its snippet, its title); another kind from the same owner opens unfolded
			const changed = mine.panel.kind !== panel.kind;
			if (changed) panel.size.fold(false);
			this.#replace(mine, { ...mine, panel, used: changed ? ++this.#clock : mine.used });
			return;
		}
		const same = panel.kind ? this.#entries.find((e) => e.panel.kind === panel.kind) : undefined;
		if (same) {
			// the next page's Flow: the shell keeps the mounted panel (same id), it gets the new props and keeps its fold
			const takeOver = !!panel.content && !!same.panel.content;
			if (!same.held) same.panel.close();
			if (takeOver) panel.size.fold(same.panel.size.folded);
			else panel.size.fold(false);
			this.#replace(same, { id: same.id, owner, panel, used: takeOver ? same.used : ++this.#clock });
			return;
		}
		panel.size.fold(false);
		const entry: DockEntry = { id: ++this.#ids, owner, panel, used: ++this.#clock };
		this.#entries = this.#sorted([...this.#entries, entry]);
		// a panel opened while another one is maximized takes the maximize over (owner 2026-10-06: the dock is what fills)
		if (this.maximizedPanel) this.#maximized = `dock:${entry.id}`;
	}

	#replace(old: DockEntry, next: DockEntry) {
		this.#entries = this.#sorted(this.#entries.map((e) => (e === old ? next : e)));
	}

	/** the owner unmounts: a panel with a `kind` is held for the next page (E.3 b), any other closes */
	leave(owner: object) {
		const mine = this.#entries.find((e) => e.owner === owner);
		if (!mine) return;
		// leaving the page ends a maximize (agents/DOCKING.md)
		if (this.maximizedPanel) this.#maximized = null;
		if (!mine.panel.kind || !mine.panel.content) return this.#drop(mine);
		this.#replace(mine, { ...mine, held: true });
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

	/** after this task (the next page's effects have run): held panels nobody took over close */
	#release() {
		setTimeout(() => {
			if (this.#holds > 0) return;
			for (const e of this.#entries) if (e.held) this.#drop(e);
		}, 0);
	}
}

// once per page even when the dev server serves this module twice (stale `?t=` timestamps after branch switches): the
// shell would show one registry while the pages dock into the other (creator utils/shared.ts); `:3` since several
// panels (10.57) – an open page keeps its old registry across a hot update
const SHELL_DOCK = Symbol.for('scutoons:shell-dock:3');
export const shellDock: ShellDock = ((globalThis as unknown as Record<symbol, ShellDock | undefined>)[SHELL_DOCK] ??= new ShellDock());

/**
 * Docks the component's open panel on the right while it is mounted (call during component init; once per panel – a
 * page with a Flow and Problems calls it twice): `current()` says whether it is open (the panel) or null. The shell
 * makes room for it – never under the page, next to the other open panels. Pages never position panels themselves. A panel with `kind` + `content` outlives the component until
 * the next page takes it over (E.3 b).
 */
export function useDock(current: () => DockPanel | null) {
	const owner = {};
	$effect(() => {
		const panel = current();
		// the registry itself is no dependency: another panel of its kind replacing this one closes it through `close`
		untrack(() => shellDock.set(owner, panel));
	});
	// only on unmount (a re-run of the effect above must not drop and re-open the panel)
	$effect(() => () => shellDock.leave(owner));
}
