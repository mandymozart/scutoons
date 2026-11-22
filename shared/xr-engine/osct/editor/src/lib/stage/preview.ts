/**
 * The editor's preview (three-free, main chunk): the AR scene runs over a simulated camera picture – the mock
 * tracker and the mock stage (`ar/mock-tracker.ts`, `ar/mock-stage.ts`) stand in for the camera and the engine.
 *
 * - `active`: the editor page is open – `ArScene` builds the mock tracker instead of the engine (a change while the
 *   scene exists rebuilds it). Not stored: the route decides.
 * - The settings the stage toolbar changes, stored on this device (`osct-preview`): the view preset or a free orbit,
 *   the shading, the sky (an HDRI), the desk's top, the hand-shake amount and the origin (the *view* group, #229), the
 *   grids – the surfaces' own grid with its vertical axis, the scene's floor grid and the optional vertical grid
 *   (#316, #319) – and the snapping (the *grid & snap* group: on / off, what it snaps to, the precision per gizmo mode –
 *   `snap.ts`).
 * - `surfaceGrids`: the open scene's surfaces' own grids (#316, #314) by surface id – spacing, subdivisions, on / off and
 *   snap, each left out = the toolbar's (`SurfaceGrid`) – kept per experience and surface in this browser by the page
 *   (`surface-grids.svelte.ts`), handed to the stage here. Not stored with the settings.
 * - `hidden`: targets hidden in the scene by the tree's eye – the mock reports them as lost (outro), on again as found
 *   (reveal), and the stage leaves out their printed images (#250). Not stored.
 * - `hiddenNodes`: nodes of an entity hidden on the stage by the tree's eye (#311 – to look into a complex tree), by
 *   target and path (`nodeKey`); the stage stops drawing that node and its children (`node-hiding.ts`). Only the stage
 *   changes – never the content. Not stored: forgotten when the editor page closes.
 */

import { isOriginId, originLabel, type OriginId } from "./origin";
import { clampSnap, SNAP_TARGETS, type SnapTarget } from "./snap";
import { readStoredJson, writeStoredJson } from "#lib/storage.js";
import { isTextureSize, TEXTURE_SIZES } from "@shared/types/textures";
import type { TextureQuality } from "@engine/types/graphics";

/**
 * The texture quality the stage shows models with a texture set at (#260): `auto` as a reader's device would choose
 * (this computer: its memory and connection), or one size – to see what readers on each quality see.
 */
export const TEXTURE_QUALITIES: readonly TextureQuality[] = ["auto", ...TEXTURE_SIZES];
export const isTextureQuality = (value: unknown): value is TextureQuality => value === "auto" || isTextureSize(value);

export const PREVIEW_VIEWS = ["reader", "top", "low", "far"] as const;
export type PreviewView = (typeof PREVIEW_VIEWS)[number];

/** The HDRIs the stage offers as sky and light (Poly Haven, CC0 – `editor/static/hdri/README.md`) */
export const HDRIS = [
  { id: "ferndale_studio_04", name: "Ferndale Studio 04" },
  { id: "wooden_studio_07", name: "Wooden Studio 07" },
  { id: "wooden_studio_13", name: "Wooden Studio 13" },
  { id: "wooden_studio_14", name: "Wooden Studio 14" },
  { id: "wooden_studio_17", name: "Wooden Studio 17" },
  { id: "wooden_studio_19", name: "Wooden Studio 19" },
] as const;
export type HdriId = (typeof HDRIS)[number]["id"];
export const isHdriId = (value: unknown): value is HdriId => HDRIS.some(h => h.id === value);

/**
 * The stage's desk (our own low-poly model – `editor/static/models/README.md`; designers' furniture, so the designers
 * who use the tool feel at home): Egon Eiermann's table frame with one of two tops of the same size (#226) – its own
 * screen-printed plywood top, or blue-grey linoleum on an oak-faced plywood board. The setting switches only the top.
 */
export const DESK_NAME = "Eiermann";
export const TOPS = [
  { id: "plywood", name: "plywood" },
  { id: "linoleum", name: "linoleum" },
] as const;
export type TopId = (typeof TOPS)[number]["id"];
export const isTopId = (value: unknown): value is TopId => TOPS.some(t => t.id === value);
const DEFAULT_TOP: TopId = "plywood";

