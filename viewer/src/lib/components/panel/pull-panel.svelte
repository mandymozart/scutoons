<script lang="ts">
	import type { Snippet } from 'svelte';
	import { tick } from 'svelte';
	import { onClickOutside } from 'runed';
	import { asset } from '$lib/utils/cdn.ts';
	import { freezeScroll } from '$lib/visual-novel/freeze-scroll.ts';

	/**
	 * The reader's pull panel (owner 2026-10-10: the inventory's panel "recycled in a component", "nach unten ziehen
	 * weg"): the dark sheet that slides up from the bottom edge with the inventory's bouncy curve, the demon-saw vector
	 * pushed behind its top edge. Used by the inventory (Items / Avatar tabs) and the chapter list.
	 *
	 * Pulling it down closes it: the header (grab handle + the caller's row) follows a pointer (mouse, pen, touch), the
	 * content follows a finger once it is scrolled to its top (the overscroll). Let go past a third of its height (at
	 * most 10rem) or with a flick → the closing animation runs on from where it is; short of that → it springs back.
	 * Escape (unless `escape` is false), the top button and a tap outside close it too. While open, the page behind
	 * doesn't scroll (freeze-scroll.ts – nests with the story's own freeze); the panel's content still does.
	 */
	let {
		open,
		onclose,
		label,
		id,
		escape = true,
		keepOpen = () => false,
		testid,
		header,
		footer,
		children
	}: {
		open: boolean;
		onclose: () => void;
		/** the dialog's accessible name */
		label: string;
		/** for the top button's aria-controls */
		id?: string;
		/** Escape closes it (the chapter list: no – owner 2026-10-02) */
		escape?: boolean;
		/** true while something opened from the panel lives outside it (camera, claim sheet): outside taps / Escape are theirs */
		keepOpen?: () => boolean;
		testid?: string;
		header?: Snippet;
		footer?: Snippet;
		children: Snippet;
	} = $props();

	let panel = $state<HTMLDivElement>();
	let body = $state<HTMLDivElement>();

	const remPx = () => parseFloat(getComputedStyle(document.documentElement).fontSize) || 16;

	// ── the drag: an offset (px from the pointer, written as rem) while a pointer / finger pulls ──────────────────
	let dragging = $state(false);
	let offset = $state(0);
	let base = 0;
	let samples: { y: number; t: number }[] = [];

	function begin(y: number) {
		dragging = true;
		base = y;
		offset = 0;
		samples = [{ y, t: performance.now() }];
	}
	function follow(y: number) {
		offset = Math.max(0, y - base);
		const t = performance.now();
		samples.push({ y, t });
		while (samples.length > 2 && t - samples[0].t > 100) samples.shift();
	}
	function release() {
		if (!dragging) return;
		const first = samples[0];
		const last = samples[samples.length - 1];
		const velocity = last && first && last.t > first.t ? (last.y - first.y) / (last.t - first.t) : 0; // px per ms
		const far = offset > Math.min((panel?.offsetHeight ?? 0) / 3, 10 * remPx());
		const flick = velocity > 0.5 && offset > remPx();
		dragging = false;
		offset = 0;
		// the class takes over: from where the finger left it on to closed (or back to open)
		if (far || flick) onclose();
	}

	// the header: pointer events (touch-action: none) – a drag starts after half a rem downwards, a tap stays a tap
	let pointer = -1;
	let startX = 0;
	let startY = 0;
	let swallowClick = false;
	function headerDown(e: PointerEvent) {
		if (e.button !== 0 || !open) return;
		pointer = e.pointerId;
		startX = e.clientX;
		startY = e.clientY;
	}
	function headerMove(e: PointerEvent) {
		if (e.pointerId !== pointer) return;
		if (!dragging) {
			const dy = e.clientY - startY;
			const dx = e.clientX - startX;
			if (Math.abs(dy) < remPx() / 2 && Math.abs(dx) < remPx() / 2) return;
			// sideways or upwards: not a pull
			if (dy <= 0 || Math.abs(dx) > dy) {
				pointer = -1;
				return;
			}
			begin(e.clientY);
			try {
				(e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);
			} catch {
				// a pointer the browser no longer knows: the moves still come while it stays over the header
			}
		}
		follow(e.clientY);
	}
	function headerUp(e: PointerEvent) {
		if (e.pointerId !== pointer) return;
		pointer = -1;
		if (!dragging) return;
		swallowClick = true; // the click after a drag is no tap on a tab
		setTimeout(() => (swallowClick = false), 0);
		release();
	}
	function headerClick(e: MouseEvent) {
		if (!swallowClick) return;
		e.preventDefault();
		e.stopPropagation();
		swallowClick = false;
	}

	// the content: touch events (not passive) – a finger pulling down while it is scrolled to its top drags the panel
	$effect(() => {
		const el = body;
		if (!el) return;
		let tracking = false;
		let x0 = 0;
		let y0 = 0;
		const start = (e: TouchEvent) => {
			tracking = e.touches.length === 1 && open;
			x0 = e.touches[0]?.clientX ?? 0;
			y0 = e.touches[0]?.clientY ?? 0;
		};
		const move = (e: TouchEvent) => {
			if (!tracking) return;
			const t = e.touches[0];
			if (!t || e.touches.length > 1) {
				tracking = false;
				if (dragging) release();
				return;
			}
			if (!dragging) {
				const dy = t.clientY - y0;
				const dx = t.clientX - x0;
				if (dy === 0 && dx === 0) return;
				// the content scrolls (or scrolled already – the browser owns the gesture) / sideways: not a pull
				if (dy < 0 || el.scrollTop > 0 || Math.abs(dx) > dy || !e.cancelable) {
					tracking = false;
					return;
				}
				begin(t.clientY);
			}
			e.preventDefault();
			follow(t.clientY);
		};
		const end = () => {
			tracking = false;
			release();
		};
		el.addEventListener('touchstart', start, { passive: true });
		el.addEventListener('touchmove', move, { passive: false });
		el.addEventListener('touchend', end);
		el.addEventListener('touchcancel', end);
		return () => {
			el.removeEventListener('touchstart', start);
			el.removeEventListener('touchmove', move);
			el.removeEventListener('touchend', end);
			el.removeEventListener('touchcancel', end);
		};
	});

	// ── open / close: the page holds still, the focus goes in and comes back to the button that opened it ─────────
	let opener: HTMLElement | null = null;
	$effect(() => {
		if (!open) return;
		const active = document.activeElement;
		opener = active instanceof HTMLElement && active !== document.body && !panel?.contains(active) ? active : null;
		tick().then(() => body?.focus({ preventScroll: true }));
		const unfreeze = freezeScroll();
		return () => {
			unfreeze();
			dragging = false;
			offset = 0;
			// only if the focus would otherwise be lost (it was in the panel); a tap elsewhere keeps its own focus
			const now = document.activeElement;
			if (opener?.isConnected && (!now || now === document.body || panel?.contains(now))) opener.focus({ preventScroll: true });
			opener = null;
		};
	});

	onClickOutside(
		() => panel,
		() => {
			if (open && !keepOpen()) onclose();
		}
	);

	$effect(() => {
		if (!open || !escape) return;
		const onKey = (e: KeyboardEvent) => {
			if (e.key !== 'Escape' || e.defaultPrevented || keepOpen()) return;
			const target = e.target as HTMLElement | null;
			if (target?.closest('input, textarea, select, [contenteditable]')) return;
			onclose();
		};
		window.addEventListener('keydown', onKey);
		return () => window.removeEventListener('keydown', onKey);
	});

	const pull = $derived(dragging ? `translateY(${offset / remPx()}rem)` : null);
