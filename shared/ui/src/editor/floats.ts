/**
 * Floating panels over a full-window stage – the pure layout, after the deck editor's floats (reference: OSCT's
 * editor/src/lib/components/floats.ts). A panel is declared once (`PanelSpec`); its place is kept by the nearest corner, so
 * it comes back after a window resize. Nothing overlaps: a gap to every other panel and to the walls. The order of the
 * specs is the order of growth (`fill` takes what the others leave). A move snaps on release (`settle`); a resize makes
 * room (`evade`): the panels in its way move on and, at the wall, shrink to their least size.
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
/** How near (px) an edge must come to a wall or another panel's edge to snap to it */
export const SNAP_PX = 12;

const right = (r: Rect) => r.x + r.w;
const bottom = (r: Rect) => r.y + r.h;
const isRight = (c: Corner) => c === "top-right" || c === "bottom-right";
const isBottom = (c: Corner) => c === "bottom-left" || c === "bottom-right";

/** Two rectangles come closer than `gap` */
export const crowd = (a: Rect, b: Rect, gap: number): boolean =>
  a.x < right(b) + gap - EPS && b.x < right(a) + gap - EPS && a.y < bottom(b) + gap - EPS && b.y < bottom(a) + gap - EPS;

const inside = (r: Rect, area: Rect): boolean =>
  r.x >= area.x - EPS && r.y >= area.y - EPS && right(r) <= right(area) + EPS && bottom(r) <= bottom(area) + EPS;

/** `r` moved into `area` as far as it fits (its top left first) */
export const clampInto = (r: Rect, area: Rect): Rect => ({
  ...r,
  x: Math.max(area.x, Math.min(r.x, right(area) - r.w)),
  y: Math.max(area.y, Math.min(r.y, bottom(area) - r.h)),
});

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

const intersect = (a: Rect, b: Rect): Rect => {
  const x = Math.max(a.x, b.x);
  const y = Math.max(a.y, b.y);
  return { x, y, w: Math.max(0, Math.min(right(a), right(b)) - x), h: Math.max(0, Math.min(bottom(a), bottom(b)) - y) };
};

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

/** A folded panel's pill: grip and icon, its name, the chevron and – unless it is essential – the close */
export const pillSize = (spec: PanelSpec, m: Metrics): Size =>
  spec.inline ? { w: m.head, h: m.head } : { w: 2 * m.head + spec.name.length * m.char + m.button * (spec.essential ? 1 : 2), h: m.head };

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
  /** The panel in the hand: it lies where the pointer has it, over the others, which lay out without it */
  held?: { id: string; rect: Rect } | null;
}