/** The top from stored settings: `top`, else the default */
export const storedTop = (stored: { top?: unknown }): TopId => (isTopId(stored.top) ? stored.top : DEFAULT_TOP);

/**
 * The stage toolbar's *view* group in a few words (#229 – the Stage tab's closed *stage* head until then): the desk and
 * its top, the origin, the hand shake and the sky – "Eiermann · plywood · origin top left · no shake · sky Wooden
 * Studio 17"; with the experience's environment the sky is its world ("· the environment's sky"). The group's tooltip; the
 * grid and snapping are the *grid & snap* group's own.
 */
export const viewSummary = (
  settings: Pick<PreviewSettings, "top" | "wobble" | "origin" | "hdri">,
  { environment = false }: { environment?: boolean } = {},
): string => [
  DESK_NAME,
  TOPS.find(t => t.id === settings.top)?.name ?? settings.top,
  `origin ${originLabel(settings.origin)}`,
  settings.wobble > 0 ? `shake ${Math.round(settings.wobble * 100) / 100}` : "no shake",
  environment ? "the environment's sky" : `sky ${HDRIS.find(h => h.id === settings.hdri)?.name ?? settings.hdri}`,
].join(" · ");

/** A free view of the stage: distance from the looked-at point in mm, tilt from the vertical and yaw in degrees */
export interface PreviewOrbit {
  distance: number;
  tilt: number;
  yaw: number;
  /** The looked-at point moved from the scene's own: mm in the page plane (a pan) */
  pan?: { x: number; y: number };
}

/**
 * The viewport's shading, after Blender's: **wireframe** (every entity as lines), **solid** (one matte grey, the
 * scene's lights), **textured** (the content's materials, the scene's lights, no sky) and **rendered** (the
 * content's materials under the HDRI's sky and light). Applied per entity mesh – the originals are kept and come
 * back – so the stage, the gizmo and the outlines keep their own materials.
 */
export const SHADING_MODES = ["wireframe", "solid", "textured", "rendered"] as const;
export type ShadingMode = (typeof SHADING_MODES)[number];
export const isShadingMode = (value: unknown): value is ShadingMode => SHADING_MODES.includes(value as ShadingMode);

export interface PreviewSettings {
  view: PreviewView;
  /** The viewport's shading (Blender's): wireframe, solid, textured, rendered (the HDRI's sky and light) */
  shading: ShadingMode;
  /** A free orbit (dragged, zoomed) replaces the preset until the next preset is chosen */
  orbit: PreviewOrbit | null;
  /** The HDRI shown as sky and used as light (while the shading is rendered) */
  hdri: HdriId;
  /** The HDRI on (#327): off = no sky and no HDRI light while rendered – the scene's own lights, as in solid / textured */
  hdriOn: boolean;
  /** The desk's top under the scene (the frame is always the Eiermann) */
  top: TopId;
  /** The size models with a texture set show their map at (the reader's texture quality, previewed) */
  textureQuality: TextureQuality;
  /** 0 = still, 1 = a hand holding a phone */
  wobble: number;
  /**
   * The surfaces' own grid (the targets' – #316 part 1): on / off, its spacing in mm (a surface may have its own:
   * `surfaceGrids`), and the origin it and the targets' positions are counted from
   */
  grid: boolean;
  gridMm: number;
  origin: OriginId;
  /** The vertical axis: a line for the height through the origin, shown with the grid; the scene's axes too */
  axis: boolean;
  /**
   * The scene's **floor grid** (#319): a transparent net on the table (y = 0), every `floorMm` – the snap grid of the
   * surfaces' places and the surface gizmo
   */
  floorGrid: boolean;
  floorMm: number;
  /** The optional **vertical grid** for wall setups: a plane at the scene's z = `wallZ` mm, the floor grid's spacing */
  wallGrid: boolean;
  wallZ: number;
  /**
   * The **target marks** (#323): every target's place framed on its surface with its id – green and solid for a target
   * in the experience, orange and dashed for a switched-off one. A helper, off by default
   */
  targetMarks: boolean;
  /** The gizmo and the placement handles snap (Shift or Ctrl held turns it around for a drag) */
  snap: boolean;
  /** What a move snaps to */
  snapTo: SnapTarget[];
  /** The precision per mode: the move's step when the grid is no snap target (mm), the rotation's (degrees), the scale's */
  snapMove: number;
  snapRotate: number;
  snapScale: number;
}

