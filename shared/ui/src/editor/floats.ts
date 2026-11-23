/**
 * Floating panels over a full-window stage – the pure layout, after the deck editor's floats (reference: OSCT's
 * editor/src/lib/components/floats.ts). A panel is declared once (`PanelSpec`); its place is kept by the nearest corner, so
 * it comes back after a window resize. Panels float over the stage and may overlap (the last touched lies on top): a
 * released panel stays where it was let go, only held inside the walls – no snap, and a resize never moves or shrinks
 * another panel. A fixed size is the size asked for; a `fill` dimension grows to the wall or the gap before a panel
 * clear of it, in the order of the specs.
 */

export type Corner = "top-left" | "top-right" | "bottom-left" | "bottom-right";
export const CORNERS: readonly Corner[] = ["top-left", "top-right", "bottom-left", "bottom-right"];

export interface Rect {
  x: number;
  y: number;
  w: number;
  h: number;
}

export interface Size {
  w: number;
  h: number;
}

/** Where a panel is (px): its distances from its corner's walls, folded, closed, the size it was resized to */
export interface Place {
  corner: Corner;
  dx: number;
  dy: number;
  collapsed?: boolean;
  closed?: boolean;
  /** The open size the person resized it to (none: the spec's) */
  size?: Size;
  /** An inline panel (a toolbar) as one column instead of one row */
  vertical?: boolean;
}

/** A dimension a spec asks for: rem, all the room there is, or what the content measured */
export type Dimension = number | "fill" | "content";

export interface PanelSpec {
  /** What its place is remembered by – never changes */
  id: string;
  name: string;
  /** An `Icon` name (kept a string here: this module is pure) */
  icon: string;
  /** What it is for: the Info view's help of its head */
  about: string;
  /** The default place: a corner and an offset in rem */
  place: { corner: Corner; dx: number; dy: number; collapsed?: boolean; closed?: boolean };
  /** The open size it asks for (rem, `fill` or `content`) */
  size: { w: Dimension; h: Dimension };
  /** The least open size (rem) a resize or a crowded window leaves it */
  min: Size;
  /** It only folds, never closes */
  essential?: boolean;
  /** A toolbar: grip and icon lead its one row, no head above; folded its icon alone */
  inline?: boolean;
  /** The tabs it holds by default (panel-tabs.ts); none for a toolbar */
  tabs?: readonly string[];
}

/** A spec, declared in one line like the deck's: `spec(id, name, icon, about, at("top-left", 0, 0), …)` */
export const at = (corner: Corner, dx: number, dy: number, more: { collapsed?: boolean; closed?: boolean } = {}): PanelSpec["place"] => ({ corner, dx, dy, ...more });

/** The sizes the model lays out with (px; from the page's CSS) */
export interface Metrics {
  /** 1rem */
  rem: number;
  /** Between two panels and to the walls */
  gap: number;
  /** A panel's head: the control height and its padding */
  head: number;
  /** A head's button (the chevron, the close) */
  button: number;
  /** About one character of a name (a pill's width) */
  char: number;
}

/** Where a shown panel landed */
export interface View {
  rect: Rect;
  collapsed: boolean;
}

/** Which edges a resize moves: -1 the left / top one, 1 the right / bottom one, 0 neither */
export interface Edge {
  x: -1 | 0 | 1;
  y: -1 | 0 | 1;
}

const EPS = 0.01;

const right = (r: Rect) => r.x + r.w;
const bottom = (r: Rect) => r.y + r.h;
const isRight = (c: Corner) => c === "top-right" || c === "bottom-right";
const isBottom = (c: Corner) => c === "bottom-left" || c === "bottom-right";

/** Two rectangles come closer than `gap` */
export const crowd = (a: Rect, b: Rect, gap: number): boolean =>
  a.x < right(b) + gap - EPS && b.x < right(a) + gap - EPS && a.y < bottom(b) + gap - EPS && b.y < bottom(a) + gap - EPS;

/** `r` moved into `area` as far as it fits (its top left first) */
export const clampInto = (r: Rect, area: Rect): Rect => ({
  ...r,
  x: Math.max(area.x, Math.min(r.x, right(area) - r.w)),
  y: Math.max(area.y, Math.min(r.y, bottom(area) - r.h)),
});

const inside = (r: Rect, area: Rect): boolean =>
  r.x >= area.x - EPS && r.y >= area.y - EPS && right(r) <= right(area) + EPS && bottom(r) <= bottom(area) + EPS;

