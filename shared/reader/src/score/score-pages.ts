/**
 * The score's pages on native CSS scroll-snap (owner 2026-10-17, research/TEXT-ACROSS-SLICES.md §9): a horizontal
 * scroller with one page per snap point. A swipe, a wheel or a trackpad turns pages natively; the view turns by
 * `go` (keys, buttons, the pedal, Play following the music). While a turn the view asked for is on its way, the
 * pages it passes are not reported – a smooth turn 1 → 5 must not show 2, 3, 4, and Play's next jump must not be
 * undone by a late scroll event. Pure parts first (unit-tested: test/score-pages.test.ts), the DOM wiring last.
 */

/** the page (1 … pages) a scroll position shows: the nearest snap point */
export function pageAt(left: number, width: number, pages: number): number {
	if (!(width > 0) || pages < 1) return 1;
	return Math.min(pages, Math.max(1, Math.round(left / width) + 1));
}

/** whether `left` rests on page `n`'s snap point (within a hundredth of a page: fractional widths, device pixels) */
export function restsOn(left: number, width: number, n: number): boolean {
	return width > 0 && Math.abs(left / width - (n - 1)) < 0.01;
}

/** what a scroll position reports: the page it shows, or null while the view's own `target` is still on its way */
export function reported(left: number, width: number, pages: number, target: number | null): { page: number | null; target: number | null } {
	if (target === null) return { page: pageAt(left, width, pages), target: null };
	return restsOn(left, width, target) ? { page: target, target: null } : { page: null, target };
}

/** where a reader's scroll that came to rest belongs: null when it rests on a page, else the nearest page (a snap the
 * browser left undone – Firefox once left a sideways wheel scroll between two pages) */
export function strayed(left: number, width: number, pages: number): number | null {
	const n = pageAt(left, width, pages);
	return width > 0 && !restsOn(left, width, n) ? n : null;
}

/** quiet this long after the last scroll event = settled (browsers without `scrollend`, a turn cut short) */
const SETTLE_MS = 150;
/** while the view's own turn is on its way: WebKit can take over 100 ms to start a smooth scroll */
const TURN_SETTLE_MS = 400;

export interface PageScroller {
	/** turn to page `n`: `smooth` = an animated turn (a reader's one-page step), else instant (a jump) */
	go(n: number, smooth: boolean): void;
	destroy(): void;
}

/** wire a scroll-snap scroller: `visit(n)` whenever scrolling lands on another page than the view's own turn */
export function pageScroller(node: HTMLElement, pages: () => number, visit: (n: number) => void): PageScroller {
	let target: number | null = null;
	/** the pending turn was finished once by an instant jump (a second miss gives up: the scroller can't get there) */
	let finished = false;
	/** the reader's scroll was put on a page once (one try, like `finished`) */
	let snapped = false;
	/** a finger or a mouse button is down on the scroller: where it rests is not where it stays */
	let held = false;
	let quiet = 0;
	const width = () => node.getBoundingClientRect().width;
	const report = () => {
		const r = reported(node.scrollLeft, width(), pages(), target);
		target = r.target;
		if (r.page !== null) visit(r.page);
	};
	const jump = (n: number, w: number) => {
		node.scrollTo({ left: (n - 1) * w, behavior: 'instant' });
		quiet = window.setTimeout(settle, SETTLE_MS);
	};
	// settled: the view's own turn stopped short (interrupted by the next one, WebKit off the snap point) ends with an
	// instant jump; a reader's scroll left between pages goes to the nearest one; then wherever it rests counts
	const settle = () => {
		clearTimeout(quiet);
		const w = width();
		if (target !== null && !finished && !restsOn(node.scrollLeft, w, target)) {
			finished = true;
			return jump(target, w);
		}
		const stray = target === null && !held && !snapped ? strayed(node.scrollLeft, w, pages()) : null;
		if (stray !== null) {
			snapped = true;
			return jump(stray, w);
		}
		target = null;
		report();
	};
	const wait = () => {
		clearTimeout(quiet);
		quiet = window.setTimeout(settle, target === null ? SETTLE_MS : TURN_SETTLE_MS);
	};
	const onscroll = () => {
		report();
		wait();
	};
	// the reader takes over: their scrolling counts at once
	const release = () => ((target = null), (snapped = false));
	const inputs = ['pointerdown', 'wheel', 'touchstart'] as const;
	// a touch is held until its last finger lifts (a touch pointer is cancelled when the browser pans); a mouse until
	// its button is up anywhere (the scrollbar)
	const touch = (e: TouchEvent) => {
		held = e.touches.length > 0;
		if (!held) wait();
	};
	const press = (e: PointerEvent) => e.pointerType === 'mouse' && (held = true);
	const lift = (e: PointerEvent) => e.pointerType === 'mouse' && held && ((held = false), wait());
	node.addEventListener('scroll', onscroll, { passive: true });
	node.addEventListener('scrollend', settle);
	for (const type of inputs) node.addEventListener(type, release, { passive: true });
	node.addEventListener('touchstart', touch, { passive: true });
	node.addEventListener('touchend', touch);
	node.addEventListener('touchcancel', touch);
	node.addEventListener('pointerdown', press);
	window.addEventListener('pointerup', lift);
	return {
		go(n, smooth) {
			const w = width();
			if (restsOn(node.scrollLeft, w, n)) {
				target = null;
				return;
			}
			target = n;
			finished = false;
			node.scrollTo({ left: (n - 1) * w, behavior: smooth ? 'smooth' : 'instant' });
			wait();
		},
		destroy() {
			clearTimeout(quiet);
			node.removeEventListener('scroll', onscroll);
			node.removeEventListener('scrollend', settle);
			for (const type of inputs) node.removeEventListener(type, release);
			node.removeEventListener('touchstart', touch);
			node.removeEventListener('touchend', touch);
			node.removeEventListener('touchcancel', touch);
			node.removeEventListener('pointerdown', press);
			window.removeEventListener('pointerup', lift);
		}
	};
}
