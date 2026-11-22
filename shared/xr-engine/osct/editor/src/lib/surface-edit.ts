import type { SurfaceData, SurfacePlace, TargetPlace } from "@shared/types/game-config";
import { SCENE_PAGE_MM, SCENE_WALL_MM } from "@shared/api/editor";

/**
 * Editing a scene's surfaces (#314, #319 – `agents/SCENES.md` "Schema 3.0"): the pure rules behind the Surface mode, the
 * Tree's Surfaces level and the selected surface – a new surface from a preset (a double page, a single page, a wall,
 * a custom one), remove, reorder, change one surface's fields, *arrange in a row*, the targets lying on a surface, the
 * choices of surface a placement offers, and a surface's name. Surfaces are free areas next to each other – no side,
 * every one at its `place` in the scene's coordinate system. Every edit returns the scene's whole new list – the API
 * takes the whole list (`PUT …/scenes/{id}` with `surfaces`).
 */

export const SURFACE_PRESETS = ["double", "single", "wall", "custom"] as const;
export type SurfacePreset = (typeof SURFACE_PRESETS)[number];

/** One page as added – the printed book's format (mm, the API's `SCENE_PAGE_MM`): a double page is two of them side by side, one surface */
export const PAGE_MM = SCENE_PAGE_MM;
/** A wall as added (mm, the API's `SCENE_WALL_MM`): it stands behind the row, facing the reader */
export const WALL_MM = SCENE_WALL_MM;
/** A custom surface as added (mm) – the Inspector sets its size */
export const CUSTOM_MM = { width: 300, height: 300 } as const;
/** How far behind the row's far edge a new wall stands (mm) */
const WALL_GAP_MM = 100;

/** The place of a surface lying flat at the origin */
export const FLAT: Required<SurfacePlace> = { x: 0, y: 0, z: 0, rx: 0, ry: 0, rz: 0 };

/** What a surface is called: its label as printed, else its id */
export const surfaceName = (surface: Pick<SurfaceData, "id" | "label">): string => surface.label?.trim() || surface.id;

/** An id for a new surface: the base itself while free, else the base with the first free number (`wall-2`) */
export const newSurfaceId = (surfaces: readonly Pick<SurfaceData, "id">[], base: string): string => {
  const taken = new Set(surfaces.map(s => s.id));
  if (!taken.has(base)) return base;
  for (let n = 2; ; n++) if (!taken.has(`${base}-${n}`)) return `${base}-${n}`;
};

/** A surface lies flat on the table: unturned (its height may be above the table) */
export const isFlat = (place: SurfacePlace): boolean => !place.rx && !place.ry && !place.rz;

/** The centres (x, mm) of surfaces laid side by side in a row, touching, the row centred on the origin */
export const rowCentres = (widths: readonly number[]): number[] => {
  const total = widths.reduce((sum, w) => sum + w, 0);
  let left = -total / 2;
  return widths.map(w => {
    const centre = left + w / 2;
    left += w;
    return round(centre);
  });
};

/**
 * **Arrange in a row** (#319): every surface flat on the table, side by side in the list's order, touching, the row
 * centred on the origin (a double page alone: its spine at the origin) – written to the surfaces' places
 */
export const arrangeInRow = (surfaces: readonly SurfaceData[]): SurfaceData[] => {
  const xs = rowCentres(surfaces.map(s => s.widthMm));
  return surfaces.map((s, i) => ({ ...s, place: { ...FLAT, x: xs[i] } }));
};

/** Whether the surfaces lie in a row already (what *arrange in a row* would write) */
export const inRow = (surfaces: readonly SurfaceData[]): boolean => sameSurfaces(arrangeInRow(surfaces), surfaces);

/** Where a new flat surface goes: right of the flat surfaces' row, touching it, its far edge in line with theirs */
const besideRow = (surfaces: readonly SurfaceData[], width: number, height: number): Required<SurfacePlace> => {
  const flat = surfaces.filter(s => isFlat(s.place) && Math.abs(s.place.y) < 1e-6);
  if (!flat.length) return { ...FLAT };
  const right = Math.max(...flat.map(s => s.place.x + s.widthMm / 2));
  const far = Math.min(...flat.map(s => s.place.z - s.heightMm / 2));
  return { ...FLAT, x: round(right + width / 2), z: round(far + height / 2) };
};

