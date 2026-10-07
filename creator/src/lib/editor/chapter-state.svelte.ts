// The chapter editor's view state (COMPONENTS.md step 12): what is open (meta / audio / a group / a slice), the canvas
// selection, the inspector's tab, the tool, snap + grid (remembered on this device), the item tool's spot. One object:
// the tree, the canvas, the inspector and the keyboard all write it (the ⚠ of 4.3 step 12).
import type { CanvasSelection, Tool } from '$lib/components/slice/slice-canvas.svelte';
import { loadCatalog } from '$lib/editor/catalog.svelte.ts';

export type Section = 'meta' | 'audio' | 'group' | 'slice';
/** `layers` = the slice's one layer stack: text boxes + placed items (BACKLOG 10.43) */
export type InspectorTab = 'layers' | 'slice' | 'audio' | 'events';

/** px of the slice's 1x size; 30 = the default (owner 2026-10-03), 15 the slices' base grid (390 = 26 × 15) */
export const GRID_SIZES = [5, 10, 15, 30];
const SNAP_KEY = 'scutoons_creator_snap';
const GRID_KEY = 'scutoons_creator_grid';

// tool = tab = layer (owner 2026-10-03): a tool opens its tab, a tab picks its tool; the canvas only takes the pointer
// on that tab's kinds (a full-slice event no longer covers the texts). The Layers tab holds text boxes AND placed items
// (BACKLOG 10.43): both tools open it; it keeps the one of the two that is picked (else text)
const TAB_TOOL: Partial<Record<InspectorTab, Tool>> = { layers: 'text', audio: 'sample', events: 'event' };
const TOOL_TAB: Record<Tool, InspectorTab> = { text: 'layers', item: 'layers', sample: 'audio', event: 'events' };
const KINDS: Partial<Record<InspectorTab, readonly CanvasSelection['kind'][]>> = { layers: ['dialog', 'item'], audio: ['sample'], events: ['event'] };

function readPref<T>(key: string, fallback: T, ok: (value: unknown) => boolean): T {
	try {
		const raw = localStorage.getItem(key);
		const value = raw === null ? fallback : JSON.parse(raw);
		return ok(value) ? value : fallback;
	} catch {
		return fallback;
	}
}

export class ChapterView {
	section = $state<Section>('meta');
	groupIndex = $state(0);
	sliceIndex = $state(-1);
	selection = $state<CanvasSelection | null>(null);
	inspectorTab = $state<InspectorTab>('layers');
	tool = $state<Tool>('text');
	/** the selected group's slices folded away: a second click on the open group (owner 2026-10-03) */
	collapsed = $state(false);
	/** snap to grid (owner 2026-10-03: one toggle for moving, resizing, scaling, rotating), remembered on this device */
	snap = $state(readPref(SNAP_KEY, true, (v) => typeof v === 'boolean'));
	gridSize = $state(readPref(GRID_KEY, 30, (v) => GRID_SIZES.includes(v as number)));
	/** the item tool's click on the canvas: where the picked item goes */
	itemSpot = $state<{ x: number; y: number } | null>(null);
	/** the item picker sheet is open */
	pickingItem = $state(false);
	/** "+ Add text" shows the presets (owner 2026-10-03) */
	pickingPreset = $state(false);

	/** `onopen`: a group opened (its separate scene file is loaded) */
	constructor(private readonly onopen: (group: number) => void = () => {}) {}

	/** the canvas snap step in px of the slice (0 = off) */
	get grid() {
		return this.snap ? this.gridSize : 0;
	}
	/** the kinds the inspector's tab is on (their boxes take the pointer); null = every kind */
	get layers() {
		return KINDS[this.inspectorTab] ?? null;
	}
	/** a group or one of its slices is open */
	get inGroup() {
		return this.section === 'group' || this.section === 'slice';
	}

	/** remembers snap + grid size on this device (storage unavailable: the choice holds for this visit) */
	savePrefs() {
		try {
			localStorage.setItem(SNAP_KEY, JSON.stringify(this.snap));
			localStorage.setItem(GRID_KEY, JSON.stringify(this.gridSize));
		} catch {
			/* not remembered */
		}
	}

	selectGroup(index: number) {
		this.collapsed = false;
		this.groupIndex = index;
		this.sliceIndex = -1;
		this.selection = null;
		this.section = 'group';
		this.onopen(index);
	}
	/** the tree's plain click: opens the group, or folds it when it is the open one */
	toggleGroup(index: number) {
		if (index === this.groupIndex && this.section === 'group' && !this.collapsed) {
			this.collapsed = true;
			return;
		}
		this.selectGroup(index);
	}
	selectSlice(index: number) {
		this.sliceIndex = index;
		this.selection = null;
		this.section = 'slice';
	}
	/** a canvas / list selection opens its layer's tab */
	select(next: CanvasSelection | null) {
		this.selection = next;
		if (next?.kind === 'dialog' || next?.kind === 'item') this.setTab('layers');
		else if (next?.kind === 'sample') this.setTab('audio');
		else if (next?.kind === 'event') this.setTab('events');
	}
	setTab(next: InspectorTab) {
		this.inspectorTab = next;
		// a selection on another layer is let go (its handles would sit on a layer that no longer takes the pointer)
		if (this.selection && !KINDS[next]?.includes(this.selection.kind)) this.selection = null;
		if (next === 'layers') {
			if (this.tool !== 'text' && this.tool !== 'item') this.tool = 'text';
			loadCatalog();
		} else if (TAB_TOOL[next]) this.tool = TAB_TOOL[next]!;
	}
	setTool(next: Tool) {
		this.tool = next;
		this.setTab(TOOL_TAB[next]);
	}
	/** "+ item" (spot = where on the slice, null = the middle) */
	pickItem(spot: { x: number; y: number } | null = null) {
		this.itemSpot = spot;
		this.pickingItem = true;
	}
}
