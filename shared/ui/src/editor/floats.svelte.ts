import { readStoredJson, writeStoredJson } from './storage.ts';
import {
	clampInto,
	defaultPlace,
	layout,
	moved,
	placeOf,
	raise,
	readPlaces,
	resized,
	wantOf,
	type Edge,
	type Metrics,
	type PanelSpec,
	type Place,
	type Rect,
	type Size,
	type View
} from './floats.ts';
import {
	activate,
	activeOf,
	closeFree,
	defaultArrangement,
	freeSpec,
	hideTab,
	isEssential,
	isHidden,
	moveTab,
	panelOf,
	readArrangement,
	showTab,
	tabsOf,
	tearOut,
	type Arrangement,
	type DropTarget,
	type TabSpec
} from './panel-tabs.ts';
import {
	EMPTY_STAGE,
	HOLD_MS,
	addTo,
	isOn,
	isPastTop,
	keepOn,
	offStage,
	placeOn,
	readStageState,
	viewsOn,
	zoneAt,
	type Cell,
	type StageState,
	type ViewState,
	type ZoneSize
} from './stage.ts';

/** Every workspace's layout in this browser: `{ [workspace]: { places, tabs } }` */
export const PANELS_KEY = 'sc-editor-panels';

/** A press that moves less than this (px) is a click, not a drag */
const CLICK_PX = 4;

interface Held {
	id: string;
	kind: 'move' | 'size';
	edge: Edge;
	start: Rect;
	from: [number, number];
	rect: Rect;
	moved: boolean;
	/** a pointer drag (not the arrow keys): the drop zones take it */
	zones: boolean;
}

/** Where a tab or a panel let go now would land on the stage: a cell, or – held past the top edge – full screen */
export interface StageZone {
	cell: Cell;
	full: boolean;
}

/** The view over everything per workspace: kept while the page lives (a workspace's floats are made anew when its tabs change) */
const FULL = new Map<string, string | null>();

/** A tab in the hand: where it was pressed, where the pointer is, where it would land and the marker there */
export interface HeldTab {
	tab: string;
	from: [number, number];
	at: [number, number];
	moved: boolean;
	target: DropTarget;
	marker: Rect | null;
}

const record = (v: unknown): Record<string, unknown> => (typeof v === 'object' && v !== null && !Array.isArray(v) ? (v as Record<string, unknown>) : {});

/**
 * One workspace's floating panels: the declared panels and tabs, where the person left each panel and which tabs it
 * holds (stored per workspace), the views on the stage and over everything, what the page draws, the room – and the panel
 * or tab in the hand. A drag is laid out once per animation frame and stored when it ends. Over 400 lines: one drag crosses
 * panels, tabs and the stage, so they share this state (the pure parts: floats.ts, panel-tabs.ts, stage.ts).
 */
export class Floats {
	places = $state<Record<string, Place>>({});
	arrangement = $state<Arrangement>({ panels: {}, active: {}, free: {} });
	/** The room for panels (client px) */
	area = $state<Rect>({ x: 0, y: 0, w: 0, h: 0 });
	metrics = $state<Metrics>({ rem: 14, gap: 14, head: 37, button: 30, char: 7.5 });
	shown = $state<Set<string>>(new Set());
	measured = $state<Record<string, Size>>({});
	asks = $state<Record<string, PanelSpec['size']>>({});
	order = $state<string[]>([]);
	held = $state.raw<Held | null>(null);
	heldTab = $state.raw<HeldTab | null>(null);
	/** The views on the stage – the whole window under the panels (stored with the layout) */
	stage = $state<StageState>(EMPTY_STAGE);
	#full = $state<string | null>(null);
	/** The window (client px): the drop zones reach its edges, over the bar and the status bar */
	win = $state<Size>({ w: 0, h: 0 });
	/** The drop zone under the pointer while a tab or a panel is in the hand; `past`: beyond the top edge */
	hover = $state.raw<{ cell: Cell; past: boolean } | null>(null);
	/** Held past the top edge long enough: a drop goes full screen */
	armed = $state(false);
	private holdTimer: ReturnType<typeof setTimeout> | 0 = 0;
	private pending: [number, number] | null = null;
	private frame = 0;
	/** How many layouts of the hand ran (one per animation frame) */
	passes = 0;

