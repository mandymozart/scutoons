/**
 * The stage's snapping math (three-free). Everything is counted in the **stage frame** in mm: the origin at the scene's
 * origin (a double page's spine), x to the right, y away from the reader, z up
 * (`placement.ts`). The scene's side – turning a node's position into this frame and back, collecting the other
 * objects' boxes – is `snap-scene.ts`.
 *
 * - **Move** snaps per axis: the moving thing has anchors (its pivot and the edges and centre of its box); the nearest
 *   line within the threshold to any anchor wins – an **axis** (the origin's zero lines and the page plane), another
 *   **object** (its box's edges and centre, its pivot) or an **outline** (a printed target's edges and centre, the
 *   page's edges). With no line near, the pivot rounds to the **grid** (every grid step from the origin's zero) when the
 *   grid is a snap target, else to the move step. Only the axes the drag may change snap (`freeAxes`).
 * - **Rotate** snaps the angle of a drag to whole steps of degrees, **scale** each factor to whole steps (never below
 *   one step, so a node never collapses).
 * - Snapping is on when the setting says so; Shift (or Ctrl) held turns it around for that drag (`snapActive`).
 */

import { frameBox, frameRects, gridAreas, rectOf, type OriginFrame, type OriginId } from "./origin";
import { MIN_PLACE_WIDTH, withWidth, type PlaceBox } from "./placement";

export const SNAP_TARGETS = ["grid", "axes", "objects", "outlines"] as const;
export type SnapTarget = (typeof SNAP_TARGETS)[number];

/** What a line belongs to: an axis, another object, an outline (a printed target, a page) */
export type SnapKind = Exclude<SnapTarget, "grid">;

/** A line one axis can snap to: its place on that axis (mm) */
export interface SnapLine {
  at: number;
  kind: SnapKind;
}

/** Steps along an axis: every `step` mm from `zero` */
export interface Increment {
  zero: number;
  step: number;
}

export type Vec3 = readonly [number, number, number];
export type PerAxis<T> = [T, T, T];

/** How near a line must be drawn to catch an anchor (CSS px); turned into mm by the camera's distance */
export const SNAP_PX = 8;

/** The precision's bounds: move step (mm), rotate step (degrees), scale step */
export const SNAP_LIMITS = {
  move: { min: 0.1, max: 100 },
  rotate: { min: 1, max: 90 },
  scale: { min: 0.01, max: 1 },
} as const;

export const clampSnap = (mode: keyof typeof SNAP_LIMITS, value: number): number =>
  Math.min(SNAP_LIMITS[mode].max, Math.max(SNAP_LIMITS[mode].min, value));

/** Snapping is on right now: the setting, turned around while Shift or Ctrl is held */
export const snapActive = (setting: boolean, modifier: boolean): boolean => setting !== modifier;

const tidy = (value: number): number => Math.round(value * 1e6) / 1e6;

/** `value` at the nearest of the steps of `step` from `zero` (a step of 0 or less leaves it) */
export const snapStep = (value: number, step: number, zero = 0): number =>
  step > 0 && Number.isFinite(value) ? tidy(zero + Math.round((value - zero) / step) * step) : value;

/** An angle (degrees) at whole steps of `step` degrees */
export const snapAngle = (degrees: number, step: number): number => snapStep(degrees, step);

/** A scale factor at whole steps of `step`, its sign kept, never below one step */
export const snapScale = (value: number, step: number): number => {
  if (!(step > 0) || !Number.isFinite(value)) return value;
  const sign = value < 0 ? -1 : 1;
  return sign * Math.max(tidy(step), snapStep(Math.abs(value), step));
};

/** The snap threshold in mm: `px` on the screen at `distanceMm` from a camera with a vertical field of view of `fovDeg` over `viewportPx` */
export const thresholdMm = (px: number, distanceMm: number, fovDeg: number, viewportPx: number): number =>
  viewportPx > 0 && distanceMm > 0 ? (px * 2 * distanceMm * Math.tan((fovDeg * Math.PI) / 360)) / viewportPx : 0;

/** The result of one axis: the value and what it snapped to – a line, the increments, or nothing */
export interface AxisSnap {
  value: number;
  line: SnapLine | null;
  stepped: boolean;
}

/**
 * One axis of a move. `value` is the pivot, `offsets` the other anchors relative to it (the box's edges and centre);
 * the nearest line within `threshold` of any anchor wins and the value moves so that anchor lies on it. With none near,
 * the value rounds to `increment` (null: it stays).
 */
export const snapAxis = (value: number, offsets: readonly number[], lines: readonly SnapLine[], threshold: number, increment: Increment | null = null): AxisSnap => {
  let best: { distance: number; value: number; line: SnapLine } | null = null;
  const anchors = offsets.includes(0) ? offsets : [0, ...offsets];
  for (const offset of anchors) {
    for (const line of lines) {
      const distance = Math.abs(value + offset - line.at);
      if (distance <= threshold && (!best || distance < best.distance - 1e-9)) best = { distance, value: tidy(line.at - offset), line };
    }
  }
  if (best) return { value: best.value, line: best.line, stepped: false };
  if (increment && increment.step > 0) return { value: snapStep(value, increment.step, increment.zero), line: null, stepped: true };
  return { value, line: null, stepped: false };
};

