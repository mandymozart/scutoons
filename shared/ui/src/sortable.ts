import { tick } from 'svelte';
import type { ActionReturn } from 'svelte/action';

/**
 * THE reorderable list (BACKLOG 10.53, agents/DESIGN.md "Reorderable lists"): put `use:sortable={{ onmove }}` on the
 * list's container, `data-sort-row` on every row (in the list's order) and a `SortGrip` at each row's start.
 *
 * - pointer: drag by the grip – a mouse right away, touch after a press-and-hold (~300 ms) on the grip, so a swipe
 *   over the list still scrolls; the row is lifted (a hard ink offset) and a drop line in `--primary-color` shows
 *   where it lands; near the scroll container's edges the list scrolls; Esc cancels;
 * - keyboard: Alt+↑ / Alt+↓ (Alt+← / → in a horizontal list) on a focused row or its grip – the grip also takes the
 *   plain arrows; the new place is announced (aria-live) and the focus follows the row;
 * - it only REPORTS `onmove(from, to)` (`to` = the row's index after the move, as `moveAt` in the creator): the caller
 *   changes its document, so one reorder = one undo step. A caller with a selection may move all picked rows.
 *
 * Rows keep their own classes (selection, state); the action only adds inline styles while a row is lifted. Lists can
 * nest (a group's slices inside the group rows): every row belongs to the nearest `use:sortable` above it.
 */
export interface SortableOptions {
	/** the row at `from` goes to `to` (its index after the move); may return a promise (focus waits for it) */
	onmove: (from: number, to: number) => void | Promise<unknown>;
	/** vertical (default) or horizontal / wrapping (a chip row) */
	axis?: 'vertical' | 'horizontal';
	/** no reordering (e.g. while a reorder is saved) */
	disabled?: boolean;
	/** the live announcement after a move; default `3 / 7` */
	announce?: (from: number, to: number, count: number) => string;
}

const ROW = '[data-sort-row]';
const GRIP = '[data-sort-grip]';
const HOLD_MS = 300;

let live: HTMLElement | null = null;
/** one polite live region for every list */
function announce(text: string) {
	if (typeof document === 'undefined') return;
	if (!live || !live.isConnected) {
		live = document.createElement('div');
		live.setAttribute('aria-live', 'polite');
		live.setAttribute('role', 'status');
		live.dataset.sortLive = '';
		Object.assign(live.style, { position: 'fixed', width: '0.0625rem', height: '0.0625rem', overflow: 'hidden', clipPath: 'inset(50%)', whiteSpace: 'nowrap', left: '0', top: '0' });
		document.body.append(live);
	}
	live.textContent = '';
	// a new text node, so the same words twice are read twice
	requestAnimationFrame(() => live && (live.textContent = text));
}

const rem = () => parseFloat(getComputedStyle(document.documentElement).fontSize) || 16;

function scrollParent(node: HTMLElement): HTMLElement {
	for (let el = node.parentElement; el; el = el.parentElement) {
		const style = getComputedStyle(el);
		if (/(auto|scroll|overlay)/.test(style.overflowY) && el.scrollHeight > el.clientHeight) return el;
	}
	return (document.scrollingElement as HTMLElement) ?? document.documentElement;
}

