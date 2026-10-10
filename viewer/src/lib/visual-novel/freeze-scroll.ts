/**
 * The reader page's scroll stop while the story waits (vn-page.svelte, `vn.state.frozen`): the page stays exactly as it
 * is – no `overflow: hidden` on the root. That changed the layout: base.css gives <body> `overflow-x: hidden`, which
 * only propagates to the viewport while <html> is `overflow: visible`; with <html> hidden, <body> became a scroll
 * container of its own, every sticky element (the full-width strip's pin, strip.svelte) stuck to the unscrolled body
 * instead of the viewport, and a pinned strip jumped out of view – a black page behind the message (owner 2026-10-10,
 * catpope ch2 scene-14). Instead the scroll inputs are blocked: the wheel, touch moves, the scroll keys (not in a text
 * field, not on a button / link), and – as a safety net (a scrollbar drag, middle-click autoscroll) – a scroll right
 * after the reader's own input goes back. Programmatic scrolls (the `?label=` arrival aligning while images load) stay
 * allowed, as with `overflow: hidden`. Scrolling inside an element that scrolls by itself (a long text) stays possible.
 */

/** the keys that scroll the page */
const SCROLL_KEYS = new Set(['ArrowUp', 'ArrowDown', 'ArrowLeft', 'ArrowRight', 'PageUp', 'PageDown', 'Home', 'End', ' ', 'Spacebar']);

/** does a key press scroll the page (and not type into a field / press a button)? */
export function scrollKey(key: string, target: { tagName?: string; isContentEditable?: boolean; closest?: (s: string) => unknown } | null): boolean {
	if (!SCROLL_KEYS.has(key)) return false;
	if (!target) return true;
	if (target.isContentEditable || /^(INPUT|TEXTAREA|SELECT)$/.test(target.tagName ?? '')) return false;
	// Space presses a focused button / link / summary – not a scroll
	if ((key === ' ' || key === 'Spacebar') && target.closest?.('button, a[href], summary, [role="button"], [role="link"]')) return false;
	return true;
}

/** an element between the target and the page that scrolls by itself in that direction */
function scrollsInside(target: EventTarget | null, vertical: boolean): boolean {
	let el = target instanceof Element ? target : null;
	while (el && el !== document.body && el !== document.documentElement) {
		const style = getComputedStyle(el);
		const overflow = vertical ? style.overflowY : style.overflowX;
		if (/(auto|scroll)/.test(overflow) && (vertical ? el.scrollHeight > el.clientHeight : el.scrollWidth > el.clientWidth)) return true;
		el = el.parentElement;
	}
	return false;
}

/** stop the page's scroll; returns the undo */
export function freezeScroll(): () => void {
	const html = document.documentElement;
	const before = { overscroll: html.style.overscrollBehavior };
	html.style.overscrollBehavior = 'none';
	html.dataset.vnBlocking = '';

	let x = window.scrollX;
	let y = window.scrollY;
	/** the reader's last input (ms) and whether a pointer is down – a scroll then is theirs */
	let input = -Infinity;
	let pressed = false;
	const touched = () => (input = performance.now());

	const onWheel = (e: WheelEvent) => {
		touched();
		if (e.ctrlKey || scrollsInside(e.target, Math.abs(e.deltaY) >= Math.abs(e.deltaX))) return;
		e.preventDefault();
	};
	let touchY = 0;
	let touchX = 0;
	const onTouchStart = (e: TouchEvent) => {
		touched();
		touchX = e.touches[0]?.clientX ?? 0;
		touchY = e.touches[0]?.clientY ?? 0;
	};
	const onTouchMove = (e: TouchEvent) => {
		touched();
		// two fingers: pinch zoom stays the browser's
		if (e.touches.length > 1 || !e.cancelable) return;
		const t = e.touches[0];
		const vertical = !t || Math.abs(t.clientY - touchY) >= Math.abs(t.clientX - touchX);
		if (scrollsInside(e.target, vertical)) return;
		e.preventDefault();
	};
	const onKey = (e: KeyboardEvent) => {
		touched();
		if (e.altKey || e.ctrlKey || e.metaKey || !scrollKey(e.key, e.target as HTMLElement | null)) return;
		if (scrollsInside(e.target, !(e.key === 'ArrowLeft' || e.key === 'ArrowRight'))) return;
		e.preventDefault();
	};
	const onDown = () => {
		touched();
		pressed = true;
	};
	const onUp = () => {
		touched();
		pressed = false;
	};
	const onScroll = () => {
		if (window.scrollX === x && window.scrollY === y) return;
		if (pressed || performance.now() - input < 1000) window.scrollTo({ left: x, top: y, behavior: 'instant' });
		// the page's own scroll (an arrival aligning, a jump) – the new place is the frozen one
		else {
			x = window.scrollX;
			y = window.scrollY;
		}
	};

	// capture on the window: before any element's own handler (strip.svelte's sideways wheel / ← → skip a prevented event)
	const active = { capture: true, passive: false } as const;
	const passive = { capture: true, passive: true } as const;
	window.addEventListener('wheel', onWheel, active);
	window.addEventListener('touchstart', onTouchStart, passive);
	window.addEventListener('touchmove', onTouchMove, active);
	window.addEventListener('keydown', onKey, active);
	window.addEventListener('pointerdown', onDown, passive);
	window.addEventListener('pointerup', onUp, passive);
	window.addEventListener('pointercancel', onUp, passive);
	window.addEventListener('scroll', onScroll, passive);

	return () => {
		window.removeEventListener('wheel', onWheel, active);
		window.removeEventListener('touchstart', onTouchStart, passive);
		window.removeEventListener('touchmove', onTouchMove, active);
		window.removeEventListener('keydown', onKey, active);
		window.removeEventListener('pointerdown', onDown, passive);
		window.removeEventListener('pointerup', onUp, passive);
		window.removeEventListener('pointercancel', onUp, passive);
		window.removeEventListener('scroll', onScroll, passive);
		html.style.overscrollBehavior = before.overscroll;
		delete html.dataset.vnBlocking;
	};
}
