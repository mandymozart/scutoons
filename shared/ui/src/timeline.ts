// The timeline's model (agents/DOCKING.md "Bottom region"; owner 2026-10-19: one timeline for the storyboard studio,
// the creator and the live mode – research/LIVE.md): ONE linear axis in milliseconds (the live bus `t` / a cue's `wait`,
// the relay's `at`, Verovio tstamps, VN durations are ms; a source keeps its own unit and its adapter converts). Musical
// time later = other tick / format functions, never a second axis. Pure: no runes, no DOM (node tests, @scutoons/live).
import { clamp } from './scrub.ts';

export type Ms = number;
/** the kept part of a clip / take, relative to its item's start; `min` = the shortest it may get */
export interface TimelineTrim { in: Ms; out: Ms; min: Ms }
/** no `length` (or 0) = an instant (a cue); `state` = its `data-state` for the app's styling; `seq` = the order at an equal start */
export interface TimelineItem { id: string; start: Ms; length?: Ms; label?: string; state?: string; seq?: number; trim?: TimelineTrim }
/** `tall` = a lane with pictures (it takes the room that is left); `step` = its trim keys' step (default: one frame at 24 fps);
 *  `selectable: false` = its items are not picked: a press on one seeks like the lane's background (a take to scrub), the
 *  arrow keys only move the focus */
export interface TimelineLane { id: string; label: string; items: TimelineItem[]; tall?: boolean; step?: Ms; selectable?: boolean }
/** a scene, group, row or bar on the ruler */
export interface TimelineMarker { at: Ms; label: string }
/** a point as a share of an item (live's `{ row, slice }`, a score's fraction of a bar) */
export interface Place { id: string; at: number }
export interface Tick { at: Ms; label: string }
/** what the scroller shows: the strip is `zoom` views wide, the view starts at `scrollFrac` of it */
export interface View { zoom: number; scrollFrac: number }
/** m:ss, seconds ("1.25 s"), or a timecode m:ss:ff at whole frames per second */
export type TimeFormat = 'clock' | 's' | { fps: number };

export const FRAME: Ms = 1000 / 24;
export const ZOOM_MIN = 1;
export const ZOOM_MAX = 64;
const len = (it: TimelineItem) => Math.max(0, it.length ?? 0);
const clampZoom = (z: number) => clamp(z, ZOOM_MIN, ZOOM_MAX);
const clampScroll = (s: number, zoom: number) => clamp(s, 0, 1 - 1 / zoom);

/** items back to back from `start` (negative lengths count as 0), `seq` = their order */
export function sequence(list: readonly { id: string; length: Ms }[], start: Ms = 0): TimelineItem[] {
	let at = start;
	return list.map(({ id, length }, seq) => {
		const item = { id, start: at, length: Math.max(0, length || 0), seq };
		at += item.length;
		return item;
	});
}

/** a sorted copy: by start, then seq (what itemAt / placeOf expect) */
export const ordered = (items: readonly TimelineItem[]): TimelineItem[] => [...items].sort((a, b) => a.start - b.start || (a.seq ?? 0) - (b.seq ?? 0));

/** index of the last item that starts at or before `t` (binary search), -1 before the first */
function lastStart(items: readonly TimelineItem[], t: Ms): number {
	let [lo, hi] = [0, items.length];
	while (lo < hi) {
		const mid = (lo + hi) >> 1;
		[lo, hi] = items[mid].start <= t ? [mid + 1, hi] : [lo, mid];
	}
	return lo - 1;
}

/** The item under `t` in a lane's sorted, non-overlapping items: [start, end) – an instant only at its start; at an equal
 *  start the highest seq that covers `t` wins. Null in a gap, before the first and from the end of the last. */
export function itemAt(items: readonly TimelineItem[], t: Ms): TimelineItem | null {
	const last = lastStart(items, t);
	for (let i = last; i >= 0 && items[i].start === items[last].start; i--) {
		const it = items[i];
		if (t < it.start + len(it) || (len(it) === 0 && t === it.start)) return it;
	}
	return null;
}

