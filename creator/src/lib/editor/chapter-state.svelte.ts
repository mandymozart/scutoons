// The chapter editor's view state (COMPONENTS.md step 12): what is open (meta / audio / a group / a slice), the canvas
// selection, the inspector's tab, the tool, snap + grid (remembered on this device), the item tool's spot. One object:
// the tree, the canvas, the inspector and the keyboard all write it (the ⚠ of 4.3 step 12).
import type { CanvasSelection, Tool } from '$lib/components/slice/slice-canvas.svelte';
import { loadCatalog } from '$lib/editor/catalog.svelte.ts';
import { piecePlace, type PiecePlace } from '$lib/editor/canvas-pieces.ts';

export type Section = 'meta' | 'audio' | 'group' | 'slice';
/**
 * `layers` = the slice's one layer stack: text boxes + placed items (BACKLOG 10.43) + click / hover areas (10.54), and
 * under them the collage pieces (research/COLLAGE.md §3)
 */
export type InspectorTab = 'group' | 'layers' | 'slice' | 'audio' | 'events';

/** px of the slice's 1x size; 30 = the default (owner 2026-10-03), 15 the slices' base grid (390 = 26 × 15) */
export const GRID_SIZES = [5, 10, 15, 30];
const SNAP_KEY = 'scutoons_creator_snap';
const GRID_KEY = 'scutoons_creator_grid';
const ZOOM_KEY = 'scutoons_creator_zoom';
const SEAMS_KEY = 'scutoons_creator_seams';
/** the slice canvas's zoom range (owner 2026-10-07, UI-CONCEPTS E.4): 50–300 % of the slice's 1x width */
export const ZOOM_MIN = 0.5;
export const ZOOM_MAX = 3;
/** the zoom buttons' steps */
const ZOOM_STEPS = [0.5, 0.67, 0.75, 1, 1.25, 1.5, 2, 2.5, 3];
export const clampZoom = (z: number) => Math.round(Math.min(ZOOM_MAX, Math.max(ZOOM_MIN, z)) * 100) / 100;

