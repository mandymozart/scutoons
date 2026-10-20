/**
 * A phone sheet whose head drags it (UI-CONCEPTS E.10; owner 2026-10-11 "warum ist das Sheet nicht mehr klein ziehbar",
 * 2026-10-18 "it follows the finger"): up = taller / open, down = lower, let go below 6rem = closed; the dragged height
 * is exactly where the finger is (`rem`), a drag never counts as a tap on what is under it (`swallowTap`). Below 768px
 * the shared `Divider` is hidden, so phones use this; the map builder and the chapter editor still carry their own copy
 * of the same gesture (to move onto this).
 */
export class SheetDrag {
	open = $state(false);
	/** the dragged height in rem; null = the sheet's own height */
	rem = $state<number | null>(null);
	dragging = $state(false);
	swallowTap = false;
	#drag: { id: number; y: number; start: number; moved: boolean } | null = null;

	/** @param element the sheet (its height is where a drag starts); @param closedRem its head's height */
	constructor(
		private element: () => HTMLElement | undefined,
		private closedRem = 2.75
	) {}

	#root() {
		return parseFloat(getComputedStyle(document.documentElement).fontSize) || 16;
	}
	down = (e: PointerEvent) => {
		const el = this.element();
		this.#drag = { id: e.pointerId, y: e.clientY, start: this.open && el ? el.getBoundingClientRect().height / this.#root() : this.closedRem, moved: false };
	};
	move = (e: PointerEvent) => {
		const drag = this.#drag;
		if (!drag || drag.id !== e.pointerId) return;
		const dy = e.clientY - drag.y;
		if (!drag.moved && Math.abs(dy) < 8) return;
		if (!drag.moved)
			try {
				(e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);
			} catch {
				/* a pointer that is gone */
			}
		drag.moved = true;
		this.dragging = true;
		this.open = true;
		this.rem = Math.max(this.closedRem, Math.min((window.innerHeight * 0.85) / this.#root(), drag.start - dy / this.#root()));
	};
	up = (e: PointerEvent) => {
		const drag = this.#drag;
		if (!drag || drag.id !== e.pointerId) return;
		this.#drag = null;
		this.dragging = false;
		if (!drag.moved) return;
		this.swallowTap = true;
		setTimeout(() => (this.swallowTap = false), 50);
		if ((this.rem ?? 0) < 6) this.close();
	};
	cancel = () => {
		this.#drag = null;
		this.dragging = false;
	};
	close() {
		this.open = false;
		this.rem = null;
	}
}