export function sortable(node: HTMLElement, initial: SortableOptions): ActionReturn<SortableOptions> {
	let options = initial;
	node.dataset.sortable = '';

	const own = (row: Element | null): row is HTMLElement => !!row && row.parentElement?.closest('[data-sortable]') === node;
	const rows = () => [...node.querySelectorAll<HTMLElement>(ROW)].filter(own);
	const gripOf = (row: HTMLElement) => [...row.querySelectorAll<HTMLElement>(GRIP)].find((g) => g.closest(ROW) === row) ?? null;
	const say = (from: number, to: number, count: number) => announce(options.announce?.(from, to, count) ?? `${to + 1} / ${count}`);

	// ── pointer ──
	type Drag = {
		row: HTMLElement;
		grip: HTMLElement;
		pointer: number;
		from: number;
		to: number;
		x0: number;
		y0: number;
		x: number;
		y: number;
		scroller: HTMLElement;
		scroll0: number;
		style: string;
		line: HTMLElement;
		frame: number;
		cursor: string;
	};
	let drag: Drag | null = null;
	let hold: { timer: ReturnType<typeof setTimeout>; x: number; y: number; pointer: number; stop: () => void } | null = null;

	function scrollTop(el: HTMLElement) {
		return el === document.scrollingElement ? window.scrollY : el.scrollTop;
	}

	function start(row: HTMLElement, grip: HTMLElement, pointer: number, x: number, y: number) {
		const list = rows();
		const from = list.indexOf(row);
		if (from < 0) return;
		const scroller = scrollParent(node);
		const line = document.createElement('div');
		line.dataset.sortLine = '';
		Object.assign(line.style, { position: 'fixed', zIndex: '1000', pointerEvents: 'none', background: 'var(--primary-color)', display: 'none' });
		document.body.append(line);
		drag = { row, grip, pointer, from, to: from, x0: x, y0: y, x, y, scroller, scroll0: scrollTop(scroller), style: row.getAttribute('style') ?? '', line, frame: 0, cursor: document.body.style.cursor };
		try {
			grip.setPointerCapture(pointer);
		} catch {
			/* a pointer that is gone already */
		}
		// lifted: a hard ink offset (DESIGN.md "Reorderable lists"), opaque, above its neighbours
		const computed = getComputedStyle(row);
		row.style.position = computed.position === 'static' ? 'relative' : computed.position;
		row.style.zIndex = '20';
		row.style.boxShadow = '0.25rem 0.25rem 0 var(--ink)';
		row.style.transition = 'none';
		if (/rgba\(0, 0, 0, 0\)|transparent/.test(computed.backgroundColor)) row.style.background = 'var(--paper)';
		row.dataset.sortLifted = '';
		node.dataset.sortDragging = '';
		document.body.style.cursor = 'grabbing';
		window.addEventListener('pointermove', onMove, true);
		window.addEventListener('pointerup', onUp, true);
		window.addEventListener('pointercancel', onCancel, true);
		window.addEventListener('keydown', onEscape, true);
		paint();
		drag.frame = requestAnimationFrame(autoScroll);
	}

	/** where the dragged row lands: its index among the other rows (= its index after the move) */
	function place() {
		if (!drag) return;
		const d = drag;
		const others = rows().filter((r) => r !== d.row);
		const boxes = others.map((r) => r.getBoundingClientRect());
		const horizontal = options.axis === 'horizontal';
		let k = 0;
		for (const b of boxes) {
			const after = horizontal ? d.y > b.bottom || (d.y >= b.top && d.x > b.left + b.width / 2) : d.y > b.top + b.height / 2;
			if (after) k++;
		}
		d.to = k;
		const line = d.line;
		if (k === d.from || !boxes.length) {
			line.style.display = 'none';
			return;
		}
		const prev = boxes[k - 1];
		const next = boxes[k];
		line.style.display = 'block';
		if (horizontal) {
			const b = next ?? prev;
			const x = next ? (prev && Math.abs(prev.top - next.top) < 1 ? (prev.right + next.left) / 2 : next.left) : prev.right;
			Object.assign(line.style, { left: `${x}px`, top: `${b.top}px`, width: 'var(--border-width-focus)', height: `${b.height}px`, transform: 'translateX(-50%)' });
		} else {
			const b = next ?? prev;
			const y = next ? (prev ? (prev.bottom + next.top) / 2 : next.top) : prev.bottom;
			Object.assign(line.style, { left: `${b.left}px`, top: `${y}px`, width: `${b.width}px`, height: 'var(--border-width-focus)', transform: 'translateY(-50%)' });
		}
	}

	function paint() {
		if (!drag) return;
		const dy = drag.y - drag.y0 + (scrollTop(drag.scroller) - drag.scroll0);
		const dx = options.axis === 'horizontal' ? drag.x - drag.x0 : 0;
		drag.row.style.transform = `translate(calc(${dx}px - 0.25rem), calc(${dy}px - 0.25rem))`;
		place();
	}

	function autoScroll() {
		if (!drag) return;
		const s = drag.scroller;
		const page = s === document.scrollingElement;
		const box = page ? { top: 0, bottom: window.innerHeight } : s.getBoundingClientRect();
		const zone = 3 * rem();
		const speed = 1.25 * rem();
		let by = 0;
		if (drag.y < box.top + zone) by = -speed * Math.min(1, (box.top + zone - drag.y) / zone);
		else if (drag.y > box.bottom - zone) by = speed * Math.min(1, (drag.y - (box.bottom - zone)) / zone);
		if (by) {
			if (page) window.scrollBy(0, by);
			else s.scrollTop += by;
			paint();
		}
		drag.frame = requestAnimationFrame(autoScroll);
	}

	function finish(commit: boolean) {
		const d = drag;
		if (!d) return;
		drag = null;
		cancelAnimationFrame(d.frame);
		window.removeEventListener('pointermove', onMove, true);
		window.removeEventListener('pointerup', onUp, true);
		window.removeEventListener('pointercancel', onCancel, true);
		window.removeEventListener('keydown', onEscape, true);
		try {
			d.grip.releasePointerCapture(d.pointer);
		} catch {
			/* released already */
		}
		d.line.remove();
		if (d.style) d.row.setAttribute('style', d.style);
		else d.row.removeAttribute('style');
		delete d.row.dataset.sortLifted;
		delete node.dataset.sortDragging;
		document.body.style.cursor = d.cursor;
		if (commit && d.to !== d.from) {
			const count = rows().length;
			Promise.resolve(options.onmove(d.from, d.to)).then(() => say(d.from, d.to, count));
		}
	}

	const onMove = (e: PointerEvent) => {
		if (!drag || e.pointerId !== drag.pointer) return;
		e.preventDefault();
		drag.x = e.clientX;
		drag.y = e.clientY;
		paint();
	};
	const onUp = (e: PointerEvent) => {
		if (drag && e.pointerId === drag.pointer) finish(true);
	};
	const onCancel = (e: PointerEvent) => {
		if (drag && e.pointerId === drag.pointer) finish(false);
	};
	const onEscape = (e: KeyboardEvent) => {
		if (e.key !== 'Escape' || !drag) return;
		e.preventDefault();
		e.stopPropagation();
		finish(false);
	};

	function cancelHold() {
		if (!hold) return;
		clearTimeout(hold.timer);
		hold.stop();
		hold = null;
	}

	function onPointerDown(e: PointerEvent) {
		if (options.disabled || drag || !e.isPrimary) return;
		const grip = (e.target as Element | null)?.closest<HTMLElement>(GRIP);
		const row = grip?.closest<HTMLElement>(ROW) ?? null;
		if (!grip || !own(row) || grip.matches(':disabled, [aria-disabled="true"]')) return;
		if (e.pointerType !== 'touch') {
			if (e.button !== 0) return;
			// no text selection while dragging
			e.preventDefault();
			start(row, grip, e.pointerId, e.clientX, e.clientY);
			return;
		}
		// touch: press and hold – moving the finger first scrolls the list (the browser cancels the pointer)
		cancelHold();
		const slop = 0.5 * rem();
		const move = (m: PointerEvent) => {
			if (hold && m.pointerId === hold.pointer && Math.hypot(m.clientX - hold.x, m.clientY - hold.y) > slop) cancelHold();
		};
		const end = (m: PointerEvent) => {
			if (hold && m.pointerId === hold.pointer) cancelHold();
		};
		window.addEventListener('pointermove', move, true);
		window.addEventListener('pointerup', end, true);
		window.addEventListener('pointercancel', end, true);
		const stop = () => {
			window.removeEventListener('pointermove', move, true);
			window.removeEventListener('pointerup', end, true);
			window.removeEventListener('pointercancel', end, true);
		};
		const x = e.clientX;
		const y = e.clientY;
		hold = {
			x,
			y,
			pointer: e.pointerId,
			stop,
			timer: setTimeout(() => {
				const h = hold;
				hold = null;
				h?.stop();
				if (h && !options.disabled) start(row, grip, h.pointer, h.x, h.y);
			}, HOLD_MS)
		};
	}

	// a held touch drag must not scroll the page (non-passive, so it can cancel the scroll)
	const onTouchMove = (e: TouchEvent) => {
		if (drag && e.cancelable) e.preventDefault();
	};
	// the long press on a grip opens no context menu
	const onContextMenu = (e: Event) => {
		if ((e.target as Element | null)?.closest(GRIP) && (hold || drag)) e.preventDefault();
	};

	// ── keyboard ──
	async function onKeyDown(e: KeyboardEvent) {
		if (options.disabled || drag || e.ctrlKey || e.metaKey || e.shiftKey) return;
		const target = e.target as HTMLElement | null;
		const row = target?.closest<HTMLElement>(ROW) ?? null;
		if (!target || !own(row)) return;
		const onGrip = !!target.closest(GRIP) && target.closest(ROW) === row;
		if (!onGrip && target !== row) return;
		if (!e.altKey && !onGrip) return;
		const back = e.key === 'ArrowUp' || e.key === 'ArrowLeft';
		const forth = e.key === 'ArrowDown' || e.key === 'ArrowRight';
		if (!back && !forth) return;
		e.preventDefault();
		e.stopPropagation();
		const list = rows();
		const from = list.indexOf(row);
		const to = from + (back ? -1 : 1);
		if (from < 0 || to < 0 || to >= list.length) return;
		await options.onmove(from, to);
		await tick();
		const moved = rows()[to];
		const focus = moved && (onGrip ? gripOf(moved) : moved);
		focus?.focus();
		say(from, to, list.length);
	}

	node.addEventListener('pointerdown', onPointerDown);
	node.addEventListener('touchmove', onTouchMove, { passive: false });
	node.addEventListener('contextmenu', onContextMenu);
	node.addEventListener('keydown', onKeyDown);

	return {
		update(next) {
			options = next;
		},
		destroy() {
			cancelHold();
			finish(false);
			node.removeEventListener('pointerdown', onPointerDown);
			node.removeEventListener('touchmove', onTouchMove);
			node.removeEventListener('contextmenu', onContextMenu);
			node.removeEventListener('keydown', onKeyDown);
			delete node.dataset.sortable;
		}
	};
}
