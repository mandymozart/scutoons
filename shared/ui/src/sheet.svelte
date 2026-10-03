<script lang="ts">
	import type { Snippet } from 'svelte';
	import CloseButton from './close-button.svelte';

	/**
	 * Modal sheet on the native <dialog> (focus trap, Escape, top layer). `side`: bottom sheet (mobile reader
	 * pattern), right drawer (detail / triage view) or centered dialog. Square, 2px ink border, no shadow.
	 * Closes on Escape, backdrop click and the close button; `onclose` fires after every close.
	 * `side="right"`: the left edge drags the drawer wider – from its normal width up to half the window (owner 2026-10-03:
	 * the pickers overlap the page anyway); arrow keys on the edge, double-click resets; remembered for all right
	 * drawers (`sc-sheet-width`).
	 */
	let {
		open = $bindable(false),
		title,
		side = 'center',
		closeLabel = 'Close',
		resizeLabel = 'Resize',
		onclose,
		footer,
		children
	}: {
		open?: boolean;
		title?: string;
		side?: 'bottom' | 'right' | 'center';
		/** translated label of the close button */
		closeLabel?: string;
		/** translated label of the drag edge (side="right") */
		resizeLabel?: string;
		onclose?: () => void;
		/** sticky action bar at the bottom */
		footer?: Snippet;
		children: Snippet;
	} = $props();

	let dialog: HTMLDialogElement | null = $state(null);

	// ---------- resizable right drawer ----------
	const WIDTH_KEY = 'sc-sheet-width';
	/** px set by dragging; null = the normal width */
	let width = $state<number | null>(null);
	let baseWidth = 0;
	$effect(() => {
		if (side !== 'right') return;
		try {
			const saved = Number(localStorage.getItem(WIDTH_KEY));
			if (saved > 0) width = saved;
		} catch {
			// not remembered
		}
	});
	function measureBase() {
		if (!baseWidth && dialog && width === null) baseWidth = dialog.offsetWidth;
		return baseWidth || 512;
	}
	const clamp = (px: number) => Math.round(Math.min(Math.max(px, measureBase()), Math.max(measureBase(), window.innerWidth / 2)));
	function remember() {
		try {
			if (width === null) localStorage.removeItem(WIDTH_KEY);
			else localStorage.setItem(WIDTH_KEY, String(width));
		} catch {
			// not remembered
		}
	}
	function startResize(event: PointerEvent) {
		event.preventDefault();
		event.stopPropagation();
		measureBase();
		const handle = event.currentTarget as HTMLElement;
		handle.setPointerCapture(event.pointerId);
		const move = (e: PointerEvent) => (width = clamp(window.innerWidth - e.clientX));
		const end = () => {
			handle.removeEventListener('pointermove', move);
			handle.removeEventListener('pointerup', end);
			handle.removeEventListener('pointercancel', end);
			remember();
		};
		handle.addEventListener('pointermove', move);
		handle.addEventListener('pointerup', end);
		handle.addEventListener('pointercancel', end);
	}
	function keyResize(event: KeyboardEvent) {
		const step = event.shiftKey ? 64 : 16;
		const current = width ?? measureBase();
		if (event.key === 'ArrowLeft') width = clamp(current + step);
		else if (event.key === 'ArrowRight') width = clamp(current - step);
		else if (event.key === 'Home') width = null;
		else return;
		event.preventDefault();
		remember();
	}

	$effect(() => {
		if (!dialog) return;
		if (open && !dialog.open) dialog.showModal();
		else if (!open && dialog.open) dialog.close();
	});

	function handleClose() {
		open = false;
		onclose?.();
	}

	function onBackdrop(event: MouseEvent) {
		// a click on the ::backdrop targets the dialog itself
		if (event.target === dialog) dialog?.close();
	}
</script>

<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_noninteractive_element_interactions -->
<dialog
	bind:this={dialog}
	class="sc-sheet side-{side}"
	style:width={side === 'right' && width !== null ? `${width}px` : undefined}
	style:max-width={side === 'right' && width !== null ? 'none' : undefined}
	aria-label={title}
	onclose={handleClose}
	onclick={onBackdrop}
>
	<div class="frame">
		{#if side === 'right'}
			<!-- a focusable separator is a widget (WAI-ARIA window splitter) -->
			<!-- svelte-ignore a11y_no_noninteractive_tabindex, a11y_no_noninteractive_element_interactions -->
			<div
				class="resize"
				role="separator"
				aria-orientation="vertical"
				aria-label={resizeLabel}
				tabindex="0"
				title={resizeLabel}
				onpointerdown={startResize}
				onkeydown={keyResize}
				ondblclick={() => ((width = null), remember())}
			></div>
		{/if}
		<header>
			{#if title}<h2>{title}</h2>{/if}
			<CloseButton class="close" label={closeLabel} onclick={() => dialog?.close()} />
		</header>
		<div class="body">
			{@render children()}
		</div>
		{#if footer}
			<footer>{@render footer()}</footer>
		{/if}
	</div>
</dialog>

<style>
	.sc-sheet {
		box-sizing: border-box;
		padding: 0;
		border: var(--border-width-regular) solid var(--ink);
		border-radius: var(--radius-none);
		background: var(--paper);
		color: var(--ink);
		font-family: var(--font-mono);
		max-width: min(40rem, calc(100vw - 2rem));
		max-height: calc(100dvh - 2rem);
	}
	.sc-sheet::backdrop {
		background: rgb(0 0 0 / 0.4);
	}
	.side-bottom {
		width: 100%;
		max-width: 100%;
		margin: auto 0 0;
		border-width: var(--border-width-regular) 0 0;
	}
	.side-right {
		height: 100dvh;
		max-height: 100dvh;
		width: min(32rem, 100vw);
		margin: 0 0 0 auto;
		border-width: 0 0 0 var(--border-width-regular);
	}
	.resize {
		position: absolute;
		z-index: 5;
		top: 0;
		bottom: 0;
		left: 0;
		width: 0.75rem;
		cursor: col-resize;
		touch-action: none;
	}
	.resize:hover,
	.resize:focus-visible {
		background: linear-gradient(var(--ink), var(--ink)) left / var(--border-width-focus) 100% no-repeat;
		outline: none;
	}
	@media (max-width: 767px) {
		.resize {
			display: none;
		}
	}
	.frame {
		position: relative;
		display: flex;
		flex-direction: column;
		max-height: inherit;
		height: 100%;
	}
	header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: var(--space-1);
		padding: var(--space-075) var(--space-1);
		border-bottom: var(--border-width-regular) solid var(--ink);
	}
	h2 {
		margin: 0;
		font-size: var(--type-label-size);
		line-height: var(--type-label-line);
		font-weight: var(--font-weight-regular);
		text-transform: uppercase;
		letter-spacing: var(--tracking-label);
	}
	/* the close cross (CloseButton): right, level with the title */
	header :global(.close) {
		margin: 0 0 0 auto;
	}
	.body {
		padding: var(--space-1);
		/* dialogs scroll vertically only; content must wrap to the sheet width */
		overflow-x: hidden;
		overflow-y: auto;
		min-width: 0;
		flex: 1;
	}
	footer {
		position: sticky;
		bottom: 0;
		display: flex;
		justify-content: flex-end;
		gap: var(--space-05);
		padding: var(--space-075) var(--space-1);
		background: var(--paper);
		border-top: var(--border-width-regular) solid var(--ink);
	}
</style>