export interface PreviewState extends PreviewSettings {
  active: boolean;
  hidden: readonly string[];
  /** Nodes hidden on the stage only, as `nodeKey` writes them ("target:0.2", the root "target:") */
  hiddenNodes: readonly string[];
  /** The open scene's surfaces' own grids by surface id; a surface not named – or a field left out – takes the toolbar's (#316) */
  surfaceGrids: Readonly<Record<string, SurfaceGrid>>;
}

/** A node of a target's entity as the hidden nodes name it: the target id, a colon, the path ("key:0.2"; the root "key:") */
export const nodeKey = (targetId: string, path: readonly number[]): string => `${targetId}:${path.join(".")}`;

/**
 * The most nodes kept hidden; a key is a target id (no spaces, no colon, up to 200 characters) and a path up to 16 deep
 */
export const MAX_HIDDEN_NODES = 50;
const NODE_KEY = /^[^\s:]{1,200}:(?:\d{1,4}(?:\.\d{1,4}){0,15})?$/;
export const isNodeKey = (value: unknown): value is string => typeof value === "string" && NODE_KEY.test(value);

/** A node key read back: the target and the path, null when it is none */
export const parseNodeKey = (key: string): { targetId: string; path: number[] } | null => {
  if (!isNodeKey(key)) return null;
  const at = key.lastIndexOf(":");
  const path = key.slice(at + 1);
  return { targetId: key.slice(0, at), path: path ? path.split(".").map(Number) : [] };
};

/** Hidden nodes from a stored or sent value: node keys only, each once, at most `MAX_HIDDEN_NODES` */
export const parseHiddenNodes = (value: unknown): string[] =>
  Array.isArray(value) ? [...new Set(value.filter(isNodeKey))].slice(0, MAX_HIDDEN_NODES) : [];

/** The node is hidden on the stage, by itself or by a node above it (its parents, its root) */
export const isNodeHidden = (hidden: readonly string[], targetId: string, path: readonly number[]): boolean =>
  Array.from({ length: path.length + 1 }, (_, depth) => nodeKey(targetId, path.slice(0, depth))).some(key => hidden.includes(key));

const STORAGE_KEY = "osct-preview";

const DEFAULTS: PreviewSettings = {
  view: "reader", shading: "rendered", orbit: null, hdri: "wooden_studio_17", hdriOn: true, top: DEFAULT_TOP, textureQuality: "auto", wobble: 0, grid: false, gridMm: 10, origin: "top-left",
  axis: true, floorGrid: true, floorMm: 50, wallGrid: false, wallZ: -500, targetMarks: false, snap: false, snapTo: [...SNAP_TARGETS], snapMove: 1, snapRotate: 15, snapScale: 0.1,
};

/** The grid's spacing stays between 1 mm and the page's size */
export const clampGrid = (mm: number): number => Math.min(500, Math.max(1, mm));

/** The floor grid's spacing: 1 mm to 1 m */
export const clampFloorGrid = (mm: number): number => Math.min(1000, Math.max(1, mm));

/** The vertical grid's z: within 10 m of the origin */
export const clampWallZ = (mm: number): number => Math.min(10000, Math.max(-10000, mm));

/** The most surfaces with a grid of their own the stage takes at once */
const MAX_SURFACE_GRIDS = 64;

/**
 * A surface's own grid (#316, #314) – an editing aid, kept in the browser, never content: its spacing (mm), its
 * **subdivisions** (finer lines between the spacing's, 1 = none; a print snaps to them), whether it is **drawn** and
 * whether a print on it **snaps** to it. A field left out is the toolbar's (`gridMm`, 1, `grid`, the grid as a snap target).
 */
export interface SurfaceGrid {
  mm?: number;
  sub?: number;
  on?: boolean;
  snap?: boolean;
}

/** The most subdivisions of a grid step */
export const MAX_SUBDIVISIONS = 10;
/** Subdivisions: a whole number from 1 (none) to `MAX_SUBDIVISIONS` */
export const clampSubdivisions = (n: number): number => Math.min(MAX_SUBDIVISIONS, Math.max(1, Math.round(n)));