/** `t` as a place, clamped to the items: before them = the first's 0, in a gap / past the end = the item before's 1 */
export function placeOf(items: readonly TimelineItem[], t: Ms): Place | null {
	if (!items.length) return null;
	if (t < items[0].start) return { id: items[0].id, at: 0 };
	const it = itemAt(items, t);
	if (it) return { id: it.id, at: len(it) ? (t - it.start) / len(it) : 0 };
	return { id: items[lastStart(items, t)].id, at: 1 };
}

/** a place back to ms (`at` clamped to 0..1), null for an unknown id */
export function msOf(items: readonly TimelineItem[], place: Place): Ms | null {
	const it = items.find((x) => x.id === place.id);
	return it ? it.start + clamp(place.at, 0, 1) * len(it) : null;
}

/** ms → a share of the span (clamped 0..1): positions on the strip are % of it */
export const frac = (t: Ms, span: Ms): number => (span > 0 ? clamp(t / span, 0, 1) : 0);
export const timeAt = (f: number, span: Ms): Ms => clamp(f, 0, 1) * Math.max(0, span);

// 1-2-5 in each unit – ms, s, min, h (50 s and 50 min would not read as clock steps)
const STEPS = [1, 2, 5, 10, 20, 50, 100, 200, 500, 1e3, 2e3, 5e3, 1e4, 2e4, 6e4, 12e4, 3e5, 6e5, 12e5, 36e5, 72e5, 18e6];

/** the smallest step that puts at most `max` ticks into `window` ms (the first tick included) */
export function tickStep(window: Ms, max: number): Ms {
	const need = window / Math.max(1, max - 1);
	return STEPS.find((s) => s >= need) ?? Math.ceil(need / 36e5) * 36e5;
}

/** the ruler: at most `max` ticks per view (span / zoom), only those inside `range` (the visible part) */
export function ticks(span: Ms, zoom: number, max: number, range: readonly [Ms, Ms] = [0, span]): Tick[] {
	if (!(span > 0)) return [];
	const step = tickStep(span / clampZoom(zoom), max);
	const digits = step >= 1000 ? 0 : step >= 100 ? 1 : step >= 10 ? 2 : 3;
	const out: Tick[] = [];
	for (let at = Math.max(0, Math.ceil(range[0] / step)) * step; at <= Math.min(span, range[1]); at += step) out.push({ at, label: formatTime(at, 'clock', digits) });
	return out;
}

/** zoom by `factor`, keeping the point under the pointer (`anchorFrac` of the view) where it is */
export function zoomAround(zoom: number, factor: number, anchorFrac: number, scrollFrac: number): View {
	const next = clampZoom(zoom * factor);
	return { zoom: next, scrollFrac: clampScroll(scrollFrac + anchorFrac / zoom - anchorFrac / next, next) };
}

export const inView = (v: View, f: number): boolean => f >= v.scrollFrac && f <= v.scrollFrac + 1 / v.zoom;

/** following the playhead: once `f` left the view, it comes back at a third of it */
export const followView = (v: View, f: number): View => (inView(v, f) ? v : { zoom: v.zoom, scrollFrac: clampScroll(f - 1 / (3 * v.zoom), v.zoom) });

/** a range's ends as shares of the span, in order */
const ends = (range: readonly [Ms, Ms], span: Ms): [number, number] => [frac(Math.min(...range), span), frac(Math.max(...range), span)];
/** the view at `zoom` with a..b in its middle */
const centred = (zoom: number, a: number, b: number): View => ({ zoom, scrollFrac: clampScroll((a + b) / 2 - 1 / (2 * zoom), zoom) });

/** show `range`: unchanged when it is in view (also one that nearly fills it) – unless `fill` asks to zoom in until it
 *  takes that share of the view; else zoomed out until it fits (90 %) and centred */
export function revealView(v: View, range: readonly [Ms, Ms], span: Ms, fill = 0): View {
	const [a, b] = ends(range, span);
	const w = b - a;
	const small = fill > 0 && w > 0 && w * v.zoom < fill;
	if (inView(v, a) && inView(v, b) && !small) return v;
	return centred(clampZoom(w * v.zoom > 0.9 ? 0.9 / w : small ? fill / w : v.zoom), a, b);
}