// tool = tab = layer (owner 2026-10-03): a tool opens its tab, a tab picks its tool; the canvas only takes the pointer
// on that tab's kinds (a full-slice event no longer covers the texts). The Layers tab holds text boxes AND placed items
// (BACKLOG 10.43): both tools open it; it keeps the one of the two that is picked (else text). Its stack also holds the
// click / hover areas of events and samples (10.54) – edited in their own tabs, but selectable and in pointer order here
// (the canvas lets only the areas of the stack take the pointer on this tab). The piece tool (COLLAGE.md §3) opens it too:
// collage pieces lie under the stack, in the same list
const TAB_TOOL: Partial<Record<InspectorTab, Tool>> = { layers: 'text', audio: 'sample', events: 'event' };
const TOOL_TAB: Record<Tool, InspectorTab> = { text: 'layers', item: 'layers', piece: 'layers', sample: 'audio', event: 'events' };
const KINDS: Partial<Record<InspectorTab, readonly CanvasSelection['kind'][]>> = { layers: ['dialog', 'item', 'piece', 'event', 'sample'], audio: ['sample'], events: ['event'] };
/** the tools of the Layers tab (it keeps the one that is picked) */
const LAYER_TOOLS: readonly Tool[] = ['text', 'item', 'piece'];

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
	/**
	 * the canvas zoom, a factor of the slice's 1x width (50–300 %); null = fit (the area's height, at most 2x; on a phone
	 * its width) – remembered on this device (UI-CONCEPTS E.4)
	 */
	zoom = $state<number | null>(readPref(ZOOM_KEY, null, (v) => v === null || (typeof v === 'number' && v >= ZOOM_MIN && v <= ZOOM_MAX)));
	/**
	 * the strip's seams as a hairline across the art (agents/research/TEXT-ACROSS-SLICES.md §4.1, the View toggle; the slice
	 * numbers at the seams always show) – remembered on this device
	 */
	seams = $state(readPref(SEAMS_KEY, false, (v) => typeof v === 'boolean'));
	/** the factor the canvas is shown at (measured – in fit mode too): the zoom's percentage, the steps start from it */
	shownZoom = $state(1);
	/** the item tool's click on the canvas: where the picked item goes */
	itemSpot = $state<{ x: number; y: number } | null>(null);
	/** the item picker sheet is open */
	pickingItem = $state(false);
	/** the piece tool's click / drag (or "+ piece"): where the picked piece goes (canvas-pieces.ts) */
	pieceSpot = $state<PiecePlace | null>(null);
	/** the collage picker sheet is open (research/COLLAGE.md §5) */
	pickingPiece = $state(false);
	/** "+ Add text" shows the presets (owner 2026-10-03) */
	pickingPreset = $state(false);
	/** the phone's text preset (E3, the presets row): a tap on the canvas with the text tool places one of it */
	textPreset = $state('speech');

	/** `onopen`: a group opened */
	constructor(private readonly onopen: (group: number) => void = () => {}) {}

	/** the canvas snap step in px of the slice (0 = off) */
	get grid() {
		return this.snap ? this.gridSize : 0;
	}
	/** the next zoom step up / down from what is shown */
	zoomIn() {
		this.zoom = ZOOM_STEPS.find((z) => z > this.shownZoom + 0.005) ?? ZOOM_MAX;
	}
	zoomOut() {
		this.zoom = ZOOM_STEPS.findLast((z) => z < this.shownZoom - 0.005) ?? ZOOM_MIN;
	}
	/** the kinds the inspector's tab is on (their boxes take the pointer); null = every kind */
	get layers() {
		return KINDS[this.inspectorTab] ?? null;
	}
	/** a group or one of its slices is open */
	get inGroup() {
		return this.section === 'group' || this.section === 'slice';
	}

	/** remembers snap, grid size and zoom on this device (storage unavailable: the choice holds for this visit) */
	savePrefs() {
		try {
			localStorage.setItem(SNAP_KEY, JSON.stringify(this.snap));
			localStorage.setItem(GRID_KEY, JSON.stringify(this.gridSize));
			localStorage.setItem(ZOOM_KEY, JSON.stringify(this.zoom));
			localStorage.setItem(SEAMS_KEY, JSON.stringify(this.seams));
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
	/**
	 * the strip (TEXT-ACROSS-SLICES.md §4): the slice in the view's centre, or the one a pointer went down on, becomes the
	 * open slice – without the jump of selectSlice (the strip scrolls itself); a selection on another slice is let go
	 */
	focusSlice(index: number) {
		if (index === this.sliceIndex && this.section === 'slice') return;
		this.sliceIndex = index;
		this.selection = null;
		this.section = 'slice';
	}
	/** a canvas / list selection opens its layer's tab (the Layers tab keeps an area picked in its stack) */
	select(next: CanvasSelection | null) {
		this.selection = next;
		if (next && KINDS[this.inspectorTab]?.includes(next.kind)) return;
		if (next?.kind === 'dialog' || next?.kind === 'item' || next?.kind === 'piece') this.setTab('layers');
		else if (next?.kind === 'sample') this.setTab('audio');
		else if (next?.kind === 'event') this.setTab('events');
	}
	setTab(next: InspectorTab) {
		this.inspectorTab = next;
		// a selection on another layer is let go (its handles would sit on a layer that no longer takes the pointer)
		if (this.selection && !KINDS[next]?.includes(this.selection.kind)) this.selection = null;
		if (next === 'layers') {
			if (!LAYER_TOOLS.includes(this.tool)) this.tool = 'text';
			loadCatalog();
		} else if (TAB_TOOL[next]) this.tool = TAB_TOOL[next]!;
	}
	setTool(next: Tool) {
		this.tool = next;
		// another tool draws over the art: a selected piece is let go (its box would keep the pointer – canvas-pieces.ts)
		if (next !== 'piece' && this.selection?.kind === 'piece') this.selection = null;
		this.setTab(TOOL_TAB[next]);
	}
	/** "+ item" (spot = where on the slice, null = the middle) */
	pickItem(spot: { x: number; y: number } | null = null) {
		this.itemSpot = spot;
		this.pickingItem = true;
	}
	/** the piece tool / "+ piece": the picker, then the piece at the spot (a click: centred; a drag: its rect; null: the middle) */
	pickPiece(spot: { x: number; y: number; width?: number } | null = null) {
		this.pieceSpot = piecePlace(spot);
		this.pickingPiece = true;
	}
}
