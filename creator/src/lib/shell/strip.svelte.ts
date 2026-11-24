// A sideways row of the bar (the open items, the workspace row) that always stops on a whole item at its left edge
// (lib/shell/strip.ts): where its last item would leave room, the row gives that width up instead of showing a cut item.
import { showWhole, spansOf, stepWhole, wholeIn, type Span, type StripPlace } from './strip.ts';

/** the wheel's travel that steps one item */
const WHEEL_STEP = 40;

const rem = () => parseFloat(getComputedStyle(document.documentElement).fontSize) || 16;

export class WholeStrip {
	el = $state<HTMLElement>();
	before = $state(false);
	after = $state(false);
	#wheeled = 0;

	/** `item`: the selector of its items; `fade`: the right end's fade (rem) while more lies beyond */
	constructor(
		private readonly item: string,
		private readonly fade = 2
	) {}

	#items = () => [...(this.el?.querySelectorAll<HTMLElement>(this.item) ?? [])];

	measure = () => {
		const el = this.el;
		if (!el) return;
		this.before = el.scrollLeft > 1;
		this.after = el.scrollLeft + el.clientWidth < el.scrollWidth - 1;
	};

	/** where the row stands now, and the width it has when it gives up nothing (the room it gave up is taken back to see) */
	#read(el: HTMLElement): { spans: Span[]; left: number; width: number } {
		const spans = spansOf(el, this.#items());
		const left = el.scrollLeft;
		el.style.maxWidth = '';
		return { spans, left, width: el.clientWidth };
	}

	#go(el: HTMLElement, to: StripPlace, width: number, smooth: boolean) {
		// set here, not through the template: the width and the scroll belong to the same moment
		el.style.maxWidth = to.pad ? `${(width - to.pad) / rem()}rem` : '';
		el.scrollTo({ left: to.left, behavior: smooth ? 'smooth' : 'instant' });
		this.measure();
	}

	/** item `at` whole in view, a whole item at the left edge (as it is when it already is) */
	show(at: number, smooth = true) {
		const el = this.el;
		if (!el) return;
		const { spans, left, width } = this.#read(el);
		this.#go(el, at >= 0 ? showWhole(spans, at, left, width, this.fade * rem()) : stepWhole(spans, left, width, 0), width, smooth);
	}

	/** the index of the first item matching `test` */
	indexOf(test: (el: HTMLElement) => boolean): number {
		return this.#items().findIndex(test);
	}

	/** a page along (‹ ›): as many items as lie whole */
	page(dir: -1 | 1) {
		const el = this.el;
		if (!el) return;
		const shown = el.clientWidth;
		const { spans, left, width } = this.#read(el);
		this.#go(el, stepWhole(spans, left, width, dir * wholeIn(spans, left, shown)), width, true);
	}

	/** a mouse wheel steps an item at a time sideways (a trackpad's own sideways move passes) */
	wheel = (e: WheelEvent) => {
		const el = this.el;
		if (!el || Math.abs(e.deltaX) >= Math.abs(e.deltaY) || el.scrollWidth <= el.clientWidth) return;
		e.preventDefault();
		this.#wheeled += e.deltaY;
		if (Math.abs(this.#wheeled) < WHEEL_STEP) return;
		const { spans, left, width } = this.#read(el);
		this.#go(el, stepWhole(spans, left, width, Math.sign(this.#wheeled)), width, true);
		this.#wheeled = 0;
	};

	/** calls `changed` whenever the row or one of its items changes size (a name arriving, the fonts, the bar); call in an effect */
	watch(changed: () => void): (() => void) | undefined {
		const el = this.el;
		if (!el) return;
		const observer = new ResizeObserver(() => {
			this.measure();
			changed();
		});
		observer.observe(el);
		for (const item of this.#items()) observer.observe(item);
		return () => observer.disconnect();
	}
}
