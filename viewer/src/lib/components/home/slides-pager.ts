import { ease, indexAt, keepPlace, nearestEdge, pageLayout, placeFrames, settleTarget, stepTarget, type Motion, type PageLayout } from '$lib/components/toon/renderer/strip-loop.ts';
import { stripDrag, type StripInput } from '$lib/components/toon/renderer/strip-drag.ts';
import { rootRem } from '$lib/components/toon/renderer/strip-dom.ts';
import { autoplayOn, autoplayRuns } from './slides-autoplay.ts';

/** what the slider shows (product-slides.svelte's data attributes) */
export interface SlidesState {
	/** the slide at the window's left */
	index: number;
	count: number;
	perView: number;
	/** more slides than fit: it loops, drags and plays */
	movable: boolean;
	/** autoplay is on (more slides than fit, no reduced motion) – a hold only pauses it */
	playing: boolean;
	dragging: boolean;
}

export interface SlidesPager {
	/** the slides changed (another variant's images) */
	update(): void;
	destroy(): void;
}

export interface SlidesOptions {
	/** ms per slide, 0 = no autoplay */
	autoplay: number;
	onstate(state: SlidesState): void;
}

/**
 * The home product slider (owner 2026-10-17 "Startseiten-Slider auf unser Werkzeug", TEXT-ACROSS-SLICES.md §9) on the
 * endless band's utility: strip-loop.ts lays the slides on the device grid and wraps them by `left: k·T` (no clones),
 * strip-drag.ts does the drag (dead zone, axis lock, click swallow, deltaX wheel). On top, paging: a drag, a wheel burst
 * or a tap that stopped a slide settles on a slide edge (the short-swipe rule – settleTarget), ← / → (the viewport is
 * focusable) and autoplay ease one slide on. No buttons (owner 2026-10-17). Autoplay: held while a mouse is over the
 * slides, keyboard focus is in the slider, a finger is on it, it is dragged or the tab is hidden, and the countdown starts
 * anew once the hold ends; an interaction restarts it; off with reduced motion (eases jump then) – slides-autoplay.ts. Per view and gap are the component's CSS
 * (`--slides-per-view`, a number; `--slides-gap`, in rem); a width-only ResizeObserver re-measures in the next frame –
 * lazy images change only the height, and a layout written inside the observer's delivery looped in WebKit (measured).
 */