/** One surface's grid from a stored or sent value: a number is a spacing (as stored before #314); null when nothing is in it */
export const parseSurfaceGrid = (value: unknown): SurfaceGrid | null => {
  if (finite(value)) return { mm: clampGrid(value) };
  if (!value || typeof value !== "object" || Array.isArray(value)) return null;
  const { mm, sub, on, snap } = value as Record<string, unknown>;
  const grid: SurfaceGrid = {
    ...(finite(mm) ? { mm: clampGrid(mm) } : {}),
    ...(finite(sub) ? { sub: clampSubdivisions(sub) } : {}),
    ...(typeof on === "boolean" ? { on } : {}),
    ...(typeof snap === "boolean" ? { snap } : {}),
  };
  return Object.keys(grid).length ? grid : null;
};

/** Surface grids from a sent value: surface ids (as the frame protocol's) with their grid each, checked */
export const parseSurfaceGrids = (value: unknown): Record<string, SurfaceGrid> => {
  if (!value || typeof value !== "object" || Array.isArray(value)) return {};
  const entries = Object.entries(value as Record<string, unknown>)
    .filter(([id]) => /^[^\s]{1,200}$/.test(id))
    .map(([id, grid]) => [id, parseSurfaceGrid(grid)] as const)
    .filter((entry): entry is readonly [string, SurfaceGrid] => entry[1] !== null)
    .slice(0, MAX_SURFACE_GRIDS);
  return Object.fromEntries(entries);
};

/** A surface's grid as it is used: its own fields, the toolbar's for the rest */
export interface ResolvedGrid {
  mm: number;
  sub: number;
  on: boolean;
  snap: boolean;
}

/** The grid a surface has: its own fields, else the toolbar's (spacing, no subdivisions, the grid switch, the grid as a snap target) */
export const resolveSurfaceGrid = (state: Pick<PreviewState, "grid" | "gridMm" | "snapTo" | "surfaceGrids">, surfaceId: string | undefined): ResolvedGrid => {
  const own = (surfaceId !== undefined ? state.surfaceGrids[surfaceId] : undefined) ?? {};
  return { mm: own.mm ?? state.gridMm, sub: own.sub ?? 1, on: own.on ?? state.grid, snap: own.snap ?? state.snapTo.includes("grid") };
};

/** The step a print snaps to on a grid: the spacing over its subdivisions; null when the grid is no snap target there */
export const gridSnapStep = (grid: ResolvedGrid): number | null => (grid.snap ? grid.mm / grid.sub : null);

/** The snap step of a print on a surface (`gridSnapStep` of its grid): null = it does not snap to the grid there */
export const surfaceGridOf = (state: Pick<PreviewState, "grid" | "gridMm" | "snapTo" | "surfaceGrids">, surfaceId: string | undefined): number | null =>
  gridSnapStep(resolveSurfaceGrid(state, surfaceId));

const finite = (value: unknown): value is number => typeof value === "number" && Number.isFinite(value);

