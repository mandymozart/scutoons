/**
 * The fixed top row – the logo and the round toggles (main-menu, inventory, avatar, map, chapter list) – slides away
 * while the reader reads or watches (owner 2026-10-08): scrolling down hides it, a little scroll up brings it back,
 * the teaser video hides it when idle, events that touch it (items, choices, questions, effects) bring it back first.
 * One class on <html>, `chrome-hidden`; the staggered slide is CSS (static/css/styles.css "Top row hide").
 * Never hidden while something holds it (an open overlay, the main menu).
 */

/** the longest stagger delay + the slide (styles.css) – what an event waits before it plays */
const SLIDE_MS = 420;
/** at the top of the page the row always shows (rem) */
const TOP_ZONE = 4;
/** scrolled down in one go before it hides, up before it shows (rem; up ≈ half a centimetre) */
const HIDE_AFTER = 1.5;
const SHOW_AFTER = 1;

let hidden = false;
/** what keeps the row showing (an open overlay, the menu, a focused question); a navigation clears them */
const holds = new Set<symbol>();

function root() {
	return typeof document === 'undefined' ? null : document.documentElement;
}

export function hideChrome() {
	const html = root();
	if (!html || hidden || holds.size > 0) return;
	hidden = true;
	html.classList.add('chrome-hidden');
}

/** brings the row back; resolves once it is in (at once when it was showing) */
export function showChrome(): Promise<void> {
	const html = root();
	if (!html || !hidden) return Promise.resolve();
	hidden = false;
	html.classList.remove('chrome-hidden');
	return new Promise((resolve) => setTimeout(resolve, SLIDE_MS));
}

/** keeps the row showing until the returned release is called (an open overlay) */
export function holdChrome(): () => void {
	const hold = Symbol();
	holds.add(hold);
	void showChrome();
	return () => void holds.delete(hold);
}

/** a new page: no holds left over from the last one, the row shows */
export function resetChrome() {
	holds.clear();
	void showChrome();
}

/** scrolling down hides the row, a little up (or the top of the page) shows it; returns the stop */
export function chromeFollowsScroll(): () => void {
	const rem = parseFloat(getComputedStyle(document.documentElement).fontSize) || 16;
	let last = window.scrollY;
	let run = 0;
	const onScroll = () => {
		const y = window.scrollY;
		const d = y - last;
		last = y;
		if (y < TOP_ZONE * rem) {
			run = 0;
			void showChrome();
			return;
		}
		// one direction in a row: a turn starts counting again (a finger's wobble never adds up)
		run = Math.sign(d) === Math.sign(run) ? run + d : d;
		if (run > HIDE_AFTER * rem) hideChrome();
		else if (run < -SHOW_AFTER * rem) void showChrome();
	};
	window.addEventListener('scroll', onScroll, { passive: true });
	return () => {
		window.removeEventListener('scroll', onScroll);
		void showChrome();
	};
}

/** while watching: hides after `ms` without input, any input shows it again; returns the stop (shows it) */
export function chromeHidesWhenIdle(ms = 2500): () => void {
	let timer = setTimeout(hideChrome, ms);
	const wake = () => {
		void showChrome();
		clearTimeout(timer);
		timer = setTimeout(hideChrome, ms);
	};
	const inputs = ['pointermove', 'pointerdown', 'keydown', 'wheel'] as const;
	for (const type of inputs) window.addEventListener(type, wake, { passive: true });
	return () => {
		clearTimeout(timer);
		for (const type of inputs) window.removeEventListener(type, wake);
		void showChrome();
	};
}
