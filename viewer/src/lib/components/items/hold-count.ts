// Press and hold to count a quantity (research/PACKING.md §2c, owner 2026-10-16: Stardew's press-and-hold picker):
// holding a tile counts 1, 2, 3 … faster and faster up to `max`; letting go hands the count over (`onhold`). A short
// press stays a normal click (the whole stack). The keyboard does the same with Enter / Space held (the button's own
// click comes on a short press); a screen reader's click stays a click. Nothing happens when `max` is 0.

export interface HoldCountOptions {
	/** the most it may count to (a stack's size, capped by the room left); 0 = no counting */
	max: number;
	/** each new count while holding (show it, announce it) */
	onstep: (count: number) => void;
	/** let go after counting: move this many */
	onhold: (count: number) => void;
	/** let go before the first count, or cancelled */
	onend?: () => void;
}

const FIRST = 450; // ms before the count starts – shorter is a tap
const START = 320; // ms between the first counts
const FASTEST = 60;

export function holdCount(node: HTMLElement, options: HoldCountOptions) {
	let opts = options;
	let timer: ReturnType<typeof setTimeout> | null = null;
	let count = 0;
	let swallowClick = false;

	function tick(delay: number) {
		timer = setTimeout(() => {
			if (count >= opts.max) return;
			count += 1;
			opts.onstep(count);
			tick(Math.max(FASTEST, delay * 0.85));
		}, delay);
	}
	function start() {
		if (opts.max < 1 || timer) return;
		count = 0;
		timer = setTimeout(() => {
			count = 1;
			opts.onstep(count);
			tick(START);
		}, FIRST);
	}
	function stop(): number {
		if (timer) clearTimeout(timer);
		timer = null;
		const n = count;
		count = 0;
		return n;
	}
	/** let go: a count was shown → hand it over (and drop the click that follows) */
	function finish(): boolean {
		const n = stop();
		if (n > 0) {
			swallowClick = true;
			opts.onhold(n);
			return true;
		}
		opts.onend?.();
		return false;
	}

	const down = (e: PointerEvent) => {
		swallowClick = false;
		if (e.button === 0) start();
	};
	const up = () => {
		if (timer || count) finish();
	};
	const cancel = () => {
		if (stop() > 0) opts.onend?.();
	};
	const click = (e: MouseEvent) => {
		if (!swallowClick) return;
		swallowClick = false;
		e.preventDefault();
		e.stopImmediatePropagation();
	};
	const keydown = (e: KeyboardEvent) => {
		if ((e.key !== 'Enter' && e.key !== ' ') || opts.max < 1) return;
		e.preventDefault(); // the button's own click waits for the key to come up
		if (!e.repeat) start();
	};
	const keyup = (e: KeyboardEvent) => {
		if ((e.key !== 'Enter' && e.key !== ' ') || opts.max < 1) return;
		e.preventDefault();
		if (finish()) swallowClick = false; // no click follows a key
		else node.click();
	};
	// a long press must not open the phone's context menu / text selection
	const menu = (e: Event) => {
		if (opts.max >= 1) e.preventDefault();
	};

	node.addEventListener('pointerdown', down);
	node.addEventListener('pointerup', up);
	node.addEventListener('pointercancel', cancel);
	node.addEventListener('pointerleave', cancel);
	node.addEventListener('click', click, true);
	node.addEventListener('keydown', keydown);
	node.addEventListener('keyup', keyup);
	node.addEventListener('contextmenu', menu);
	return {
		update(next: HoldCountOptions) {
			opts = next;
		},
		destroy() {
			stop();
			node.removeEventListener('pointerdown', down);
			node.removeEventListener('pointerup', up);
			node.removeEventListener('pointercancel', cancel);
			node.removeEventListener('pointerleave', cancel);
			node.removeEventListener('click', click, true);
			node.removeEventListener('keydown', keydown);
			node.removeEventListener('keyup', keyup);
			node.removeEventListener('contextmenu', menu);
		}
	};
}
