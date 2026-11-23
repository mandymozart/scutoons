import { Euler, Matrix4, Vector3 } from "three";
import type { SurfaceData, SurfacePlace } from "@shared/types/game-config";

/**
 * A scene's surfaces on the stage (#314, #319 – `agents/SCENES.md` "Schema 3.0"): where each lies, and the math between a
 * surface's own millimetres and the stage. Two frames meet here:
 *
 * - the **scene's coordinate system** (`SurfacePlace`): mm, the origin on the table at the middle of the first row of
 *   surfaces, +x right, +y up, +z towards the reader; unrotated a surface lies flat, face up, its top edge away from the
 *   reader; `rx`, `ry`, `rz` are degrees, applied as three.js' Euler order `XYZ`;
 * - the **stage frame** the mock stage, the tracker and the snapping count in: the same origin, x right, y away from
 *   the reader (the top of a page lying on the table), z up – so a scene point (x, y, z) is the stage's (x, −z, y).
 *
 * A surface's own frame is the content's for a target's place: mm from its top left corner, x to the right, y downwards.
 * Every surface lies at its `place` (schema 3.0: required) – plain boards, no book, no fold, no side.
 */

export type V3 = readonly [number, number, number];

/** A surface of the scene and where it lies */
export interface SceneSurface {
  id: string;
  label?: string;
  widthMm: number;
  heightMm: number;
  place: Required<SurfacePlace>;
}

/** A surface in the stage frame (mm): its top left corner and its axes – x along its width, y down it, its face's normal */
export interface SurfaceFrame {
  id: string;
  widthMm: number;
  heightMm: number;
  origin: V3;
  right: V3;
  down: V3;
  normal: V3;
  /** Flat on the table plane (face up, unturned, at height 0): the surface grid's 2D frame and the stage's 2D snapping reach it */
  flat: boolean;
}

/** What a surface's rectangle is on the table plane (stage mm, y up): its left edge, its top edge, its size */
export interface SurfaceRect {
  id: string;
  left: number;
  top: number;
  width: number;
  height: number;
}

/** The scene as the stage reads it: its surfaces (a `SceneData` / `Scene`) */
export interface SceneShape {
  surfaces?: readonly SurfaceData[];
}

const ZERO_PLACE: Required<SurfacePlace> = { x: 0, y: 0, z: 0, rx: 0, ry: 0, rz: 0 };
const EPS = 1e-6;

/** The stand-in while no scene is open: one double page of the printed book's size, flat at the origin */
export const DEFAULT_SURFACE: SceneSurface = { id: "surface", widthMm: 460, heightMm: 320, place: { ...ZERO_PLACE } };

const usable = (surface: SurfaceData): boolean => surface.widthMm > 0 && surface.heightMm > 0 && !!surface.id;

/** The scene's surfaces where they lie: each at its `place` (a broken number counts 0) */
export const sceneSurfaces = (scene: SceneShape | undefined): SceneSurface[] =>
  (scene?.surfaces ?? []).filter(usable).map(s => ({
    id: s.id,
    ...(s.label !== undefined ? { label: s.label } : {}),
    widthMm: s.widthMm,
    heightMm: s.heightMm,
    place: { ...ZERO_PLACE, ...definedOf(s.place ?? ZERO_PLACE) },
  }));

/** A place's finite values only (a stray NaN never moves a surface off the stage) */
const definedOf = (place: SurfacePlace): Partial<SurfacePlace> =>
  Object.fromEntries(Object.entries(place).filter(([, v]) => typeof v === "number" && Number.isFinite(v)));

const sceneToStage = (v: Vector3): V3 => [tidy(v.x), tidy(-v.z), tidy(v.y)];
const tidy = (value: number): number => (Math.abs(value) < 1e-9 ? 0 : Math.round(value * 1e6) / 1e6);

/** A surface in the stage frame: its corner and its axes, turned by its place */
export const surfaceFrame = (surface: SceneSurface): SurfaceFrame => {
  const { place } = surface;
  const degrees = Math.PI / 180;
  const turn = new Matrix4().makeRotationFromEuler(new Euler(place.rx * degrees, place.ry * degrees, place.rz * degrees, "XYZ"));
  // Unturned (scene): x along the width, down the surface towards the reader (+z), the face up (+y)
  const right = sceneToStage(new Vector3(1, 0, 0).applyMatrix4(turn));
  const down = sceneToStage(new Vector3(0, 0, 1).applyMatrix4(turn));
  const normal = sceneToStage(new Vector3(0, 1, 0).applyMatrix4(turn));
  const centre = sceneToStage(new Vector3(place.x, place.y, place.z));
  const origin: V3 = [0, 1, 2].map(k => tidy(centre[k] - (right[k] * surface.widthMm) / 2 - (down[k] * surface.heightMm) / 2)) as unknown as V3;
  const flat = Math.abs(normal[2] - 1) < EPS && Math.abs(right[0] - 1) < EPS && Math.abs(origin[2]) < EPS;
  return { id: surface.id, widthMm: surface.widthMm, heightMm: surface.heightMm, origin, right, down, normal, flat };
};

/** Every surface of the scene in the stage frame */
export const sceneFrames = (scene: SceneShape | undefined): SurfaceFrame[] => sceneSurfaces(scene).map(surfaceFrame);

/** The frame of the stand-in surface (no scene open) */
export const DEFAULT_FRAME: SurfaceFrame = surfaceFrame(DEFAULT_SURFACE);

/** A point of a surface (mm from its top left corner, y down; `lift` mm off its face) in the stage frame (mm) */
export const toStage = (frame: SurfaceFrame, x: number, y: number, lift = 0): V3 =>
  [0, 1, 2].map(k => frame.origin[k] + frame.right[k] * x + frame.down[k] * y + frame.normal[k] * lift) as unknown as V3;