</script>

<div
	class="pull-panel"
	class:visible={open}
	class:dragging
	style:transform={pull}
	bind:this={panel}
	{id}
	role="dialog"
	aria-label={label}
	aria-hidden={!open}
	inert={!open}
	data-testid={testid}
>
	<img class="frame" src={asset(`catpope/vectors/vector-demon-saw-1.svg`)} alt="" />

	<!-- svelte-ignore a11y_no_static_element_interactions – the pull is a pointer gesture; Escape / the button / outside close it for keyboards -->
	<div
		class="pull-header"
		onpointerdown={headerDown}
		onpointermove={headerMove}
		onpointerup={headerUp}
		onpointercancel={headerUp}
		onclickcapture={headerClick}
		data-testid="pull-panel-header"
	>
		<span class="grip" aria-hidden="true"></span>
		{@render header?.()}
	</div>

	<div class="pull-body" bind:this={body} tabindex="-1" data-testid="pull-panel-body">
		{@render children()}
	</div>

	{#if footer}
		<div class="pull-footer">{@render footer()}</div>
	{/if}
</div>

<style>
	/* the whole panel is dark (owner 2026-09-30): the tokens are inverted here, so everything inside follows */
	.pull-panel {
		--ink: #ffffff;
		--paper: #000000;
		--text-muted: #a6a6a6;
		--border-muted: #555555;
		--pull-curve: linear(0, 0.996 33.2%, 0.875 39.4%, 0.84 44.4%, 0.999 57.8%, 0.946 65%, 1 74.6%, 1);
		position: fixed;
		left: 0;
		right: 0;
		bottom: 0;
		z-index: 1000;
		background: black;
		color: var(--ink);
		border-top: var(--border-width-regular) solid black;
		max-height: 85dvh;
		display: flex;
		flex-direction: column;
		transform: translate(0, 140%);
		transition: transform 0.3s var(--pull-curve);
		touch-action: pan-y;
	}
	.pull-panel.visible {
		transform: translate(0, 0);
	}
	/* following the finger: no transition (the curve runs again from where it is let go) */
	.pull-panel.dragging {
		transition: none;
	}

	/* the vector pushed behind the panel's top edge, glowing */
	.frame {
		position: absolute;
		pointer-events: none;
		top: -1rem;
		right: -0.625rem;
		height: 100vw;
		transform: rotate(90deg);
		transform-origin: top;
		transition: transform 1s var(--pull-curve);
		filter: drop-shadow(0 0 0.8rem var(--primary-color));
	}
	.visible .frame {
		transform: rotate(90deg) translateY(-0.3125rem);
	}

	.pull-header {
		position: relative;
		z-index: 1;
		flex: none;
		background: black;
		padding-top: 1rem;
		touch-action: none;
		cursor: grab;
		user-select: none;
		-webkit-user-select: none;
	}
	.dragging .pull-header {
		cursor: grabbing;
	}
	/* the grip: says "pull me" – a short muted bar centred at the top */
	.grip {
		position: absolute;
		top: 0.375rem;
		left: 50%;
		width: 2.5rem;
		height: 0.25rem;
		margin-left: -1.25rem;
		background: var(--border-muted);
	}

	.pull-body {
		position: relative;
		z-index: 1;
		flex: 1 1 auto;
		min-height: 8rem;
		background: var(--paper);
		color: var(--ink);
		padding: 1rem calc(1rem + env(safe-area-inset-right)) calc(1rem + env(safe-area-inset-bottom)) calc(1rem + env(safe-area-inset-left));
		overflow-y: auto;
		overscroll-behavior: contain;
		outline: none;
	}
	.pull-footer {
		position: relative;
		z-index: 1;
		flex: none;
		background: var(--paper);
		border-top: var(--border-width-hairline) solid var(--border-muted);
		padding-bottom: env(safe-area-inset-bottom);
	}
	/* with a footer the safe area is the footer's */
	.pull-body:has(+ .pull-footer) {
		padding-bottom: 1rem;
	}

	@media (prefers-reduced-motion: reduce) {
		.pull-panel,
		.frame {
			transition: none;
		}
	}
</style>