/** The free place for `r` nearest to it: beside a wall or the gap away from another panel (null: none is free) */
export const nearestFree = (r: Rect, taken: readonly Rect[], area: Rect, gap: number): Rect | null => {
  const xs = [r.x, area.x, right(area) - r.w];
  const ys = [r.y, area.y, bottom(area) - r.h];
  for (const o of taken) {
    xs.push(right(o) + gap, o.x - gap - r.w, o.x);
    ys.push(bottom(o) + gap, o.y - gap - r.h, o.y);
  }
  let best: { d: number; r: Rect } | null = null;
  for (const x of xs) {
    for (const y of ys) {
      const c = { ...r, x, y };
      if (!inside(c, area) || taken.some(o => crowd(c, o, gap))) continue;
      const d = Math.abs(x - r.x) + Math.abs(y - r.y);
      if (!best || d < best.d) best = { d, r: c };
    }
  }
  return best?.r ?? null;
};

/**
 * How far a panel may reach along an axis from its anchored `edge` – away from it, up or left when `back` – inside the
 * band `lo..hi` across: to the wall, or the gap before an obstacle in the band
 */
export const reach = (edge: number, back: boolean, [lo, hi]: [number, number], obstacles: readonly Rect[], area: Rect, gap: number, horizontal: boolean): number => {
  const along = (o: Rect): [number, number] => (horizontal ? [o.x, right(o)] : [o.y, bottom(o)]);
  const across = (o: Rect): [number, number] => (horizontal ? [o.y, bottom(o)] : [o.x, right(o)]);
  const [wallLo, wallHi] = horizontal ? [area.x, right(area)] : [area.y, bottom(area)];
  const inBand = (o: Rect) => across(o)[0] < hi + gap - EPS && across(o)[1] + gap > lo + EPS;
  if (back) return obstacles.filter(o => inBand(o) && along(o)[0] < edge - EPS).reduce((l, o) => Math.min(l, edge - along(o)[1] - gap), edge - wallLo);
  return obstacles.filter(o => inBand(o) && along(o)[1] > edge + EPS).reduce((l, o) => Math.min(l, along(o)[0] - gap - edge), wallHi - edge);
};

/** A rectangle of `size` held at `anchor` by its corner */
export const fromCorner = (anchor: [number, number], corner: Corner, { w, h }: Size): Rect => ({
  x: isRight(corner) ? anchor[0] - w : anchor[0],
  y: isBottom(corner) ? anchor[1] - h : anchor[1],
  w,
  h,
});

/** A spec's default place in px */
export const defaultPlace = (spec: PanelSpec, m: Metrics): Place => ({
  corner: spec.place.corner,
  dx: spec.place.dx * m.rem,
  dy: spec.place.dy * m.rem,
  ...(spec.place.collapsed ? { collapsed: true } : {}),
  ...(spec.place.closed && !spec.essential ? { closed: true } : {}),
});

/** The place a rectangle keeps: by the corner nearest to its middle, the distances to that corner's walls */
export const placeOf = (r: Rect, area: Rect): Pick<Place, "corner" | "dx" | "dy"> => {
  const toRight = r.x + r.w / 2 > area.x + area.w / 2;
  const toBottom = r.y + r.h / 2 > area.y + area.h / 2;
  const corner: Corner = toBottom ? (toRight ? "bottom-right" : "bottom-left") : toRight ? "top-right" : "top-left";
  return {
    corner,
    // Not rounded: a rounded place could come a pixel closer than the gap to a neighbour and shrink it on the next layout
    dx: Math.max(0, toRight ? right(area) - right(r) : r.x - area.x),
    dy: Math.max(0, toBottom ? bottom(area) - bottom(r) : r.y - area.y),
  };
};

/** A folded panel's pill: its head's room, its name and – unless it is essential – the close */
export const pillSize = (spec: PanelSpec, m: Metrics): Size =>
  spec.inline ? { w: m.head, h: m.head } : { w: 2 * m.head + spec.name.length * m.char + (spec.essential ? 0 : m.button), h: m.head };

/**
 * A panel's chrome around its content: an inline toolbar has its grip and its button (the orientation) in its row – or,
 * vertical, in its column –, the others a head above
 */
export const chromeOf = (spec: PanelSpec, m: Metrics, vertical = false): Size => {
  if (!spec.inline) return { w: 2, h: m.head + 2 };
  const along = m.head + m.button * (spec.essential ? 1 : 2) + m.gap;
  const across = m.head - m.button + 2;
  return vertical ? { w: across, h: along } : { w: along, h: across };
};