/** Every shown panel's view: each inside the walls, none crowding another, grown in the order of the specs */
export const layout = ({ specs, places, shown, area, m, measured = {}, asks = {}, held = null }: LayoutInput): Record<string, View> => {
  const { gap } = m;
  const views: Record<string, View> = {};
  const panels = specs
    .filter(s => shown.has(s.id) && !places[s.id]?.closed && s.id !== held?.id)
    .map(spec => {
      const place = places[spec.id] ?? defaultPlace(spec, m);
      const collapsed = !!place.collapsed && !spec.inline;
      let { want, min } = collapsed ? { want: pillSize(spec, m), min: pillSize(spec, m) } : wantOf(spec, place, m, measured[spec.id], asks[spec.id]);
      want = { w: Math.min(want.w, area.w), h: Math.min(want.h, area.h) };
      min = { w: Math.min(min.w, want.w), h: Math.min(min.h, want.h) };
      const anchor: [number, number] = [isRight(place.corner) ? right(area) - place.dx : area.x + place.dx, isBottom(place.corner) ? bottom(area) - place.dy : area.y + place.dy];
      return { spec, corner: place.corner, collapsed, want, min, anchor };
    });
  // What a panel claims from the others: its want, but a `fill` dimension only its least (it takes what is left)
  const claim = (p: (typeof panels)[number]) => intersect(fromCorner(p.anchor, p.corner, {
    w: Number.isFinite(p.want.w) && p.want.w < area.w ? p.want.w : p.min.w,
    h: Number.isFinite(p.want.h) && p.want.h < area.h ? p.want.h : p.min.h,
  }), area);
  // The least rectangles first: each inside the walls; one crowding another moves where it is in no other one's way
  const mins: Rect[] = [];
  panels.forEach((p, k) => {
    let r = clampInto(fromCorner(p.anchor, p.corner, p.min), area);
    const taken = [...mins];
    if (taken.some(o => crowd(r, o, gap))) {
      const clear = [...taken, ...panels.filter((_, j) => j > k).map(claim)];
      r = nearestFree(r, clear, area, gap) ?? nearestFree(r, taken, area, gap) ?? r;
    }
    p.anchor = [isRight(p.corner) ? right(r) : r.x, isBottom(p.corner) ? bottom(r) : r.y];
    mins.push(r);
  });
  // Then each grows from its corner: the ones before it as they grew, the later ones as large as they claim
  const claims = panels.map(claim);
  const grown: Rect[] = [];
  panels.forEach((p, k) => {
    const obstacles = [...grown, ...claims.slice(k + 1)];
    const m0 = mins[k];
    const w = Math.max(p.min.w, Math.min(p.want.w, reach(p.anchor[0], isRight(p.corner), [m0.y, bottom(m0)], obstacles, area, gap, true)));
    const band = fromCorner(p.anchor, p.corner, { w, h: p.min.h });
    const h = Math.max(p.min.h, Math.min(p.want.h, reach(p.anchor[1], isBottom(p.corner), [band.x, right(band)], obstacles, area, gap, false)));
    const rect = fromCorner(p.anchor, p.corner, { w, h });
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

/** The nearest of `candidates` to `value` within `px`, else `value` */
const nearest = (value: number, candidates: readonly number[], px: number): number => {
  let best = value;
  let d = px + EPS;
  for (const c of candidates) {
    if (Math.abs(c - value) < d) {
      d = Math.abs(c - value);
      best = c;
    }
  }
  return best;
};

/**
 * A moved panel let go: its edges snap to a wall, to the gap beside another panel or in line with another panel's edge
 * when one comes within `px`; a rectangle that then lies on another panel goes to the nearest free place
 */
export const settle = (r: Rect, others: readonly Rect[], area: Rect, gap: number, px = SNAP_PX): Rect => {
  const lefts = [area.x, ...others.flatMap(o => [right(o) + gap, o.x])];
  const rights = [right(area), ...others.flatMap(o => [o.x - gap, right(o)])];
  const tops = [area.y, ...others.flatMap(o => [bottom(o) + gap, o.y])];
  const bottoms = [bottom(area), ...others.flatMap(o => [o.y - gap, bottom(o)])];
  const left = nearest(r.x, lefts, px);
  const toRight = nearest(right(r), rights, px) - r.w;
  const x = Math.abs(left - r.x) <= Math.abs(toRight - r.x) && left !== r.x ? left : toRight !== r.x ? toRight : left;
  const top = nearest(r.y, tops, px);
  const toBottom = nearest(bottom(r), bottoms, px) - r.h;
  const y = Math.abs(top - r.y) <= Math.abs(toBottom - r.y) && top !== r.y ? top : toBottom !== r.y ? toBottom : top;
  const snapped = clampInto({ ...r, x, y }, area);
  if (!others.some(o => crowd(snapped, o, gap))) return snapped;
  return nearestFree(snapped, others, area, gap) ?? snapped;
};

/**
 * A resized panel let go: the edges it moved stop the gap before another panel (the far edges stay), never below its
 * least size
 */
export const settleSize = (r: Rect, edge: Edge, others: readonly Rect[], min: Size, area: Rect, gap: number): Rect => {
  let { x, y, w, h } = r;
  if (edge.x === 1) w = Math.max(min.w, Math.min(w, reach(x, false, [y, y + h], others, area, gap, true)));
  if (edge.x === -1) {
    const far = x + w;
    w = Math.max(min.w, Math.min(w, reach(far, true, [y, y + h], others, area, gap, true)));
    x = far - w;
  }
  if (edge.y === 1) h = Math.max(min.h, Math.min(h, reach(y, false, [x, x + w], others, area, gap, false)));
  if (edge.y === -1) {
    const far = y + h;
    h = Math.max(min.h, Math.min(h, reach(far, true, [x, x + w], others, area, gap, false)));
    y = far - h;
  }
  return { x, y, w, h };
};

/** A panel a resize may push or shrink: where it lies and its least size */
export interface Neighbour {
  id: string;
  rect: Rect;
  min: Size;
}

const swapAxes = (r: Rect): Rect => ({ x: r.y, y: r.x, w: r.h, h: r.w });

/**
 * One axis of `evade`, turned so the edge moves right: the panels ahead of the edge where the resize started (`start`)
 * keep the gap to the held panel and to each other – a panel in the way moves right, one at the wall shrinks, down to
 * its least width (the ones nearest the wall first); the edge stops where they can give no more
 */
const pushRight = (start: Rect, target: Rect, others: readonly Neighbour[], area: Rect, gap: number): { held: Rect; rects: Rect[] } => {
  const beside = (a: Rect, b: Rect) => a.y < bottom(b) + gap - EPS && b.y < bottom(a) + gap - EPS;
  const ahead = others.map((o, i) => ({ ...o, i })).filter(o => o.rect.x >= right(start) - EPS).sort((a, b) => a.rect.x - b.rect.x);
  // From the wall back: how far right each may end, every panel ahead of it at its least size against the wall
  const maxRight: number[] = [];
  const maxLeft: number[] = [];
  for (let k = ahead.length - 1; k >= 0; k--) {
    const p = ahead[k].rect;
    let limit = right(area);
    for (let j = k + 1; j < ahead.length; j++) {
      if (ahead[j].rect.x >= right(p) - EPS && beside(p, ahead[j].rect)) limit = Math.min(limit, maxLeft[j] - gap);
    }
    maxRight[k] = Math.max(limit, right(p));
    maxLeft[k] = maxRight[k] - Math.min(ahead[k].min.w, p.w);
  }
  // The edge: where the pointer has it, but no further than the panels in its way can give
  let edge = right(target);
  if (edge > right(start)) {
    const most = ahead.reduce((l, o, k) => (beside(target, o.rect) ? Math.min(l, maxLeft[k] - gap) : l), right(area));
    edge = Math.max(right(start), Math.min(edge, most));
  }
  const held = { ...target, w: edge - target.x };
  // Nearest first: each moves to keep the gap to what pushes it, and ends no further right than it may
  const rects = others.map(o => o.rect);
  const ends: number[] = [];
  ahead.forEach((o, k) => {
    let left = o.rect.x;
    if (beside(held, o.rect)) left = Math.max(left, edge + gap);
    for (let j = 0; j < k; j++) {
      if (right(ahead[j].rect) <= o.rect.x + EPS && beside(ahead[j].rect, o.rect)) left = Math.max(left, ends[j] + gap);
    }
    const end = Math.min(left + o.rect.w, maxRight[k]);
    ends[k] = end;
    if (left !== o.rect.x || end !== right(o.rect)) rects[o.i] = { ...o.rect, x: left, w: end - left };
  });
  return { held, rects };
};

/**
 * A resize that makes room: while a panel's edge (`edge`) moves from `start` to `rect`, the panels it would come
 * near move out of its way – away from the moving edge, a chain of them pushed along – and at the wall they shrink, down to
 * their least size; the resized panel stops growing where they can give no more. Panels not in the way stay. The
 * horizontal edge first, then the vertical one (corner grips). Returns every panel's rectangle, the resized one's at `id`.
 * Pure and O(n²) in the panels: run once per animation frame, from the rectangles where the resize started.
 */
export const evade = (panels: readonly Neighbour[], id: string, rect: Rect, edge: Edge, area: Rect, gap: number): Record<string, Rect> => {
  const start = panels.find(p => p.id === id)?.rect ?? rect;
  let others = panels.filter(p => p.id !== id);
  let held = rect;
  // One axis in a frame where its edge moves right: turned (vertical) and mirrored (the left or top edge) there and back
  const axis = (horizontal: boolean, back: boolean, from: Rect, to: Rect) => {
    const a = horizontal ? area : swapAxes(area);
    const mirror = (r: Rect): Rect => (back ? { ...r, x: 2 * a.x + a.w - r.x - r.w } : r);
    const into = (r: Rect) => mirror(horizontal ? r : swapAxes(r));
    const out = (r: Rect) => { const m = mirror(r); return horizontal ? m : swapAxes(m); };
    const turned = others.map(o => ({ ...o, rect: into(o.rect), min: horizontal ? o.min : { w: o.min.h, h: o.min.w } }));
    const done = pushRight(into(from), into(to), turned, a, gap);
    others = others.map((o, i) => ({ ...o, rect: done.rects[i] === turned[i].rect ? o.rect : out(done.rects[i]) }));
    held = out(done.held);
  };
  if (edge.x) axis(true, edge.x === -1, start, rect);
  if (edge.y) axis(false, edge.y === -1, { ...held, y: start.y, h: start.h }, held);
  return Object.fromEntries([...others.map(o => [o.id, o.rect] as const), [id, held] as const]);
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