	constructor(
		readonly workspace: string,
		readonly panels: readonly PanelSpec[],
		readonly tabs: readonly TabSpec[] = []
	) {
		const stored = record(record(readStoredJson(PANELS_KEY))[workspace]);
		this.arrangement = readArrangement(stored.tabs, panels, tabs);
		this.places = readPlaces({ [workspace]: stored.places }, workspace, this.specs);
		this.stage = keepOn(readStageState(stored.stage), (t) => !!panelOf(this.arrangement, t));
		this.#full = FULL.get(workspace) ?? null;
		this.order = this.specs.map((s) => s.id);
	}

	/** The view over everything (the bar and the panels hidden), or none */
	get full(): string | null {
		return this.#full;
	}
	set full(tab: string | null) {
		this.#full = tab;
		FULL.set(this.workspace, tab);
	}

	/** What the panels show: the arrangement less the views on the stage or over everything (they keep their place in it) */
	readonly shownArrangement: Arrangement = $derived.by(() => {
		const a = this.arrangement;
		const away = [...viewsOn(this.stage), ...(this.#full ? [this.#full] : [])].filter((t) => !isHidden(a, t));
		return away.length ? { ...a, hidden: [...(a.hidden ?? []), ...away] } : a;
	});

	/** The panels there are now, in the order of growth: the declared ones that hold tabs (or are toolbars), then the free ones */
	readonly specs: PanelSpec[] = $derived.by(() => {
		const a = this.shownArrangement;
		const name = (id: string, fallback: string) => this.tab(activeOf(a, id) ?? '')?.name ?? fallback;
		const declared = this.panels
			.filter((p) => !p.tabs || tabsOf(a, p.id).length)
			.map((p) => (p.tabs ? { ...p, name: name(p.id, p.name), essential: p.essential || isEssential(a, p.id, this.tabs) } : p));
		const free = Object.entries(a.free).flatMap(([id, origin]) => {
			const tab = this.tab(origin);
			return tab && tabsOf(a, id).length ? [{ ...freeSpec(id, tab), name: name(id, tab.name), essential: isEssential(a, id, this.tabs) }] : [];
		});
		return [...declared, ...free];
	});

	readonly views: Record<string, View> = $derived.by(() =>
		layout({
			specs: this.specs,
			places: this.resolved,
			shown: this.shown,
			area: this.area,
			m: this.metrics,
			measured: this.measured,
			asks: this.asks,
			placed: new Set(Object.keys(this.places)),
			held: this.held && this.held.moved ? { id: this.held.id, rect: this.held.rect } : null
		})
	);

	get resolved(): Record<string, Place> {
		return Object.fromEntries(this.specs.map((s) => [s.id, this.places[s.id] ?? defaultPlace(s, this.metrics)]));
	}

	spec(id: string): PanelSpec | undefined {
		return this.specs.find((s) => s.id === id);
	}

	tab(id: string): TabSpec | undefined {
		return this.tabs.find((t) => t.id === id);
	}

	/** A panel's shown tabs (not those on the stage or over everything) */
	tabsOf(panel: string): readonly string[] {
		return tabsOf(this.shownArrangement, panel);
	}

	activeOf(panel: string): string | undefined {
		return activeOf(this.shownArrangement, panel);
	}

	place(id: string): Place {
		const spec = this.spec(id);
		return this.places[id] ?? (spec ? defaultPlace(spec, this.metrics) : { corner: 'top-left', dx: 0, dy: 0 });
	}

	z(id: string): number {
		const i = this.order.indexOf(id);
		return (i < 0 ? this.order.length : i) + 1;
	}

	show(id: string): void {
		if (!this.shown.has(id)) this.shown = new Set([...this.shown, id]);
	}

	hide(id: string): void {
		if (!this.shown.has(id)) return;
		const next = new Set(this.shown);
		next.delete(id);
		this.shown = next;
	}

	measure(id: string, size: Size): void {
		const was = this.measured[id];
		if (was && Math.abs(was.w - size.w) < 0.5 && Math.abs(was.h - size.h) < 0.5) return;
		this.measured = { ...this.measured, [id]: size };
	}

	// ── folded, closed, reset ──

	private set(id: string, place: Place): void {
		this.places = { ...this.places, [id]: place };
		this.save();
	}

	collapsed(id: string): boolean {
		return !!this.place(id).collapsed;
	}

	closed(id: string): boolean {
		return !!this.place(id).closed;
	}

	setCollapsed(id: string, collapsed: boolean): void {
		if (this.spec(id)?.inline) return;
		this.set(id, { ...this.place(id), collapsed });
		this.front(id);
	}

	vertical(id: string): boolean {
		return !!(this.spec(id)?.inline && this.place(id).vertical);
	}

	setVertical(id: string, vertical: boolean): void {
		const { size: _size, collapsed: _collapsed, vertical: _vertical, ...place } = this.place(id);
		this.set(id, vertical ? { ...place, vertical } : place);
		this.front(id);
	}

	/** Close a panel (an essential one only folds); a free panel's tabs go back to their home panels */
	close(id: string): void {
		const spec = this.spec(id);
		if (!spec || spec.essential) return;
		if (this.arrangement.free[id]) {
			this.arrangement = closeFree(this.arrangement, id, this.panels);
			const { [id]: _gone, ...places } = this.places;
			this.places = places;
			this.save();
			return;
		}
		this.set(id, { ...this.place(id), closed: true });
	}

	reopen(id: string): void {
		this.set(id, { ...this.place(id), closed: false, collapsed: false });
		this.front(id);
	}

	flip(id: string): void {
		if (this.closed(id)) this.reopen(id);
		else this.close(id);
	}

	/** The panels that close and whether each is open – the panels menu */
	closable(): Array<{ id: string; name: string; icon: string; open: boolean }> {
		return this.specs.filter((s) => !s.essential).map((s) => ({ id: s.id, name: s.name, icon: s.icon, open: !this.closed(s.id) }));
	}

	/** Every panel and tab back where the specs put them, the stage empty (this workspace's stored layout goes) */
	reset(): void {
		this.places = {};
		this.stage = EMPTY_STAGE;
		this.full = null;
		this.arrangement = defaultArrangement(this.panels);
		this.order = this.specs.map((s) => s.id);
		const all = { ...record(readStoredJson(PANELS_KEY)) };
		delete all[this.workspace];
		writeStoredJson(PANELS_KEY, all);
	}

	private save(): void {
		const all = { ...record(readStoredJson(PANELS_KEY)) };
		all[this.workspace] = { places: this.places, tabs: this.arrangement, stage: this.stage };
		writeStoredJson(PANELS_KEY, all);
	}

	front(id: string): void {
		if (this.order.at(-1) !== id) this.order = raise(this.order, id);
	}

	// ── tabs ──

	select(panel: string, tab: string): void {
		this.arrangement = activate(this.arrangement, panel, tab);
		this.save();
	}

	/** The panel holding a tab, shown or hidden */
	panelOf(tab: string): string | undefined {
		return panelOf(this.arrangement, tab);
	}

	/** A tab shows: in an open panel and not hidden */
	tabShown(tab: string): boolean {
		const panel = panelOf(this.arrangement, tab);
		return !!panel && !isHidden(this.arrangement, tab) && !!this.spec(panel) && !this.closed(panel);
	}

	/** A tab hidden (the Views eye): its panel keeps its place for it; a panel left without a shown tab goes */
	hideTab(tab: string): void {
		this.arrangement = hideTab(this.arrangement, tab);
		this.save();
	}

	/** A tab shown where it last was: its panel opens on it */
	showTab(tab: string): void {
		const panel = panelOf(this.arrangement, tab);
		if (!panel) return;
		this.arrangement = showTab(this.arrangement, tab);
		if (this.closed(panel) || this.collapsed(panel)) this.reopen(panel);
		else {
			this.front(panel);
			this.save();
		}
	}

	// ── the four states of a view: hidden · in a panel · on the stage · full screen ──

	stateOf(tab: string): ViewState {
		if (this.#full === tab) return 'full';
		if (isOn(this.stage, tab)) return 'stage';
		return this.tabShown(tab) ? 'panel' : 'hidden';
	}

	/** A view to a state (Views): leaving the stage or full screen it goes where it is asked to – its panel, or hidden */
	setState(tab: string, state: ViewState): void {
		if (state === 'full') {
			this.full = tab;
			return;
		}
		if (this.#full === tab) this.full = null;
		if (state === 'stage') return this.toStage(tab);
		this.stage = offStage(this.stage, tab);
		if (state === 'panel') this.showTab(tab);
		else this.hideTab(tab);
	}

	/** A view onto the stage – into a cell (a drop), or where Views finds room; it keeps its place in its panel for when it leaves */
	toStage(tab: string, cell?: Cell): void {
		if (!panelOf(this.arrangement, tab)) return;
		if (isHidden(this.arrangement, tab)) this.arrangement = showTab(this.arrangement, tab);
		this.stage = cell ? placeOn(this.stage, tab, cell) : addTo(this.stage, tab);
		if (this.#full === tab) this.full = null;
		this.save();
	}

	/** Full screen on and off for a view; off, it is where it was before */
	toggleFull(tab: string): void {
		this.full = this.#full === tab ? null : tab;
	}

	/** What is in the hand now: the tab, or the front tab of the panel dragged by its head */
	get dragged(): string | null {
		if (this.heldTab?.moved) return this.heldTab.tab;
		const h = this.held;
		return h?.moved && h.kind === 'move' && h.zones ? (this.activeOf(h.id) ?? null) : null;
	}

	/** Where a drop now lands on the stage */
	get zone(): StageZone | null {
		return this.hover ? { cell: this.hover.cell, full: this.hover.past && this.armed } : null;
	}

	/** How far the drop zones reach in from the edges (px) */
	get zoneSize(): ZoneSize {
		return { band: 2 * this.metrics.rem, corner: 6 * this.metrics.rem };
	}

	/** The pointer at x, y with something in the hand: the zone under it; past the top edge a hold starts toward full screen */
	private track(x: number, y: number): void {
		const cell = zoneAt(x, y, this.win, this.zoneSize);
		const past = isPastTop(y);
		if (past && !this.armed && !this.holdTimer)
			this.holdTimer = setTimeout(() => {
				this.holdTimer = 0;
				this.armed = true;
			}, HOLD_MS);
		if (!past) this.disarm();
		this.hover = cell ? { cell: past ? 'whole' : cell, past } : null;
	}

	private disarm(): void {
		if (this.holdTimer) clearTimeout(this.holdTimer);
		this.holdTimer = 0;
		this.armed = false;
	}

	/** The hand is empty: no zone */
	private endZone(): void {
		this.disarm();
		this.hover = null;
	}

	/** A tab or a panel let go over a zone: its view onto the stage, or full screen */
	private land(tab: string, zone: StageZone): void {
		if (zone.full) this.setState(tab, 'full');
		else this.toStage(tab, zone.cell);
	}

	grabTab(tab: string, x: number, y: number): void {
		const panel = panelOf(this.arrangement, tab);
		if (panel) this.front(panel);
		this.heldTab = { tab, from: [x, y], at: [x, y], moved: false, target: null, marker: null };
	}

	/** The pointer moved with a tab in the hand: where it would land (the page measures the panels under it) */
	dragTab(x: number, y: number, target: DropTarget, marker: Rect | null): void {
		const h = this.heldTab;
		if (!h) return;
		if (!h.moved && Math.hypot(x - h.from[0], y - h.from[1]) < CLICK_PX) return;
		this.track(x, y);
		// a zone at the screen's edge comes first: over it the tab goes to the stage, not into a panel there
		this.heldTab = this.hover ? { ...h, at: [x, y], moved: true, target: null, marker: null } : { ...h, at: [x, y], moved: true, target, marker };
	}

	/**
	 * Let go of a tab: a click selects it; over a drop zone its view goes onto the stage (or full screen); over a panel it
	 * moves there; anywhere else it becomes a panel of its own at the pointer – or, the only tab of its panel, that panel
	 * moves there. A view dragged off the stage leaves it
	 */
	dropTab(): void {
		const h = this.heldTab;
		const zone = this.zone;
		this.heldTab = null;
		this.endZone();
		if (!h) return;
		const from = panelOf(this.arrangement, h.tab);
		if (!from) return;
		if (!h.moved) return isOn(this.stage, h.tab) ? undefined : this.select(from, h.tab);
		if (zone) return this.land(h.tab, zone);
		const [x, y] = h.at;
		this.stage = offStage(this.stage, h.tab);
		if (h.target) {
			this.arrangement = moveTab(this.arrangement, h.tab, h.target.panel, h.target.index);
			this.front(h.target.panel);
			this.save();
			return;
		}
		const torn = tearOut(this.arrangement, h.tab);
		const id = torn?.panel ?? from;
		if (torn) this.arrangement = torn.arrangement;
		const spec = this.spec(id);
		if (!spec) return;
		const view = this.views[id];
		const size = torn || !view ? this.openSize(spec) : { w: view.rect.w, h: view.rect.h };
		const half = this.metrics.head / 2;
		const start = moved({ x: x - half, y: y - half, ...size }, 0, 0, this.area);
		const was = this.places[id] ?? (torn ? {} : this.place(id));
		this.places = { ...this.keepOthers(id), [id]: { ...was, ...placeOf(start, this.area), collapsed: false, closed: false } };
		this.front(id);
		this.save();
	}

	cancelTab(): void {
		this.heldTab = null;
		this.endZone();
	}

	/** The open size a panel asks for (px), held inside the room */
	private openSize(spec: PanelSpec): Size {
		const { want } = wantOf(spec, this.place(spec.id), this.metrics, this.measured[spec.id], this.asks[spec.id]);
		return { w: Math.min(want.w, this.area.w), h: Math.min(want.h, this.area.h) };
	}

	// ── the hand: a move by the handle, a resize by an edge ──

	/** A panel taken by its head (a move) or an edge (a resize); `zones`: false for the arrow keys, which never drop on the stage */
	grab(id: string, x: number, y: number, edge: Edge | null = null, zones = true): void {
		const view = this.views[id];
		if (!view) return;
		this.cancelFrame();
		this.front(id);
		this.held = { id, kind: edge ? 'size' : 'move', edge: edge ?? { x: 0, y: 0 }, start: view.rect, from: [x, y], rect: view.rect, moved: false, zones: zones && !edge };
	}

	/** The pointer moved while a panel is held: laid out at the next animation frame (without frames – a test – at once) */
	drag(x: number, y: number): void {
		if (!this.held) return;
		this.pending = [x, y];
		if (typeof requestAnimationFrame !== 'function') return this.flush();
		if (!this.frame)
			this.frame = requestAnimationFrame(() => {
				this.frame = 0;
				this.flush();
			});
	}

	private flush(): void {
		const at = this.pending;
		this.cancelFrame();
		if (at) this.follow(at[0], at[1]);
	}

	private cancelFrame(): void {
		if (this.frame && typeof cancelAnimationFrame === 'function') cancelAnimationFrame(this.frame);
		this.frame = 0;
		this.pending = null;
	}

	private follow(x: number, y: number): void {
		const h = this.held;
		if (!h) return;
		const dx = x - h.from[0];
		const dy = y - h.from[1];
		if (!h.moved && Math.hypot(dx, dy) < CLICK_PX) return;
		const spec = this.spec(h.id);
		if (!spec) return;
		this.passes++;
		if (h.kind === 'move') {
			if (h.zones) this.track(x, y);
			this.held = { ...h, rect: moved(h.start, dx, dy, this.area), moved: true };
			return;
		}
		this.held = { ...h, rect: resized(h.start, h.edge, dx, dy, this.minOf(spec), this.area), moved: true };
	}

	/**
	 * Let go: the panel stays where it is, held inside the walls – no snap; a resized one keeps its size. No other panel
	 * moves or changes size. Over a drop zone the panel goes back where it was and its front view lands on the stage (the
	 * panel never docks). True when it was a drag, not a click
	 */
	drop(): boolean {
		this.flush();
		const h = this.held;
		const zone = h?.zones ? this.zone : null;
		this.held = null;
		this.endZone();
		if (!h) return false;
		if (!h.moved) return false;
		const front = zone && this.activeOf(h.id);
		if (zone && front) {
			this.land(front, zone);
			return true;
		}
		const spec = this.spec(h.id);
		if (!spec) return true;
		const rect = clampInto(h.rect, this.area);
		const old = this.place(h.id);
		// a `fill` panel let go keeps the size it had: it stays as it was dropped
		const keep = h.kind === 'size' || (!old.collapsed && (spec.size.w === 'fill' || spec.size.h === 'fill'));
		this.places = { ...this.keepOthers(h.id), [h.id]: { ...old, ...placeOf(rect, this.area), ...(keep ? { size: { w: rect.w, h: rect.h } } : {}) } };
		this.save();
		return true;
	}

	/**
	 * The places with every other panel held where it lies now – a panel let go must change no other: one still at its
	 * default place keeps that rectangle, an open `fill` one the size it has (it grows only to panels clear of it)
	 */
	private keepOthers(id: string): Record<string, Place> {
		const places = { ...this.places };
		for (const [other, v] of Object.entries(this.views)) {
			const spec = this.spec(other);
			if (other === id || !spec) continue;
			const stored = this.places[other];
			const fill = !v.collapsed && !stored?.size && (spec.size.w === 'fill' || spec.size.h === 'fill');
			if (stored && !fill) continue;
			places[other] = { ...this.place(other), ...placeOf(v.rect, this.area), ...(fill ? { size: { w: v.rect.w, h: v.rect.h } } : {}) };
		}
		return places;
	}

	private minOf(spec: PanelSpec): Size {
		return wantOf(spec, this.place(spec.id), this.metrics, this.measured[spec.id], this.asks[spec.id]).min;
	}

	/** A resized panel back to the size it asks for (a double-click on an edge) */
	unsize(id: string): void {
		const { size: _size, ...rest } = this.place(id);
		this.set(id, rest);
	}
}