/** Settings from a stored (or sent) value, each one checked – a missing or wrong one is the default */
export const parseSettings = (parsed: unknown): PreviewSettings => {
  if (!parsed || typeof parsed !== "object") return { ...DEFAULTS, snapTo: [...DEFAULTS.snapTo] };
  const stored = parsed as Partial<PreviewSettings>;
  return {
    view: PREVIEW_VIEWS.includes(stored.view as PreviewView) ? (stored.view as PreviewView) : DEFAULTS.view,
    shading: isShadingMode(stored.shading) ? stored.shading : DEFAULTS.shading,
    orbit: isOrbit(stored.orbit) ? stored.orbit : null,
    hdri: isHdriId(stored.hdri) ? stored.hdri : DEFAULTS.hdri,
    hdriOn: stored.hdriOn !== false,
    top: storedTop(stored),
    textureQuality: isTextureQuality(stored.textureQuality) ? stored.textureQuality : DEFAULTS.textureQuality,
    wobble: typeof stored.wobble === "number" && Number.isFinite(stored.wobble) ? Math.min(1, Math.max(0, stored.wobble)) : 0,
    grid: stored.grid === true,
    gridMm: typeof stored.gridMm === "number" && Number.isFinite(stored.gridMm) ? clampGrid(stored.gridMm) : DEFAULTS.gridMm,
    origin: isOriginId(stored.origin) ? stored.origin : DEFAULTS.origin,
    axis: typeof stored.axis === "boolean" ? stored.axis : DEFAULTS.axis,
    floorGrid: typeof stored.floorGrid === "boolean" ? stored.floorGrid : DEFAULTS.floorGrid,
    floorMm: finite(stored.floorMm) ? clampFloorGrid(stored.floorMm) : DEFAULTS.floorMm,
    wallGrid: stored.wallGrid === true,
    wallZ: finite(stored.wallZ) ? clampWallZ(stored.wallZ) : DEFAULTS.wallZ,
    targetMarks: stored.targetMarks === true,
    snap: stored.snap === true,
    snapTo: Array.isArray(stored.snapTo) ? SNAP_TARGETS.filter(target => (stored.snapTo as unknown[]).includes(target)) : [...DEFAULTS.snapTo],
    snapMove: finite(stored.snapMove) ? clampSnap("move", stored.snapMove) : DEFAULTS.snapMove,
    snapRotate: finite(stored.snapRotate) ? clampSnap("rotate", stored.snapRotate) : DEFAULTS.snapRotate,
    snapScale: finite(stored.snapScale) ? clampSnap("scale", stored.snapScale) : DEFAULTS.snapScale,
  };
};

const readSettings = (): PreviewSettings => parseSettings(readStoredJson(STORAGE_KEY));

const isOrbit = (value: unknown): value is PreviewOrbit => {
  if (!value || typeof value !== "object") return false;
  const orbit = value as Record<string, unknown>;
  if (!["distance", "tilt", "yaw"].every(key => Number.isFinite(orbit[key]))) return false;
  const pan = orbit.pan as Record<string, unknown> | undefined;
  return pan === undefined || (!!pan && typeof pan === "object" && Number.isFinite(pan.x) && Number.isFinite(pan.y));
};

const writeSettings = (settings: PreviewSettings): void => {
  writeStoredJson(STORAGE_KEY, settings);
};

let state: PreviewState = { ...readSettings(), active: false, hidden: [], hiddenNodes: [], surfaceGrids: {} };
const listeners = new Set<(state: PreviewState) => void>();

const publish = (): void => {
  listeners.forEach(listener => listener(state));
};

export const getPreview = (): PreviewState => state;

/** The editor page is open (true) or left (false) */
export const setPreviewActive = (active: boolean): void => {
  if (state.active === active) return;
  state = { ...state, active, hidden: active ? state.hidden : [], hiddenNodes: active ? state.hiddenNodes : [], surfaceGrids: active ? state.surfaceGrids : {} };
  publish();
};

/** Change settings (stored) and / or the hidden targets; a preset view clears the free orbit */
export const updatePreview = (patch: Partial<Omit<PreviewState, "active">>): void => {
  const next: PreviewState = { ...state, ...patch };
  if (patch.view !== undefined && patch.orbit === undefined) next.orbit = null;
  state = next;
  const { active: _active, hidden: _hidden, hiddenNodes: _hiddenNodes, surfaceGrids: _surfaceGrids, ...settings } = state;
  writeSettings(settings);
  publish();
};

/** Show or hide a target on the stage (hidden = reported as lost) */
export const setTargetHidden = (targetId: string, hidden: boolean): void => {
  const without = state.hidden.filter(id => id !== targetId);
  updatePreview({ hidden: hidden ? [...without, targetId] : without });
};

/** Show or hide a node of a target's entity on the stage only (#311): it and its children are not drawn */
export const setNodeHidden = (targetId: string, path: readonly number[], hidden: boolean): void => {
  const key = nodeKey(targetId, path);
  if (!isNodeKey(key)) return;
  const without = state.hiddenNodes.filter(k => k !== key);
  if (hidden && without.length >= MAX_HIDDEN_NODES) return;
  updatePreview({ hiddenNodes: hidden ? [...without, key] : without });
};

export const subscribePreview = (listener: (state: PreviewState) => void): (() => void) => {
  listeners.add(listener);
  return () => listeners.delete(listener);
};