/** The open size a panel asks for (px; Infinity = all the room there is) and its least */
export const wantOf = (spec: PanelSpec, place: Place, m: Metrics, content: Size | undefined, ask?: PanelSpec["size"]): { want: Size; min: Size } => {
  const vertical = !!(spec.inline && place.vertical);
  const chrome = chromeOf(spec, m, vertical);
  const dim = (d: Dimension, measured: number | undefined, extra: number) =>
    d === "fill" ? Infinity : d === "content" ? (measured ?? 0) + extra : d * m.rem;
  const asked = ask ?? spec.size;
  // A resized panel keeps its size; a height the content decides (a toolbar that wraps) follows the content
  const want = {
    w: place.size ? place.size.w : dim(asked.w, content?.w, chrome.w),
    h: place.size && asked.h !== "content" ? place.size.h : dim(asked.h, content?.h, chrome.h),
  };
  // A toolbar is as tall as its row (vertical: as wide as its column); shorter along it, its groups fold into their menus
  const least = spec.inline
    ? vertical ? { w: want.w, h: spec.min.w * m.rem } : { w: spec.min.w * m.rem, h: want.h }
    : { w: spec.min.w * m.rem, h: spec.min.h * m.rem };
  return { want, min: { w: Math.min(least.w, want.w), h: Math.min(least.h, want.h) } };
};

export interface LayoutInput {
  specs: readonly PanelSpec[];
  /** Each panel's place (closed ones are left out of the layout) */
  places: Readonly<Record<string, Place>>;
  /** The ids whose content is there to show (a panel the page does not draw now takes no room) */
  shown: ReadonlySet<string>;
  /** The room for panels: the window under the top bar, less the gap */
  area: Rect;
  m: Metrics;
  /** What `content` dimensions measured (px, the content alone) */
  measured?: Readonly<Record<string, Size>>;
  /** Sizes asked for now instead of the spec's (the Inspector's Code tab is wider) */
  asks?: Readonly<Record<string, PanelSpec["size"]>>;
  /** The panels the person placed (default: every one in `places`); the others are still at their default places */
  placed?: ReadonlySet<string>;
  /** The panel in the hand: it lies where the pointer has it, over the others, which stay as they are */
  held?: { id: string; rect: Rect } | null;
}

/**
 * Every shown panel's view, each inside the walls. A panel keeps the size it asks for where its place puts it – one the
 * person placed may lie over another; one still at its default place does not start on another default one (it goes to
 * the nearest free place). Only a `fill` dimension grows from its corner to the wall or the gap before a panel clear of
 * it (the ones before it as they grew, the later ones as large as they claim)
 */
export const layout = ({ specs, places, shown, area, m, measured = {}, asks = {}, placed = new Set(Object.keys(places)), held = null }: LayoutInput): Record<string, View> => {
  const { gap } = m;
  const views: Record<string, View> = {};
  const panels = specs
    .filter(s => shown.has(s.id) && !places[s.id]?.closed)
    .map(spec => {
      const place = places[spec.id] ?? defaultPlace(spec, m);
      const collapsed = !!place.collapsed && !spec.inline;
      const asked = collapsed ? { want: pillSize(spec, m), min: pillSize(spec, m) } : wantOf(spec, place, m, measured[spec.id], asks[spec.id]);
      const fill = { w: !Number.isFinite(asked.want.w), h: !Number.isFinite(asked.want.h) };
      const want = { w: Math.min(asked.want.w, area.w), h: Math.min(asked.want.h, area.h) };
      const min = { w: Math.min(asked.min.w, want.w), h: Math.min(asked.min.h, want.h) };
      const at: [number, number] = [isRight(place.corner) ? right(area) - place.dx : area.x + place.dx, isBottom(place.corner) ? bottom(area) - place.dy : area.y + place.dy];
      // the least rectangle inside the walls: what the panel claims and where it is held from
      const least = clampInto(fromCorner(at, place.corner, { w: fill.w ? min.w : want.w, h: fill.h ? min.h : want.h }), area);
      return { spec, corner: place.corner, collapsed, fill, want, min, least, placed: placed.has(spec.id), anchor: [0, 0] as [number, number] };
    });
  const defaults = (list: readonly (typeof panels)[number][]) => list.filter(q => !q.placed).map(q => q.least);
  panels.forEach((p, k) => {
    const before = defaults(panels.slice(0, k));
    if (!p.placed && before.some(o => crowd(p.least, o, gap))) {
      p.least = nearestFree(p.least, [...before, ...defaults(panels.slice(k + 1))], area, gap) ?? nearestFree(p.least, before, area, gap) ?? p.least;
    }
    p.anchor = [isRight(p.corner) ? right(p.least) : p.least.x, isBottom(p.corner) ? bottom(p.least) : p.least.y];
  });
  const claims = panels.map(p => p.least);
  const grown: Rect[] = [];
  panels.forEach((p, k) => {
    // only panels clear of it bound a `fill`: one lying over it is an overlap, not a wall
    const obstacles = [...grown, ...claims.slice(k + 1)].filter(o => !crowd(p.least, o, gap));
    const w = p.fill.w ? Math.max(p.min.w, Math.min(p.want.w, reach(p.anchor[0], isRight(p.corner), [p.least.y, bottom(p.least)], obstacles, area, gap, true))) : p.want.w;
    const band = fromCorner(p.anchor, p.corner, { w, h: p.least.h });
    const h = p.fill.h ? Math.max(p.min.h, Math.min(p.want.h, reach(p.anchor[1], isBottom(p.corner), [band.x, right(band)], obstacles, area, gap, false))) : p.want.h;
    const rect = clampInto(fromCorner(p.anchor, p.corner, { w, h }), area);
    grown.push(rect);
    views[p.spec.id] = { rect, collapsed: p.collapsed };
  });
  if (held && shown.has(held.id)) views[held.id] = { rect: held.rect, collapsed: !!places[held.id]?.collapsed && !specs.find(s => s.id === held.id)?.inline };
  return views;
};

