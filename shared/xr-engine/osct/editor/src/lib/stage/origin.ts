import type { SurfaceData } from "@shared/types/game-config";
import type { SurfaceRect } from "./surfaces";

/**
 * The coordinate origin of the editor's positions (three-free). A target's position is stored as the content has it:
 * mm from the top left corner of its surface (`TargetPlace`). The author may read and enter it from another origin: one
 * of the nine points of an alignment square (a design tool's anchor picker) – the four corners and the four edge centres
 * of the surface the position lies on, and the centre: the scene's origin (a double page's spine). The axes always run
 * x to the right and y downwards; only the zero moves, so a value left of or above it is negative.
 */

/** The nine origins, row by row as the alignment square shows them */
export const ORIGINS = [
  "top-left", "top-centre", "top-right",
  "centre-left", "centre", "centre-right",
  "bottom-left", "bottom-centre", "bottom-right",
] as const;
export type OriginId = (typeof ORIGINS)[number];
export const isOriginId = (value: unknown): value is OriginId => ORIGINS.includes(value as OriginId);

/** Where an origin lies on a surface: its column and row, 0 / 1 / 2 = start / middle / end */
export const ORIGIN_SPOTS: Record<OriginId, { col: 0 | 1 | 2; row: 0 | 1 | 2 }> = {
  "top-left": { col: 0, row: 0 }, "top-centre": { col: 1, row: 0 }, "top-right": { col: 2, row: 0 },
  "centre-left": { col: 0, row: 1 }, centre: { col: 1, row: 1 }, "centre-right": { col: 2, row: 1 },
  "bottom-left": { col: 0, row: 2 }, "bottom-centre": { col: 1, row: 2 }, "bottom-right": { col: 2, row: 2 },
};

export const ORIGIN_LABELS: Record<OriginId, string> = {
  "top-left": "top left",
  "top-centre": "top centre",
  "top-right": "top right",
  "centre-left": "left centre",
  centre: "scene centre",
  "centre-right": "right centre",
  "bottom-left": "bottom left",
  "bottom-centre": "bottom centre",
  "bottom-right": "bottom right",
};

/** The origin's name */
export const originLabel = (origin: OriginId): string => ORIGIN_LABELS[origin];

/** What the origin is, for the Info view */
export const originHelp = (origin: OriginId): string =>
  origin === "centre"
    ? "The scene's centre: its origin (a double page's spine), one zero for every surface lying flat."
    : `The ${ORIGIN_LABELS[origin]} ${ORIGIN_SPOTS[origin].col === 1 || ORIGIN_SPOTS[origin].row === 1 ? "point" : "corner"} of the surface a position lies on.`;

/** A surface by its id */
export type SurfaceId = string;

export interface OriginFrame {
  /** The scene's surfaces lying flat on the table (`surfaces.ts` `surfaceRects`: stage mm, y up), each at its own size */
  surfaces: readonly SurfaceRect[];
}

/** The stand-in rectangle while the frame has no surface: a double page centred on the origin */
const DEFAULT_RECT: SurfaceRect = { id: "surface", left: -230, top: 160, width: 460, height: 320 };

/**
 * The frame of a scene's surfaces lying flat on the table, from the content alone (three-free, as `surfaces.ts`
 * `surfaceRects` would have them): unturned, at height 0 – a surface's centre (x, z) is the stage's (x, −z)
 */
export const flatFrame = (surfaces: readonly Pick<SurfaceData, "id" | "widthMm" | "heightMm" | "place">[] | undefined): OriginFrame => ({
  surfaces: (surfaces ?? [])
    .filter(s => s.widthMm > 0 && s.heightMm > 0 && !s.place.rx && !s.place.ry && !s.place.rz && !s.place.y)
    .map(s => ({ id: s.id, left: round(s.place.x - s.widthMm / 2), top: round(-s.place.z + s.heightMm / 2), width: s.widthMm, height: s.heightMm })),
});

/** The surfaces the frame counts on: the scene's, else the stand-in */
export const frameRects = (frame: OriginFrame): readonly SurfaceRect[] => (frame.surfaces.length ? frame.surfaces : [DEFAULT_RECT]);

/** The rectangle around all the frame's surfaces (stage mm, y up): what its grid and lines count from */
export const frameBox = (frame: OriginFrame): { left: number; top: number; width: number; height: number } => {
  const rects = frameRects(frame);
  const left = Math.min(...rects.map(r => r.left));
  const top = Math.max(...rects.map(r => r.top));
  const right = Math.max(...rects.map(r => r.left + r.width));
  const bottom = Math.min(...rects.map(r => r.top - r.height));
  return { left, top, width: right - left, height: top - bottom };
};

/** A surface of the frame by id, else the first */
export const rectOf = (frame: OriginFrame, surface: SurfaceId | undefined): SurfaceRect => {
  const rects = frameRects(frame);
  return rects.find(r => r.id === surface) ?? rects[0];
};

/**
 * The origin's position in the surface's own frame (mm from its top left corner): the centre is the scene's origin,
 * the others a point of the surface itself
 */