/** Where a new wall stands: centred on x, its foot on the table, behind the flat surfaces' far edge, facing the reader */
export const wallPlace = (surfaces: readonly SurfaceData[], height: number): Required<SurfacePlace> => {
  const flat = surfaces.filter(s => isFlat(s.place));
  const far = flat.length ? Math.min(...flat.map(s => s.place.z - s.heightMm / 2)) : 0;
  return { x: 0, y: round(height / 2), z: round(far - WALL_GAP_MM), rx: 90, ry: 0, rz: 0 };
};

/**
 * A new surface from a preset: a **double** page (one surface twice the page's width), a **single** page, a **wall**
 * (`WALL_MM`, standing ahead – `rx` 90 – behind the row, its foot on the table), a **custom** one (`CUSTOM_MM`). A page or
 * a custom one lies flat, right beside the row.
 */
export const presetSurface = (preset: SurfacePreset, surfaces: readonly SurfaceData[], page: { width: number; height: number } = PAGE_MM): SurfaceData => {
  if (preset === "wall") return { id: newSurfaceId(surfaces, "wall"), widthMm: WALL_MM.width, heightMm: WALL_MM.height, place: wallPlace(surfaces, WALL_MM.height) };
  const size = preset === "double" ? { width: 2 * page.width, height: page.height } : preset === "single" ? page : CUSTOM_MM;
  const base = preset === "custom" ? "surface" : "page";
  const [width, height] = [round(size.width), round(size.height)];
  return { id: newSurfaceId(surfaces, base), widthMm: width, heightMm: height, place: besideRow(surfaces, width, height) };
};

/** Add a preset's surface: the new list and the new surface */
export const addSurface = (preset: SurfacePreset, surfaces: readonly SurfaceData[], page?: { width: number; height: number }): { surfaces: SurfaceData[]; added: SurfaceData } => {
  const added = presetSurface(preset, surfaces, page);
  return { surfaces: [...surfaces, added], added };
};

/** The list without the surface */
export const removeSurface = (surfaces: readonly SurfaceData[], id: string): SurfaceData[] => surfaces.filter(s => s.id !== id);

/** The surface one place earlier (−1) or later (+1); the list as it was at an end or for an unknown id */
export const moveSurface = (surfaces: readonly SurfaceData[], id: string, direction: -1 | 1): SurfaceData[] => {
  const from = surfaces.findIndex(s => s.id === id);
  const to = from + direction;
  if (from < 0 || to < 0 || to >= surfaces.length) return [...surfaces];
  const next = [...surfaces];
  [next[from], next[to]] = [next[to], next[from]];
  return next;
};

/** The surface moved to another index (a drop) */
export const moveSurfaceTo = (surfaces: readonly SurfaceData[], id: string, index: number): SurfaceData[] => {
  const from = surfaces.findIndex(s => s.id === id);
  if (from < 0) return [...surfaces];
  const next = surfaces.filter(s => s.id !== id);
  next.splice(Math.max(0, Math.min(next.length, index)), 0, surfaces[from]);
  return next;
};

/** What one surface's edit may change: its label (empty = none), size, place */
export interface SurfacePatch {
  label?: string;
  widthMm?: number;
  heightMm?: number;
  place?: Partial<SurfacePlace>;
}

/** The list with one surface changed – a label trimmed (empty: none), a size at least 1 mm, a place's numbers rounded */
export const updateSurface = (surfaces: readonly SurfaceData[], id: string, patch: SurfacePatch): SurfaceData[] =>
  surfaces.map(s => {
    if (s.id !== id) return s;
    const { label: _label, ...rest } = s;
    const label = patch.label !== undefined ? patch.label.trim() : (s.label ?? "");
    return {
      ...rest,
      ...(patch.widthMm !== undefined ? { widthMm: Math.max(1, round(patch.widthMm)) } : {}),
      ...(patch.heightMm !== undefined ? { heightMm: Math.max(1, round(patch.heightMm)) } : {}),
      ...(label ? { label } : {}),
      place: tidyPlace({ ...s.place, ...(patch.place ?? {}) }),
    };
  });