/** show `range` at the view's zoom (one the app set – its remembered zoom per step): unchanged while the range is in view
 *  or covers the whole view; else centred */
export function panView(v: View, range: readonly [Ms, Ms], span: Ms): View {
	const [a, b] = ends(range, span);
	const zoom = clampZoom(v.zoom);
	const shows = (inView(v, a) && inView(v, b)) || (a <= v.scrollFrac && b >= v.scrollFrac + 1 / zoom);
	return shows ? v : centred(zoom, a, b);
}

/** a box in the window's px (getBoundingClientRect, a pointer's clientX) – only ever turned into shares here */
export interface Rect { left: number; top: number; right: number; bottom: number }
/** where the hover preview goes, as shares of the window: `x` the pointer; `shift` how much of the box lies left of it
 *  (0 all right of it … 1 all left – it slides along the area, never out of it); `y` the hovered item's edge it sits on –
 *  `above` it when there is more room there, else below –; `room` the window's height on that side; `width` the area's */
export interface PreviewPlace { x: number; y: number; shift: number; above: boolean; room: number; width: number }
/** a hovering pointer on the Timeline's axis: the time under it; over an item: its lane, its id and where its preview goes */
export interface TimelineHover { t: Ms; lane?: string; item?: string; place?: PreviewPlace }

/** the hover preview next to pointer `x` over `item`: on the item's top / bottom edge (never over the item), sliding
 *  along `area` (the timeline's visible part), everything clamped to the window */
export function previewPlace(x: number, item: Rect, area: Rect, win: { width: number; height: number }): PreviewPlace {
	const [w, h] = [Math.max(1, win.width), Math.max(1, win.height)];
	const [l, r] = [clamp(area.left, 0, w), clamp(area.right, 0, w)];
	const top = clamp(Math.max(item.top, area.top), 0, h);
	const bottom = clamp(Math.min(item.bottom, area.bottom), top, h);
	const above = top >= h - bottom;
	const at = clamp(x, l, r);
	return { x: at / w, shift: r > l ? (at - l) / (r - l) : 0.5, y: (above ? top : bottom) / h, above, room: (above ? top : h - bottom) / h, width: (r - l) / w };
}

/** keeps a trim inside 0..length and at least `min` long; the dragged `edge` stops at the other one */
export function clampTrim(trim: TimelineTrim, length: Ms, edge?: 'in' | 'out'): TimelineTrim {
	const total = Math.max(0, length);
	const min = clamp(trim.min, 0, total);
	const out = clamp(trim.out, min, total);
	if (edge === 'in') return { in: clamp(trim.in, 0, out - min), out, min };
	const start = clamp(trim.in, 0, total - min);
	return { in: start, out: clamp(trim.out, start + min, total), min };
}

/** 'clock' = [h:]m:ss (+ `digits` decimals), 's' = "1.25 s" (`digits` default 2, trailing zeros dropped), { fps } = m:ss:ff */
export function formatTime(ms: Ms, format: TimeFormat = 'clock', digits?: number): string {
	const [sign, a] = [ms < 0 ? '-' : '', Math.abs(ms)];
	if (format === 's') {
		const s = (a / 1000).toFixed(digits ?? 2);
		return `${sign}${s.includes('.') ? s.replace(/\.?0+$/, '') : s} s`;
	}
	const fps = typeof format === 'object' ? Math.max(1, Math.round(format.fps)) : 0;
	const d = fps ? 0 : (digits ?? 0);
	const per = fps || 10 ** d;
	// whole units of the last digit shown; the epsilon keeps 1000 / 24 × 24 = 999.99… on the full second
	const n = Math.floor((a * per) / 1000 + 1e-6);
	const sec = Math.floor(n / per);
	const pad = (v: number, w = 2) => String(v).padStart(w, '0');
	const head = sec >= 3600 ? `${Math.floor(sec / 3600)}:${pad(Math.floor(sec / 60) % 60)}` : `${Math.floor(sec / 60)}`;
	const tail = fps ? `:${pad(n % per, String(fps - 1).length)}` : d ? `.${pad(n % per, d)}` : '';
	return `${sign}${head}:${pad(sec % 60)}${tail}`;
}
