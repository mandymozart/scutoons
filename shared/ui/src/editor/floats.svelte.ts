import { readStoredJson, writeStoredJson } from './storage.ts';
import {
	defaultPlace,
	evade,
	layout,
	moved,
	placeOf,
	raise,
	readPlaces,
	resized,
	settle,
	settleSize,
	wantOf,
	type Edge,
	type Metrics,
	type Neighbour,
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
	isEssential,
	moveTab,
	panelOf,
	readArrangement,
	tabsOf,
	tearOut,
	type Arrangement,
	type DropTarget,
	type TabSpec
} from './panel-tabs.ts';

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
	/** A resize: every panel's view when it started and its least size – what `evade` lays out from */
	base?: Record<string, View>;
	neighbours?: Neighbour[];
	views?: Record<string, View>;
}

/** A tab in the hand: where it was pressed, where the pointer is, where it would land and the marker there */
export interface HeldTab {
	tab: string;
	from: [number, number];
	at: [number, number];
	moved: boolean;
	target: DropTarget;
	marker: Rect | null;
}

const sameRect = (a: Rect, b: Rect) => a.x === b.x && a.y === b.y && a.w === b.w && a.h === b.h;
const record = (v: unknown): Record<string, unknown> => (typeof v === 'object' && v !== null && !Array.isArray(v) ? (v as Record<string, unknown>) : {});

/**
 * One workspace's floating panels: the declared panels and tabs, where the person left each panel and which tabs it
 * holds (stored per workspace), what the page draws, the room – and the panel or tab in the hand. A drag is laid out once
 * per animation frame and stored when it ends.
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
		this.order = this.specs.map((s) => s.id);
	}

	/** The panels there are now, in the order of growth: the declared ones that hold tabs (or are toolbars), then the free ones */
	readonly specs: PanelSpec[] = $derived.by(() => {
		const a = this.arrangement;
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

	readonly views: Record<string, View> = $derived.by(
		() =>
			this.held?.views ??
			layout({
				specs: this.specs,
				places: this.resolved,
				shown: this.shown,
				area: this.area,
				m: this.metrics,
				measured: this.measured,
				asks: this.asks,
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

	tabsOf(panel: string): readonly string[] {
		return tabsOf(this.arrangement, panel);
	}

	activeOf(panel: string): string | undefined {
		return activeOf(this.arrangement, panel);
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

	/** Every panel and tab back where the specs put them (this workspace's stored layout goes) */
	reset(): void {
		this.places = {};
		this.arrangement = defaultArrangement(this.panels);
		this.order = this.specs.map((s) => s.id);
		const all = { ...record(readStoredJson(PANELS_KEY)) };
		delete all[this.workspace];
		writeStoredJson(PANELS_KEY, all);
	}

	private save(): void {
		const all = { ...record(readStoredJson(PANELS_KEY)) };
		all[this.workspace] = { places: this.places, tabs: this.arrangement };
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
		this.heldTab = { ...h, at: [x, y], moved: true, target, marker };
	}

	/**
	 * Let go of a tab: a click selects it; over a panel it moves there; over the stage it becomes a panel of its own at the
	 * pointer – or, the only tab of its panel, that panel moves there
	 */
	dropTab(): void {
		const h = this.heldTab;
		this.heldTab = null;
		if (!h) return;
		const from = panelOf(this.arrangement, h.tab);
		if (!from) return;
		if (!h.moved) return this.select(from, h.tab);
		const [x, y] = h.at;
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
		const others = Object.entries(this.views)
			.filter(([o]) => o !== id)
			.map(([, v]) => v.rect);
		const rect = settle(start, others, this.area, this.metrics.gap);
		const was = this.places[id] ?? (torn ? {} : this.place(id));
		this.places = { ...this.places, [id]: { ...was, ...placeOf(rect, this.area), collapsed: false, closed: false } };
		this.front(id);
		this.save();
	}

	cancelTab(): void {
		this.heldTab = null;
	}

	/** The open size a panel asks for (px), held inside the room */
	private openSize(spec: PanelSpec): Size {
		const { want } = wantOf(spec, this.place(spec.id), this.metrics, this.measured[spec.id], this.asks[spec.id]);
		return { w: Math.min(want.w, this.area.w), h: Math.min(want.h, this.area.h) };
	}

	// ── the hand: a move by the handle, a resize by an edge ──

	grab(id: string, x: number, y: number, edge: Edge | null = null): void {
		const view = this.views[id];
		if (!view) return;
		this.cancelFrame();
		this.front(id);
		const held: Held = { id, kind: edge ? 'size' : 'move', edge: edge ?? { x: 0, y: 0 }, start: view.rect, from: [x, y], rect: view.rect, moved: false };
		if (edge) {
			held.base = this.views;
			held.neighbours = Object.entries(this.views).map(([other, v]) => {
				const spec = this.spec(other);
				const min = v.collapsed || !spec ? { w: v.rect.w, h: v.rect.h } : this.minOf(spec);
				return { id: other, rect: v.rect, min: { w: Math.min(min.w, v.rect.w), h: Math.min(min.h, v.rect.h) } };
			});
		}
		this.held = held;
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
			this.held = { ...h, rect: moved(h.start, dx, dy, this.area), moved: true };
			return;
		}
		const target = resized(h.start, h.edge, dx, dy, this.minOf(spec), this.area);
		const base = h.base ?? {};
		const rects = evade(h.neighbours ?? [], h.id, target, h.edge, this.area, this.metrics.gap);
		const views = Object.fromEntries(Object.entries(base).map(([id, v]) => [id, sameRect(v.rect, rects[id]) ? v : { ...v, rect: rects[id] }]));
		this.held = { ...h, rect: rects[h.id], moved: true, views };
	}

	/** Let go: a moved panel settles; a resized one keeps the room it made. True when it was a drag, not a click */
	drop(): boolean {
		this.flush();
		const h = this.held;
		if (!h) return false;
		const views = this.views;
		this.held = null;
		if (!h.moved) return false;
		const spec = this.spec(h.id);
		if (!spec) return true;
		const others = Object.entries(views)
			.filter(([id]) => id !== h.id)
			.map(([, v]) => v.rect);
		const { gap } = this.metrics;
		const old = this.place(h.id);
		if (h.kind === 'move') {
			const rect = settle(h.rect, others, this.area, gap);
			this.set(h.id, { ...old, ...placeOf(rect, this.area) });
			return true;
		}
		const rect = settleSize(h.rect, h.edge, others, this.minOf(spec), this.area, gap);
		const places = { ...this.places, [h.id]: { ...old, ...placeOf(rect, this.area), size: { w: rect.w, h: rect.h } } };
		for (const [id, v] of Object.entries(h.views ?? {})) {
			const was = h.base?.[id];
			if (id === h.id || !was || v === was) continue;
			const shrank = Math.abs(v.rect.w - was.rect.w) > 0.5 || Math.abs(v.rect.h - was.rect.h) > 0.5;
			places[id] = { ...this.place(id), ...placeOf(v.rect, this.area), ...(shrank ? { size: { w: v.rect.w, h: v.rect.h } } : {}) };
		}
		this.places = places;
		this.save();
		return true;
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
