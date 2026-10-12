// The world map's camera, shared by the reader's map and the creator's map builder (agents/MAP.md M1/M2): one transform
// for the whole content (`translate(x, y) scale(zoom)`, world units → screen px), zoom at a point, pan, fit, and the
// pointer gestures (drag = pan, wheel / pinch = zoom at the cursor, double-click zooms in). Pure state + a DOM attachment;
// no app state.
import { MAP_MAX_ZOOM, MAP_MIN_ZOOM, type Point, type Rect } from '@scutoons/content';

export class MapViewport {
	/** screen px of the world's origin */
	x = $state(0);
	y = $state(0);
	zoom = $state(1);
	/** the element's size (set by the attachment / the app) */
	width = $state(0);
	height = $state(0);
	/** world size (for clamping the pan) */
	worldWidth = $state(0);
	worldHeight = $state(0);
	minZoom = $state(MAP_MIN_ZOOM);
	maxZoom = $state(MAP_MAX_ZOOM);
	/** the camera stays on the world (`WorldMap.bounded`): zoom out only until it fills the view, pan only to its edges */
	bounded = $state(false);
	/** a gesture is running (the apps raise `will-change` while it does) */
	moving = $state(false);
	#settle: ReturnType<typeof setTimeout> | undefined;
	/**
	 * the camera moves by something without a clear end (the wheel, a pinch, a flight's frames): `moving` until it has been
	 * still for a moment – so the terrain stays one composited layer while it zooms instead of being re-rastered per frame
	 */
	stir(ms = 160) {
		this.moving = true;
		clearTimeout(this.#settle);
		this.#settle = setTimeout(() => (this.moving = false), ms);
	}

	readonly transform = $derived(`translate(${this.x}px, ${this.y}px) scale(${this.zoom})`);

	/** world → screen */
	toScreen(p: Point): Point {
		return { x: p.x * this.zoom + this.x, y: p.y * this.zoom + this.y };
	}
	/** screen (relative to the element) → world */
	toWorld(p: Point): Point {
		return { x: (p.x - this.x) / this.zoom, y: (p.y - this.y) / this.zoom };
	}

	/** the zoom at which the world just covers the view (bounded: the furthest out) */
	get coverZoom(): number {
		return this.worldWidth && this.width ? Math.max(this.width / this.worldWidth, this.height / this.worldHeight) : 0;
	}

	clampZoom(z: number) {
		const min = this.bounded ? Math.max(this.minZoom, this.coverZoom) : this.minZoom;
		return Math.max(min, Math.min(this.maxZoom, z));
	}

	/** keeps some of the world on screen (half the view may leave it, not more); bounded: none of the view leaves it */
	clamp() {
		if (!this.worldWidth || !this.width) return;
		if (this.bounded && this.zoom < this.coverZoom) this.zoom = this.coverZoom;
		const w = this.worldWidth * this.zoom;
		const h = this.worldHeight * this.zoom;
		const mx = this.bounded ? 0 : this.width / 2;
		const my = this.bounded ? 0 : this.height / 2;
		this.x = Math.min(mx, Math.max(this.width - w - mx, this.x));
		this.y = Math.min(my, Math.max(this.height - h - my, this.y));
	}

	/** zoom by a factor keeping the screen point `at` (default the centre) where it is */
	zoomAt(factor: number, at: Point = { x: this.width / 2, y: this.height / 2 }) {
		const next = this.clampZoom(this.zoom * factor);
		const world = this.toWorld(at);
		this.zoom = next;
		this.x = at.x - world.x * next;
		this.y = at.y - world.y * next;
		this.clamp();
	}

	panBy(dx: number, dy: number) {
		this.x += dx;
		this.y += dy;
		this.clamp();
	}

	/** centre a world point at a zoom (default: the current) */
	centerOn(p: Point, zoom = this.zoom) {
		this.zoom = this.clampZoom(zoom);
		this.x = this.width / 2 - p.x * this.zoom;
		this.y = this.height / 2 - p.y * this.zoom;
		this.clamp();
	}

	/** fit world points (with a padding in screen px), not closer than `maxZoom` */
	fit(points: Point[], padding = 48, maxZoom = 2) {
		if (!points.length || !this.width) return;
		const xs = points.map((p) => p.x);
		const ys = points.map((p) => p.y);
		const w = Math.max(1, Math.max(...xs) - Math.min(...xs));
		const h = Math.max(1, Math.max(...ys) - Math.min(...ys));
		const zoom = Math.min(maxZoom, (this.width - padding * 2) / w, (this.height - padding * 2) / h);
		this.centerOn({ x: (Math.min(...xs) + Math.max(...xs)) / 2, y: (Math.min(...ys) + Math.max(...ys)) / 2 }, zoom);
	}

	/** is a world point on screen (with a margin in px)? */
	/** the world rectangle on screen, `margin` screen px around it (the culling's query – `pointIndex`) */
	worldRect(margin = 0): Rect {
		const a = this.toWorld({ x: -margin, y: -margin });
		const b = this.toWorld({ x: this.width + margin, y: this.height + margin });
		return { x: a.x, y: a.y, width: b.x - a.x, height: b.y - a.y };
	}
	onScreen(p: Point, margin = 0): boolean {
		const s = this.toScreen(p);
		return s.x >= -margin && s.y >= -margin && s.x <= this.width + margin && s.y <= this.height + margin;
	}
}

export interface MapGestureOptions {
	/** may this pointerdown start a pan? (the builder's tools decide; default: any primary / middle button) */
	canPan?: (event: PointerEvent) => boolean;
	/** a click that was not a drag (screen point relative to the element) */
	onTap?: (event: PointerEvent, at: Point) => void;
	/** zoom with the wheel (default true) */
	wheel?: boolean;
	/** a double click; return true when handled (no zoom) – the builder finishes a route with it */
	onDoubleClick?: (event: MouseEvent, at: Point) => boolean;
	/** the pointer moved over the map (no button needed; the builder's route preview) */
	onHover?: (event: PointerEvent, at: Point) => void;
}

/**
 * Attachment for the map's viewport element: keeps the viewport's size in sync and runs the gestures. Usage:
 * `<div {@attach mapGestures(view, { … })}>`.
 */
export function mapGestures(view: MapViewport, options: MapGestureOptions = {}) {
	return (node: HTMLElement) => {
		const pointers = new Map<number, Point>();
		let start: Point | null = null;
		let dragged = false;
		/** the first pointer may pan (else it is only a tap: the builder's place / route tools) */
		let panning = true;
		let pinch: { distance: number; zoom: number } | null = null;
		// pointer and wheel events can come several times per frame: collected here, applied once per animation frame
		let frame = 0;
		let pan = { dx: 0, dy: 0 };
		let zoomBy: { factor: number; at: Point } | null = null;
		let zoomTo: { zoom: number; at: Point } | null = null;
		// momentum (owner 2026-10-12: "if I drag and release it will take some time till it stops"): the last ~100 ms of
		// a pan give the speed; released, it glides on and slows down (FRICTION per ms); a new touch or the wheel stops it
		let samples: { x: number; y: number; t: number }[] = [];
		let glide = 0;
		const FRICTION = 0.996;
		const stopGlide = () => {
			if (glide) cancelAnimationFrame(glide);
			glide = 0;
		};
		function startGlide() {
			const last = samples[samples.length - 1];
			const first = samples.find((s) => last && last.t - s.t <= 100) ?? samples[0];
			samples = [];
			if (!first || !last || last.t - first.t < 16 || performance.now() - last.t > 60) return;
			let vx = (last.x - first.x) / (last.t - first.t);
			let vy = (last.y - first.y) / (last.t - first.t);
			if (Math.hypot(vx, vy) < 0.2) return; // px per ms: a slow release just stops
			let before = performance.now();
			view.moving = true;
			const step = (now: number) => {
				const dt = Math.min(32, Math.max(0, now - before));
				before = now;
				const fromX = view.x;
				const fromY = view.y;
				view.panBy(vx * dt, vy * dt);
				const decay = FRICTION ** dt;
				vx *= decay;
				vy *= decay;
				// at an edge or slow enough: done
				if ((dt > 0 && view.x === fromX && view.y === fromY) || Math.hypot(vx, vy) < 0.02) {
					glide = 0;
					view.moving = false;
					return;
				}
				glide = requestAnimationFrame(step);
			};
			glide = requestAnimationFrame(step);
		}
		const flush = () => {
			frame = 0;
			if (pan.dx || pan.dy) view.panBy(pan.dx, pan.dy);
			pan = { dx: 0, dy: 0 };
			if (zoomTo) view.zoomAt(zoomTo.zoom / view.zoom, zoomTo.at);
			else if (zoomBy) view.zoomAt(zoomBy.factor, zoomBy.at);
			zoomTo = zoomBy = null;
		};
		const schedule = () => {
			if (!frame) frame = requestAnimationFrame(flush);
		};
		const local = (e: { clientX: number; clientY: number }): Point => {
			const r = node.getBoundingClientRect();
			return { x: e.clientX - r.left, y: e.clientY - r.top };
		};

		const resize = new ResizeObserver(() => {
			const before = { w: view.width, h: view.height };
			view.width = node.clientWidth;
			view.height = node.clientHeight;
			// keep the centre in place when the element changes size
			if (before.w && before.h) {
				view.x += (view.width - before.w) / 2;
				view.y += (view.height - before.h) / 2;
			}
		});
		resize.observe(node);
		view.width = node.clientWidth;
		view.height = node.clientHeight;

		function down(e: PointerEvent) {
			if (e.button !== 0 && e.button !== 1) return;
			stopGlide();
			samples = [];
			if (pointers.size === 0) panning = !options.canPan || options.canPan(e);
			pointers.set(e.pointerId, local(e));
			if (pointers.size === 1) {
				start = local(e);
				dragged = false;
			} else if (pointers.size === 2) {
				const [a, b] = [...pointers.values()];
				pinch = { distance: Math.hypot(a.x - b.x, a.y - b.y), zoom: view.zoom };
			}
			if (e.button === 1) e.preventDefault();
		}
		function move(e: PointerEvent) {
			options.onHover?.(e, local(e));
			const previous = pointers.get(e.pointerId);
			if (!previous) return;
			const p = local(e);
			pointers.set(e.pointerId, p);
			if (pointers.size === 2 && pinch) {
				const [a, b] = [...pointers.values()];
				const distance = Math.hypot(a.x - b.x, a.y - b.y);
				const mid = { x: (a.x + b.x) / 2, y: (a.y + b.y) / 2 };
				zoomTo = { zoom: (pinch.zoom * distance) / pinch.distance, at: mid };
				zoomBy = null;
				view.stir();
				schedule();
				dragged = true;
				return;
			}
			if (!dragged && start && Math.hypot(p.x - start.x, p.y - start.y) < 4) return;
			if (!panning) {
				dragged = true;
				return;
			}
			if (!dragged) {
				dragged = true;
				view.moving = true;
				try {
					node.setPointerCapture(e.pointerId);
				} catch {
					/* the pointer is gone already */
				}
			}
			pan = { dx: pan.dx + p.x - previous.x, dy: pan.dy + p.y - previous.y };
			samples.push({ x: p.x, y: p.y, t: performance.now() });
			if (samples.length > 12) samples.shift();
			schedule();
		}
		function up(e: PointerEvent) {
			if (!pointers.has(e.pointerId)) return;
			pointers.delete(e.pointerId);
			if (pointers.size < 2) pinch = null;
			if (pointers.size === 1) samples = []; // a pinch ending: no glide from its last finger
			if (pointers.size === 0) {
				flush();
				view.moving = false;
				if (!dragged && options.onTap) options.onTap(e, local(e));
				else if (dragged && panning && e.type === 'pointerup') startGlide();
				start = null;
			}
		}
		function wheel(e: WheelEvent) {
			if (options.wheel === false) return;
			stopGlide();
			e.preventDefault();
			// trackpads send many small deltas (ctrlKey = pinch); a mouse wheel sends ±100
			const delta = e.deltaMode === 1 ? e.deltaY * 16 : e.deltaY;
			const factor = Math.exp(-delta * (e.ctrlKey ? 0.01 : 0.0015));
			zoomBy = { factor: (zoomBy?.factor ?? 1) * factor, at: local(e) };
			view.stir();
			schedule();
		}
		function dblclick(e: MouseEvent) {
			if (options.onDoubleClick?.(e, local(e))) return;
			if ((e.target as HTMLElement | null)?.closest('button, a, input, select, textarea')) return;
			view.zoomAt(e.shiftKey ? 0.5 : 2, local(e));
		}

		node.addEventListener('pointerdown', down);
		node.addEventListener('pointermove', move);
		node.addEventListener('pointerup', up);
		node.addEventListener('pointercancel', up);
		node.addEventListener('wheel', wheel, { passive: false });
		node.addEventListener('dblclick', dblclick);
		return () => {
			cancelAnimationFrame(frame);
			stopGlide();
			resize.disconnect();
			node.removeEventListener('pointerdown', down);
			node.removeEventListener('pointermove', move);
			node.removeEventListener('pointerup', up);
			node.removeEventListener('pointercancel', up);
			node.removeEventListener('wheel', wheel);
			node.removeEventListener('dblclick', dblclick);
		};
	};
}