export const originPoint = (origin: OriginId, frame: OriginFrame, surface: SurfaceId | undefined): { x: number; y: number } => {
  const rect = rectOf(frame, surface);
  if (origin === "centre") return { x: round(0 - rect.left), y: round(rect.top) };
  const { col, row } = ORIGIN_SPOTS[origin];
  return { x: (col * rect.width) / 2, y: (row * rect.height) / 2 };
};

const round = (value: number): number => Math.round(value * 100) / 100;

/** A position from the surface's top left corner → the same spot read from the origin */
export const toOrigin = (origin: OriginId, frame: OriginFrame, surface: SurfaceId | undefined, x: number, y: number): { x: number; y: number } => {
  const zero = originPoint(origin, frame, surface);
  return { x: round(x - zero.x), y: round(y - zero.y) };
};

/** A position read from the origin → mm from the surface's top left corner */
export const fromOrigin = (origin: OriginId, frame: OriginFrame, surface: SurfaceId | undefined, x: number, y: number): { x: number; y: number } => {
  const zero = originPoint(origin, frame, surface);
  return { x: round(x + zero.x), y: round(y + zero.y) };
};

/** A surface's rectangle (mm from the top left corner of all the flat surfaces' box) the grid covers, with the origin's zero */
export interface GridArea {
  /** The surface it covers (its own spacing, #316) */
  surface: string;
  left: number;
  top: number;
  width: number;
  height: number;
  /** The zero, in mm from the box's top left corner */
  zeroX: number;
  zeroY: number;
}

/**
 * The grid's areas: one per flat surface (each with its own spacing, #316), counting from the origin's point on that
 * surface – for the scene's centre from the scene's origin, so equal spacings line up across the surfaces
 */
export const gridAreas = (origin: OriginId, frame: OriginFrame): GridArea[] => {
  const box = frameBox(frame);
  return frameRects(frame).map(rect => {
    const left = round(rect.left - box.left);
    const top = round(box.top - rect.top);
    const zero = originPoint(origin, frame, rect.id);
    return { surface: rect.id, left, top, width: rect.width, height: rect.height, zeroX: round(left + zero.x), zeroY: round(top + zero.y) };
  });
};

/**
 * The grid's lines along one axis inside [start, start + size], as positions in the box's frame: every `step`
 * from the zero in both directions, so a line always runs through the origin
 */
export const gridLines = (zero: number, start: number, size: number, step: number): number[] => {
  if (!(step > 0) || !(size > 0)) return [];
  const lines: number[] = [];
  let at = zero + Math.ceil((start - zero) / step - 1e-9) * step;
  for (; at <= start + size + 1e-9; at += step) lines.push(round(at));
  return lines;
};

/**
 * The **origin gizmo** (#316, #314): the scene's axes at its origin as line segments in the stage frame (metres; scene x =
 * stage x, scene y – up – = stage z, scene z – towards the reader – = stage −y), each `length` mm with an arrowhead at
 * its tip, and a small ring on the table round the origin. Pairs of points, ready for line segments.
 */
export const originGizmo = (length: number, ring = 8): { x: number[]; y: number[]; z: number[]; ring: number[] } => {
  const a = length / 1000;
  const back = a * 0.82;
  const spread = a * 0.06;
  const circle: number[] = [];
  const r = ring / 1000;
  const parts = 24;
  for (let i = 0; i < parts; i++) {
    const [t0, t1] = [(i / parts) * 2 * Math.PI, ((i + 1) / parts) * 2 * Math.PI];
    circle.push(r * Math.cos(t0), r * Math.sin(t0), 0, r * Math.cos(t1), r * Math.sin(t1), 0);
  }
  return {
    // x: right; its head opens in the table's plane
    x: [0, 0, 0, a, 0, 0, a, 0, 0, back, spread, 0, a, 0, 0, back, -spread, 0],
    // y (up): stage z; its head opens along x
    y: [0, 0, 0, 0, 0, a, 0, 0, a, spread, 0, back, 0, 0, a, -spread, 0, back],
    // z (towards the reader): stage −y; its head opens along x
    z: [0, 0, 0, 0, -a, 0, 0, -a, 0, spread, -back, 0, 0, -a, 0, -spread, -back, 0],
    ring: circle,
  };
};

/** The most subdivision lines a surface's grid draws along one axis – a finer net is left out (the main lines stay) */
export const MAX_SUBDIVISION_LINES = 400;

/**
 * A grid's **subdivision** lines along one axis (#314): every `step / sub` from the zero, without those on the main
 * lines (`gridLines`); none for `sub` ≤ 1 or when there would be more than `MAX_SUBDIVISION_LINES`
 */
export const subdivisionLines = (zero: number, start: number, size: number, step: number, sub: number): number[] => {
  if (!(sub > 1) || !(step > 0) || size / (step / sub) > MAX_SUBDIVISION_LINES) return [];
  const main = new Set(gridLines(zero, start, size, step));
  return gridLines(zero, start, size, step / sub).filter(at => !main.has(at));
};