/** A place's six numbers rounded to a tenth (mm, degrees); a missing or broken one is 0 */
export const tidyPlace = (place: Partial<SurfacePlace>): Required<SurfacePlace> => ({
  x: num(place.x), y: num(place.y), z: num(place.z), rx: num(place.rx), ry: num(place.ry), rz: num(place.rz),
});
const num = (value: number | undefined): number => (typeof value === "number" && Number.isFinite(value) ? round(value) : 0);

const round = (value: number): number => {
  const rounded = Math.round(value * 10) / 10;
  return Object.is(rounded, -0) ? 0 : rounded;
};

/** Two lists the same (an edit that changed nothing is not saved) */
export const sameSurfaces = (a: readonly SurfaceData[] | undefined, b: readonly SurfaceData[] | undefined): boolean =>
  JSON.stringify(a ?? []) === JSON.stringify(b ?? []);

/**
 * The surface a target's place lies on: the one it names (`place.surface`), else – naming none, or one the scene does
 * not have – the scene's first (its only one, as a rule)
 */
export const surfaceOfPlace = <S extends Pick<SurfaceData, "id">>(place: Pick<TargetPlace, "surface"> | undefined | null, surfaces: readonly S[]): S | undefined =>
  (place?.surface ? surfaces.find(s => s.id === place.surface) : undefined) ?? surfaces[0];

/** A target's place lies on this surface (`surfaceOfPlace`) */
export const liesOn = (place: Pick<TargetPlace, "surface"> | undefined | null, surface: Pick<SurfaceData, "id">, surfaces: readonly Pick<SurfaceData, "id">[]): boolean =>
  !!place && surfaceOfPlace(place, surfaces)?.id === surface.id;

/** The targets (their ids) and entries (their titles) lying on a surface – what a removal names */
export const onSurface = (
  entries: readonly { id: string; title: string; surface?: string; target: { id: string; place?: Pick<TargetPlace, "surface"> } | null }[],
  surface: Pick<SurfaceData, "id">,
  surfaces: readonly Pick<SurfaceData, "id">[],
): { targets: string[]; entries: string[] } => ({
  targets: entries.filter(e => e.target && liesOn(e.target.place, surface, surfaces)).map(e => e.target!.id),
  entries: entries.filter(e => e.surface === surface.id).map(e => e.title),
});

/** What a removal's confirmation says lies on the surface – empty when nothing does */
export const removalWarning = (on: { targets: string[]; entries: string[] }): string => {
  const parts = [
    on.targets.length ? `${on.targets.length === 1 ? "target" : "targets"} ${on.targets.join(", ")}` : "",
    on.entries.length ? `${on.entries.length === 1 ? "entry" : "entries"} ${on.entries.join(", ")}` : "",
  ].filter(Boolean);
  return parts.length ? `${parts.join(" and ")} ${on.targets.length + on.entries.length === 1 ? "lies" : "lie"} on it – they fall back to the scene's first surface.` : "";
};

/** A surface a placement may name: its id and its name */
export interface SurfaceChoice {
  value: string;
  label: string;
}

/** The surfaces a placement offers: the scene's, by id */
export const surfaceChoices = (surfaces: readonly SurfaceData[] | undefined): SurfaceChoice[] =>
  (surfaces ?? []).map(s => ({ value: s.id, label: surfaceName(s) }));

/** The choice a placement has now: the surface it names, else the first */
export const choiceOf = (choices: readonly SurfaceChoice[], place: Pick<TargetPlace, "surface">): string | null =>
  (place.surface ? choices.find(c => c.value === place.surface)?.value : undefined) ?? choices[0]?.value ?? null;

/** What a choice writes into a place: the surface's id – only while the scene has several (`TargetPlace.surface`) */
export const placeOnChoice = <P extends Pick<TargetPlace, "surface">>(place: P, choice: SurfaceChoice, choices: readonly SurfaceChoice[]): P => {
  const { surface: _old, ...rest } = place;
  return (choices.length > 1 ? { ...rest, surface: choice.value } : rest) as P;
};