/**
 * Which stage axes a constrained drag may change: those lying in the constraint – a line (one direction) or a plane
 * (two directions), each a unit vector in the stage frame; three directions (or none) leave every axis free. An axis
 * only nearly in the constraint (a turned node) stays put: snapping it would pull the node off its handle's line.
 */
export const freeAxes = (directions: readonly Vec3[], tolerance = 1e-3): PerAxis<boolean> => {
  if (directions.length === 1) {
    const [d] = directions;
    return [0, 1, 2].map(k => Math.abs(d[k]) >= 1 - tolerance) as PerAxis<boolean>;
  }
  if (directions.length === 2) {
    const [a, b] = directions;
    const n: Vec3 = [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]];
    const length = Math.hypot(...n) || 1;
    return [0, 1, 2].map(k => Math.abs(n[k] / length) <= tolerance) as PerAxis<boolean>;
  }
  return [true, true, true];
};

/** A guide: a line something snapped to, drawn while the drag lasts (axis 0 = x, 1 = y, 2 = z) */
export interface SnapGuide {
  axis: 0 | 1 | 2;
  line: SnapLine;
}

export interface MoveSnap {
  /** The pivot (stage mm) */
  point: Vec3;
  /** The other anchors relative to the pivot, per axis (the box's edges and centre) */
  offsets: PerAxis<readonly number[]>;
  lines: PerAxis<readonly SnapLine[]>;
  free: PerAxis<boolean>;
  threshold: number;
  increments: PerAxis<Increment | null>;
}

/** A move snapped on every free axis; the guides name the lines it snapped to */
export const snapMove = (input: MoveSnap): { point: Vec3; guides: SnapGuide[] } => {
  const guides: SnapGuide[] = [];
  const point = input.point.map((value, k) => {
    if (!input.free[k]) return value;
    const result = snapAxis(value, input.offsets[k], input.lines[k], input.threshold, input.increments[k]);
    if (result.line) guides.push({ axis: k as 0 | 1 | 2, line: result.line });
    return result.value;
  }) as unknown as Vec3;
  return { point, guides };
};

// ── The stage's lines ──

/**
 * The scene on the stage: its surfaces lying flat on the table (#314). `local`: the frame is one wall's or turned
 * surface's own plane (its top left
 * corner at 0, y up – `surfaces.ts` `ownRect`), where a print on it snaps to that surface's edges and prints only.
 */
export type StageFrame = OriginFrame & { local?: boolean };

/** A point of the scene's frame (mm from its top left corner, y down) in the stage frame */
export const frameToStage = (frame: StageFrame, x: number, y: number): { x: number; y: number } => {
  const box = frameBox(frame);
  return { x: tidy(box.left + x), y: tidy(box.top - y) };
};

const rectFor = rectOf;

/** A box in the stage frame (mm) */
export interface StageBox {
  min: Vec3;
  max: Vec3;
}

/** A box's lines per axis: both edges and the centre */
export const boxLines = (box: StageBox, kind: SnapKind): PerAxis<SnapLine[]> =>
  [0, 1, 2].map(k => [box.min[k], (box.min[k] + box.max[k]) / 2, box.max[k]].map(at => ({ at: tidy(at), kind }))) as PerAxis<SnapLine[]>;

/** A point's lines per axis (an object's pivot) */
export const pointLines = (point: Vec3, kind: SnapKind): PerAxis<SnapLine[]> =>
  [0, 1, 2].map(k => [{ at: tidy(point[k]), kind }]) as PerAxis<SnapLine[]>;

/** The axes: the origin's zero lines (one per page for a corner origin, one for the scene's centre) and the page plane */
export const axisLines = (origin: OriginId, frame: StageFrame): PerAxis<SnapLine[]> => {
  const x: SnapLine[] = [];
  const y: SnapLine[] = [];
  for (const area of gridAreas(origin, frame)) {
    const zero = frameToStage(frame, area.zeroX, area.zeroY);
    if (!x.some(line => line.at === zero.x)) x.push({ at: zero.x, kind: "axes" });
    if (!y.some(line => line.at === zero.y)) y.push({ at: zero.y, kind: "axes" });
  }
  return [x, y, [{ at: 0, kind: "axes" }]];
};

/** The pages' outlines: their edges (the spine too) and the page plane */
export const surfaceLines = (frame: StageFrame): PerAxis<SnapLine[]> => {
  const unique = (values: number[]) => [...new Set(values.map(tidy))];
  const rects = frameRects(frame);
  return [
    unique(rects.flatMap(r => [r.left, r.left + r.width])).map(at => ({ at, kind: "outlines" as const })),
    unique(rects.flatMap(r => [r.top, r.top - r.height])).map(at => ({ at, kind: "outlines" as const })),
    [{ at: 0, kind: "outlines" }],
  ];
};