/** A dragged panel's rectangle: moved by the pointer, kept inside the walls */
export const moved = (start: Rect, dx: number, dy: number, area: Rect): Rect => clampInto({ ...start, x: start.x + dx, y: start.y + dy }, area);

/** A resized panel's rectangle: the edges the grip holds follow the pointer, never below the least size or past a wall */
export const resized = (start: Rect, edge: Edge, dx: number, dy: number, min: Size, area: Rect): Rect => {
  let { x, y, w, h } = start;
  if (edge.x === 1) w = Math.max(min.w, Math.min(start.w + dx, right(area) - x));
  if (edge.x === -1) {
    const left = Math.min(Math.max(area.x, start.x + dx), right(start) - min.w);
    w = right(start) - left;
    x = left;
  }
  if (edge.y === 1) h = Math.max(min.h, Math.min(start.h + dy, bottom(area) - y));
  if (edge.y === -1) {
    const top = Math.min(Math.max(area.y, start.y + dy), bottom(start) - min.h);
    h = bottom(start) - top;
    y = top;
  }
  return { x, y, w, h };
};

/** The front-to-back order with `id` brought to the front (the panel last touched lies over the others) */
export const raise = (order: readonly string[], id: string): string[] => [...order.filter(o => o !== id), id];

/** Whether a stored value is a place (a layout stored by an older editor, or edited by hand, is checked) */
export const isPlace = (value: unknown): value is Place => {
  if (typeof value !== "object" || value === null) return false;
  const p = value as Record<string, unknown>;
  const finite = (v: unknown) => typeof v === "number" && Number.isFinite(v) && v >= 0;
  const size = p.size as Record<string, unknown> | undefined;
  return (CORNERS as readonly unknown[]).includes(p.corner) && finite(p.dx) && finite(p.dy)
    && (p.collapsed === undefined || typeof p.collapsed === "boolean") && (p.closed === undefined || typeof p.closed === "boolean")
    && (p.vertical === undefined || typeof p.vertical === "boolean")
    && (size === undefined || (typeof size === "object" && size !== null && finite(size.w) && finite(size.h)));
};

/** The places of one workspace from what storage holds: the valid ones of known panels; an essential panel never closed */
export const readPlaces = (stored: unknown, workspace: string, specs: readonly PanelSpec[]): Record<string, Place> => {
  const all = typeof stored === "object" && stored !== null ? (stored as Record<string, unknown>)[workspace] : undefined;
  if (typeof all !== "object" || all === null) return {};
  const places: Record<string, Place> = {};
  for (const spec of specs) {
    const place = (all as Record<string, unknown>)[spec.id];
    if (isPlace(place)) places[spec.id] = spec.essential && place.closed ? { ...place, closed: false } : place;
  }
  return places;
};

/** Storage's value with one workspace's places replaced (null: that workspace's removed – the default layout) */
export const withPlaces = (stored: unknown, workspace: string, places: Readonly<Record<string, Place>> | null): Record<string, unknown> => {
  const all = typeof stored === "object" && stored !== null && !Array.isArray(stored) ? { ...(stored as Record<string, unknown>) } : {};
  if (places === null || !Object.keys(places).length) delete all[workspace];
  else all[workspace] = places;
  return all;
};