export function slidesPager(region: HTMLElement, viewport: HTMLElement, track: HTMLElement, o: SlidesOptions): SlidesPager {
	const doc = viewport.ownerDocument;
	const win = doc.defaultView ?? window;
	const reduced = win.matchMedia('(prefers-reduced-motion: reduce)');
	const slides = () => [...track.children] as HTMLElement[];

	let pages: PageLayout | null = null;
	let width = 0;
	let rem = 16;
	let dpr = 1;
	let perView = 1;
	/** how far the picture has moved right, px (any number – it repeats every turn) */
	let offset = 0;
	let motion: Motion | null = null;
	let frame = 0;
	let timer = 0;
	let hover = false;
	let touch = false;
	let focus = false;
	let dragging = false;
	let shown = -1;
	let resize = 0;

	const movable = () => pages?.fit.mode === 'loop';
	const toRem = (px: number) => `${px / rem}rem`;
	const playing = () => autoplayOn(o.autoplay, reduced.matches) && movable();
	const state = (): SlidesState => ({ index: Math.max(0, shown), count: slides().length, perView, movable: movable(), playing: playing(), dragging });
	const emit = () => o.onstate(state());

	function render() {
		if (!pages) return;
		const els = slides();
		if (!movable()) {
			track.style.transform = '';
			for (const el of els) el.style.left = '';
			return show(0);
		}
		const placement = placeFrames(offset, pages.layout, pages.fit, dpr);
		track.style.transform = `translate3d(${toRem(placement.x)}, 0, 0)`;
		els.forEach((el, i) => (el.style.left = placement.turns[i] ? toRem(placement.turns[i] * pages!.layout.turn) : ''));
		show(indexAt(offset, pages.layout));
	}

	/** the slides out of view are hidden from screen readers (their alt texts are the same) */
	function show(index: number) {
		if (index === shown) return;
		shown = index;
		const els = slides();
		els.forEach((el, i) => el.setAttribute('aria-hidden', String((i - index + els.length) % els.length >= perView)));
		emit();
	}

	function measure() {
		const els = slides();
		const style = win.getComputedStyle(viewport);
		rem = rootRem(doc);
		dpr = win.devicePixelRatio || 1;
		width = viewport.getBoundingClientRect().width;
		perView = Math.max(1, parseInt(style.getPropertyValue('--slides-per-view'), 10) || 1);
		const gap = (parseFloat(style.getPropertyValue('--slides-gap')) || 0) * rem;
		const next = pageLayout(els.length, perView, width, gap, dpr);
		// a running ease would write the old scale back
		stop();
		offset = pages ? nearestEdge(keepPlace(offset, pages.layout.turn, next.layout.turn), next.layout) : 0;
		pages = next;
		for (const el of els) el.style.width = toRem(next.pitch);
		shown = -1;
		render();
		schedule();
	}

	function tick(now: number) {
		frame = 0;
		if (!motion) return;
		const at = motion.at(now);
		offset = at.x;
		if (at.done) motion = null;
		render();
		if (motion) frame = win.requestAnimationFrame(tick);
	}
	function stop() {
		const ran = motion !== null;
		motion = null;
		if (frame) win.cancelAnimationFrame(frame);
		frame = 0;
		return ran;
	}
	/** an ease to `to` – with reduced motion a jump */
	function go(to: number) {
		stop();
		if (reduced.matches) {
			offset = to;
			return render();
		}
		motion = ease(offset, to, performance.now());
		frame = win.requestAnimationFrame(tick);
	}
	/** one slide on from where a running ease ends (presses add up) */
	function step(by: 1 | -1) {
		if (pages && movable()) go(stepTarget(motion?.ease ? motion.end : offset, by, pages.layout));
	}

	function schedule() {
		win.clearTimeout(timer);
		timer = 0;
		if (autoplayRuns(autoplayOn(o.autoplay, reduced.matches), movable(), { hover, focus, touch, dragging, hidden: doc.hidden }))
			timer = win.setTimeout(() => {
				timer = 0;
				step(1);
				schedule();
			}, o.autoplay);
	}
	/** the reader moved it: the countdown starts anew */
	const interact = schedule;

	const input: StripInput = {
		enabled: movable,
		stop,
		move(dx) {
			stop();
			interact();
			offset += dx;
			render();
		},
		// a drag let go, or a tap that stopped a slide
		release(velocity, swipe) {
			interact();
			if (pages) go(settleTarget(offset, pages.layout, velocity, swipe));
		},
		step(by) {
			interact();
			step(by);
		},
		settle() {
			if (pages) go(settleTarget(offset, pages.layout));
		},
		dragging(on) {
			dragging = on;
			emit();
			schedule();
		}
	};

	const onEnter = (e: PointerEvent) => e.pointerType === 'mouse' && ((hover = true), schedule());
	const onLeave = () => ((hover = false), schedule());
	const keyboard = (el: EventTarget | null) => el instanceof Element && el.matches(':focus-visible');
	const onFocus = (e: FocusEvent) => ((focus = keyboard(e.target)), schedule());
	const onBlur = (e: FocusEvent) => !region.contains(e.relatedTarget as Node | null) && ((focus = false), schedule());
	// a finger (or pen) on the slides holds it until it lifts – also when the page takes the gesture (a vertical swipe)
	const onDown = (e: PointerEvent) => e.pointerType !== 'mouse' && ((touch = true), schedule());
	const onUp = (e: PointerEvent) => touch && e.pointerType !== 'mouse' && ((touch = false), schedule());
	const onMotion = () => (emit(), schedule());
	// width only: a lazy image loading changes the height, and must not reset the place
	const observer = new ResizeObserver(() => {
		if (!resize)
			resize = win.requestAnimationFrame(() => {
				resize = 0;
				if (viewport.getBoundingClientRect().width !== width) measure();
			});
	});
	const undrag = stripDrag(input)(viewport) as () => void;
	viewport.addEventListener('pointerenter', onEnter);
	viewport.addEventListener('pointerleave', onLeave);
	viewport.addEventListener('pointerdown', onDown);
	win.addEventListener('pointerup', onUp);
	win.addEventListener('pointercancel', onUp);
	region.addEventListener('focusin', onFocus);
	region.addEventListener('focusout', onBlur);
	doc.addEventListener('visibilitychange', schedule);
	reduced.addEventListener('change', onMotion);
	observer.observe(viewport);
	measure();

	return {
		update: measure,
		destroy() {
			stop();
			win.clearTimeout(timer);
			win.cancelAnimationFrame(resize);
			undrag();
			observer.disconnect();
			viewport.removeEventListener('pointerenter', onEnter);
			viewport.removeEventListener('pointerleave', onLeave);
			viewport.removeEventListener('pointerdown', onDown);
			win.removeEventListener('pointerup', onUp);
			win.removeEventListener('pointercancel', onUp);
			region.removeEventListener('focusin', onFocus);
			region.removeEventListener('focusout', onBlur);
			doc.removeEventListener('visibilitychange', schedule);
			reduced.removeEventListener('change', onMotion);
		}
	};
}