const dot = (a: V3, b: V3): number => a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
const minus = (a: V3, b: V3): V3 => [a[0] - b[0], a[1] - b[1], a[2] - b[2]];

/** A point of the stage frame (mm) read on a surface: mm from its top left corner, y down (projected onto its plane) */
export const toSurface = (frame: SurfaceFrame, point: V3): { x: number; y: number } => {
  const d = minus(point, frame.origin);
  return { x: dot(d, frame.right), y: dot(d, frame.down) };
};

/** A point read on one surface, read on another (the same point of the stage, projected onto the other's plane) */
export const onOtherSurface = (from: SurfaceFrame, to: SurfaceFrame, x: number, y: number): { x: number; y: number } =>
  from === to || from.id === to.id ? { x, y } : toSurface(to, toStage(from, x, y));

/** The surface with this id, else none */
export const findSurface = <F extends { id: string }>(frames: readonly F[], surfaceId: string | undefined): F | undefined =>
  surfaceId !== undefined ? frames.find(f => f.id === surfaceId) : undefined;

/** A ray (stage mm) meets a surface's plane: where (mm on the surface), how far along the ray, and how far outside its edges */
const meet = (frame: SurfaceFrame, origin: V3, direction: V3): { x: number; y: number; t: number; outside: number } | null => {
  const facing = dot(direction, frame.normal);
  if (Math.abs(facing) < 1e-9) return null;
  const t = dot(minus(frame.origin, origin), frame.normal) / facing;
  if (!(t >= 0)) return null;
  const point: V3 = [origin[0] + direction[0] * t, origin[1] + direction[1] * t, origin[2] + direction[2] * t];
  const { x, y } = toSurface(frame, point);
  const dx = x < 0 ? -x : x > frame.widthMm ? x - frame.widthMm : 0;
  const dy = y < 0 ? -y : y > frame.heightMm ? y - frame.heightMm : 0;
  return { x, y, t, outside: Math.hypot(dx, dy) };
};

/**
 * Where a ray (stage mm) meets the scene's surfaces: the nearest surface it hits inside its edges; outside every
 * surface, the one whose edge is nearest where the ray meets its plane (a drag beyond a page's edge stays on that page,
 * as on the open book, where the spine decides) – mm from that surface's top left corner. Null: the ray meets none.
 */
export const hitSurfaces = (frames: readonly SurfaceFrame[], origin: V3, direction: V3): { surface: string; x: number; y: number } | null => {
  let best: { frame: SurfaceFrame; x: number; y: number; t: number; outside: number } | null = null;
  for (const frame of frames) {
    const hit = meet(frame, origin, direction);
    if (!hit) continue;
    const better = !best
      || (hit.outside === 0 && (best.outside > 0 || hit.t < best.t - 1e-9))
      || (hit.outside > 0 && best.outside > 0 && hit.outside < best.outside - 1e-9);
    if (better) best = { frame, ...hit };
  }
  return best ? { surface: best.frame.id, x: tidy(best.x), y: tidy(best.y) } : null;
};

/** The flat surfaces' rectangles on the table plane (stage mm, y up) – what the surface grid and the 2D snapping count on */
export const surfaceRects = (frames: readonly SurfaceFrame[]): SurfaceRect[] =>
  frames.filter(f => f.flat).map(f => ({ id: f.id, left: f.origin[0], top: f.origin[1], width: f.widthMm, height: f.heightMm }));

/** The rectangle of one surface in its own plane (its top left corner at 0, y up): a wall's or a turned surface's 2D frame */
export const ownRect = (frame: Pick<SurfaceFrame, "id" | "widthMm" | "heightMm">): SurfaceRect =>
  ({ id: frame.id, left: 0, top: 0, width: frame.widthMm, height: frame.heightMm });

// ── Reference images (#314): a picture per surface, fitted ──

export const REFERENCE_FITS = ["cover", "contain", "stretch"] as const;
export type ReferenceFit = (typeof REFERENCE_FITS)[number];

/** A surface's reference image as the stage takes it (`surfaces[].reference` of the scenes overview, editor-only) */
export interface SurfaceReference {
  url: string;
  fit: ReferenceFit;
}

/** The reference images the page hands the stage: by scene id, then by surface id */
export type SurfaceReferences = Record<string, Record<string, SurfaceReference>>;

/**
 * How an image (`aspect` = its height per width) lies on a surface `width` × `height` mm: `stretch` fills it as it is;
 * `cover` fills it and crops the overhang (the texture's repeat / offset show the middle part); `contain` shows it whole,
 * as large as fits, centred (paper around it). The plane's size in mm and the texture's window.
 */
export const fitImage = (fit: ReferenceFit, aspect: number, width: number, height: number): { width: number; height: number; repeat: [number, number]; offset: [number, number] } => {
  const full = { width, height, repeat: [1, 1] as [number, number], offset: [0, 0] as [number, number] };
  if (fit === "stretch" || !(aspect > 0) || !(width > 0) || !(height > 0)) return full;
  const surface = height / width;
  if (fit === "contain") {
    return aspect > surface
      ? { ...full, width: tidy(height / aspect) }
      : { ...full, height: tidy(width * aspect) };
  }
  // cover: the image's part that has the surface's proportions
  if (aspect > surface) {
    const v = surface / aspect;
    return { ...full, repeat: [1, tidy(v)], offset: [0, tidy((1 - v) / 2)] };
  }
  const u = aspect / surface;
  return { ...full, repeat: [tidy(u), 1], offset: [tidy((1 - u) / 2), 0] };
};
