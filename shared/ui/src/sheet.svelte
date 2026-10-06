<script lang="ts">
	import type { Snippet } from 'svelte';
	import CloseButton from './close-button.svelte';
	import Divider from './divider.svelte';
	import { resizable } from './resizable.svelte.ts';

	/**
	 * Modal sheet on the native <dialog> (focus trap, Escape, top layer). `side`: bottom sheet (mobile reader
	 * pattern), right drawer (detail / triage view) or centered dialog. Square, 2px ink border, no shadow.
	 * Closes on Escape, backdrop click and the close button; `onclose` fires after every close.
	 * `side="right"`: the left edge (the one `Divider`, agents/DOCKING.md) drags the drawer wider – from its normal width
	 * up to half the window (owner 2026-10-03: the pickers overlap the page anyway); arrow keys on the edge, Home /
	 * double-click resets; remembered per kind of drawer (`widthKey`: `sc-sheet-width:<kind>`, default one width for
	 * all). It stays modal: a choice or a side task, not a docked panel.
	 */
	let {
		open = $bindable(false),
		title,
		side = 'center',
		closeLabel = 'Close',
		resizeLabel = 'Resize',
		widthKey = '',
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
		/** side="right": the drawer's kind – its width is remembered per kind ('' = the width all drawers share) */
		widthKey?: string;
		onclose?: () => void;
		/** sticky action bar at the bottom */
		footer?: Snippet;
		children: Snippet;
	} = $props();

	let dialog: HTMLDialogElement | null = $state(null);

	// ---------- resizable right drawer (resizable.svelte.ts) ----------
	let baseWidth = 0;
	function measureBase() {
		if (!baseWidth && dialog && size.width === null) baseWidth = dialog.offsetWidth;
		return baseWidth || 512;
	}
	const size = resizable({ storageKey: () => (widthKey ? `sc-sheet-width:${widthKey}` : 'sc-sheet-width'), edge: 'left', anchor: () => window.innerWidth, min: measureBase, max: () => window.innerWidth / 2 });
	$effect(() => {
		if (side === 'right') size.restore();
	});
	const width = $derived(size.width);

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
			<!-- inside the drawer (the modal dialog clips what lies outside it) -->
			<Divider {size} edge="left" at="calc(100% - 0.375rem)" label={resizeLabel} />
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