/** A printed target's box in the stage frame (flat on the page), from its place and its printed height */
export const placeStageBox = (box: PlaceBox, height: number, frame: StageFrame): StageBox => {
  const rect = rectFor(frame, box.surface);
  const left = rect.left + box.x;
  const top = rect.top - box.y;
  return { min: [tidy(left), tidy(top - height), 0], max: [tidy(left + box.width), tidy(top), 0] };
};

/** Lines of several sources joined per axis */
export const joinLines = (...sources: PerAxis<readonly SnapLine[]>[]): PerAxis<SnapLine[]> =>
  [0, 1, 2].map(k => sources.flatMap(source => source[k])) as PerAxis<SnapLine[]>;

/**
 * The steps at a point (stage mm): every `step` mm from the origin's zero of the grid area the point lies in (the page
 * under it for a corner origin, the scene for its centre) along x and y, from the page plane along z
 */
export const incrementsAt = (x: number, origin: OriginId, frame: StageFrame, step: number): PerAxis<Increment> => {
  const areas = gridAreas(origin, frame).map(area => ({
    left: frameToStage(frame, area.left, 0).x,
    right: frameToStage(frame, area.left + area.width, 0).x,
    zero: frameToStage(frame, area.zeroX, area.zeroY),
  }));
  const distance = (area: (typeof areas)[number]) => (x < area.left ? area.left - x : x > area.right ? x - area.right : 0);
  const area = areas.reduce((best, next) => (distance(next) < distance(best) ? next : best), areas[0]);
  return [{ zero: area.zero.x, step }, { zero: area.zero.y, step }, { zero: 0, step }];
};

/** The snap settings a drag reads (the preview settings, `preview.ts`) */
export interface SnapSettings {
  snapTo: readonly SnapTarget[];
  /** The grid's step (mm) */
  gridMm: number;
  /** The move's step when the grid is no snap target (mm) */
  snapMove: number;
  origin: OriginId;
}

/** The increments of a move at a point: the grid's step when the grid is a snap target, else the move step */
export const moveIncrements = (x: number, settings: SnapSettings, frame: StageFrame): PerAxis<Increment> =>
  incrementsAt(x, settings.origin, frame, settings.snapTo.includes("grid") ? settings.gridMm : settings.snapMove);

/**
 * The settings a print on a surface snaps with (#316, #314): its surface grid's step (`step` – the spacing over its
 * subdivisions) as the grid's, the grid a snap target there; `null` – the surface's grid snaps not – the grid no snap
 * target there (the move step then); `undefined` – no surface grid known – the settings as they are
 */
export const surfaceSnapSettings = <S extends SnapSettings>(settings: S, step: number | null | undefined): S => {
  if (step === undefined) return settings;
  const others = settings.snapTo.filter(t => t !== "grid");
  return step === null ? { ...settings, snapTo: others } : { ...settings, gridMm: step, snapTo: SNAP_TARGETS.filter(t => t === "grid" || others.includes(t)) };
};

/** The snap targets with `target` switched on or off, in their fixed order */
export const toggleSnapTarget = (targets: readonly SnapTarget[], target: SnapTarget): SnapTarget[] =>
  SNAP_TARGETS.filter(t => (t === target ? !targets.includes(t) : targets.includes(t)));

/** Only the lines of the chosen snap targets */
export const chosenLines = (lines: PerAxis<readonly SnapLine[]>, snapTo: readonly SnapTarget[]): PerAxis<SnapLine[]> =>
  [0, 1, 2].map(k => lines[k].filter(line => snapTo.includes(line.kind))) as PerAxis<SnapLine[]>;

// ── The placement handles: a printed target dragged on the page ──

/**
 * A print's place snapped (`draggedPlace`'s result): a **move** snaps the box's left edge, centre or right edge and its
 * top edge, centre or bottom edge to the lines (other prints, the page's edges, the axes, objects) – else its top left
 * corner to the steps; the **corner** (the width) snaps the right edge. `height` is the printed height (mm).
 */
export const snapPlace = <P extends PlaceBox>(
  place: P,
  height: number,
  kind: "move" | "width",
  input: { frame: StageFrame; lines: PerAxis<readonly SnapLine[]>; threshold: number; settings: SnapSettings },
): { place: P; guides: SnapGuide[] } => {
  const { frame, lines, threshold, settings } = input;
  const box = placeStageBox(place, height, frame);
  const left = box.min[0];
  const top = box.max[1];
  const steps = moveIncrements(left, settings, frame);
  const guides: SnapGuide[] = [];
  if (kind === "width") {
    const right = snapAxis(box.max[0], [], lines[0], threshold, steps[0]);
    if (right.line) guides.push({ axis: 0, line: right.line });
    return { place: withWidth(place, Math.max(MIN_PLACE_WIDTH, tidy(right.value - left))), guides };
  }
  const x = snapAxis(left, [place.width / 2, place.width], lines[0], threshold, steps[0]);
  const y = snapAxis(top, [-height / 2, -height], lines[1], threshold, steps[1]);
  if (x.line) guides.push({ axis: 0, line: x.line });
  if (y.line) guides.push({ axis: 1, line: y.line });
  return {
    place: { ...place, x: tidy(x.value - rectFor(frame, place.surface).left), y: tidy(rectFor(frame, place.surface).top - y.value) },
    guides,
  };
};
