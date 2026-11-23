import { DEFAULT_FRAME, findSurface, onOtherSurface, toStage, type SurfaceFrame, type V3 } from "./surfaces";

/**
 * The placement math of a target's printed image on the stage (three-free). A place is stored as the content has it
 * (`TargetPlace`): the surface (its id) and the box in mm from that surface's top left corner. The stage's frame has its
 * origin at the scene's origin (`surfaces.ts`), x to the right, y away from the reader. The printed image is as wide as
 * the place says and as high as the image's own aspect makes it – the stage, the mock tracker's anchor (the image's
 * centre, as the engine reports it) and the placement handles all count from here, so the print, its outline, its entity
 * and the inspector cannot drift apart.
 */

/** A printed image's place: its surface and its box (mm from the surface's top left corner) */
export interface PlaceBox {
  surface: string;
  x: number;
  y: number;
  width: number;
  /** The box's height as stored (the book's layout), used until the image's own aspect is known */
  height?: number;
}

/** A print's height per width while its image has not loaded and the place stores none */
export const DEFAULT_ASPECT = 0.75;
/** The narrowest print the handles make (mm) */
export const MIN_PLACE_WIDTH = 10;

/** Where the surfaces lie: the open scene's surfaces in the stage frame (`surfaces.ts`); a place's `surface` is one's id */
export type Surfaces = readonly SurfaceFrame[];

/** The surfaces as frames: none (no scene open) stands for the stand-in double page */
export const framesOf = (surfaces: Surfaces): readonly SurfaceFrame[] => (surfaces.length ? surfaces : [DEFAULT_FRAME]);

/** A surface's frame by its id, else the scene's first surface – a print always lies somewhere */
export const frameFor = (surfaces: Surfaces, surface: string | undefined): SurfaceFrame => {
  const frames = framesOf(surfaces);
  return findSurface(frames, surface) ?? frames[0];
};

/** Where a surface's left edge lies in the stage's frame (mm) */
export const surfaceLeft = (surface: string, page: Surfaces): number => frameFor(page, surface).origin[0];

const tidy = (value: number): number => (Math.abs(value) < 1e-9 ? 0 : Math.round(value * 1e6) / 1e6);

/** A surface's axes in the stage frame: x along its width, y up it (towards its top edge), z out of its face */
export const surfaceAxes = (surfaces: Surfaces, surface: string): { x: V3; y: V3; z: V3 } => {
  const frame = frameFor(surfaces, surface);
  return { x: frame.right, y: [-frame.down[0] || 0, -frame.down[1] || 0, -frame.down[2] || 0], z: frame.normal };
};

/** The printed height (mm): the image's aspect once known, else the stored height, else the default aspect */
export const placeHeight = (box: Pick<PlaceBox, "width" | "height">, aspect?: number | null): number =>
  aspect && aspect > 0 ? box.width * aspect : box.height && box.height > 0 ? box.height : box.width * DEFAULT_ASPECT;

/**
 * The print's centre in the stage's frame (mm; on the table x right, y up the page, z up) – where its plane lies and
 * where the tracker anchors its entity. On a wall or a turned surface it lies in that surface's plane.
 */
export const placeCentre = (box: PlaceBox, surfaces: Surfaces, aspect?: number | null): { x: number; y: number; z: number } => {
  const height = placeHeight(box, aspect);
  const [x, y, z] = toStage(frameFor(surfaces, box.surface), box.x + box.width / 2, box.y + height / 2);
  return { x: tidy(x), y: tidy(y), z: tidy(z) };
};

/** A point on a surface: the surface it lies on (its id) and mm from that surface's top left corner */
export interface SurfaceHit {
  surface: string;
  x: number;
  y: number;
}

/** A point on a surface read on another surface (mm from its top left corner): the same point of the stage */
const onSurface = (hit: SurfaceHit, surface: string, surfaces: Surfaces): { x: number; y: number } =>
  onOtherSurface(frameFor(surfaces, hit.surface), frameFor(surfaces, surface), hit.x, hit.y);

/** A drag of a print's handle: the image (move) or its bottom right corner (width), with the grab point inside the image */
export interface PlacementDrag {
  kind: "move" | "width";
  grabX: number;
  grabY: number;
}

/** The grab point inside the image (mm), so the image does not jump to the pointer when a drag starts */
export const grabOffset = (place: PlaceBox, hit: SurfaceHit, surfaces: Surfaces): { grabX: number; grabY: number } => {
  const at = onSurface(hit, place.surface, surfaces);
  return { grabX: tidy(at.x - place.x), grabY: tidy(at.y - place.y) };
};

/** The same place `width` mm wide: a stored height scales with it, so the box keeps the image's proportions */
export const withWidth = <P extends { width: number; height?: number }>(place: P, width: number): P =>
  place.height && place.width > 0 ? { ...place, width, height: Math.round(place.height * (width / place.width) * 100) / 100 } : { ...place, width };

/**
 * Where a drag puts the print (whole mm): a move keeps the grab point under the pointer and takes the surface the
 * pointer is over (across the spine it changes the page); the corner sets the width from the image's left edge, on
 * the image's own surface, never under `MIN_PLACE_WIDTH`
 */
export const draggedPlace = <P extends PlaceBox>(place: P, drag: PlacementDrag, hit: SurfaceHit, surfaces: Surfaces): P => {
  if (drag.kind === "move") {
    // The grab offset is a distance on the surface: it holds on whichever surface the pointer is over
    return { ...place, surface: hit.surface, x: Math.round(hit.x - drag.grabX), y: Math.round(hit.y - drag.grabY) };
  }
  const width = Math.max(MIN_PLACE_WIDTH, Math.round(onSurface(hit, place.surface, surfaces).x - place.x));
  return withWidth(place, width);
};

/** A place shown instead of the content's while it is edited: the target's id and where it lies now */
export interface LivePlacement {
  id: string;
  place: PlaceBox;
}

/**
 * The targets as laid out from the content, the one being placed where the placement says (the ghost *is* the
 * target: its print and its entity move with the handles). A stored height follows a changed width.
 */
export const withLivePlacement = <T extends { id: string; surface: string; x: number; y: number; width: number; height: number }>(
  targets: readonly T[], live: LivePlacement | null,
): T[] =>
  targets.map(target => {
    if (!live || target.id !== live.id) return target;
    const { surface, x, y, width } = live.place;
    const height = live.place.height ?? (target.width > 0 ? target.height * (width / target.width) : width * DEFAULT_ASPECT);
    return { ...target, surface, x, y, width, height };
  });
