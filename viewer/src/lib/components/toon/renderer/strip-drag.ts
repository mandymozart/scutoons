import type { Attachment } from 'svelte/attachments';
import { releaseVelocity, VELOCITY_WINDOW, type Sample, type Swipe } from './strip-loop.ts';
import { rootRem } from './strip-dom.ts';

/** the band's side of the input – read at event time, so one object over the band's state works */
export interface StripInput {
	/** the band takes input (false while it is whole and still) */
	enabled(): boolean;
	/** stop a running glide / ease – true when something moved (the tap that stopped it is then no click) */
	stop(): boolean;
	/** move the picture by `dx` px (right = +) */
	move(dx: number): void;
	/**
	 * a drag let go at `velocity` px/ms (0 = none: a cancel, a rest before letting go) – the band glides; `swipe`: the
	 * whole drag (a pager's short-swipe rule – strip-loop.ts settleTarget), missing after a cancel or a pinch. Also a press
	 * that stopped a motion and then didn't drag sideways (a tap, a vertical swipe): velocity 0, no swipe
	 */
	release(velocity: number, swipe?: Swipe): void;
	/** ← (−1) / → (+1): ease to the previous / next frame edge, replacing a running glide or ease */
	step(direction: 1 | -1): void;
	/** keyboard focus landed on `target` in the band: slide its frame in */
	reveal?(target: Element): void;
	/** a drag runs (the grabbing cursor) */
	dragging?(on: boolean): void;
	/** something else owns the input now – nothing moves (the band: the story blocks the page, `html[data-vn-blocking]`) */
	blocked?(): boolean;
	/** a sideways wheel burst ended (no event for 150 ms) – a pager settles on a page */
	settle?(): void;
}

/**
 * the page asks the band to show a slice (chapter +page.svelte `arriveAt`: arrival, resume, the creator's Reader) –
 * dispatched on the slice, it bubbles to the band, which brings that frame to the window's left
 */
export const STRIP_SHOW = 'strip-show';

/** the pointer's dead zone before the axis locks, rem */
export const DEAD_ZONE_REM = 0.5;
/** wheel events closer than this (ms) are one burst – one axis */
export const WHEEL_BURST = 150;
/** vertical swipes and pinch-zoom stay the browser's (plain `pan-y` blocked zooming on the band – measured) */
export const TOUCH_ACTION = 'pan-y pinch-zoom';

/** a pointer gesture's axis once it has left the dead zone (px) – null while inside; a tie goes to the page */
export function gestureAxis(dx: number, dy: number, deadZone: number): 'x' | 'y' | null {
	if (Math.hypot(dx, dy) < deadZone) return null;
	return Math.abs(dx) > Math.abs(dy) ? 'x' : 'y';
}

/** the parts of a wheel event the latch reads */
export interface WheelInput {
	deltaX: number;
	deltaY: number;
	shiftKey: boolean;
	timeStamp: number;
}

/**
 * The wheel's axis per burst: the burst's first event decides (deltaX-led, or Shift turning a vertical wheel sideways),
 * kept while events come < 150 ms apart. Returns the band's sideways delta (in the event's deltaMode units; 0 inside
 * the band's burst still belongs to it) or null – the page's.
 */
export function wheelLatch(): (e: WheelInput) => number | null {
	let axis: 'x' | 'y' | null = null;
	let last = -Infinity;
	return (e) => {
		let dx = e.deltaX;
		let dy = e.deltaY;
		if (e.shiftKey && !dx) [dx, dy] = [dy, 0];
		if (e.timeStamp - last > WHEEL_BURST) axis = null;
		last = e.timeStamp;
		if (!axis && (dx || dy)) axis = Math.abs(dx) > Math.abs(dy) ? 'x' : 'y';
		return axis === 'x' ? dx : null;
	};
}

const typing = (target: EventTarget | null) => {
	const el = target as HTMLElement | null;
	return !!el && (el.isContentEditable || /^(INPUT|TEXTAREA|SELECT)$/.test(el.tagName ?? ''));
};
const keyboardFocus = (el: Element) => {
	try {
		return el.matches(':focus-visible');
	} catch {
		return false;
	}
};

/**
 * The endless band's input (strip-endless.svelte; BACKLOG 10.100, TEXT-ACROSS-SLICES.md §8), `{@attach stripDrag(input)}`:
 * - pointer: a 0.5rem dead zone, then the axis locks – sideways the band follows 1:1 (the pointer captured) and the
 *   release flings it; up / down the gesture is the page's. Touch: `touch-action: pan-y pinch-zoom` (set here) leaves
 *   vertical swipes and pinch-zoom to the browser (box text on the band is ≈ 12 px); a swipe whose first ≈ 10 px go
 *   sideways is the band's for that gesture (inherent, Swiper too)
 * - the click after a drag is swallowed, as is the one after a tap that stopped a glide; a keyboard click
 *   (`detail === 0`) passes
 * - wheel: only a deltaX-led burst (or Shift) moves it 1:1; vertical always scrolls the page (no Swiper trap)
 * - ← → step to the next frame edge while the band (or something in it) has focus
 * - keyboard focus in a frame slides it in; a pointer's focus doesn't (a mouse-down on a button started that slide
 *   under the drag – measured)
 * - a second finger during a drag ends it where it began (the browser zooms; its first ≈ 10 px moved the band – measured)
 * - a new drag / touch / wheel stops a running glide or ease
 * - nothing while the band is still (`enabled`) or `blocked` (the band: the story blocks the page)
 * - `settle` once a sideways wheel burst ends; `release` (no swipe) when a press stopped a motion but didn't drag
 */
