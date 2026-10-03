// The slice canvas' pure geometry (COMPONENTS.md step 10): rects in % of the slice, snapping to the grid, the pointer
// deltas of move / resize (also of rotated / scaled boxes), the rotate handle's angle and the scale handle's factor.
// No imports: unit-tested with node's own runner (creator/test/canvas-geometry.test.ts).

export interface Rect {
	x: number;
	y: number;
	width: number;
	height?: number;
}
export type Handle = 'move' | 'n' | 's' | 'e' | 'w' | 'ne' | 'nw' | 'se' | 'sw' | 'rotate' | 'scale';
/** the slice in px (rotation and scale work in px: the slice isn't square) */
export interface Px {
	width: number;
	height: number;
}
export interface Deltas {
	ddx: number;
	ddy: number;
	dw: number;
	dh: number;
}

const RAD = Math.PI / 180;

/** the grid step in % of the slice: across (x, widths) and down (y, heights); 0 = no snapping */
export function gridSteps(grid: number, sliceWidth: number, sliceRatio: number) {
	const stepX = grid > 0 ? (grid / sliceWidth) * 100 : 0;
	const stepY = grid > 0 ? (grid / (sliceWidth * sliceRatio)) * 100 : 0;
	return {
		stepX,
		stepY,
		snapX: (value: number) => (stepX > 0 ? Math.round(value / stepX) * stepX : value),
		snapY: (value: number) => (stepY > 0 ? Math.round(value / stepY) * stepY : value)
	};
}
export type Snap = Pick<ReturnType<typeof gridSteps>, 'snapX' | 'snapY'>;

/** move / resize of an upright box: the offsets to apply for a pointer delta (snapped edges; at least 1 % wide / high) */
export function deltas(handle: Handle, dx: number, dy: number, s: Required<Rect>, snap: Snap): Deltas {
	let ddx = 0,
		ddy = 0,
		dw = 0,
		dh = 0;
	if (handle === 'move') {
		ddx = snap.snapX(s.x + dx) - s.x;
		ddy = snap.snapY(s.y + dy) - s.y;
	} else {
		if (handle.includes('e')) dw = Math.max(1 - s.width, snap.snapX(s.x + s.width + dx) - (s.x + s.width));
		if (handle.includes('w')) {
			ddx = Math.min(s.width - 1, snap.snapX(s.x + dx) - s.x);
			dw = -ddx;
		}
		if (handle.includes('s')) dh = Math.max(1 - s.height, snap.snapY(s.y + s.height + dy) - (s.y + s.height));
		if (handle.includes('n')) {
			ddy = Math.min(s.height - 1, snap.snapY(s.y + dy) - s.y);
			dh = -ddy;
		}
	}
	return { ddx, ddy, dw, dh };
}

/**
 * Resize of a rotated / scaled box: the pointer delta in the box's own frame (un-rotated, un-scaled), the dragged
 * edges follow it and the opposite edges stay where they are on screen (the box turns around its centre). With snap
 * on, the new width / height are whole grid steps.
 */
export function transformedDeltas(handle: Handle, dx: number, dy: number, s: Required<Rect>, box: { rotation: number; scale: number }, px: Px, snap: Snap | null): Deltas {
	const W = px.width / 100;
	const H = px.height / 100;
	const a = box.rotation * RAD;
	const k = box.scale || 1;
	// pointer delta (px) → local frame
	const sx = dx * W;
	const sy = dy * H;
	const lx = (Math.cos(a) * sx + Math.sin(a) * sy) / k;
	const ly = (-Math.sin(a) * sx + Math.cos(a) * sy) / k;
	const w = s.width * W;
	const h = s.height * H;
	let l = -w / 2,
		r = w / 2,
		t = -h / 2,
		b = h / 2;
	const min = W; // 1 % of the slice width
	if (handle.includes('e')) r = Math.max(l + min, r + lx);
	if (handle.includes('w')) l = Math.min(r - min, l + lx);
	if (handle.includes('s')) b = Math.max(t + H, b + ly);
	if (handle.includes('n')) t = Math.min(b - H, t + ly);
	if (snap) {
		// whole grid steps for the new size: the dragged edge moves to fit (the opposite edge stays)
		const sw = Math.max(W, snap.snapX((r - l) / W) * W);
		const sh = Math.max(H, snap.snapY((b - t) / H) * H);
		if (handle.includes('e')) r = l + sw;
		else if (handle.includes('w')) l = r - sw;
		if (handle.includes('s')) b = t + sh;
		else if (handle.includes('n')) t = b - sh;
	}
	const nw = r - l;
	const nh = b - t;
	// the new centre: the old one + the local offset, scaled and rotated back to the slice
	const ox = ((l + r) / 2) * k;
	const oy = ((t + b) / 2) * k;
	const cx = s.x * W + w / 2 + Math.cos(a) * ox - Math.sin(a) * oy;
	const cy = s.y * H + h / 2 + Math.sin(a) * ox + Math.cos(a) * oy;
	const nx = (cx - nw / 2) / W;
	const ny = (cy - nh / 2) / H;
	const horizontal = /[ew]/.test(handle);
	const vertical = /[ns]/.test(handle);
	return { ddx: nx - s.x, ddy: ny - s.y, dw: horizontal ? nw / W - s.width : 0, dh: vertical ? nh / H - s.height : 0 };
}

/** the rotate handle: the angle from the box's centre to the pointer (0° = straight up), whole degrees or 15° steps, −180 … 180 */
export function rotationAngle(pointer: { x: number; y: number }, shown: Required<Rect>, px: Px, steps: boolean): number {
	const W = px.width / 100;
	const H = px.height / 100;
	const cx = (shown.x + shown.width / 2) * W;
	const cy = (shown.y + shown.height / 2) * H;
	let deg = Math.atan2(pointer.y * H - cy, pointer.x * W - cx) / RAD + 90;
	deg = steps ? Math.round(deg / 15) * 15 : Math.round(deg);
	return ((((deg + 180) % 360) + 360) % 360) - 180;
}

/** the scale handle: distance centre → pointer against the start (null while the start is (almost) the centre) */
export function scaleFactor(start: { x: number; y: number }, pointer: { x: number; y: number }, shown: Required<Rect>, px: Px): number | null {
	const W = px.width / 100;
	const H = px.height / 100;
	const cx = (shown.x + shown.width / 2) * W;
	const cy = (shown.y + shown.height / 2) * H;
	const d0 = Math.hypot(start.x * W - cx, start.y * H - cy);
	if (d0 < 2) return null;
	return Math.hypot(pointer.x * W - cx, pointer.y * H - cy) / d0;
}

/** a text box's new `scale` (0.1 … 10; with snap in 0.05 steps), rounded to 2 places */
export function dialogScale(current: number, factor: number, snapped: boolean): number {
	const raw = Math.min(10, Math.max(0.1, current * factor));
	return Math.round((snapped ? Math.max(0.1, Math.round(raw / 0.05) * 0.05) : raw) * 100) / 100;
}

/** a placed item scaled around its centre: the new rect (height only when it had one), rounded to 2 places */
export function scaledRect(start: Rect, shownHeight: number, factor: number, snapX: (v: number) => number): Rect {
	const r2 = (v: number) => Math.round(v * 100) / 100;
	const width = Math.max(1, snapX(start.width * factor));
	const baseHeight = start.height ?? shownHeight;
	const height = baseHeight * (width / start.width);
	const out: Rect = { x: r2(start.x + start.width / 2 - width / 2), y: r2(start.y + baseHeight / 2 - height / 2), width: r2(width) };
	if (start.height !== undefined) out.height = r2(height);
	return out;
}