// ── a surface's format (#314): the presets' sizes, which one a size is ──

/** A preset's size (mm): a double page twice the page's width, a single page, a wall; custom has none of its own */
export const presetSize = (preset: Exclude<SurfacePreset, "custom">, page: { width: number; height: number } = PAGE_MM): { width: number; height: number } =>
  preset === "double" ? { width: round(2 * page.width), height: round(page.height) }
  : preset === "single" ? { width: round(page.width), height: round(page.height) }
  : { width: WALL_MM.width, height: WALL_MM.height };

/** The preset a size is (either way round – a page turned on its side is still a page), else custom */
export const formatOf = (size: { widthMm: number; heightMm: number }, page: { width: number; height: number } = PAGE_MM): SurfacePreset => {
  const same = (a: number, b: number) => Math.abs(a - b) < 0.05;
  for (const preset of ["double", "single", "wall"] as const) {
    const { width, height } = presetSize(preset, page);
    if ((same(size.widthMm, width) && same(size.heightMm, height)) || (same(size.widthMm, height) && same(size.heightMm, width))) return preset;
  }
  return "custom";
};

/** A print lies wholly on a surface of this size (a twentieth of a millimetre of slack); its height: saved, else `height`, else none */
export const fitsOn = (place: Pick<TargetPlace, "x" | "y" | "width" | "height">, size: { widthMm: number; heightMm: number }, height?: number): boolean => {
  const tall = place.height ?? height ?? 0;
  const slack = 0.05;
  return place.x >= -slack && place.y >= -slack && place.x + place.width <= size.widthMm + slack && place.y + tall <= size.heightMm + slack;
};

/**
 * The targets lying (partly) outside their surface – after a surface got smaller, or turned from a double page into a
 * page: what the Inspector's surface flags. A print's height is its saved one, else `heights[id]` (the stage's, from the
 * image), else only its top edge counts.
 */
export const targetsOutside = (
  entries: readonly { target: { id: string; place?: TargetPlace } | null }[],
  surface: Pick<SurfaceData, "id" | "widthMm" | "heightMm">,
  surfaces: readonly Pick<SurfaceData, "id">[],
  heights: Readonly<Record<string, number>> = {},
): string[] =>
  entries
    .filter(e => !!e.target?.place && liesOn(e.target.place, surface, surfaces) && !fitsOn(e.target.place, surface, heights[e.target.id]))
    .map(e => e.target!.id);

/** How a print came onto another surface: at its place, centred, or made smaller to fit and centred */
export type MovedHow = "kept" | "centred" | "scaled";

/**
 * **A target moved to another surface** (#314): its place keeps its numbers (mm from the new surface's top left corner)
 * where the print fits there wholly; else it is centred on the surface – made smaller first (its aspect kept) when it is
 * wider or taller than the surface. The surface's id is written only while the scene has several (`placeOnChoice`).
 * `height` is the print's height when its place has none (the stage's, from the image); without either it counts as square.
 */
export const moveToSurface = <P extends TargetPlace>(
  place: P,
  to: Pick<SurfaceData, "id" | "widthMm" | "heightMm">,
  several: boolean,
  height?: number,
): { place: P; how: MovedHow } => {
  const { surface: _old, ...rest } = place;
  const on = (next: Omit<P, "surface">): P => (several ? { ...next, surface: to.id } : next) as P;
  const tall = place.height ?? height ?? place.width;
  if (fitsOn({ ...place, height: tall }, to)) return { place: on(rest), how: "kept" };
  const scale = Math.min(1, to.widthMm / place.width, to.heightMm / tall);
  const width = round(place.width * scale);
  const sized = { ...rest, width, ...(place.height !== undefined ? { height: round(place.height * scale) } : {}) };
  return { place: on({ ...sized, x: round((to.widthMm - width) / 2), y: round((to.heightMm - tall * scale) / 2) }), how: scale < 1 ? "scaled" : "centred" };
};