export function stripDrag(input: StripInput): Attachment<HTMLElement> {
	return (band) => {
		const doc = band.ownerDocument;
		const blocked = () => input.blocked?.() === true;
		const off = () => !input.enabled() || blocked();
		const remPx = () => rootRem(doc);
		const latch = wheelLatch();
		/** the pointer gesture – axis null while it is in the dead zone */
		let gesture: { id: number; x0: number; y0: number; t0: number; x: number; zone: number; axis: 'x' | null; samples: Sample[] } | null = null;
		/** the next pointer click is no tap (it ends a drag, or its press stopped a glide) */
		let swallow = false;
		/** the wheel burst's settle timer */
		let wheelTimer: ReturnType<typeof setTimeout> | undefined;

		function down(e: PointerEvent) {
			if (gesture && gesture.id !== e.pointerId) return pinch();
			if (off() || (e.pointerType === 'mouse' && e.button !== 0)) return;
			swallow = input.stop() === true;
			gesture = { id: e.pointerId, x0: e.clientX, y0: e.clientY, t0: e.timeStamp, x: e.clientX, zone: DEAD_ZONE_REM * remPx(), axis: null, samples: [{ t: e.timeStamp, x: e.clientX }] };
		}

		function drag(e: PointerEvent) {
			if (!gesture || e.pointerId !== gesture.id) return;
			// a mouse let go outside the window, or the story took over mid-drag
			if (e.pointerType === 'mouse' && e.buttons === 0) return finish(e);
			if (blocked()) return finish(e, false);
			if (!gesture.axis) {
				const axis = gestureAxis(e.clientX - gesture.x0, e.clientY - gesture.y0, gesture.zone);
				if (!axis) return;
				if (axis === 'y') {
					gesture = null;
					// a motion this press stopped comes to rest (a pager settles)
					if (swallow) input.release(0);
					return;
				}
				gesture.axis = axis;
				input.dragging?.(true);
				try {
					band.setPointerCapture(e.pointerId);
				} catch {
					// the pointer is gone already – its up / cancel ends the drag
				}
			}
			e.preventDefault();
			input.move(e.clientX - gesture.x);
			gesture.x = e.clientX;
			gesture.samples.push({ t: e.timeStamp, x: e.clientX });
			while (gesture.samples.length > 2 && e.timeStamp - gesture.samples[0].t > VELOCITY_WINDOW) gesture.samples.shift();
		}

		function finish(e: PointerEvent, fling = e.type !== 'pointercancel') {
			if (!gesture || e.pointerId !== gesture.id) return;
			const { axis, samples, x0, x, t0 } = gesture;
			gesture = null;
			if (!axis) {
				// a tap that stopped a motion: it comes to rest (a pager settles)
				if (swallow) input.release(0);
				return;
			}
			input.dragging?.(false);
			swallow = true;
			if (fling) input.release(releaseVelocity(samples, e.timeStamp), { dx: x - x0, ms: e.timeStamp - t0 });
			else input.release(0);
		}

		/** a second finger: the browser's pinch-zoom – the first finger's sideways start is undone, no fling */
		function pinch() {
			if (!gesture) return;
			const { id, axis, x0, x } = gesture;
			gesture = null;
			if (!axis) return;
			input.move(x0 - x);
			input.dragging?.(false);
			input.release(0);
			try {
				band.releasePointerCapture(id);
			} catch {
				// gone already
			}
		}

		function click(e: MouseEvent) {
			if (e.detail === 0) return;
			if (swallow) {
				e.preventDefault();
				e.stopPropagation();
			}
			swallow = false;
		}

		function wheel(e: WheelEvent) {
			// ctrl: a trackpad pinch – the browser's zoom
			if (e.ctrlKey || off()) return;
			const dx = latch(e);
			if (dx === null) return;
			e.preventDefault();
			input.stop();
			input.move(-dx * (e.deltaMode === 1 ? remPx() : e.deltaMode === 2 ? band.clientWidth : 1));
			if (input.settle) {
				clearTimeout(wheelTimer);
				wheelTimer = setTimeout(() => input.settle?.(), WHEEL_BURST);
			}
		}

		function key(e: KeyboardEvent) {
			if (e.key !== 'ArrowLeft' && e.key !== 'ArrowRight') return;
			if (e.defaultPrevented || e.altKey || e.ctrlKey || e.metaKey || typing(e.target) || off()) return;
			e.preventDefault();
			input.step(e.key === 'ArrowRight' ? 1 : -1);
		}

		function focus(e: FocusEvent) {
			const target = e.target as Element | null;
			if (!target || target === band || gesture || !input.reveal || off() || !keyboardFocus(target)) return;
			input.reveal(target);
		}

		const listen = <K extends keyof HTMLElementEventMap>(type: K, fn: (e: HTMLElementEventMap[K]) => void, options?: AddEventListenerOptions) => {
			band.addEventListener(type, fn, options);
			return () => band.removeEventListener(type, fn, options);
		};
		const touchAction = band.style.touchAction;
		band.style.touchAction = TOUCH_ACTION;
		const undo = [
			listen('pointerdown', down),
			listen('pointermove', drag),
			listen('pointerup', (e) => finish(e)),
			listen('pointercancel', (e) => finish(e)),
			// only the band's own: a touch's implicit capture on the slice moving to the band fires one there too, and it
			// bubbles – it ended every touch drag at its first move (check:endless)
			listen('lostpointercapture', (e) => e.target === band && finish(e)),
			listen('click', click, { capture: true }),
			listen('dragstart', (e) => e.preventDefault()),
			listen('wheel', wheel, { passive: false }),
			listen('keydown', key),
			listen('focusin', focus)
		];
		return () => {
			for (const fn of undo) fn();
			clearTimeout(wheelTimer);
			band.style.touchAction = touchAction;
			if (gesture?.axis) input.dragging?.(false);
			gesture = null;
		};
	};
}
